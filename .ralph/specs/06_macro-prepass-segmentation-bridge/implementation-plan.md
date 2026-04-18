# Implementation Plan: macro-prepass-segmentation-bridge

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add WIT record types for prepass segmentation views

- Task IDs:
  - `TASK-128a` (MeshObjectView), `TASK-128b` (PaintSegmentationObjectView)
- Objective:
  Define `mesh-object-view` and `paint-segmentation-object-view` records in `wit/deps/ir-types.wit` as WIT counterparts to the SDK types in `crates/slicer-sdk/src/prepass_types.rs`.
- Precondition:
  - `wit/deps/ir-types.wit` is a valid WIT file that compiles with `wit-component`
  - No existing `mesh-object-view` or `paint-segmentation-object-view` record definitions exist in any WIT file
- Postcondition:
  - `wit/deps/ir-types.wit` contains a `mesh-object-view` record with fields: `object-id`, `vertices` (list of `point3`), `triangles` (list of `triangle-index`), `paint-layers` (list of `paint-layer-view`)
  - `wit/deps/ir-types.wit` contains a `paint-segmentation-object-view` record with fields: `object-id`, `vertices`, `triangles`, `paint-layers`, `transform-matrix` (list<f64> of length 16), `participating-layer-indices` (list<u32>)
  - `wit/deps/ir-types.wit` contains supporting records: `paint-layer-view`, `paint-stroke-view`, `paint-value-view`, `triangle-index`
  - `wit-component` can parse and generate bindings for the updated WIT
- Files expected to change:
  - `wit/deps/ir-types.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` § `deps/ir-types.wit`
  - `docs/05_module_sdk.md` § `PrepassModule` trait
- Verification:
  - `cargo build --workspace` succeeds after WIT regeneration
  - Generated `slicer_wit` crate contains `mesh-object-view` and `paint-segmentation-object-view` types
- Exit condition:
  - `wit/deps/ir-types.wit` parses without errors under `wit-component`
  - `cargo build -p slicer-wit` (if such a package exists) or equivalent bindgen check passes

---

### Step 2: Add WIT converter functions in `wit_host.rs`

- Task IDs:
  - `TASK-128a` (MeshObjectView converter), `TASK-128b` (PaintSegmentationObjectView converter)
- Objective:
  Add converter functions in `crates/slicer-host/src/wit_host.rs` to convert from host IR (`ObjectMesh`, `LayerPlanIR`) to WIT representations (`mesh-object-view`, `paint-segmentation-object-view`).
- Precondition:
  - Step 1 is complete — WIT record types are defined
  - `crates/slicer-ir/src/` contains `MeshIR`, `ObjectMesh`, `FacetPaintData`, `PaintLayer`, `PaintSemantic`, `PaintValue`, `PaintStroke`, `LayerPlanIR`, `GlobalLayer`, `ObjectLayerRef`
  - `crates/slicer-host/src/wit_host.rs` exists and compiles
