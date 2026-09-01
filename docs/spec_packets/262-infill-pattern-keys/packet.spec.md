---
status: draft
packet: infill-pattern-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P08)
context_cost_estimate: M
---

# Packet Contract: infill-pattern-keys

## Goal

Wire the four angle/multiline keys (`solid_infill_direction`, `sparse_infill_rotate_template`, `solid_infill_rotate_template`, `fill_multiline`) into the infill modules' existing decision points (the solid-role angle read, the per-layer angle computation, the sparse scan-line emission) with canonical defaults so the default path is byte-identical, declare the three pattern/gap-fill keys (`sparse_infill_pattern`, `internal_solid_infill_pattern`, `gap_fill_target`) with-gap (pattern selection is module identity in this port; no fill-side gap fill exists), and correct the `sparse_infill_pattern` CONFIG_BLOCK padding twin to the canonical default.

## Scope Boundaries

The packet touches the three infill-module manifests (`rectilinear-infill.toml` 7 tables, `gyroid-infill.toml` 7 tables, `lightning-infill.toml` 3 tables — 17 net-new `[config.schema]` tables), the rectilinear and gyroid module sources (wired reads: solid-role angle, per-layer rotation templates, sparse multiline), one value correction in the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`), the module/scheduler/runtime test suites, and the generated `docs/15_config_keys_reference.md`. It does not implement pattern dispatch (the port's pattern is module identity — the host `*_fill_holder` resolution), fill-side gap fill (canonical `FillBase.cpp::Fill::_create_gap_fill`), the template metalanguage (joints/repeats/units — only the comma-separated list form is wired), or gyroid/lightning multiline (curve offsetting is Tier B+ geometry).

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 262 derived from disk at authoring time); ticket 05 (packet-list P08 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence: 4 keys wired, 3 re-adjudicated declared-with-gap); ticket 105 (infill-angle rename — resolved; the wired keys build on the renamed `infill_direction`); ticket 107 (infill duplicate collapses — resolved; the manifests' density/speed reads are the multiline/angle context).
- Ordering, not gating: packets 253–261 precede this packet in the queue but touch different modules (259 fuzzy-skin is a different module). P09/P10 (tickets 16/17) touch the same infill modules — same-module merge churn, not a hard gate; this packet lands first.
- Unblocks: wayfinder ticket 15's resolution; nothing downstream gates on this packet specifically.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the three infill-module manifests, **when** their `[config.schema]` tables are parsed, **then** `rectilinear-infill.toml` and `gyroid-infill.toml` each carry exactly these 7 tables and `lightning-infill.toml` exactly these 3 — `fill_multiline` (`type = "int"`, `default = 1`, `min = 1`, `max = 10`, `display = "Fill Multiline"`, `group = "Infill"`), `sparse_infill_pattern` (`type = "enum"`, `values` = the 26 canonical InfillPattern strings in canonical order, `default = "crosshatch"`, `display = "Sparse Infill Pattern"`, `group = "Infill"`), `sparse_infill_rotate_template` (`type = "string"`, `default = ""`, `display = "Sparse Infill Rotation Template"`, `group = "Infill"`), `internal_solid_infill_pattern` (`type = "enum"`, `values` = the 8 canonical top-fill strings, `default = "monotonic"`, `display = "Internal Solid Infill Pattern"`, `group = "Infill"`), `solid_infill_direction` (`type = "float"`, `default = 45.0`, `min = 0.0`, `max = 360.0`, `display = "Solid Infill Direction"`, `group = "Infill"`), `solid_infill_rotate_template` (`type = "string"`, `default = ""`, `display = "Solid Infill Rotation Template"`, `group = "Infill"`), `gap_fill_target` (`type = "enum"`, `values = ["everywhere", "topbottom", "nowhere"]`, `default = "nowhere"`, `display = "Gap Fill Target"`, `group = "Infill"`) — with lightning carrying only the three sparse keys (`fill_multiline`, `sparse_infill_pattern`, `sparse_infill_rotate_template`), each table with a `description` field recording the disposition (wired consumer or decision-point gap). | `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a rectilinear and a gyroid run over the square fixture, **when** the 7 keys are set explicitly to their canonical defaults (`fill_multiline = 1`, `sparse_infill_pattern = "crosshatch"`, `sparse_infill_rotate_template = ""`, `internal_solid_infill_pattern = "monotonic"`, `solid_infill_direction = 45`, `solid_infill_rotate_template = ""`, `gap_fill_target = "nowhere"`), **then** the emitted `InfillIR` paths are byte-identical to the same run with the 7 keys absent — the wired keys are default-path identity and the declared-with-gap keys are unread. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a rectilinear run with `infill_direction = 45` and `solid_infill_direction = 90`, **when** the region has both a sparse area and a top solid area, **then** the sparse paths' direction is 45° and the solid paths' direction is 90° (the solid-role angle read is wired; sparse keeps `infill_direction`). | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a rectilinear run with `sparse_infill_rotate_template = "0,90"` (and empty `solid_infill_rotate_template`), **when** `run_infill` is invoked for layer indices 0, 1, 2, **then** the sparse paths' directions are 0°, 90°, 0° (comma-separated list cycled by layer index) and the solid paths stay at the base angle; with `solid_infill_rotate_template = "0,90"` the solid paths cycle the same way. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a rectilinear run with `fill_multiline = 3` over the square fixture, **when** the sparse area is scanned, **then** the sparse path count is 3× the `fill_multiline = 1` count and the solid path count is unchanged (multiline applies to sparse only, canonical `Fill.cpp::Layer::make_fills` `erInternalInfill` branch); with `fill_multiline = 1` the count matches the pre-packet baseline. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the scheduler's config bounds index loaded from the real `rectilinear-infill.toml` manifest, **when** `fill_multiline = 0` or `11` (outside [1,10]) or `solid_infill_direction = -1` or `361` (outside [0,360]) is resolved, **then** resolution rejects the value with the numeric `OutOfRange` error; `fill_multiline = "abc"` is rejected with `TypeMismatch`; and `sparse_infill_pattern = "bogus"` (not in the 26-value list) is rejected as an unknown enum value. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a slice run over the square fixture, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block carries `; sparse_infill_pattern = crosshatch` (padding twin corrected from `"grid"` — canonical default is `crosshatch`) and `; gap_fill_target = nowhere` (padding twin unchanged), and carries zero `fill_multiline` / `internal_solid_infill_pattern` / `solid_infill_direction` / `solid_infill_rotate_template` / `sparse_infill_rotate_template` lines; with an explicit `sparse_infill_pattern = "gyroid"` the line `; sparse_infill_pattern = gyroid` appears exactly once (padding twin suppressed by the emitted-key dedup in `serialize_config_block`). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key table carries all 7 keys (17 rows: `rectilinear-infill` 7, `gyroid-infill` 7, `lightning-infill` 3), and the deviations block (`<!-- BEGIN GENERATED: orca-deviations ... -->` … `<!-- END GENERATED: orca-deviations -->`) still contains exactly 26 data rows (pre-packet count, measured at authoring — the two numeric keys' declared defaults 1/45 match canonical, and the enum/string defaults never enter the numeric comparison map in `render_deviations`). | `cargo xtask gen-config-docs --check && rg -q 'fill_multiline' docs/15_config_keys_reference.md && rg -q 'solid_infill_direction' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "26" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** any of the 17 tables is removed from its manifest or its `type`/`default`/`min`/`max`/`values`/`display`/`group` drifts from AC-1's exact table, **then** the guard fails naming the offending key and manifest. | `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the deliberate omission ruling, **when** `lightning-infill.toml` is parsed, **then** it does NOT declare the four solid keys (`internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `gap_fill_target`) — lightning holds only `claim:sparse-fill` and has no solid-fill surface; the omission is pinned so a future packet that wires solid fill for lightning must update the guard. | `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifests and module sources are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape (`[config.schema]` type table: `bool`/`int`/`float`/`string`/`enum` with `values`; `description` field); delegated SUMMARY if a worker needs the contract.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; the multiline spacing math converts via `mm_to_units` (delegated SUMMARY).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains 17 rows for the 7 keys (owner columns `rectilinear-infill` ×7, `gyroid-infill` ×7, `lightning-infill` ×3); the generated deviations block is unchanged (26 data rows — the two numeric declared defaults 1/45 match canonical, enum/string defaults are outside the numeric comparison). The doc has no per-module subheadings, so verification is key-presence + row-count, not headings. Verification greps: `rg -q 'fill_multiline' docs/15_config_keys_reference.md`, `rg -q 'sparse_infill_pattern' docs/15_config_keys_reference.md`, `rg -q 'sparse_infill_rotate_template' docs/15_config_keys_reference.md`, `rg -q 'internal_solid_infill_pattern' docs/15_config_keys_reference.md`, `rg -q 'solid_infill_direction' docs/15_config_keys_reference.md`, `rg -q 'solid_infill_rotate_template' docs/15_config_keys_reference.md`, `rg -q 'gap_fill_target' docs/15_config_keys_reference.md`, and the AC-8 deviation-block probe. The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 7), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 7 keys (`fill_multiline` coInt default 1 min 1 max 10; `sparse_infill_pattern` coEnum 26 values default `crosshatch`; `sparse_infill_rotate_template` coString default ""; `internal_solid_infill_pattern` coEnum 8 top-fill values default `monotonic`; `solid_infill_direction` coFloat default 45 min 0 max 360; `solid_infill_rotate_template` coString default ""; `gap_fill_target` coEnum everywhere/topbottom/nowhere default `nowhere`). Authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` (the `erInternalInfill` multiline branch and the sparse/solid angle branches: `infill_direction` + `sparse_infill_rotate_template` for sparse, `solid_infill_direction` + `solid_infill_rotate_template` for solid, `top_layer_direction`/`bottom_layer_direction` overrides), `group_fills` (pattern assignment per surface type: `sparse_infill_pattern` default, `internal_solid_infill_pattern` for `is_solid_infill()` at density 100), `calculate_infill_rotation_angle` (the template parser: empty → base angle; non-empty → comma-separated list cycled by layer id, or the metalanguage).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `multiline_fill` (offset copies at multiples of spacing; odd N: center + rings at i·spacing, even N: 0.5·spacing + i·spacing), `Fill::_create_gap_fill` (the `gap_fill_target` gate: `nowhere` → no gap fill; `topbottom` → internal-solid surfaces excluded; `everywhere` → all solid surfaces), `Fill::new_from_type` (pattern → filler class mapping).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `fill_surface_by_multilines` (base `line_spacing = spacing * multiline / density`, polygon pre-expanded by `0.5 * multiline * spacing`, then `multiline_fill`).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `combine_infill` (sparse density 100% → `internal_solid_infill_pattern`; else `sparse_infill_pattern`).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
