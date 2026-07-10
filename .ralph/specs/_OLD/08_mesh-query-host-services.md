---
status: implemented
packet: mesh-query-host-services
task_ids:
  - TASK-147
  - TASK-148
---

# 08_mesh-query-host-services

## Goal

Implement live mesh-data wiring for `raycast_z_down`, `surface_normal_at`, and `object_bounds` on one shared mesh-query backing surface, replacing the current stub/trap behavior with tested results. Covers DEV-015.

## Problem Statement

The mesh-query host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`) are currently stubs that return `None` or trap with a diagnostic message. They need to be wired to live `MeshIR` data so modules can query mesh geometry at runtime. This is blocking non-planar surface projection, seam placement heuristics, and any module that needs surface normal or bounds information.

The three functions share a common backing surface: they all must look up an `ObjectMesh` by `ObjectId` in `MeshIR.objects`, apply the world transform, and then perform geometry queries on the `IndexedTriangleSet`.

## Architecture Constraints

1. **WIT boundary rule**: Mesh geometry never crosses the WASM boundary. Modules query via host services; the host performs all geometry calculations.

2. **Shared backing surface**: All three functions (`raycast_z_down`, `surface_normal_at`, `object_bounds`) share the same mesh lookup: find `ObjectMesh` by `ObjectId` in `MeshIR.objects`, apply `ObjectMesh.transform`, then query `IndexedTriangleSet`.

3. **World-space semantics**: All returned Z values, normals, and bounds must be in world space, accounting for `ObjectMesh.transform` (column-major 4x4 f64 matrix).

4. **Error handling**: Invalid `object_id` lookups must return fatal wasmtime errors with code `OBJECT_NOT_FOUND`. Coordinate-based mesh queries that do not land on a transformed surface point return `None`. The host must NOT panic or silently fabricate geometry.

5. **Four WIT worlds**: The `Host` trait is implemented separately in four `wit_world` modules inside `wit_host.rs`:
   - `layer` (world-layer@1.0.0)
   - `prepass` (world-prepass@1.0.0)
   - `finalization` (world-finalization@1.0.0)
   - `postpass` (world-postpass@1.0.0)
   All four must be updated to use the shared mesh-query logic.

## Data and Contract Notes

### IR or manifest contracts touched

- `HostExecutionContext` gains a new `mesh_ir: Option<Arc<MeshIR>>` field. The context is constructed per-call, so the permanent blackboard mesh must be cloned into the context from the dispatch path.

### WIT boundary considerations

- The WIT signature for `raycast_z_down` returns `option<f32>` (just the Z). The `facet_index` is computed internally for hit selection but is NOT returned through the current WIT boundary.
- `surface_normal_at` returns `option<point3>` for a queried world-space point. Off-surface points return `None`; invalid object lookup remains a fatal host error.
- `object_bounds` returns `bounding-box3` — already correct WIT signature.

### Determinism or scheduler constraints

- Raycast result must be deterministic: given identical mesh, transform, and origin, the same Z must be returned. Floating-point ordering must be consistent.
- The algorithm finds the *closest* hit below `start_z`. If two triangles tie (co-planar), any deterministic tiebreak is acceptable.
- `surface_normal_at` must use a fixed epsilon for plane-distance and barycentric containment checks so repeated queries return stable `Some` versus `None` decisions.

## Locked Assumptions and Invariants

- `ObjectMesh.transform` is a column-major 4x4 matrix. Transformation must be applied correctly to go from object-local to world space.
- Coordinates: X/Y in `Point2` use scaled integers (1 unit = 100nm), but 3D `Point3` uses millimeters (`f32`). The mesh vertices are in mm. `start_z` is in mm.
- `IndexedTriangleSet.indices` contains `u32` indices, 3 per triangle. `indices.len() / 3` is the triangle count.
- `surface_normal_at` matching uses world-space coordinates and a small epsilon for plane-distance checks; it does not take a facet index under the current WIT contract.

## Risks and Tradeoffs

1. **No BVH acceleration**: Brute-force `O(triangle_count)` raycasting is acceptable for initial correctness, but performance may need a follow-up optimization task.

2. **Transform handling complexity**: Incorrectly applying the transform would cause wrong Z values, normals, and bounds. The implementation must carefully distinguish object-local vs world space at each step.

3. **Coordinate tolerance for `surface_normal_at`**: The host must choose an epsilon that is tight enough to avoid false positives but loose enough for stable floating-point matching on transformed surfaces.
