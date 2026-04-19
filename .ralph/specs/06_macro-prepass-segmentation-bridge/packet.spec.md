---
status: active
packet: macro-prepass-segmentation-bridge
task_ids:
  - TASK-128
  - TASK-128a
  - TASK-128b
backlog_source: docs/07_implementation_status.md
copy_note: Copy this file into ./.ralph/specs/macro-prepass-segmentation-bridge/ and change status to draft or active.
---

# Packet Contract: macro-prepass-segmentation-bridge

## Goal

Resolve prepass segmentation input-shape gaps on the macro/WIT path so macro-authored `MeshSegmentation` and `PaintSegmentation` modules receive real geometry and paint data instead of hollow SDK inputs. Covers DEV-025.

## Scope Boundaries

- In scope:
  - `MeshObjectView` geometry sourcing for `PrePass::MeshSegmentation` macro path
  - `PaintSegmentationObjectView` inputs for `PrePass::PaintSegmentation` macro path: transform matrices, paint layers, participating layer indices
  - WIT boundary coverage for both prepass segmentation stages
  - End-to-end round-trip tests proving real geometry and paint data cross the WIT boundary
- Out of scope:
  - Non-segmentation WIT boundary gaps (covered by TASK-129)
  - Mesh-query host services `raycast_z_down`, `surface_normal_at`, `object_bounds` (covered by TASK-147/148)
  - Path-optimization behavior
  - Postpass GCode boundary coverage

## Prerequisites and Blockers

- Depends on:
  - TASK-130 (prepass segmentation bridge in `slicer-macros`) for macro-authored module support — this packet provides the input wiring that TASK-130's output path drains
  - `MeshSegmentationIR` and `PaintRegionIR` IR types existing in `slicer-ir` (already present)
- Unblocks:
  - TASK-130a (draining `PaintSegmentationOutput` back through WIT `push-paint-region`)
  - TASK-130b (end-to-end macro-path regression tests for both segmentation stages)
- Activation blockers:
  - `crates/slicer-host/tests/dispatch_tdd.rs` must compile and pass basic prepass dispatch smoke tests
  - `wit/deps/ir-types.wit` must define `mesh-object-view` and `paint-segmentation-object-view` records for world-prepass

## Acceptance Criteria

- **Given** a macro-authored `MeshSegmentation` module (using `#[slicer_module]`), **when** it receives `&[MeshObjectView]` via `PrepassModule::run_mesh_segmentation`, **then** each `MeshObjectView` in the slice exposes real triangle geometry (`vertices: Vec<[f32; 3]>`, `triangles: Vec<[u32; 3]>`) sourced from the host `MeshIR` and not just object-id shells. | `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd 2>&1 | grep -E "MeshObjectView|triangle|geometry.*pass"`
- **Given** a macro-authored `PaintSegmentation` module, **when** it receives `&[PaintSegmentationObjectView]` via `PrepassModule::run_paint_segmentation`, **then** each view includes non-empty `transform_matrix: [f64; 16]`, non-empty `paint_layers`, and non-empty `participating_layer_indices` sourced from host IR and not empty collections. | `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd 2>&1 | grep -E "transform|paint_layer|participating.*pass"`
- **Given** `PaintRegionIR` is populated for a layer with Material semantic paint, **when** a macro-authored module reads it via `PaintRegionLayerView::get_regions(&PaintSemantic::Material)`, **then** the returned `&[SemanticRegion]` entries are non-empty with valid polygon data. | `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd 2>&1 | grep -E "SemanticRegion|Material.*non-empty"`
- **Given** a macro-authored module calls `host.mesh().raycast_z_down(object_id, x, y, start_z)` on a painted object, **when** the raycast is executed on the macro path, **then** it returns a hit with correct world-space Z (proving the object-id maps to a real mesh with geometry). | `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd 2>&1 | grep -E "raycast.*hit|world.*Z.*pass"`

## Negative Test Cases

- **Given** a macro-authored `MeshSegmentation` module, **when** `MeshObjectView` is constructed with empty `vertices` or `triangles`, **then** the host wired to dispatch produces a fatal contract error at the WIT boundary (not a silent empty-data pass). | `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd 2>&1 | grep -E "empty.*fatal|fatal.*empty"`
- **Given** a macro-authored `PaintSegmentation` module, **when** `PaintSegmentationObjectView` is constructed with missing transform matrix or empty `participating_layer_indices`, **then** the host wired to dispatch produces a diagnostic and does not silently proceed with zeroed data. | `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd 2>&1 | grep -E "missing.*diagnostic|diagnostic.*missing"`

## Verification

- `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd`
- `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd`
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`
- `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd`
- `cargo clippy --workspace -- -D warnings` (backpressure gate)

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
