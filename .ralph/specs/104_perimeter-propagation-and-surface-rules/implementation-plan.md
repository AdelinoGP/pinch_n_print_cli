# Implementation Plan: 104_perimeter-propagation-and-surface-rules

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add `overhang_areas` + `surface_group` accessors to `SliceRegionView` (including WIT `surface-group` record definition)

- Task IDs:
  - `T-023` — Expose `OverhangRegion` lookup (`overhang_areas()`)
  - Implicit: `surface_group()` accessor for future T-074b/c/d non-planar consumption (D-11 close, D-4 close)
- Objective: add the two accessor methods on `SliceRegionView`, define the NEW `surface-group` WIT record + `surface-group-id` type in `ir-types.wit`, add the two `func()` declarations to `slice-region-view`, fill them in the host populator. Both fields start empty/`None` for any region whose upstream PrePass has not yet emitted the data (forward-dep on P106/O-T010; IR field `OverhangRegion.xy_footprint` exists at `crates/slicer-ir/src/slice_ir.rs:581`).
- Precondition: workspace builds clean; packet 102 has landed (shared utils crate exists at `crates/slicer-core/src/perimeter_utils.rs`).
- Postcondition: AC-3 verification grep passes; `cargo xtask build-guests --check` reports no STALE.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-sdk/src/views.rs` — range-read by `rg -n 'impl SliceRegionView|fn (bridge_areas|has_nonplanar)'`.
  - `crates/slicer-wasm-host/src/host.rs` — range-read by `rg -n 'sliced_region_to_data|SliceRegionData'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full file (≤ 200 lines).
- Sub-step 1a — accessors + WIT + host populator (≤ 3 files):
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-wasm-host/src/host.rs` — `overhang_areas` populator returns `Vec::new()` (does NOT reference the net-new, not-yet-existent `OverhangRegion.xy_footprint`); `surface_group` resolves from `SurfaceClassificationIR`.
- Sub-step 1b — AC-3-EMPTY regression test + aggregator (≤ 2 files):
  - `crates/slicer-runtime/tests/contract/overhang_areas_empty_until_p106_tdd.rs` (NEW) — asserts `overhang_areas().is_empty()` for a constructed `SliceRegionView`; the regression bed P106 later flips to non-empty.
  - `crates/slicer-runtime/tests/contract/main.rs` — add `mod overhang_areas_empty_until_p106_tdd;`.
- Files explicitly out-of-bounds for this step:
  - Any perimeter module `lib.rs`.
  - Any `slicer-core` file.
  - `crates/slicer-ir/src/slice_ir.rs` — the net-new `OverhangRegion.xy_footprint` field is P106's edit, NOT this packet's.
- Expected sub-agent dispatches:
  - "Summarize the existing `bridge_areas` accessor + populator pattern across `crates/slicer-sdk/src/views.rs` and `crates/slicer-wasm-host/src/host.rs`; return SUMMARY ≤ 150 words."
  - "Summarize `docs/02_ir_schemas.md` for the `SurfaceGroup` Rust struct fields; return SUMMARY ≤ 100 words."
  - "Run `cargo build --tests --workspace`; return FACT (pass/fail) — catches WIT type identity break."
  - "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)."
- Context cost: `M` (three crates touched)
- Authoritative docs:
  - `docs/05_module_sdk.md` — delegate SUMMARY for §"SliceRegionView accessors".
  - `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist".
  - `CLAUDE.md` — §"WIT/Type Changes Checklist" and §"Guest WASM Staleness".
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'pub fn overhang_areas\(&self\) -> &\[ExPolygon\]' crates/slicer-sdk/src/views.rs` — exit 0.
  - `rg -q 'pub fn surface_group\(&self\) -> Option<&SurfaceGroup>' crates/slicer-sdk/src/views.rs` — exit 0.
  - `rg -q 'overhang-areas: func\(\) -> list<ex-polygon>' crates/slicer-schema/wit/deps/ir-types.wit` — exit 0.
  - `rg -q 'record surface-group' crates/slicer-schema/wit/deps/ir-types.wit` — exit 0.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test contract overhang_areas_empty_until_p106_tdd 2>&1 | tee -a target/test-output.log` — FACT (AC-3-EMPTY).
  - `cargo xtask build-guests --check` — no STALE.
