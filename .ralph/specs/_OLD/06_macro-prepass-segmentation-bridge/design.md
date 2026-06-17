# Design: macro-prepass-segmentation-bridge

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/dispatch.rs` → `dispatch_prepass_call()` for `PrePass::MeshSegmentation` and `PrePass::PaintSegmentation`
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/dispatch_tdd.rs` — existing prepass dispatch tests (MeshAnalysis, LayerPlanning)
  - `modules/core-modules/mesh-segmentation/tests/mesh_segmentation_tdd.rs` — native unit tests (not macro-path)
  - `modules/core-modules/paint-segmentation/tests/paint_segmentation_tdd.rs` — native unit tests (not macro-path)
  - Existing `wit_host.rs` converter functions for prepass stages
- OrcaSlicer comparison surface: N/A — segmentation is Pinch 'n Print-native

## Architecture Constraints

- WIT boundary rule (authoritative in `docs/03_wit_and_manifest.md`): all module access is validated at the WIT boundary. Undeclared reads = fatal contract error. Undeclared writes = fatal contract error.
- Mesh geometry never crosses the boundary — modules query via host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`).
- `MeshIR` is host-owned; modules receive scoped read/write views.
- `PaintRegionIR` is host-owned; modules receive `PaintRegionLayerView` (read-only query interface).
- `PrepassModule::run_mesh_segmentation` and `PrepassModule::run_paint_segmentation` must receive SDK types (`MeshObjectView`, `PaintSegmentationObjectView`) populated from host IR, not object-id shells.

## Code Change Surface

- Selected approach:
  - Extend `dispatch_prepass_call()` to look up `ObjectMesh` data from `MeshIR` for each `object_id` and construct WIT-native `mesh-object-view` and `paint-segmentation-object-view` records before calling the WASM guest for `MeshSegmentation` and `PaintSegmentation` stages.
  - Add WIT type definitions in `wit/deps/ir-types.wit` for `mesh-object-view` and `paint-segmentation-object-view` as records parallel to the existing SDK types in `crates/slicer-sdk/src/prepass_types.rs`.
  - Add converter functions in `crates/slicer-host/src/wit_host.rs` to convert `ObjectMesh` → WIT `mesh-object-view` and `ObjectMesh` + `LayerPlanIR` → WIT `paint-segmentation-object-view`.
  - The macro path (`#[slicer_module]` on `PrepassModule`) already has the correct signature; no changes to `slicer-macros` are needed for the input side — only the host dispatch side is affected.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_prepass_call()` branches for `MeshSegmentation` and `PaintSegmentation`
  - `crates/slicer-host/src/wit_host.rs` — add WIT converter helpers (new functions or extend existing)
  - `wit/deps/ir-types.wit` — add `mesh-object-view` and `paint-segmentation-object-view` records
  - `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` — new TDD test
  - `crates/slicer-host/tests/macro_paint_segmentation_input_tdd.rs` — new TDD test
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — new TDD test
  - `crates/slicer-host/tests/macro_mesh_raycast_z_down_tdd.rs` — new TDD test
- Rejected alternatives that were considered and why they were not chosen:
  - **Pass object_ids only and let the SDK fetch geometry**: rejected because the SDK cannot directly access `MeshIR` — the host must own all IR access and provide data through WIT. Module authors using the SDK expect `MeshObjectView` to already contain geometry.
  - **Use a WIT resource (with `resource mesh-object-view`)** instead of a record: rejected because resources are identity-based and require the host to maintain handles; records are simpler for read-only views and avoid handle management complexity.
  - **Change `dispatch_prepass_call` to accept richer object data for all prepass stages**: rejected because only `MeshSegmentation` and `PaintSegmentation` need geometry; other prepass stages (MeshAnalysis, LayerPlanning) receive object IDs only.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `MeshIR` (`ObjectMesh`) is read but not mutated by this packet
  - `LayerPlanIR` (`global_layers`, `object_participation`) is read to derive `participating_layer_indices` for paint segmentation
  - No manifest changes required
- WIT boundary considerations:
  - `wit/deps/ir-types.wit` must define both records before the bindgen codegen can generate bindings
  - `wit/deps/ir-types.wit` already defines `object-id` (`type object-id = string`) — no new primitive types needed
  - The `paint-layer-view`, `paint-stroke-view`, `paint-value-view` types already exist in the SDK (`crates/slicer-sdk/src/prepass_types.rs`) and need WIT counterparts
- Determinism or scheduler constraints:
  - Object ordering in the `Vec<MeshObjectView>` passed to the module must be deterministic (same order across runs with identical inputs)
  - Use object ID lexicographic ordering as the tiebreaker (consistent with `load_order` assignment in `docs/02_ir_schemas.md`)

## Locked Assumptions and Invariants

- `MeshIR` is loaded before prepass stages run — this is enforced by the scheduler DAG ordering (MeshSegmentation is the first prepass stage).
- `LayerPlanIR` is available before `PaintSegmentation` runs — this is enforced by DAG dependency (LayerPlanning → PaintSegmentation).
- `PaintRegionIR` is produced by `PaintSegmentation` and stored on the Blackboard before per-layer stages run.
- Modules receive read-only views; no module can mutate `MeshIR` through these views.
- `transform_matrix` on `PaintSegmentationObjectView` is the world-space 4x4 column-major transform from `ObjectMesh.transform`.

## Risks and Tradeoffs

- **Risk**: If `wit/deps/ir-types.wit` bindgen is not regenerated after adding new record types, the WASM component build will fail or use stale types.
  - **Mitigation**: After updating `ir-types.wit`, regenerate bindings with `cargo build --workspace` and verify the new types appear in `slicer_wit` crate.
- **Risk**: Adding new WIT record types may break existing WIT-compatible modules if the SDK version is not synchronized.
  - **Mitigation**: New record types are additive (no existing types change); WIT additive compatibility rules apply.
- **Tradeoff**: Passing full geometry (`vertices` + `triangles`) across the WIT boundary increases memory transfer per module invocation. However, `MeshSegmentation` is prepass (runs once, not per-layer) so the one-time cost is acceptable.
- **Risk**: `participating_layer_indices` derivation requires scanning `LayerPlanIR.object_participation` for each object — O(objects × layers) which is acceptable for prepass scale.
  - **Mitigation**: This scan happens once before the prepass stage, not per-layer.

## Open Questions

- **Q1**: Should `mesh-object-view` include the full `FacetPaintData` (`paint_layers`) or only the paint layer indices for the macro path?
  - **A1**: Include full `paint_layers` — the SDK type `MeshObjectView` already has `paint_layers: Vec<PaintLayerView>` and macro modules need this data to normalize sub-facet strokes.
- **Q2**: Does `PaintSegmentationObjectView.transform_matrix` need to be applied to the geometry before passing to the module, or does the module receive raw geometry and applies the transform itself?
  - **A2**: Module receives raw geometry; transform is passed separately so the module can apply it as needed (same pattern as OrcaSlicer which stores transform on the object). The module is responsible for applying transforms during paint projection.
- **Q3**: Should negative geometry (holes, modifier volumes) be included in `mesh-object-view` or only the primary mesh?
  - **A3**: Include primary mesh only; `modifier_volumes` are handled separately by the host at planning time and do not participate in paint segmentation.
