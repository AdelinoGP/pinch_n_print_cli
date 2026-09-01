---
status: draft
packet: infill-pattern-holder-mapping
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P08; re-authored under the map's Authoring rules 1–6 and split, 210a/210b precedent)
context_cost_estimate: L
---

# Packet Contract: infill-pattern-holder-mapping

## Goal

Turn OrcaSlicer's two infill *pattern* enums into this port's claim-holder mechanism, and make gap fill a real fill-side pass. `sparse_infill_pattern` becomes a value→`claim:sparse-fill` holder mapping resolved in the scheduler's global config resolution; `internal_solid_infill_pattern` becomes the same mapping onto `claim:top-fill`; both ship their canonical defaults as real modules (`crosshatch-infill`, `monotonic-infill`). `gap_fill_target` gates a new `Layer::InfillPostProcess` module that emits medial-axis gap fill in the region-scope canonical selects. Every unshipped enum value is rejected by name rather than silently accepted.

## Scope Boundaries

The packet creates three module crates (`crosshatch-infill`, `monotonic-infill`, `infill-gap-fill`) with their manifests, guest wrappers, and tests; adds a pattern→holder derivation to `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`); registers the three modules in the workspace, the integrated registry, and `pnp-cli`'s passthrough features; adds one claim row to `docs/03_wit_and_manifest.md` and the two selection keys to `docs/04_host_scheduler.md` §Claim Resolution. It does **not** add a WIT interface, bump an IR schema, or add a `ResolvedConfig` field — the derivation writes the existing `sparse_fill_holder` / `top_fill_holder` fields, and `ExtrusionRole::GapFill` already exists in `crates/slicer-ir/src/slice_ir.rs`. The one edit outside those surfaces is a `GapFill` verbatim passthrough in `infill-linker` (`modules/core-modules/infill-linker/src/lib.rs`), without which the linker would clip and short-filter gap-fill paths whenever it ran after `infill-gap-fill`. It does not port canonical's ant-colony monotonic chaining, does not ship the remaining 22 sparse / 6 solid enum values, does not change any default (`sparse_fill_holder` stays `"rectilinear-infill"`), and does not touch `ORCA_CONFIG_PADDING`.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — 262b allocated by the approved 262a/262b split this session); packet **262a** for merge order only (it edits `rectilinear-infill.toml` / `gyroid-infill.toml`, which this packet reads but does not edit).
- Ordering, not gating: packet 263 adds three more `claim:sparse-fill` modules. When both have landed, those three module ids are the natural targets for the `lockedzag` / `lateral_lattice` / `lateral_honeycomb` values — a follow-up, not this packet's scope.
- Unblocks: wayfinder ticket 15's resolution (jointly with 262a).
- Activation blockers: none. No `[BLOCK]` in `design.md`.

## Acceptance Criteria