- Exit condition: AC-3 + AC-3-EMPTY verification pass; build green; guests not STALE.

### Step 2: Rename + extend `build_outer_wall_flags` → `build_wall_flags` for inner walls + bridge per-vertex propagation; register new contract test mod entries

- Task IDs:
  - `T-020` — Per-vertex `is_bridge` from `region.bridge_areas()`
  - `T-021` — Per-vertex `tool_index` propagated to inner walls
  - `T-022` — Drop hardcoded `WallBoundaryType::Interior` for inner walls
- Objective: in `crates/slicer-core/src/perimeter_utils.rs`, rename `build_outer_wall_flags` → `build_wall_flags` and add `is_outer: bool` parameter; run same Material/FuzzySkin/boundary-type extraction on inner walls under `if !is_outer`; add `pub fn point_in_any_polygon(pt: &Point2, polys: &[ExPolygon]) -> bool` helper. **T-024 (helper site):** the shared `expolygon_to_path3d` already sets `flow_factor: 1.0, overhang_quartile: None` (perimeter_utils.rs:138-139); add an inline doc-comment to the `overhang_quartile: None` line citing sibling roadmap O-T031 (satisfies AC-6's `perimeter_utils.rs` grep; covers BOTH classic and arachne's helper-emitted vertices). Write `inner_wall_material_boundary_tdd.rs` (slicer-core standalone) and `per_vertex_is_bridge_propagation_tdd.rs`; register the contract test in `crates/slicer-runtime/tests/contract/main.rs`.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-1, AC-2, AC-N1 verification commands pass.
- Sub-step 2a — helper + slicer-core test (≤ 3 files):
  - `crates/slicer-core/src/perimeter_utils.rs` — rename + extend.
  - `crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/contract/per_vertex_is_bridge_propagation_tdd.rs` (NEW; contains AC-1 and AC-N1 cases).
- Sub-step 2b — aggregator registration (≤ 1 file):
  - `crates/slicer-runtime/tests/contract/main.rs` — add `mod per_vertex_is_bridge_propagation_tdd;`.
- Files explicitly out-of-bounds for Step 2:
  - Any perimeter module `lib.rs` — consumed in Step 3.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test inner_wall_material_boundary_tdd`; return FACT pass/fail + assertion text on fail."
  - "Run `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd`; return FACT pass/fail."
- Context cost: `M` (helper rename+extension + two new tests + aggregator edit)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for `BridgeRegion` shape.
- OrcaSlicer refs:
  - None for this step.
- Verification:
  - `cargo test -p slicer-core --test inner_wall_material_boundary_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1, AC-2, AC-N1 green; `boundary_paint_tdd` regression tests still green for both modules.

### Step 3: Consume new accessors + extended helper in both perimeter modules

- Task IDs:
  - `T-020`, `T-021`, `T-022` — consumer wiring
  - `T-024` — explicit `overhang_quartile = None` with doc-comment citing sibling roadmap
  - `T-025` — `flow_factor` per-vertex; current packet sets `1.0` and documents the future-work rationale
