# Design: mesh-query-host-services

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/wit_host.rs` — `hs::Host for HostExecutionContext` implementation (lines ~1172-1217)
- Neighboring tests or fixtures: `crates/slicer-host/tests/host_services_tdd.rs` (existing stub tests)
- OrcaSlicer comparison surface: None — this is internal host wiring, not geometry algorithm porting

## Architecture Constraints

1. **WIT boundary rule**: Mesh geometry never crosses the WASM boundary. Modules query via host services; the host performs all geometry calculations.

2. **Shared backing surface**: All three functions (`raycast_z_down`, `surface_normal_at`, `object_bounds`) share the same mesh lookup: find `ObjectMesh` by `ObjectId` in `MeshIR.objects`, apply `ObjectMesh.transform`, then query `IndexedTriangleSet`.

3. **World-space semantics**: All returned Z values and bounds must be in world space, accounting for `ObjectMesh.transform` (column-major 4x4 f64 matrix).

4. **Error handling**: Invalid inputs (bad object_id, bad facet_index) must return fatal wasmtime errors with specific codes — they must NOT panic or silently proceed.

5. **Four WIT worlds**: The `Host` trait is implemented separately in four `wit_world` modules inside `wit_host.rs`:
   - `layer` (world-layer@1.0.0)
   - `prepass` (world-prepass@1.0.0)
   - `finalization` (world-finalization@1.0.0)
   - `postpass` (world-postpass@1.0.0)
   All four must be updated to use the shared mesh-query logic.

## Code Change Surface

### Selected approach

Add a `mesh_ir: Option<MeshIR>` field to `HostExecutionContext`, plumb `MeshIR` through the dispatch path, and implement the three mesh-query functions against live data using the following algorithm:

**`raycast_z_down`**:
1. Look up `ObjectMesh` by `object_id` in `mesh_ir.objects`. If not found, return `OBJECT_NOT_FOUND` fatal error.
2. For each triangle in `IndexedTriangleSet`:
   a. Fetch the 3 vertex positions (indices into vertices array)
   b. Apply `ObjectMesh.transform` to each vertex to get world-space positions
   c. Skip triangle if all vertices have Z <= start_z (no hit possible)
   d. Compute ray-triangle intersection (ray origin at (x, y, start_z), direction (0, 0, -1))
   e. Keep the hit with the largest intersection Z (closest to start_z, still below it)
3. If a hit was found, return `Some(hit_z)` (world-space Z as f32)
4. Otherwise return `None`

**`surface_normal_at`**:
1. Look up `ObjectMesh` by `object_id`. If not found, return `OBJECT_NOT_FOUND`.
2. Compute triangle index: `triangle_index = facet_index * 3`. Verify `triangle_index + 2 < indices.len()`. If out of bounds, return `FACET_INDEX_OUT_OF_BOUNDS`.
3. Fetch the 3 vertex indices and their world-space positions (applying transform).
4. Compute cross product of edge vectors: `e1 = v1 - v0`, `e2 = v2 - v0`. `normal = e1.cross(e2)`.
5. Normalize to unit length. Return `Some(normal)`.

**`object_bounds`**:
1. Look up `ObjectMesh` by `object_id`. If not found, return `OBJECT_NOT_FOUND`.
2. For each vertex in `IndexedTriangleSet.vertices`, apply `ObjectMesh.transform`.
3. Track `min_x/max_x/min_y/max_y/min_z/max_z` across all transformed vertices.
4. Return `BoundingBox3 { min: Point3 { x: min_x, y: min_y, z: min_z }, max: Point3 { x: max_x, y: max_y, z: max_z } }`.

### Exact functions, traits, manifests, tests, or fixtures expected to change

- `crates/slicer-host/src/wit_host.rs`:
  - Add `mesh_ir: Option<MeshIR>` field to `HostExecutionContext` struct (~line 901)
  - Update `HostExecutionContext::new` to accept `mesh_ir: Option<MeshIR>`
  - Implement `raycast_z_down` (~line 1185-1198): replace `Ok(None)` stub with live mesh query
  - Implement `surface_normal_at` (~line 1200-1209): replace `Ok(None)` stub with live normal computation
  - Implement `object_bounds` (~line 1211-1217): replace error stub with live bounds computation
  - Update all four world trait implementations (`layer`, `prepass`, `finalization`, `postpass`) with the same logic
- `crates/slicer-host/src/dispatch.rs` (or wherever context is constructed):
  - Pass `mesh_ir` from the blackboard into `HostExecutionContext::new`
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

2. **Build a acceleration structure (BVH) upfront**: Rejected — the initial implementation should be correct before being optimized. A BVH can be added as a follow-up optimization task if profiling shows raycasting is a bottleneck. The current `IndexedTriangleSet` is small enough that brute-force intersection is acceptable for MVP.

3. **Return `Option<(f32, u32)>` from `raycast_z_down` instead of `Hit` record**: The WIT signature `raycast-z-down` returns `option<f32>`. The `facet_index` is internal to the host implementation but does not cross the WIT boundary under the current signature. This is explicitly out of scope for this packet. A separate future packet may address a WIT signature change to return a `hit` record with `z` and `facet-index` fields — that is a WIT extension task, not a wiring task.

   The current packet implements wiring to match the existing WIT signature: `raycast_z_down` returns `Option<f32>` (world-Z only). All acceptance criteria and tests are written against this behavior.

4. **Use `nalgeom` or similar for ray-triangle intersection**: Rejected — the math is simple enough (3 cross products) that adding an external dependency is not warranted. Implement directly using f32 arithmetic.

## Data and Contract Notes

### IR or manifest contracts touched

- `HostExecutionContext` gains a new `mesh_ir: Option<MeshIR>` field. The context is constructed per-call, so the `MeshIR` (which is permanent blackboard state) must be passed in at context construction time from the dispatch path.

### WIT boundary considerations

- The WIT signature for `raycast_z_down` returns `option<f32>` (just the Z). The `facet_index` is computed internally for hit selection but is NOT returned through the current WIT boundary.
- `surface_normal_at` returns `option<point3>` — already correct WIT signature.
- `object_bounds` returns `bounding-box3` — already correct WIT signature.

### Determinism or scheduler constraints

- Raycast result must be deterministic: given identical mesh, transform, and origin, the same Z must be returned. Floating-point ordering must be consistent.
- The algorithm finds the *closest* hit below `start_z`. If two triangles tie (co-planar), any deterministic tiebreak is acceptable (e.g., lower facet_index wins).

## Locked Assumptions and Invariants

- `ObjectMesh.transform` is a column-major 4x4 matrix. Transformation must be applied correctly to go from object-local to world space.
- Coordinates: X/Y in `Point2` use scaled integers (1 unit = 100nm), but 3D `Point3` uses millimeters (f32). The mesh vertices are in mm. `start_z` is in mm.
- `IndexedTriangleSet.indices` contains u32 indices, 3 per triangle. `indices.len() / 3` is the triangle count.
- `facet_index` in `surface_normal_at` is a triangle index (not a flat index). `facet_index * 3` gives the start of the triangle's 3 indices.

## Risks and Tradeoffs

1. **No BVH acceleration**: Brute-force O(triangle_count) raycasting is acceptable for meshes up to ~1M triangles (typical print models). If performance becomes an issue, a BVH can be added later.

2. **Transform handling complexity**: Incorrectly applying the transform would cause wrong Z values. The implementation must carefully distinguish object-local vs world space at each step.

3. **WIT signature mismatch**: The current WIT returns `option<f32>` for raycast. If the acceptance criteria truly require returning `Hit { z, facet_index }`, the WIT signature would need to change in a separate task. The current packet assumes the existing WIT signature is correct and tests verify Z correctness.

## Open Questions

1. **Resolved** (Q1 from prior draft): The WIT signature for `raycast_z_down` returns `option<f32>` (Z only). Changing this to return a `hit` record with `z` and `facet-index` is a WIT extension task, not in scope for this wiring packet. This packet implements against the current `option<f32>` WIT signature.

2. **Resolved** (Q2): `object_bounds` returns world-space bounding box (after transform), consistent with world-space Z semantics. This is confirmed correct.

3. **Resolved** (Q3): BVH acceleration is deferred. MVP uses brute-force O(triangle_count) iteration. BVH follow-up is tracked separately.
