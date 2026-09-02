---
status: draft
packet: wipe-tower-bed-exclude-area
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P04; re-authored under the map's Authoring rules 1–6)
context_cost_estimate: L
tier: C
---

# Packet Contract: wipe-tower-bed-exclude-area

Re-authored in place (number and slug retained) with explicit user approval. The prior revision wired `bed_exclude_area` only to the wipe-tower rectangle's corner check and recorded canonical's real feature — **object-footprint validation** — as a gap. The map's ticket-11 entry carries a ⚠ correction requiring that gap to be built at the port's validation seam. This revision builds it, as a module.

## Goal

Give this port the pre-slice print-validation seam it does not have, and make `bed_exclude_area` drive it. A new core module — `print-validator`, at the earliest module-hostable stage `PrePass::MeshAnalysis` — reads the exclusion polygon, pre-filters objects by their XY bounds, probes the excluded region with the host's existing `raycast-z-down` service, and fails the slice fatally when an object occupies excluded space, mirroring canonical `Print::validate`'s collision-risk rejection. The existing `wipe-tower` corner check gains the same exclusion test as a second decision point, because the tower is generated after slicing and is invisible to any pre-slice validator.

## Scope Boundaries

In scope: a new core module `modules/core-modules/print-validator/` (crate, `print-validator.toml`, `wit-guest/`, `src/lib.rs`, tests, guest `.wasm`) at `PrePass::MeshAnalysis`; its registration surface (root `Cargo.toml` workspace member, `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`, `crates/pnp-cli/Cargo.toml` passthrough features, the manifest-ingestion core-module count); `modules/core-modules/wipe-tower/{wipe-tower.toml, src/lib.rs, tests/bed_bounds_tdd.rs}` for the tower-side check; one new runtime integration test plus its aggregator registration; one runtime leakage arm; and the generated `docs/15_config_keys_reference.md`.