- Objective: in both `lib.rs` files, call `build_wall_flags(.., is_outer=false)` for inner walls; iterate per-vertex `is_bridge` via `point_in_any_polygon(&pt, region.bridge_areas())`. **T-024/T-025:** classic emits per-vertex `Point3WithWidth` via the shared `expolygon_to_path3d` (no inline literal — doc-comment handled at the helper in Step 2). Arachne has a SECOND, inline `Point3WithWidth { … overhang_quartile: None, flow_factor: 1.0 }` at `arachne-perimeters/src/lib.rs:428`; add the sibling-roadmap O-T031 doc-comment to that line (satisfies AC-6's arachne grep). Also add the contract test `inner_wall_boundary_type_tdd.rs` (AC-2b) and register it in `main.rs`.
- Precondition: Step 2 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-1 + AC-2 + AC-2b + AC-6 verification commands pass (AC-1 and AC-2 now exercise the modules end-to-end).
- Sub-step 3a — perimeter modules (≤ 2 files):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Sub-step 3b — AC-2b contract test + aggregator (≤ 2 files):
  - `crates/slicer-runtime/tests/contract/inner_wall_boundary_type_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/main.rs` — add `mod inner_wall_boundary_type_tdd;`
- Files explicitly out-of-bounds for Step 3:
  - `crates/slicer-core/src/perimeter_utils.rs` — already extended in Step 2.
  - Manifests — Step 4.
- Expected sub-agent dispatches:
  - "Run `cargo test -p classic-perimeters --tests`; return FACT pass/fail with failing-test names."
  - "Run `cargo test -p arachne-perimeters --tests`; return FACT pass/fail."
  - "Run `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd`; return FACT pass/fail."
  - "Run `cargo test -p slicer-runtime --test contract inner_wall_boundary_type_tdd`; return FACT pass/fail."
- Context cost: `M` (two-module rewrite + new contract test)
- Authoritative docs:
  - `docs/specs/overhang-pipeline-restructuring.md` — confirm T-024 deferral wording matches sibling roadmap.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p classic-perimeters --tests 2>&1 | tee target/test-output.log` — FACT (all green including `boundary_paint_tdd`).
  - `cargo test -p arachne-perimeters --tests 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd 2>&1 | tee target/test-output.log` — FACT (now end-to-end).
  - `cargo test -p slicer-runtime --test contract inner_wall_boundary_type_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'overhang_quartile.*None.*sibling roadmap' crates/slicer-core/src/perimeter_utils.rs` — exit 0 (shared `expolygon_to_path3d` site; covers classic + arachne helper path).
  - `rg -q 'overhang_quartile.*None.*sibling roadmap' modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 (arachne inline variable-width site).
- Exit condition: AC-1, AC-2, AC-2b, AC-6 partial (only doc-comment greps; deviation log handled in Step 5) green.

### Step 4: Implement `only_one_wall_top` + `only_one_wall_first_layer`

- Task IDs:
  - `T-030` — Register `only_one_wall_top` config key
  - `T-031` — Implement gate
  - `T-032` — Register `only_one_wall_first_layer` config key
  - `T-033` — Implement gate
- Objective: register the two config keys in both manifests; in both `lib.rs` files, read both keys via `_config.get_bool` per-call; clamp `wall_count = 1` when the corresponding gate fires (`top_shell_index() == Some(0)` for `only_one_wall_top`; `_layer_index == 0` for `only_one_wall_first_layer`); write `only_one_wall_top_tdd.rs` (covers AC-4 + AC-N2) and `only_one_wall_first_layer_tdd.rs` (covers AC-5); register both in `main.rs`.
- Precondition: Step 3 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-4, AC-5, AC-N2 verification commands pass.
- Sub-step 4a — manifests (≤ 2 files):
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
- Sub-step 4b — consumers (≤ 2 files):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Sub-step 4c — tests (≤ 2 files):
  - `crates/slicer-runtime/tests/contract/only_one_wall_top_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` (NEW)
- Sub-step 4d — aggregator (≤ 1 file):
  - `crates/slicer-runtime/tests/contract/main.rs` — add `mod only_one_wall_top_tdd;` and `mod only_one_wall_first_layer_tdd;`
- Files explicitly out-of-bounds for this step:
  - Any other source file.
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1574-1577,1715` for `only_one_wall_top`/`only_one_wall_first_layer` gate logic; return SUMMARY ≤ 100 words."
  - "Run `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd only_one_wall_first_layer_tdd`; return FACT pass/fail per test."
- Context cost: `M` (two manifests + two-module consumer wiring + two new tests + aggregator)
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — read full to confirm no "Walls" section; align key format for the new section created in Step 5.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1574-1577,1715` — delegate SUMMARY.
- Verification:
  - `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test contract only_one_wall_first_layer_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'only_one_wall_top' modules/core-modules/classic-perimeters/classic-perimeters.toml modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — both manifests carry the key.
- Exit condition: AC-4, AC-5, AC-N2 green.

### Step 5: Doc impact + deviation registration

- Task IDs:
  - Doc impact for `T-023`, `T-030`, `T-032`: `docs/05_module_sdk.md`, `docs/15_config_keys_reference.md`.
  - Deviation for `T-024`: `docs/DEVIATION_LOG.md` (`D-104-OVERHANG-QUARTILE-NONE`).
  - MED-2 sub-top `only_one_wall_top` reduction was implemented this session via `split_top_surfaces` (top_solid_fill-scoped carve); no deviation is registered for it.
- Objective: land the three doc edits + register both deferral deviations. For `docs/15_config_keys_reference.md`, CREATE a new §"Walls" section (no existing section exists).
- Precondition: Step 4 exit condition met.
- Postcondition: All three Doc Impact Statement greps return hits + AC-6 deviation grep passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/05_module_sdk.md` — range-read §"SliceRegionView accessors".
  - `docs/15_config_keys_reference.md` — read full (confirm no "Walls" section; align creation format).
  - `docs/DEVIATION_LOG.md` — range-read the most recent N entries to align format (`D-<pkt>-<SLUG>`).
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
  - `docs/15_config_keys_reference.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out-of-bounds for this step:
  - Any source file.
- Expected sub-agent dispatches:
  - "For each grep in the Doc Impact Statement, run `rg -q` on the listed path; return FACT pass/fail per grep."
- Context cost: `S` (three doc edits)
- Authoritative docs:
  - The three files being edited.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'overhang_areas.*ExPolygon' docs/05_module_sdk.md` — exit 0.
  - `rg -q 'surface_group.*SurfaceGroup' docs/05_module_sdk.md` — exit 0.
  - `rg -q 'only_one_wall_top.*bool.*default: false' docs/15_config_keys_reference.md` — exit 0.
  - `rg -q 'only_one_wall_first_layer.*bool.*default: false' docs/15_config_keys_reference.md` — exit 0.
  - `rg -q 'D-104-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md` — exit 0.
- Exit condition: all Doc Impact Statement greps pass; AC-6 fully green (doc-comment greps from Step 3 + deviation greps from this step).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Three crates (SDK + WIT + host) — view-accessor plumbing + new WIT surface-group record. |
| Step 2 | M | Helper rename+extension + two new tests + aggregator registration. |
| Step 3 | M | Two-module rewrite for per-vertex propagation + AC-2b contract test. |
| Step 4 | M | Manifests + two-module gate logic + two new tests + aggregator registration. |
| Step 5 | S | Three doc edits + two deviation entries. |

Aggregate context cost: `M`. No single step is `L`. Per-step file edit count never exceeds 3 within each sub-step.

## Packet Completion Gate

- All five steps complete; each step's exit condition met.
- AC-1, AC-2, AC-2b, AC-3, AC-3-EMPTY, AC-4, AC-5, AC-6, AC-N1, AC-N2 verification commands all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-020..T-033 entry — via worker dispatch.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record any test-fixture re-baseline (re-baselined integration tests touching wall geometry on top-shell / first-layer fixtures) in `.ralph/specs/104_perimeter-propagation-and-surface-rules/closure-log.md` with the new SHA.
- Note both sibling-roadmap silent dependencies in the closure log:
  - `overhang_areas()` returns empty Vec until P106 (`106_overhang-pipeline-prepass-foundation`) O-T010 lands; when that lands, no further change to this packet is needed.
  - Sub-top `only_one_wall_top` reduction implemented this session via `split_top_surfaces` (top_solid_fill-scoped carve); no deviation registered.
- Repo-hygiene followup (RESOLVED in this commit, see amend): `docs/adr/` previously contained two ADRs both prefixed `0013` (`0013-mmu-per-color-outer-wall-fragmentation.md` and `0013-producer-trait-for-host-builtin-seam.md`) — a duplicate-slot collision. The producer-trait ADR is renumbered to `0024-producer-trait-for-host-builtin-seam.md` (slot `0022` was reserved by packet 106 for `0022-overhang-classification-at-prepass.md`; slot `0023` by packet 110 for `0023-arachne-port-strategy.md`; `0024` was the next free). `0013-mmu-per-color-outer-wall-fragmentation.md` keeps slot `0013` (25 cross-references in spec docs and roadmap vs. 2 in this packet's documentation).
- Confirm the implementer's peak context usage stayed under 70%.
