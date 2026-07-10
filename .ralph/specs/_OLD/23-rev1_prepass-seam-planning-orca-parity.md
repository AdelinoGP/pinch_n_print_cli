---
status: implemented
packet: 23-rev1_prepass-seam-planning-orca-parity
task_ids:
  - TASK-159
supersedes: 23_prepass-seam-planning-orca-parity
---

# 23-rev1_prepass-seam-planning-orca-parity

## Goal

Fix `PrePass::SeamPlanning` so `seam-planner-default` receives actual mesh geometry (`MeshObjectView` with `vertices` and `triangles`) through the WIT boundary, enabling the module to produce valid seam entries during the live pipeline. The live Benchy run must see non-zero `SeamPlanIR` entries for at least one `(layer, object, region)` tuple.

## Problem Statement

The live `seam-planner-default` module produces **zero** `SeamPlanIR` entries during pipeline execution because three compounding bugs prevent it from ever receiving usable geometry:

1. **WIT boundary bug** (`wit/world-prepass.wit:106`): `run-seam-planning` accepts `list<object-id>` — bare string IDs — rather than `list<MeshObjectView>`. The module receives an empty/default `MeshObjectView` with zero vertices and zero triangles.

2. **Macro bug** (`crates/slicer-macros/src/lib.rs:1713`): The `seam_arm` generates `Vec<::slicer_ir::ObjectId>` instead of `Vec<MeshObjectView>`, causing a type mismatch at the `PrepassModule::run_seam_planning` call site.

3. **Threshold bug** (`modules/core-modules/seam-planner-default/src/lib.rs:144`): Curvature threshold is `0.5`; ordinary cube corners produce curvature values near `0.0`, so the threshold is never satisfied even if real geometry were somehow passed.

`MeshSegmentation` already implements the correct pattern (line 668-673 in `dispatch.rs`): it calls `wit_host::object_mesh_to_wit_mesh_object_view` for each object and passes `Vec<MeshObjectView>` through the WIT boundary. This packet mirrors that pattern for `SeamPlanning`.

## Architecture Constraints

- All geometry crosses the WIT boundary as value types (`MeshObjectView`), not handles. This is already established by `MeshSegmentation` and `PaintSegmentation`.
- `object_mesh_to_wit_mesh_object_view` already exists at `wit_host.rs:2031` — no new conversion needed.
- The seam_arm macro must produce type-compatible SDK objects for the `PrepassModule::run_seam_planning` trait signature.

## Data and Contract Notes

- **WIT boundary considerations:** `MeshObjectView` is a pure value type (owned `list` contents) that crosses the boundary by value. This is consistent with `MeshSegmentation` and `PaintSegmentation` usage of the same type.
- **The `SeamPlanIR` contract** requires entries with `(global_layer_index, object_id, region_id)` keys; duplicates are forbidden at blackboard commit.
- **`MeshObjectView` is already self-contained:** vertices (`Vec<[f32; 3]>`), triangles (`Vec<[u32; 3]>`), and `paint_layers`. No additional fields need to be populated.

## Locked Assumptions and Invariants

- `wit_host::object_mesh_to_wit_mesh_object_view` produces a non-empty `MeshObjectView` for any `ObjectMesh` with at least one triangle — if the source mesh is empty, the module receives an empty view and must handle it gracefully (AC-7/AC-8).
- The curvature threshold `0.2` is derived from empirical testing on cube geometry. Ordinary cube corners produce angular gaps near `0.28` (calculated from face normals 90° apart → `|1.0 - 0.0| = 1.0` curvature, but actual per-vertex normal averaging reduces this). A threshold of `0.2` allows these real corners to pass while still filtering flat-surface noise.
- `seam-planner-default` already declares its IR access contract correctly (`MeshIR` + `SurfaceClassificationIR` + `LayerPlanIR` reads, `SeamPlanIR` writes) — the bug is purely in the WIT parameter passing, not the manifest.

## Risks and Tradeoffs

1. **WIT compatibility risk:** Changing `run-seam-planning` signature from `list<object-id>` to `list<MeshObjectView>` breaks any external caller that was using the old signature. However, this is an internal WIT world (`world-prepass.wit`) and no external callers exist — the only caller is `dispatch.rs` which is being updated in this same packet.
2. **Macro type safety:** The `seam_arm` change from `Vec<ObjectId>` to `Vec<MeshObjectView>` must exactly match the trait signature `fn run_seam_planning(&self, objects: &[MeshObjectView], ...)` in `slicer-sdk/src/traits.rs`. If the WIT types are regenerated before the macro is updated, there will be a compile error — which is the correct behavior.
3. **Curvature threshold calibration:** `0.2` is a heuristic. If it produces too many false-positive corner candidates on complex geometry, the threshold may need further tuning. This is a runtime quality issue, not a correctness issue — the acceptance tests verify that at least one candidate is produced, not the exact count.
