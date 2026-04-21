# Design: mesh-query-host-services

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/wit_host.rs` — `HostExecutionContext` plus the four `Host` trait implementations that currently stub `raycast_z_down`, `surface_normal_at`, and `object_bounds`
- Dispatch path: `crates/slicer-host/src/dispatch.rs` — `dispatch_layer_call`, `dispatch_prepass_call`, `dispatch_finalization_call`, `dispatch_postpass_gcode_call`, and `dispatch_postpass_text_call`
- Blackboard owner: `crates/slicer-host/src/blackboard.rs` — `Blackboard::mesh()` is the authoritative `Arc<MeshIR>` source
- Neighboring tests or fixtures: `crates/slicer-host/tests/host_services_tdd.rs` and `crates/slicer-host/tests/macro_mesh_raycast_z_down_tdd.rs`
- OrcaSlicer comparison surface: None — this is internal host wiring, not geometry algorithm porting

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

## Code Change Surface

### Selected approach

Add a `mesh_ir: Option<Arc<MeshIR>>` field to `HostExecutionContext`, plumb blackboard mesh ownership through the dispatch path, and implement the three mesh-query functions against live data using a shared helper that all four WIT world implementations call.

**`raycast_z_down`**:
1. Look up `ObjectMesh` by `object_id` in `mesh_ir.objects`. If not found, return `OBJECT_NOT_FOUND` fatal error.
2. For each triangle in `IndexedTriangleSet`:
   a. Fetch the 3 vertex positions (indices into vertices array)
   b. Apply `ObjectMesh.transform` to each vertex to get world-space positions
   c. Skip triangle if it cannot intersect the vertical ray from `(x, y, start_z)`
   d. Compute ray-triangle intersection for direction `(0, 0, -1)`
   e. Keep the hit with the largest intersection Z that is still `<= start_z`
3. If a hit was found, return `Some(hit_z)` (world-space Z as `f32`)
4. Otherwise return `None`

**`surface_normal_at`**:
1. Look up `ObjectMesh` by `object_id`. If not found, return `OBJECT_NOT_FOUND`.
2. Iterate transformed triangles and test whether the queried world-space point `(x, y, z)` lies on the triangle plane within a small epsilon and inside the triangle's barycentric footprint.
3. For the matching triangle, compute the cross product of edge vectors: `e1 = v1 - v0`, `e2 = v2 - v0`. `normal = e1.cross(e2)`.
4. Normalize to unit length and return `Some(normal)`.
5. If no transformed triangle contains the queried point, return `None`.

**`object_bounds`**:
1. Look up `ObjectMesh` by `object_id`. If not found, return `OBJECT_NOT_FOUND`.
2. For each vertex in `IndexedTriangleSet.vertices`, apply `ObjectMesh.transform`.
3. Track `min_x/max_x/min_y/max_y/min_z/max_z` across all transformed vertices.
4. Return `BoundingBox3 { min, max }` in world space.

### Exact functions, traits, manifests, tests, or fixtures expected to change

- `crates/slicer-host/src/wit_host.rs`:
  - Add `mesh_ir: Option<Arc<MeshIR>>` field to `HostExecutionContext`
  - Update `HostExecutionContext::new` to accept `mesh_ir: Option<Arc<MeshIR>>`
  - Add a shared mesh-query helper used by all four world implementations
  - Implement `raycast_z_down`: replace `Ok(None)` stub with live mesh query
  - Implement `surface_normal_at`: replace `Ok(None)` stub with live normal computation
  - Implement `object_bounds`: replace error stub with live bounds computation
  - Update all four world trait implementations (`layer`, `prepass`, `finalization`, `postpass`) with the same logic
- `crates/slicer-host/src/dispatch.rs`:
  - Thread `blackboard.mesh().clone()` into `HostExecutionContext::new` from `dispatch_layer_call`, `dispatch_prepass_call`, `dispatch_finalization_call`, `dispatch_postpass_gcode_call`, and `dispatch_postpass_text_call`
  - Update the corresponding runner call paths that already receive `&Blackboard` so every WIT world gets the same mesh-query backing surface
- `crates/slicer-host/src/blackboard.rs`:
  - Use `Blackboard::mesh()` as the authoritative source of `Arc<MeshIR>` for host-service wiring
- New test files (7 total):
  - `crates/slicer-host/tests/raycast_z_down_hit_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_miss_tdd.rs`
  - `crates/slicer-host/tests/surface_normal_at_unit_length_tdd.rs`
  - `crates/slicer-host/tests/object_bounds_transform_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs`
  - `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs`

### Rejected alternatives that were considered and why they were not chosen

1. **Separate mesh cache in each function**: Rejected — all three functions share the same lookup pattern. A shared `mesh_ir` field on the context is more maintainable.

2. **Build an acceleration structure (BVH) upfront**: Rejected — the initial implementation should be correct before being optimized. A BVH can be added as a follow-up optimization task if profiling shows raycasting is a bottleneck. The current `IndexedTriangleSet` is small enough that brute-force intersection is acceptable for MVP.

3. **Return `Option<(f32, u32)>` from `raycast_z_down` instead of the current `option<f32>` WIT contract**: Rejected — the WIT signature `raycast-z-down` returns `option<f32>`. The `facet_index` is internal to the host implementation but does not cross the WIT boundary under the current signature. A separate future packet may address a WIT signature change to return a hit record with `z` and `facet-index` fields.

4. **Treat `surface_normal_at` as a facet-index lookup**: Rejected — the current WIT and SDK signatures are coordinate-based (`object_id, x, y, z`). Requiring a facet index would force a separate WIT extension task and would not be implementable within this wiring packet.

5. **Use an external math crate for ray-triangle intersection**: Rejected — the math is simple enough that adding a new dependency is not warranted. Implement directly using `f32` arithmetic.

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

## Resolved Scope Decisions

1. The packet implements against the current WIT contract: `raycast_z_down` returns `option<f32>` and `surface_normal_at` remains coordinate-based.

2. `object_bounds` returns world-space bounding boxes after applying `ObjectMesh.transform`, consistent with world-space Z semantics.

3. All four WIT worlds remain in scope. `PrepassStageRunner`, `LayerStageRunner`, `FinalizationStageRunner`, and `PostpassStageRunner` already receive `&Blackboard` at the runner boundary; the missing work is threading that mesh data into `HostExecutionContext::new`.

4. BVH acceleration is deferred. MVP uses brute-force `O(triangle_count)` iteration.
