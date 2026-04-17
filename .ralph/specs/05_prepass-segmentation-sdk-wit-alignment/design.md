# Design: prepass-segmentation-sdk-wit-alignment

## Controlling Code Paths

- Primary code path: `crates/slicer-sdk/src/` (SDK prepass types), `crates/slicer-host/src/wit/` (WIT boundary), `crates/slicer-macros/src/` (`#[slicer_module]` macro)
- Neighboring tests or fixtures: `crates/slicer-host/tests/prepass_macro_path_roundtrip_tdd.rs` (to be added)
- OrcaSlicer comparison surface: None (check OrcaSlicerDocumented/ for segmentation reference)

## Architecture Constraints

- The macro path uses `#[slicer_module]` proc-macro to generate WIT guest glue. The macro must correctly bridge SDK types to WIT types.
- `MeshObjectView` in the SDK must provide access to real triangle/vertex data, not just an object ID string.
- `PaintSegmentation` inputs must include transform matrices (`Transform3d`) applied to the mesh, paint layers with their semantics, and the set of layer indices that participate in each semantic.

## Proposed Changes

### TASK-128a — MeshSegmentation Real Geometry Inputs

1. **Audit current `MeshObjectView` SDK interface**: Confirm it currently returns only object IDs and no actual geometry.
2. **Extend `MeshObjectView` to provide geometry**: Add methods that return triangle data, vertex positions, and face normals. The data should come from `MeshIR` already loaded in the Blackboard.
3. **Update macro bridge**: Ensure the `#[slicer_module]` prepass bridge correctly converts `MeshObjectView` geometry calls to WIT calls.

### TASK-128b — PaintSegmentation Real Inputs

4. **Audit current `PaintSegmentation` input surface**: Confirm what is currently passed — likely object IDs and no transform/paint layer data.
5. **Add transform matrices to PaintSegmentation inputs**: Include the `Transform3d` applied to each object so paint regions can be correctly transformed to world space.
6. **Add paint layers and participating layer indices**: Pass `PaintLayer` data with semantic, facet values, and the layer indices that participate in each semantic.

### TASK-130/130a — Prepass Segmentation Bridge Completion

7. **Audit `#[slicer_module]` prepass bridge status**: Find the current implementation and identify what is incomplete.
8. **Complete the bridge**: Ensure all prepass stages (MeshSegmentation, MeshAnalysis, LayerPlanning, PaintSegmentation) are covered by the bridge.
9. **Implement `push-paint-region` drainage**: `PaintSegmentationOutput` must flow back through WIT `push-paint-region` calls without hand-written glue.

### TASK-130b — End-to-End Round-Trip Tests

10. **Add `prepass_macro_path_roundtrip_tdd.rs`**: Tests that exercise `MeshSegmentation` and `PaintSegmentation` on the macro path and assert that real data (not stubs) round-trips through WIT.
11. **Verify all tests pass**: Ensure round-trip tests cover both `MeshSegmentation` and `PaintSegmentation`.

## Data and Contract Notes

- `MeshObjectView` geometry methods must return data compatible with `IndexedTriangleSet` in `MeshIR`.
- `PaintSegmentation` inputs must include: `transform: Transform3d`, `paint_layers: Vec<PaintLayer>`, `participating_layer_indices: Vec<u32>`.
- `PaintSegmentationOutput.push_paint_region` must accept the same data shape that `PaintRegionIR` expects.

## Risks and Tradeoffs

- Extending `MeshObjectView` with real geometry may require careful handling of large meshes to avoid excessive memory copies crossing the WIT boundary.
- Paint region drainage must handle the case where no paint exists for a given semantic gracefully (empty polygons, not error).

## Open Questions

- Does `MeshObjectView` currently have any geometry methods, or is it purely ID-based? Check `crates/slicer-sdk/src/`.
- Does `PaintSegmentation` currently receive transform matrices? Check `crates/slicer-sdk/src/` and `crates/slicer-host/src/prepass/`.
- Is `Transform3d` the correct type for mesh transforms in the SDK? Check `crates/slicer-ir/` for the IR definition.
- Does OrcaSlicer have documented segmentation behavior we should reference? Check `OrcaSlicerDocumented/`.