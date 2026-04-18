# Task Map: macro-prepass-segmentation-bridge

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Notes |
| --- | --- | --- | --- | --- |
| `TASK-128` | `Step 6` (raycast harness), `Step 9` (backpressure gate) | `docs/01_system_architecture.md`, `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/macro_mesh_raycast_z_down_tdd.rs` | Parent task — umbrella for the overall gap. raycast harness validates the geometry wiring end-to-end. |
| `TASK-128a` | `Step 1` (WIT types), `Step 2` (converters), `Step 3` (TDD harness), `Step 7` (dispatch wiring) | `docs/01_system_architecture.md` § PrePass::MeshSegmentation, `docs/02_ir_schemas.md` § MeshIR | `wit/deps/ir-types.wit`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs`, `crates/slicer-host/src/dispatch.rs` | MeshObjectView must carry real `vertices` and `triangles` from `ObjectMesh`, not just object-id shells. |
| `TASK-128b` | `Step 1` (WIT types), `Step 2` (converters), `Step 4` (paint segmentation harness), `Step 5` (PaintRegionIR round-trip), `Step 8` (dispatch wiring) | `docs/01_system_architecture.md` § PrePass::PaintSegmentation, `docs/02_ir_schemas.md` § LayerPlanIR, PaintRegionIR | `wit/deps/ir-types.wit`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/tests/macro_paint_segmentation_input_tdd.rs`, `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs`, `crates/slicer-host/src/dispatch.rs` | PaintSegmentationObjectView must carry transform_matrix, paint_layers, and participating_layer_indices. PaintRegionIR round-trip validates the read path. |