Out of scope: **any WIT change** — the `mesh-analysis` interface's `run` takes `objects: list<object-id>` and the module works entirely through the already-exported `slicer:common/host-services` (`object-bounds`, `raycast-z-down`); no IR schema bump; no new `ResolvedConfig` field (the key rides the existing `extensions` overflow bucket into `to_config_map`); canonical's convex-hull footprint (DIV-1); canonical's secondary consumers — `GCode.cpp::get_path_of_change_filament`'s 4-point cutter form, `GCodeProcessor.cpp::apply_config`'s viewer copy, `TimelapsePosPicker.cpp::construct_printable_area_by_printer`'s subtractive use — each of which needs its own decision point and is listed in `requirements.md` § Returned to Queue; the `printable_height` / `extruder_printable_area` / `extruder_clearance_*` siblings (P18/P19); and `ORCA_CONFIG_PADDING`.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 (packet numbering — re-derived from disk at authoring), 05 (P04 membership), 11 (the packet ticket and its ⚠ correction), 100 (the Orca point-string value-format adaptation this packet's reader reuses).
- **Shared ledger fact — the core-module count.** `manifest_ingestion_tdd`'s `core_modules_directory_is_discoverable_and_all_load` asserts an exact core-module count (23 at authoring; **re-derive with `ls -d modules/core-modules/*/ | wc -l` at implementation time**). Draft packet `254b-prime-tower-interface-and-ramming` also adds a module. Whichever lands second increments from whatever the tree says then — never from a number frozen here.
- Ordering, not gating: packets `254a` / `254b` / `255` share the `wipe-tower` manifest and `run_finalization`. Any order works; the manifest assertion re-derives the rest of the key set from disk.
- Unblocks: wayfinder ticket 11's resolution; gives P18/P19's `printable_height` family a validation home that already exists.
- Activation blockers: none. No `[BLOCK]` in `design.md`.

## Acceptance Criteria

- **AC-1. Given** the new `modules/core-modules/print-validator/print-validator.toml`, **when** it is parsed, **then** `[module] id = "com.core.print-validator"`, `[stage] id = "PrePass::MeshAnalysis"`, `[ir-access] reads = ["MeshIR"]` with `writes = []`, `[claims] holds = []` (the module displaces no built-in — `PrePass::MeshAnalysis`'s host built-in `host:mesh_analysis` still commits `SurfaceClassification`, and this module commits nothing), and `[config.schema]` declares exactly two keys: `bed_exclude_area` (`type = "float-list"`, **no `default`**, no `min`/`max`, `display = "Excluded bed area"`, `group = "Printer"`, `advanced = true`) and `printable_area` (`type = "float-list"`, `required = true`, mirroring the `wipe-tower` declaration); and the scheduler's core-module discovery count is exactly one higher than the pre-packet count re-derived from disk. | `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `bed_exclude_area = [0.0, 0.0, 20.0, 0.0, 20.0, 20.0, 0.0, 20.0]` (non-default; the key is absent by default) and an object whose geometry occupies `(10, 10)`, **when** `run_mesh_analysis` runs, **then** it returns `ModuleError::fatal` whose message names the object id, the key `bed_exclude_area`, and the colliding sample point, and the slice aborts as `PipelineError::Prepass` carrying `PrepassExecutionError::FatalModule { stage_id: "PrePass::MeshAnalysis", .. }` — not a degraded warning. | `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the same non-default exclusion polygon and an object placed entirely clear of it, **when** `run_mesh_analysis` runs, **then** it returns `Ok(())`, commits nothing to the blackboard, and the resulting G-code is byte-identical to the same slice run with `bed_exclude_area` absent. | `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** an object overlapping the exclusion polygon, **when** the module runs, **then** the probe order is observable: the object is admitted to probing only because its `object-bounds` XY rectangle overlaps the exclusion polygon's bounding rectangle, and probing evaluates only grid points **strictly inside** the exclusion polygon (even-odd test), spaced `1.0` mm apart in both axes, submitted as one `slicer_sdk::host_batch::raycast_z_down_batch` call (single-point `slicer_sdk::host::raycast_z_down` is the fallback) with `start_z` above the object's `object_bounds` max Z; the first `Some(_)` result is the reported collision. An object whose bounds rectangle does not overlap the exclusion rectangle produces **zero** raycasts — asserted by counting calls on the SDK's `test_support::mock_host`. | `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `bed_exclude_area` supplied in Orca's 3MF point-string form `["0x0", "20x0", "20x20", "0x20"]` (non-default), **when** the module parses it, **then** it expands to the same interleaved polygon as AC-2 and rejects the same object — the ticket-100 `parse_orca_point_string` path carries this key with no host-side change. | `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a degenerate value — absent, empty, a single point (canonical's own default `{ Vec2d(0,0) }`), or any list with fewer than 3 vertices or an odd float count — **when** the module runs, **then** it performs no probing and returns `Ok(())`: a degenerate polygon excludes nothing, matching canonical `get_bed_excluded_area`, and a malformed value never fails a slice. | `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the same non-default exclusion polygon and a wipe tower whose footprint corner falls at `(10, 10)`, **when** `run_finalization` runs, **then** it returns `ModuleError::fatal(3, …)` whose message names `bed_exclude_area` and the offending corner; with the tower placed clear of the polygon it returns `Ok`; and with the key absent the check is skipped and behaviour is unchanged from today. This is a **second** decision point, not a duplicate: the tower is generated at `PostPass::LayerFinalization` and does not exist when the pre-slice validator runs. | `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** `bed_exclude_area` appears under owners `print-validator` and `wipe-tower`, the Orca-deviations table gains **no** row for it (the key carries no numeric default, so the generator's comparator has no comparand — the same non-row `printable_area` renders), and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'bed_exclude_area' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** a `LoadedModule` whose manifest declares neither key, **when** `bind_module_config_view` binds it against a source map containing `bed_exclude_area`, **then** `ConfigView::get("bed_exclude_area")` is `None` — declaring the key on two modules leaks it to no third. The arm lands in the already-registered `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`. | `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** no `bed_exclude_area` in the config at all, **when** a full slice runs with the new module registered, **then** the G-code is byte-identical to the pre-packet baseline and the module issues zero raycasts — registering a validator costs nothing when nothing is excluded. | `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest guard, **when** either declared key is removed from `print-validator.toml` or its `type` / `default` / `advanced` drifts from AC-1's table, **or** the stage id changes, **then** the guard fails naming the offending field. | `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — this packet adds a **new guest**, so `discover_guests` (`xtask/src/build_guests.rs`) must pick it up and the check must return exit `0` before closure. Exit `3` is `wasm-tools` missing — an infrastructure error, not clean.

## Authoritative Docs

- `docs/04_host_scheduler.md` § stage order and § Claim Resolution — where `PrePass::MeshAnalysis` sits and what a module at it may assume.
- `docs/03_wit_and_manifest.md` § Host-Boundary Access Enforcement (Normative) and the stage-declaration sections — the new manifest's contract and AC-N1.
- `docs/01_system_architecture.md` § Claim System — read for `design.md`'s mechanism check (why this module holds no claim).
- `docs/08_coordinate_system.md` — the module works in plain mm floats; the exclusion polygon and the probe grid are mm.
- `docs/15_config_keys_reference.md` — generated by `cargo xtask gen-config-docs`, never hand-edited.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` module-config-keys table — regenerated; verification grep in AC-8.
- `docs/04_host_scheduler.md` — gains one sentence noting that `PrePass::MeshAnalysis` now hosts a guest validator alongside the host built-in, and that a fatal module error there aborts the slice. Verification: `rg -q 'print-validator' docs/04_host_scheduler.md`.
- No prose doc claims the port validates the model against the bed today (`bed_exclude_area` has zero source occurrences at authoring); if implementation finds one (`rg -n 'exclude' docs/*.md`), it names this packet in its amendment.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `..\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`; re-derive the absolute path on first use):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` (`bed_exclude_area`: `coPoints`, `comAdvanced`, default `Vec2d(0,0)`, no bounds) and `get_bed_excluded_area` (all configured points become **one** counter-clockwise polygon; no rectangle pairing).
- `src/libslic3r/Print.cpp` — `Print::validate` → `layered_print_cleareance_valid` / `sequential_print_clearance_valid`: each model volume's 2D convex hull intersected with the exclude polygon, fatal `"<object> is too close to exclusion area, there may be collisions when printing."`. The wipe tower is never tested against the key in canonical.
- `src/libslic3r/GCode.cpp` — `get_path_of_change_filament` (4-point cutter-area form); `src/libslic3r/GCode/GCodeProcessor.cpp` — `apply_config`; `src/libslic3r/GCode/TimelapsePosPicker.cpp` — `construct_printable_area_by_printer`. Evidence for the three returned consumers only; not imitated.
<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
