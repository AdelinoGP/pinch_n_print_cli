---
status: implemented
packet: macro-prepass-segmentation-bridge
task_ids:
  - TASK-128
  - TASK-128a
  - TASK-128b
---

# 06_macro-prepass-segmentation-bridge

## Goal

Resolve prepass segmentation input-shape gaps on the macro/WIT path so macro-authored `MeshSegmentation` and `PaintSegmentation` modules receive real geometry and paint data instead of hollow SDK inputs. Covers DEV-025.

## Problem Statement

DEV-025 tracks that the prepass segmentation stages (`PrePass::MeshSegmentation` and `PrePass::PaintSegmentation`) receive hollow SDK inputs on the macro/WIT path. Concretely:

1. **MeshSegmentation**: Macro-authored modules that implement `PrepassModule::run_mesh_segmentation` receive `&[MeshObjectView]` but the current dispatch wiring (`dispatch_prepass_call`) passes only `object_ids: &[String]` — no geometry is attached. The SDK type `MeshObjectView` is defined in `crates/slicer-sdk/src/prepass_types.rs` with real fields (`vertices`, `triangles`, `paint_layers`), but the host dispatch path never populates them.

2. **PaintSegmentation**: Macro-authored modules that implement `PrepassModule::run_paint_segmentation` receive `&[PaintSegmentationObjectView]` with the same gap — `object_ids` are passed without the corresponding `transform_matrix`, `paint_layers`, or `participating_layer_indices` that the SDK type requires.

The result is that segmentation modules execute with empty geometry and paint data, making the prepass stages no-ops on the macro path even when the host IR contains valid mesh and paint data.

If this packet reopens or supersedes a prior packet: N/A — no prior packet addressed DEV-025 for the macro/WIT path.

## Architecture Constraints

- WIT boundary rule (authoritative in `docs/03_wit_and_manifest.md`): all module access is validated at the WIT boundary. Undeclared reads = fatal contract error. Undeclared writes = fatal contract error.
- Mesh geometry never crosses the boundary — modules query via host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`).
- `MeshIR` is host-owned; modules receive scoped read/write views.
- `PaintRegionIR` is host-owned; modules receive `PaintRegionLayerView` (read-only query interface).
- `PrepassModule::run_mesh_segmentation` and `PrepassModule::run_paint_segmentation` must receive SDK types (`MeshObjectView`, `PaintSegmentationObjectView`) populated from host IR, not object-id shells.

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