- Postcondition:
  - `wit_host.rs` contains a function `object_mesh_to_wit_mesh_object_view(obj: &ObjectMesh) -> wit::MeshObjectView`
  - `wit_host.rs` contains a function `object_to_wit_paint_segmentation_view(obj: &ObjectMesh, participating_layers: Vec<u32>) -> wit::PaintSegmentationObjectView`
  - Both converters are tested via the TDD harness files from Step 3
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md` § `MeshIR`, `LayerPlanIR`
  - `docs/03_wit_and_manifest.md` § WIT converter pattern
- Verification:
  - `cargo build -p slicer-host` succeeds
  - Unit tests for the converter functions pass
- Exit condition:
  - Converter functions exist and are callable from the dispatch path

---

### Step 3: Write TDD harness for MeshObjectView geometry population

- Task IDs:
  - `TASK-128a`
- Objective:
  Create a test harness that proves `MeshObjectView` received by a macro-authored `MeshSegmentation` module contains real geometry.
- Precondition:
  - WIT record types (Step 1) are defined and bindings are generated
  - Converter functions (Step 2) are stubbed or implemented
- Postcondition:
  - `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` exists with tests that:
    - Construct a host IR scene with one `ObjectMesh` containing known `vertices` and `triangles`
    - Dispatch a macro-authored `MeshSegmentation` module (or mock the WIT call)
    - Assert that the `MeshObjectView` received by the module has non-empty `vertices` and `triangles` matching the input geometry
    - Assert that empty geometry produces a fatal diagnostic
  - All tests in the file pass
- Files expected to change:
  - `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` (new file)
- Authoritative docs:
  - `docs/01_system_architecture.md` § `PrePass::MeshSegmentation` I/O
  - `docs/05_module_sdk.md` § `MeshObjectView`
- Verification:
  - `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd`
- Exit condition:
  - Test file compiles and passes
  - `grep -E "MeshObjectView|triangle|geometry.*pass"` on test output confirms geometry is populated

---

### Step 4: Write TDD harness for PaintSegmentationObjectView inputs

- Task IDs:
  - `TASK-128b`
- Objective:
  Create a test harness that proves `PaintSegmentationObjectView` received by a macro-authored `PaintSegmentation` module contains transform matrices, paint layers, and participating layer indices.
- Precondition:
  - WIT record types (Step 1) are defined
  - Converter functions (Step 2) are stubbed or implemented
- Postcondition:
  - `crates/slicer-host/tests/macro_paint_segmentation_input_tdd.rs` exists with tests that:
    - Construct a host IR scene with one painted `ObjectMesh` and a `LayerPlanIR` with known `participating_layer_indices`
    - Dispatch a macro-authored `PaintSegmentation` module (or mock the WIT call)
    - Assert that `PaintSegmentationObjectView.transform_matrix` is a non-identity 4x4 matrix
    - Assert that `PaintSegmentationObjectView.paint_layers` is non-empty
    - Assert that `PaintSegmentationObjectView.participating_layer_indices` is non-empty and matches `LayerPlanIR.object_participation`
    - Assert that missing transform or empty `participating_layer_indices` produces a diagnostic
  - All tests in the file pass
- Files expected to change:
  - `crates/slicer-host/tests/macro_paint_segmentation_input_tdd.rs` (new file)
- Authoritative docs:
  - `docs/01_system_architecture.md` § `PrePass::PaintSegmentation` I/O
  - `docs/02_ir_schemas.md` § `LayerPlanIR.object_participation`
  - `docs/05_module_sdk.md` § `PaintSegmentationObjectView`
- Verification:
  - `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd`
- Exit condition:
  - Test file compiles and passes
  - `grep -E "transform|paint_layer|participating.*pass"` on test output confirms all fields populated

---

### Step 5: Write TDD harness for PaintRegionIR round-trip

- Task IDs:
  - `TASK-128b` (PaintRegionIR part)
- Objective:
  Create a test harness that proves `PaintRegionIR` round-trips non-empty `SemanticRegion` data with Material semantic through the WIT boundary.
- Precondition:
  - `PaintRegionIR` IR type exists in `slicer-ir`
  - `PaintRegionLayerView` SDK type exists in `crates/slicer-sdk/src/traits.rs`
  - `PaintRegionLayerView::get_regions(&PaintSemantic::Material)` is implemented
- Postcondition:
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` exists with tests that:
    - Construct a host IR with `PaintRegionIR` populated with one `SemanticRegion` having Material semantic and valid polygon data
    - Dispatch a macro-authored module that reads via `PaintRegionLayerView`
    - Assert that `get_regions(&PaintSemantic::Material)` returns a non-empty slice
    - Assert that returned `SemanticRegion` entries have valid polygon data (non-empty `polygons` with valid `ExPolygon` contour points)
  - All tests in the file pass
- Files expected to change:
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` § `PaintRegionIR`, `SemanticRegion`
  - `docs/05_module_sdk.md` § `PaintRegionLayerView`
- Verification:
  - `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`
- Exit condition:
  - Test file compiles and passes
  - `grep -E "SemanticRegion|Material.*non-empty"` on test output confirms non-empty regions

---

### Step 6: Write TDD harness for `raycast_z_down` on macro path

- Task IDs:
  - `TASK-128` (raycast part)
- Objective:
  Create a test harness that proves `host.mesh().raycast_z_down` returns correct world-space Z when called from a macro-authored module on the prepass path.
- Precondition:
  - `raycast_z_down` host service is defined in `wit/host-api.wit`
  - `host.mesh().raycast_z_down(...)` is callable from a macro-authored module via `slicer_sdk::host`
- Postcondition:
  - `crates/slicer-host/tests/macro_mesh_raycast_z_down_tdd.rs` exists with tests that:
    - Construct a host IR scene with one `ObjectMesh` with known Z extent
    - Dispatch a macro-authored module that calls `host.mesh().raycast_z_down(object_id, x, y, start_z)`
    - Assert that the returned `Option<f32>` is `Some(world_z)` where `world_z` matches the intersected world-space Z of the mesh
    - Assert that a miss (ray exits build volume) returns `None`
  - All tests in the file pass
- Files expected to change:
  - `crates/slicer-host/tests/macro_mesh_raycast_z_down_tdd.rs` (new file)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` § `host-api.wit` `raycast-z-down`
  - `docs/05_module_sdk.md` § `host.mesh()` service wrappers