- **AC-1. Given** the core-module manifest roots, **when** `load_modules_from_roots` walks `modules/core-modules/`, **then** it reports three more modules than before the packet (re-derive the current count from the assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` at implementation time, never quote a frozen number), with `com.core.crosshatch-infill` (`[stage] Layer::Infill`, `holds = ["claim:sparse-fill"]`), `com.core.monotonic-infill` (`Layer::Infill`, `holds = ["claim:top-fill"]`), and `com.core.infill-gap-fill` (`[stage] Layer::InfillPostProcess`, `reads = ["InfillIR", "PerimeterIR", "RegionMapIR"]`, `writes = ["InfillIR"]`, `holds = ["claim:infill-gap-fill"]`), and zero `Error`-level diagnostics. | `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a raw config carrying `sparse_infill_pattern`, **when** `resolve_global_config` runs, **then** `"crosshatch"` yields `sparse_fill_holder == "crosshatch-infill"`, `"gyroid"` yields `"gyroid-infill"`, `"lightning"` yields `"lightning-infill"`, `"rectilinear"` yields `"rectilinear-infill"`; an explicit `sparse_fill_holder` in the same config wins over the pattern key; the key being absent leaves the default `"rectilinear-infill"` untouched; and an unshipped canonical value such as `"honeycomb"` is rejected with an error naming the key, the offending value, and the shipped value list — never silently ignored and never silently mapped to rectilinear. | `cargo test -p slicer-scheduler --test scheduler_integration config_resolution_pattern_holder 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a raw config carrying `internal_solid_infill_pattern`, **when** `resolve_global_config` runs, **then** `"monotonic"` yields `top_fill_holder == "monotonic-infill"`, `"rectilinear"` yields `"rectilinear-infill"`, an explicit `top_fill_holder` wins, absence leaves the default untouched, and an unshipped value such as `"monotonicline"` is rejected with an error naming the key, the value, and the shipped list. | `cargo test -p slicer-scheduler --test scheduler_integration config_resolution_pattern_holder 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `crosshatch-infill` over a 20 mm square `sparse_infill_area` at `sparse_infill_density = 20`, **when** the module is run across a z sweep spanning at least two full periods, **then** the emitted line direction alternates between the two orthogonal orientations once per period (canonical `FillCrossHatch::_fill_surface_single` → `generate_infill_layers`' `phase`-driven `direction` flip, a function of absolute z and never of `layer_index`), the layers inside a repeat band emit straight parallel lines at the grid spacing (`generate_repeat_pattern`), and the layers inside a transition band emit the four-point zig-zag cycles (`generate_transform_pattern` / `generate_one_cycle`) whose amplitude increases to the band midpoint and decreases after it; two runs at the same z with different `layer_index` produce identical geometry. | `cargo test -p crosshatch-infill --test crosshatch_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `monotonic-infill` over a solid fill region, **when** it emits `TopSolidInfill` paths, **then** every successive polyline's sweep coordinate is greater than or equal to the previous one's and every polyline runs in the same direction along the sweep axis (monotonic order, canonical `FillParams::monotonic`), whereas the same region through `rectilinear-infill` alternates direction between adjacent lines — asserted as a direction-alternation count of 0 for monotonic and non-zero for rectilinear on the identical fixture. | `cargo test -p monotonic-infill --test monotonic_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** an `InfillIR` layer whose solid region leaves a wedge-shaped uncovered band between the fill lines and the region boundary, **when** `infill-gap-fill` runs, **then** with `gap_fill_target = "everywhere"` it appends at least one `ExtrusionRole::GapFill` path whose centerline lies inside the uncovered band and whose per-vertex widths fall within `[0.2 · spacing · (1 − INSET_OVERLAP_TOLERANCE), 2 · spacing]` where `INSET_OVERLAP_TOLERANCE = 0.4` is defined locally by this packet in `modules/core-modules/infill-gap-fill/src/lib.rs` (canonical `libslic3r/libslic3r.h`; the tree has no Rust definition today) — canonical `Fill::_create_gap_fill`'s medial-axis band, with `"topbottom"` it appends such paths on top/bottom surfaces and none on internal-solid surfaces, and with `"nowhere"` (the default) it adds exactly zero paths and re-emits `prior_infill` verbatim — the module owns the complete replacement `InfillIR` under ADR-0028 Option 1b (`run_infill_postprocess`, `crates/slicer-sdk/src/traits.rs`), so emitting nothing would delete every infill path on the layer. | `cargo test -p infill-gap-fill --test infill_gap_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the same fixture in linked and unlinked form (segments concatenated into polylines vs. raw segments covering the identical area), **when** `infill-gap-fill` runs on both with `gap_fill_target = "everywhere"`, **then** the emitted gap-fill geometry is identical — the pass measures covered *area*, not path topology, so it is order-insensitive with respect to `infill-linker` within `Layer::InfillPostProcess`. | `cargo test -p infill-gap-fill --test infill_gap_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `infill-gap-fill.toml`, **when** the schema guard parses it, **then** it declares `gap_fill_target` as `type = "enum"`, `values = ["everywhere", "topbottom", "nowhere"]`, `default = "nowhere"`, `display = "Gap Fill Target"`, `group = "Infill"`, with a `description` naming canonical `Fill::_create_gap_fill`; and no other module declares `gap_fill_target`. | `cargo test -p infill-gap-fill --test infill_gap_fill_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** the scheduler's bounds index, **when** `gap_fill_target = "bogus"` is resolved, **then** it is rejected as an unknown enum value naming the key and the three legal values; `gap_fill_target = 3` is rejected with `TypeMismatch`; and each of the three legal values resolves. | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** `cargo xtask gen-config-docs` has run, **then** `docs/15_config_keys_reference.md`'s generated module-key table carries `gap_fill_target` with owner `infill-gap-fill` exactly once, and the generated deviations block has the same number of data rows as immediately before the packet's manifest edits (re-derive that number from disk at implementation time; do not freeze it). | `cargo xtask gen-config-docs --check && [ "$(rg -c '^\| `gap_fill_target`' docs/15_config_keys_reference.md)" = "1" ]; echo "exit=$?"`
- **AC-11. Given** the two selection keys are host-side and therefore absent from every module manifest, **when** `docs/04_host_scheduler.md` §Claim Resolution is read, **then** it documents `sparse_infill_pattern` → `sparse_fill_holder` and `internal_solid_infill_pattern` → `top_fill_holder`, the shipped value→module table, the explicit-holder-wins precedence, and the reject-unshipped-values rule; and `docs/03_wit_and_manifest.md` §Known claim IDs carries a row for `claim:infill-gap-fill` (kind `non-fill`, dedup `first-winner`, owner `infill-gap-fill`, `Layer::InfillPostProcess`). | `rg -q 'sparse_infill_pattern' docs/04_host_scheduler.md && rg -q 'internal_solid_infill_pattern' docs/04_host_scheduler.md && rg -q 'claim:infill-gap-fill' docs/03_wit_and_manifest.md; echo "exit=$?"`

- **AC-12. Given** an `InfillIR` region whose sparse and solid buckets already contain `ExtrusionRole::GapFill` paths (the state after `infill-gap-fill` has run), **when** `infill-linker` runs on it, **then** every `GapFill` path is re-emitted verbatim — identical point count, coordinates, per-vertex widths, `speed_factor`, and order — with no boundary clipping and no short-polyline filtering, while every non-`GapFill` role is linked exactly as before (the existing linker suites stay green). This is the `InfillLinker::copy_ironing` passthrough pattern (`modules/core-modules/infill-linker/src/lib.rs`) extended to one more role, and it is what makes the two `Layer::InfillPostProcess` modules order-independent in both directions rather than only in the direction AC-7 tests. | `cargo test -p infill-linker --test gap_fill_passthrough_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Negative Test Cases

- **AC-N1. Given** default configuration (neither pattern key supplied, `gap_fill_target` absent), **when** a slice runs over the square fixture, **then** the holders stay `rectilinear-infill`, `infill-gap-fill` emits nothing, and the emitted G-code is byte-identical to the pre-packet baseline — adding three modules changes no default print. This is an *additional* criterion; it is never the sole evidence for any key. | `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`), **when** the packet's diff is inspected, **then** it contains zero added, removed, or edited lines — in particular the previous revision's `("sparse_infill_pattern", "grid")` → `"crosshatch"` correction is **not** a deliverable of this packet (map Authoring rule 2). | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` (expect `0`)
- **AC-N3. Given** an unshipped pattern value, **when** it is supplied, **then** resolution fails loudly rather than falling back: no run may proceed with a pattern the port does not implement. Asserted for `sparse_infill_pattern = "honeycomb"`, `= "zigzag"`, and `internal_solid_infill_pattern = "monotonicline"`, each error naming the key, the value, and the shipped list. | `cargo test -p slicer-scheduler --test scheduler_integration config_resolution_pattern_holder 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** `monotonic-infill`, **when** a sparse or bridge role is requested, **then** it emits nothing (it holds only `claim:top-fill`), and `crosshatch-infill` likewise emits no solid or bridge path (it holds only `claim:sparse-fill`). | `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p crosshatch-infill --test crosshatch_infill_tdd`, `cargo test -p monotonic-infill --test monotonic_infill_tdd`, `cargo test -p infill-gap-fill --test infill_gap_fill_tdd` (primary behaviour contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the three new guests must build and the check must return exit 0 before closure.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System; `docs/04_host_scheduler.md` § Claim Resolution — the mechanism this packet extends, and the doc this packet updates.
- `docs/03_wit_and_manifest.md` § Known claim IDs — gains the `claim:infill-gap-fill` row.
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` and `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` — the `Layer::InfillPostProcess` contract the gap-fill module must obey.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — registration contract for a new core module.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm.
- `docs/15_config_keys_reference.md` — generated.

## Doc Impact Statement (Required)

- `docs/04_host_scheduler.md` § Claim Resolution — hand-maintained; gains the pattern→holder mapping subsection (both keys, the shipped value→module table, explicit-holder precedence, the reject-unshipped rule). This is the discoverability home for the two selection keys, which are host-side and therefore never appear in the generated module-key table. Verification: the AC-11 command.
- `docs/03_wit_and_manifest.md` § Known claim IDs — hand-maintained; gains one row for `claim:infill-gap-fill`. Verification: the AC-11 command.
- `docs/15_config_keys_reference.md` — generated; gains one row for `gap_fill_target` (owner `infill-gap-fill`). The deviations block must not change row count; capture the pre-edit count from disk and diff it. Verification: the AC-10 command.
- `docs/07_implementation_status.md` — three new core modules join the module inventory; crosswalk in `task-map.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillCrossHatch.cpp` — `FillCrossHatch::_fill_surface_single` and the file-static helpers `generate_infill_layers`, `generate_repeat_pattern`, `generate_transform_pattern`, `generate_one_cycle` (the z-driven period/phase arithmetic, the low-density `repeat_ratio` tweak, the grid alignment, the transition-layer morph, the short-polyline drop at `0.8 · spacing`).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillMonotonic::fill_surface`, `FillMonotonicLines::fill_surface`, and the `params.monotonic` branch of `fill_surface_by_lines` (`generate_montonous_regions`, `connect_monotonic_regions`, `chain_monotonic_regions`); note which parts this port deliberately does not reproduce (see `design.md` DIV-3).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::_create_gap_fill` (the `gap_fill_target` branch, `polygons_covered_by_spacing`, the min/max gap band, `chain_points` ordering, `douglas_peucker`, `ExPolygon::medial_axis`, `variable_width(..., erGapFill, ...)`), and `Fill::new_from_type` (the `ipMonotonic` / `ipMonotonicLine` / `ipCrossHatch` mapping).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` — `sparse_infill_pattern` / `internal_solid_infill_pattern` value lists and `enum GapFillTarget` (`gftEverywhere` / `gftTopBottom` / `gftNowhere`, default `gftNowhere`).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
