# Implementation Plan: bridge-detector-orca-parity

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-166.
- TDD first (Step 1 sets up failing tests at multiple levels), then implementation in stages.
- Each step honors the context-discipline preamble.

## Steps

### Step 0: FACT-confirm offset utility and Orca defaults

- Task IDs:
  - `TASK-166`
- Objective: read-only discovery — confirm `slicer-helpers` Minkowski/offset utility availability and Orca-default values for `min_bridge_length`, `anchor_width_mm`, `expansion_margin`.
- Precondition: Step 0 not yet run.
- Postcondition: three FACTs recorded.
- Files allowed to read: none directly (delegate dispatches only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "Find polygon-offset / Minkowski-sum / `expand_polygon` utility in `crates/slicer-helpers/`. Return FACT yes/no with file:line and signature."
  - "Confirm Orca defaults `min_bridge_length`, `anchor_width_mm`, `expansion_margin` from `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` and `BridgeDetector.hpp`. Return FACT (numeric values only)."
  - "Summarize `BridgeDetector::detect_angle` algorithm in ≤ 200 words. Return SUMMARY."
- Context cost: `S`.
- Authoritative docs: `docs/13_slicer_helpers_crate.md`.
- OrcaSlicer refs: `BridgeDetector.hpp`, `BridgeDetector.cpp`.
- Verification: the three FACTs/SUMMARY.
- Exit condition: defaults known; offset utility availability known; algorithm understood at summary level.

### Step 1: Author failing TDD files at all three levels

- Task IDs:
  - `TASK-166`
- Objective: create `crates/slicer-host/tests/bridge_detector_tdd.rs` (mesh-analysis + slice-time tests), `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (module-level tests), append `benchy_gcode_contains_bridge_infill_evidence` to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`. Also add `bridge_detector_schema_versions_are_correct` to a `slicer-ir` test file. All tests fail until later steps land.
- Precondition: Step 0 complete.
- Postcondition: all new tests compile and FAIL.
- Files allowed to read:
  - `crates/slicer-host/src/mesh_analysis.rs` — full file (~700 lines OK; one-time read).
  - `crates/slicer-host/src/layer_slice.rs` — full file (small post-12-rev1).
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — full file.
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` — pattern reference.
- Files allowed to edit (≤ 3 per writing pass; multiple passes if needed):
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` (new).
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (new).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (append).
  - (separate pass) `crates/slicer-ir/tests/ir_tests.rs` (append schema-version test).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test bridge_detector_tdd`; return FACT (every test FAIL)."
  - "Run `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd`; return FACT (every test FAIL)."
  - "Run `cargo test -p slicer-ir bridge_detector_schema_versions_are_correct`; return FACT (FAIL)."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/08_coordinate_system.md`.
- OrcaSlicer refs: `Surface.hpp` (FACT confirming `stBottomBridge` semantics).
- Verification: tests compile and every new test in FAIL.
- Exit condition: TDD scaffolding complete.

### Step 2: Schema bumps and `BridgeRegion` / `SlicedRegion` field additions

- Task IDs:
  - `TASK-166`
- Objective: extend `BridgeRegion` with the 5 new fields; extend `SlicedRegion` with `bridge_areas` and `bridge_orientation_deg`; bump `SurfaceClassificationIR.schema_version` to `1.1.0` and `SliceIR.schema_version` to `1.2.0`. Update workspace's literal constructors with default values.
- Precondition: Step 1 complete.
- Postcondition: workspace builds; the schema-version test PASSES; remaining bridge-detector tests still FAIL (because behavior not yet implemented).
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — lines `336-380, 1000-1050`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - any test files with literal `BridgeRegion {}` or `SlicedRegion {}` constructors that need the new defaults (delegate "find all literal constructors" first; mechanical edits).
- Expected sub-agent dispatches:
  - "Find every literal `BridgeRegion {` and `SlicedRegion {` constructor across `crates/`; return LOCATIONS."
  - "Run `cargo build --workspace`; return FACT pass/fail."
  - "Run `cargo test -p slicer-ir bridge_detector_schema_versions_are_correct`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md` — additive-minor rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace`
  - `cargo test -p slicer-ir bridge_detector_schema_versions_are_correct`
- Exit condition: build green; schema test PASSES; literal constructors all carry default values.

### Step 3: Implement mesh adjacency analysis and `MeshAnalysisConfig`

- Task IDs:
  - `TASK-166`
- Objective: in `crates/slicer-host/src/mesh_analysis.rs`, add `MeshAnalysisConfig` struct (consolidating `overhang_threshold_deg` and the new bridge-config fields) and a private `compute_bridge_metrics` function that walks mesh half-edge adjacency to compute `anchor_width_mm`, `bridge_length_mm`, `is_valid`, `xy_footprint`, and an updated `bridge_direction_deg`. Wire to `execute_mesh_analysis_with` (extend signature to take `MeshAnalysisConfig`). Update `execute_mesh_analysis` (the no-config wrapper) to use Orca defaults.
- Precondition: Step 2 complete.
- Postcondition: mesh-analysis-level tests in `bridge_detector_tdd.rs` PASS (`valid_bridge_passes_min_length_filter`, `short_bridge_fails_min_length_filter`, `narrow_anchor_fails_anchor_width_filter`).
- Files allowed to read:
  - `crates/slicer-host/src/mesh_analysis.rs` — full.
  - `crates/slicer-helpers/src/lib.rs` — public API only via symbol search.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/mesh_analysis.rs`
  - `crates/slicer-host/src/lib.rs` (re-export `MeshAnalysisConfig` if public).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test bridge_detector_tdd valid_bridge_passes_min_length_filter short_bridge_fails_min_length_filter narrow_anchor_fails_anchor_width_filter -- --exact`; return FACT pass/fail per test."
- Context cost: `M`.
- Authoritative docs: none new (rely on Step 0 SUMMARY).
- OrcaSlicer refs: `BridgeDetector.cpp` (already FACTed in Step 0).
- Verification: targeted cargo test.
- Exit condition: 3 mesh-analysis tests PASS; all others still FAIL.

### Step 4: Extend WIT, host record, and SDK for `bridge_areas` + `bridge_orientation_deg`

- Task IDs:
  - `TASK-166`
- Objective: add `bridge_areas: list<expolygon>` and `bridge_orientation_deg: f32` to the `slice-region-data` WIT host record; update `crates/slicer-host/src/wit_host.rs` (record definition at lines ~`140-175`; `sliced_region_to_data` at lines `2517-2580`); add `SliceRegionView::bridge_areas()` and `bridge_orientation_deg()` accessors in `crates/slicer-sdk/src/views.rs`; if `crates/slicer-macros` mediates the WIT/SDK boundary, update its codegen.
- Precondition: Step 3 complete.
- Postcondition: workspace builds; `cargo build --tests` succeeds (per `docs/03` § WIT checklist).
- Files allowed to read:
  - `wit/<file>.wit` — locate `slice-region-data` definition (delegate FACT to find the file).
  - `crates/slicer-host/src/wit_host.rs` — only lines `135-180, 2517-2580`.
  - `crates/slicer-sdk/src/views.rs` — full file.
  - `crates/slicer-macros/src/lib.rs` — public API only via symbol search.
- Files allowed to edit (≤ 3):
  - `wit/<file>.wit`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-sdk/src/views.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`.
- Expected sub-agent dispatches:
  - "Locate the `slice-region-data` WIT type definition; return FACT with file:line."
  - "Search every `wit_host.rs`, `dispatch.rs`, and `wit_guest` reference to `slice-region-data` to confirm type-identity coverage; return LOCATIONS."
  - "Run `cargo build --tests --workspace`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/03_wit_and_manifest.md` § "WIT/Type Changes Checklist".
- OrcaSlicer refs: none.
- Verification: `cargo build --tests --workspace`.
- Exit condition: build + tests compile; no linker errors at the WIT boundary.

### Step 5: Update `crates/slicer-macros` codegen + rebuild WASM core modules

- Task IDs:
  - `TASK-166`
- Objective: if `crates/slicer-macros` codegen needs updating to expose the new fields to guests, update it. Then rebuild all WASM core modules.
- Precondition: Step 4 complete.
- Postcondition: `./modules/core-modules/build-core-modules.sh` succeeds.
- Files allowed to read:
  - `crates/slicer-macros/src/lib.rs` — public API only.
  - `modules/core-modules/build-core-modules.sh`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-macros/src/lib.rs` (only if needed).
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with the failing module name on failure."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (delegate FACT confirming SDK API additivity rules).
- OrcaSlicer refs: none.
- Verification: build script.
- Exit condition: every core module rebuilds successfully.

### Step 6: Implement slice-time bridge polygon assembly

- Task IDs:
  - `TASK-166`
- Objective: in `crates/slicer-host/src/layer_slice.rs`, add `assemble_bridge_areas` helper. For each region, iterate valid `BridgeRegion`s with `xy_footprint` overlap, compute `xy_footprint ∩ region.infill_areas`, offset by `+expansion_margin_mm`, intersect with `region.infill_areas`. Populate `SlicedRegion.bridge_areas` and `bridge_orientation_deg`. Update `wit_host.rs::sliced_region_to_data` to read the new fields off `SlicedRegion`.
- Precondition: Step 5 complete (WIT/SDK in sync).
- Postcondition: slice-time tests in `bridge_detector_tdd.rs` PASS (`slice_assembles_expanded_bridge_polygons`, `non_bridge_region_has_empty_bridge_areas`, `sharp_anchor_offset_does_not_self_intersect`, `invalid_bridge_excluded_from_slice_areas`).
- Files allowed to read:
  - `crates/slicer-helpers/src/lib.rs` — public API only.
  - `crates/slicer-host/src/layer_slice.rs` — full.
  - `crates/slicer-host/src/wit_host.rs` — lines `2517-2580` only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs`
  - `crates/slicer-host/src/wit_host.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test bridge_detector_tdd slice_assembles_expanded_bridge_polygons non_bridge_region_has_empty_bridge_areas sharp_anchor_offset_does_not_self_intersect invalid_bridge_excluded_from_slice_areas -- --exact`; return FACT pass/fail per test."
- Context cost: `M`.
- Authoritative docs: `docs/13_slicer_helpers_crate.md`, `docs/08_coordinate_system.md`.
- OrcaSlicer refs: none.
- Verification: targeted cargo test.
- Exit condition: 4 slice-time tests PASS; module-level + Benchy E2E still FAIL.

### Step 7: Update `rectilinear-infill` to emit BridgeInfill

- Task IDs:
  - `TASK-166`
- Objective: in `modules/core-modules/rectilinear-infill/src/lib.rs:95-135`, split fill emission. For paths over `bridge_areas`: emit `ExtrusionRole::BridgeInfill` at `bridge_orientation_deg`. For paths over `infill_areas \ bridge_areas`: emit existing roles per surface flags. Set difference uses `slicer-helpers` polygon subtraction.
- Precondition: Step 6 complete.
- Postcondition: `bridge_infill_emission_tdd.rs` test `bridge_areas_emit_bridge_infill_at_oriented_angle` PASSES.
- Files allowed to read:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — full.
  - `crates/slicer-helpers/src/lib.rs` — public API only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail (post-edit rebuild)."
  - "Run `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (delegate FACT for any SDK rules).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `make_fills` per-surface-role pattern selection (FACT delegation; confirm BridgeInfill direction comes from per-region `bridge_direction_deg`).
- Verification:
  - rebuild script
  - `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd -- --nocapture`
- Exit condition: module test PASSES; rebuild succeeds.

### Step 8: Acceptance — Benchy E2E + workspace gates + deviation log

- Task IDs:
  - `TASK-166`
- Objective: confirm `benchy_gcode_contains_bridge_infill_evidence` PASSES; confirm full workspace test + clippy pass; retire two deviations registered by 12-rev1 in `docs/DEVIATION_LOG.md`; update `docs/07_implementation_status.md`.
- Precondition: Step 7 complete.
- Postcondition: every AC verification command in `packet.spec.md` PASSES; deviation log updated; backlog row added.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md` (delegate row insertion)
  - `docs/02_ir_schemas.md` (schema-bump documentation)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_bridge_infill_evidence -- --nocapture`; return FACT pass/fail."
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Insert TASK-166 row into `docs/07_implementation_status.md`; return FACT confirming the new line:line."
- Context cost: `S`.
- Authoritative docs: `docs/DEVIATION_LOG.md`, `docs/02_ir_schemas.md`.
- OrcaSlicer refs: none.
- Verification: every AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; deviation log retires the two 12-rev1 entries; `docs/07` carries TASK-166.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Three FACT dispatches. |
| Step 1 | M | Three new test files + 1 schema test. |
| Step 2 | S | Schema bumps + mechanical fix-ups. |
| Step 3 | M | Mesh adjacency + bridge metrics. |
| Step 4 | M | WIT + host record + SDK (3 files; coupled). |
| Step 5 | M | Macros codegen + WASM rebuild. |
| Step 6 | M | Slice-time polygon assembly. |
| Step 7 | M | rectilinear-infill split emission + rebuild. |
| Step 8 | S | Acceptance + doc updates. |

Aggregate: `M`. No single step is `L`. If Step 4 trends to L (because the WIT change touches more than 3 files), split into "WIT type + host record" and "SDK + macro" sub-steps before activation.

## Packet Completion Gate

- All steps complete.
- Every AC verification command PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `docs/07_implementation_status.md` carries TASK-166.
- `docs/02_ir_schemas.md` documents `SurfaceClassificationIR.schema_version = 1.1.0` and `SliceIR.schema_version = 1.2.0`.
- `docs/DEVIATION_LOG.md` retires the two 12-rev1 deviations (any-vertex-in-polygon; Benchy bridge heuristic).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command.
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Record any remaining packet-local risk (especially: non-manifold STL handling; thermal/cooling overrides not yet wired).
- Confirm implementer's peak context usage stayed under 70%.
