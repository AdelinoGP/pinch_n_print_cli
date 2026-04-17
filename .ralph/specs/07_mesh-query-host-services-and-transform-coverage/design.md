# Design: mesh-query-host-services-and-transform-coverage

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/host-services/` or `crates/slicer-host/src/mesh/` (mesh query implementation)
- Neighboring tests or fixtures: `crates/slicer-host/tests/mesh_query_services_tdd.rs` (to be added), transformed STL/3MF fixture
- OrcaSlicer comparison surface: Check `OrcaSlicerDocumented/` for raycast/surface-normal reference

## Architecture Constraints

- Mesh query functions are host-services exposed via `host-api.wit`. They must read from `MeshIR` which is host-owned and never serialized to WASM.
- All three functions must use the same mesh-query backing surface to ensure consistency.
- `Transform3d` from `MeshIR.ObjectMesh.transform` must be applied to convert local-space hits to world-space coordinates.
- World-space Z for transformed objects must account for the full transform, not assume identity.

## Proposed Changes

### TASK-147 — raycast_z_down Implementation

1. **Audit current stub**: Find the current `raycast_z_down` implementation and confirm it is a stub/trap.
2. **Implement ray-mesh intersection**: Use the `MeshIR` triangle data to perform raycast. Start from `start-z` along -Z direction. Return `Some(z)` where the ray first hits a triangle.
3. **Apply object transform**: Convert local-space hit point to world-space using `ObjectMesh.transform`. Return world-space Z.
4. **Handle miss**: Return `None` when no intersection is found.
5. **Cover hit/miss across WIT worlds**: Ensure behavior is consistent for all WIT worlds that expose `raycast_z_down` (likely world-prepass, world-layer).

### TASK-148 — surface_normal_at and object_bounds Implementation

6. **Implement `surface_normal_at`**: Find the triangle containing `(x, y, z)` (or nearest to it), compute the face normal, transform to world space, return `Some(point3)`. Return `None` if point is not on mesh surface.
7. **Implement `object_bounds`**: Compute the axis-aligned bounding box of all triangles in world space (apply `ObjectMesh.transform` to all vertices), return `bounding-box3`.
8. **Use same backing surface as raycast**: Both `raycast_z_down` and `surface_normal_at` should use the same mesh spatial index (e.g., BVH) for performance.

### TASK-157 — Transform Integration Coverage

9. **Add transformed STL fixture test**: Create or use an existing STL with a non-identity transform (rotation, translation, or non-uniform scale). Run through the full slice pipeline.
10. **Assert world-space Z correctness**: At each stage that uses Z (LayerPlanning, Slice, etc.), verify that world-space Z values are consistent with the transformed mesh.
11. **Cover multiple transform types**: Test rotation, translation, and non-uniform scale separately.

### TASK-158 — World-Space Z Canonical Contract Surface

12. **Define the canonical surface**: Determine if world-space Z extent should be a first-class IR field (e.g., `ObjectMesh.world_space_z_extent`) or documented as config-only behavior. If IR, add it; if config-only, document clearly in `docs/08_coordinate_system.md`.
13. **Regression-lock**: Add a regression test that uses transformed objects and asserts the canonical surface holds correctly.

## Data and Contract Notes

- `raycast_z_down` signature: `fn raycast_z_down(object-id: string, x: f32, y: f32, start-z: f32) -> option<f32>`
- `surface_normal_at` signature: `fn surface_normal_at(object-id: string, x: f32, y: f32, z: f32) -> option<point3>`
- `object_bounds` signature: `fn object_bounds(object-id: string) -> bounding-box3`
- All three must apply `ObjectMesh.transform` (Transform3d, column-major f64) to convert local to world space.
- `Transform3d` in MeshIR: column-major 4x4 matrix. Point transformation: `world = transform * local`.

## Risks and Tradeoffs

- Mesh query performance matters for hot paths. Consider building a BVH (bounding volume hierarchy) over the mesh triangles for O(log n) raycast instead of O(n) linear scan.
- Non-uniform scale in transforms may affect normal computation. Ensure normals are correctly transformed (use inverse-transpose of rotation/scale part).

## Open Questions

- Does a BVH or spatial index already exist over the mesh for other purposes? Check `crates/slicer-core/` for geometry utilities.
- Is `Transform3d` column-major or row-major in the IR? Check `crates/slicer-ir/src/` for the definition.
- Does OrcaSlicer have documented behavior for raycast or surface normal that we should reference? Check `OrcaSlicerDocumented/`.