- Verification:
  - `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd`
- Exit condition:
  - Test file compiles and passes
  - `grep -E "raycast.*hit|world.*Z.*pass"` on test output confirms correct world-space Z

---

### Step 7: Wire MeshObjectView population into `dispatch_prepass_call`

- Task IDs:
  - `TASK-128a`
- Objective:
  Extend `dispatch_prepass_call()` in `crates/slicer-host/src/dispatch.rs` to look up `ObjectMesh` from `MeshIR` for the `PrePass::MeshSegmentation` branch and construct a `Vec<mesh-object-view>` from real geometry.
- Precondition:
  - Step 1 (WIT types), Step 2 (converters), Step 3 (TDD harness stub) are complete or far enough along to validate
  - `dispatch_prepass_call` currently passes only `object_ids: Vec<String>` to the WASM guest for `MeshSegmentation`
- Postcondition:
  - The `PrePass::MeshSegmentation` branch in `dispatch_prepass_call` converts each `object_id` to a `mesh-object-view` using the converter from Step 2
  - The WASM guest receives `Vec<mesh-object-view>` instead of `Vec<object-id>`
  - `macro_mesh_segmentation_geometry_tdd` tests pass
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs` — `PrePass::MeshSegmentation` branch in `dispatch_prepass_call`
- Authoritative docs:
  - `docs/01_system_architecture.md` § `PrePass::MeshSegmentation`
  - `docs/04_host_scheduler.md` § `dispatch_prepass_call`
- Verification:
  - `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd`
- Exit condition:
  - Macro-authored `MeshSegmentation` module receives populated `MeshObjectView` with real geometry

---

### Step 8: Wire PaintSegmentationObjectView population into `dispatch_prepass_call`

- Task IDs:
  - `TASK-128b`
- Objective:
  Extend `dispatch_prepass_call()` in `crates/slicer-host/src/dispatch.rs` to look up `ObjectMesh` and `LayerPlanIR` for the `PrePass::PaintSegmentation` branch and construct `Vec<paint-segmentation-object-view>` from real data.
- Precondition:
  - Step 1 (WIT types), Step 2 (converters), Step 4 (TDD harness stub) are complete or far enough along to validate
  - `dispatch_prepass_call` currently passes only `object_ids: Vec<String>` to the WASM guest for `PaintSegmentation`
  - `LayerPlanIR` is available on the Blackboard (ensured by DAG ordering — LayerPlanning runs before PaintSegmentation)
- Postcondition:
  - The `PrePass::PaintSegmentation` branch in `dispatch_prepass_call` converts each `object_id` to a `paint-segmentation-object-view` using the converter from Step 2, with `participating_layer_indices` derived from `LayerPlanIR.object_participation`
  - The WASM guest receives `Vec<paint-segmentation-object-view>` instead of `Vec<object-id>`
  - `macro_paint_segmentation_input_tdd` tests pass
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs` — `PrePass::PaintSegmentation` branch in `dispatch_prepass_call`
- Authoritative docs:
  - `docs/01_system_architecture.md` § `PrePass::PaintSegmentation`
  - `docs/02_ir_schemas.md` § `LayerPlanIR.object_participation`
  - `docs/04_host_scheduler.md` § `dispatch_prepass_call`
- Verification:
  - `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd`
- Exit condition:
  - Macro-authored `PaintSegmentation` module receives populated `PaintSegmentationObjectView` with all required fields

---

### Step 9: Backpressure gate

- Task IDs:
  - `TASK-128`
- Objective:
  Verify the workspace is in a clean, committable state after all steps land.
- Precondition:
  - All steps 1-8 exit conditions are met
  - All 4 TDD harness tests pass
- Postcondition:
  - `cargo build --workspace` succeeds
  - `cargo test --workspace` succeeds
  - `cargo clippy --workspace -- -D warnings` produces no warnings
- Files expected to change:
  - None (verification only)
- Authoritative docs:
  - `CLAUDE.md` § Build & Test Commands
- Verification:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
- Exit condition:
  - All three commands pass without modification

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (`macro_mesh_segmentation_geometry_tdd`, `macro_paint_segmentation_input_tdd`, `macro_paint_region_roundtrip_tdd`, `macro_mesh_raycast_z_down_tdd`).
- `docs/07_implementation_status.md` updated: TASK-128, TASK-128a, TASK-128b marked `[x]` complete.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all 4 TDD test files pass.
- Confirm packet-level `cargo clippy` is clean.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
