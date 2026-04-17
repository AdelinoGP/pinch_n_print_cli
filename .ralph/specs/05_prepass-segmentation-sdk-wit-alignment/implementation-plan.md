# Implementation Plan: prepass-segmentation-sdk-wit-alignment

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit current MeshObjectView SDK interface

- Task IDs:
  - `TASK-128a`
- Objective: Confirm that `MeshObjectView` currently returns only object IDs and no geometry. Identify what methods exist and their current behavior.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/05_module_sdk.md` — SDK interface
  - `docs/02_ir_schemas.md` — MeshIR, ObjectMesh
- OrcaSlicer refs: Check `OrcaSlicerDocumented/` for segmentation reference
- Verification: `grep -r "MeshObjectView" crates/slicer-sdk/src/`

### Step 2: Extend MeshObjectView with real geometry access

- Task IDs:
  - `TASK-128a`
- Objective: Add geometry access methods to `MeshObjectView` that return triangle data, vertex positions, and face normals from `MeshIR` in the Blackboard.
- Files expected to change:
  - `crates/slicer-sdk/src/prepass.rs` (MeshObjectView extension)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — MeshIR, ObjectMesh, IndexedTriangleSet
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-sdk` succeeds.

### Step 3: Audit current PaintSegmentation input surface

- Task IDs:
  - `TASK-128b`
- Objective: Confirm what `PaintSegmentation` currently receives as input — likely object IDs and no transform/paint layer data.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/05_module_sdk.md` — SDK interface
  - `docs/02_ir_schemas.md` — PaintRegionIR, PaintSemantic, PaintLayer
- OrcaSlicer refs: Check `OrcaSlicerDocumented/`
- Verification: `grep -r "PaintSegmentation" crates/slicer-sdk/src/`

### Step 4: Add transform matrices, paint layers, and participating layer indices to PaintSegmentation inputs

- Task IDs:
  - `TASK-128b`
- Objective: Extend `PaintSegmentation` inputs to include transform matrices, paint layers with semantics, and the set of layer indices that participate in each semantic.
- Files expected to change:
  - `crates/slicer-sdk/src/prepass.rs` (PaintSegmentationInput extension)
  - `crates/slicer-host/src/prepass/paint_segmentation.rs` (input wiring)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — PaintRegionIR, PaintLayer, Transform3d
  - `docs/05_module_sdk.md` — SDK interface
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` succeeds.

### Step 5: Audit #[slicer_module] prepass bridge status

- Task IDs:
  - `TASK-130`
- Objective: Find the current `#[slicer_module]` prepass bridge implementation and identify what is incomplete or missing.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/05_module_sdk.md` — `#[slicer_module]` macro
  - `docs/03_wit_and_manifest.md` — world-prepass.wit
- OrcaSlicer refs: None
- Verification: `grep -r "slicer_module" crates/slicer-macros/src/`

### Step 6: Complete the #[slicer_module] prepass segmentation bridge

- Task IDs:
  - `TASK-130`
- Objective: Complete the bridge for all prepass stages (MeshSegmentation, MeshAnalysis, LayerPlanning, PaintSegmentation). Ensure macro-authored modules can use the bridge without hand-written `wit-guest` glue.
- Files expected to change:
  - `crates/slicer-macros/src/` (bridge implementation)
  - `crates/slicer-sdk/src/` (SDK prepass types)
- Authoritative docs:
  - `docs/05_module_sdk.md` — `#[slicer_module]` macro
  - `docs/03_wit_and_manifest.md` — world-prepass.wit
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-sdk` and `cargo build --package slicer-macros` succeed.

### Step 7: Implement PaintSegmentationOutput push-paint-region drainage

- Task IDs:
  - `TASK-130a`
- Objective: Ensure `PaintSegmentationOutput` correctly drains through WIT `push-paint-region` without hand-written glue. The macro bridge should handle the conversion.
- Files expected to change:
  - `crates/slicer-macros/src/` (bridge for paint region output)
  - `crates/slicer-host/src/prepass/paint_segmentation.rs` (output handler)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — PaintRegionIR
  - `docs/03_wit_and_manifest.md` — world-prepass.wit (push-paint-region)
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` succeeds with paint region output.

### Step 8: Add end-to-end macro-path round-trip tests

- Task IDs:
  - `TASK-130b`
- Objective: Add `prepass_macro_path_roundtrip_tdd.rs` test that exercises `MeshSegmentation` and `PaintSegmentation` on the macro path and asserts real data (not stubs) round-trips through WIT.
- Files expected to change:
  - `crates/slicer-host/tests/prepass_macro_path_roundtrip_tdd.rs` (new file)
- Authoritative docs:
  - `docs/05_module_sdk.md` — `#[slicer_module]` macro
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test prepass_macro_path_roundtrip_tdd -- --nocapture` — pass.

### Step 9: Full build and test verification

- Task IDs:
  - `TASK-128`
  - `TASK-130`
- Objective: Run full build and test suite to confirm all bridge, SDK, and round-trip tests pass.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host --package slicer-sdk --package slicer-macros` and `cargo test --package slicer-host --test prepass_macro_path_roundtrip_tdd -- --nocapture` — all pass.

## Packet Completion Gate

- `MeshObjectView` provides real geometry on macro path.
- `PaintSegmentation` inputs include transform matrices, paint layers, and participating layer indices.
- `#[slicer_module]` prepass bridge complete for all prepass stages.
- `PaintSegmentationOutput` drains through WIT `push-paint-region`.
- End-to-end round-trip tests pass.
- `docs/07_implementation_status.md` TASK-128/128a/128b/130/130a/130b marked complete.
- `packet.spec.md` ready to move to `status: implemented`.