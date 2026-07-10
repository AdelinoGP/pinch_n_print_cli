---
status: implemented
packet: transform-aware-world-z
task_ids:
  - TASK-157
  - TASK-158
---

# 10_transform-aware-world-z

## Goal

Add fixture-level integration coverage for non-identity object transforms proving correct world-space Z behavior through `PrePass::LayerPlanning`, and promote world-space Z extent to one canonical derived contract surface (either as a first-class IR field on `ObjectMesh` or as explicitly documented config-only behavior). Regression-lock transformed-object behavior. Closes DEV-027.

## Problem Statement

AUDIT-21 (DEV-027) confirmed that `object_world_z_extent` exists and is wired into the live pipeline via `object_height:{id}` config keys at `main.rs:153`, but two critical gaps remain:

1. **No integration-level test** loads a real STL/3MF fixture with a non-identity `<transform>` element and verifies world-space Z behavior through `LayerPlanIR` generation. Only unit tests exist in `model_loader_tdd.rs`.

2. **World-space Z is not a first-class derived field** on `ObjectMesh`. It is computed on-demand via `object_world_z_extent(object: &ObjectMesh) -> Option<(f32, f32)>` and not cached. This means the contract surface for world-space Z is implicit, not explicit — any future caller who inadvertently reads local mesh Z (bypassing the transform) would produce incorrect results silently.

TASK-157 closes gap 1. TASK-158 closes gap 2 by promoting world-space Z to a canonical surface.

## Architecture Constraints

1. **Transform is not applied to mesh geometry at load time.** The raw vertices remain in object-local space. The transform is applied at query time (`object_world_z_extent`, `mesh_analysis::apply_transform`).

2. **Column-major transform convention.** `Transform3d.matrix` is stored column-major (index = col*4 + row). Translation is in column 3 (indices 12, 13, 14 for X, Y, Z). This matches the WASM matrix layout.

3. **Z is in millimeters.** `GlobalLayer.z` is `f32` in mm. `object_world_z_extent` returns `(f32, f32)` in mm. The 100-nm scaling applies only to X/Y `Point2` coordinates.

4. **World-space Z is the canonical surface for planning.** All Z-plane sequencing in `PrePass::LayerPlanning` must use world-space Z, never object-local Z. This invariant must be enforced by tests.

5. **Degenerate extent = no print surface.** When `z_max <= z_min` (e.g., lay-flat rotation collapses vertical extent), `object_world_z_extent` returns `None`. LayerPlanning must handle this gracefully (object contributes zero layers).

## Data and Contract Notes

### IR or Manifest Contracts Touched

- `MeshIR.ObjectMesh` — potentially adds `world_z_extent: Option<(f32, f32)>` (Option A) or documents existing behavior (Option B)
- `LayerPlanIR.GlobalLayer.z` — the output that must be world-space when transforms are present
- `MeshIR.schema_version` — no change needed (v1.0.0 not released)

### WIT Boundary Considerations

None. World-space Z is computed entirely on the host side. No WIT types are changed.

### Determinism or Scheduler Constraints

- `object_world_z_extent` must be deterministic: same `ObjectMesh` (same vertex positions and transform matrix) must produce the same `(z_min, z_max)` across runs.
- LayerPlanning with transforms must be deterministic: same input `MeshIR` with transforms must produce identical `LayerPlanIR.global_layers[*].z` values.
- Non-uniform scale detection must be deterministic: `scale_x != scale_y != scale_z` triggers `NON_UNIFORM_SCALE_UNSUPPORTED`.

## Locked Assumptions and Invariants

1. **Transform is column-major.** `matrix[col*4 + row]` is the correct indexing. This is verified by the existing `object_world_z_extent_applies_rotation_about_x` test.

2. **Zero matrix = identity fallback.** Fixtures or generated meshes that leave `Transform3d.matrix` all-zero must behave as identity. Verified by `object_world_z_extent_zero_matrix_treated_as_identity` test.

3. **World-space Z is the only correct Z for planning.** No code path in `PrePass::LayerPlanning` may read object-local vertex Z for Z-plane sequencing.

4. **Degenerate extent is `None`.** When `z_max <= z_min`, the object contributes zero layers. This is not an error — it is a valid (if trivial) print scenario.

## Risks and Tradeoffs

### Risk: Option A Schema Change
If `ObjectMesh` is deeply embedded in serialization (serde JSON, binary formats), adding a derived field may have unexpected side effects. Mitigation: verify the field is marked `#[serde(skip)]` or equivalent, since it is derived, not input.

### Risk: LayerPlanning depends on config keys
Today, world-space Z flows through `object_height:{id}` config keys into LayerPlanning. If Option B is chosen (config-only), this dependency is explicit. If Option A is chosen, the cached field should be used directly rather than re-computing via config keys.

### Open Question: Scale Application
When a uniform scale is applied (e.g., `scale_z = 2.0`), the world-space Z extent is correctly scaled. However, the layer height is specified in world-space mm. Should `effective_layer_height` also be scaled? The current architecture says no — layer height is a user-facing config in mm, applied in world space. The scale applies to the mesh geometry, not to the layer height.

Resolution: Document this clearly in the canonical surface decision. If the question cannot be answered definitively before packet completion, mark the packet as `draft` until resolved.
