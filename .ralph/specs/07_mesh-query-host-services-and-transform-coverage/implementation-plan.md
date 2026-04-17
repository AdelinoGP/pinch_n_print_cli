# Implementation Plan: mesh-query-host-services-and-transform-coverage

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit current raycast_z_down stub

- Task IDs:
  - `TASK-147`
- Objective: Find the current `raycast_z_down` implementation and confirm it is a stub/trap. Identify the current return behavior.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — host-api.wit
  - `docs/02_ir_schemas.md` — MeshIR
- OrcaSlicer refs: Check `OrcaSlicerDocumented/` for raycast reference
- Verification: `grep -r "raycast_z_down" crates/`

### Step 2: Implement raycast_z_down with real mesh data

- Task IDs:
  - `TASK-147`
- Objective: Wire `raycast_z_down` to real `MeshIR` triangle data. Implement ray-mesh intersection, apply `ObjectMesh.transform`, return world-space Z or `None` on miss.
- Files expected to change:
  - `crates/slicer-host/src/host-services/mesh_query.rs` (or similar)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — MeshIR, ObjectMesh, Transform3d
  - `docs/03_wit_and_manifest.md` — host-api.wit
- OrcaSlicer refs: None (or cite if found)
- Verification: `cargo test --package slicer-host --test raycast_z_down_hit_miss -- --nocapture`

### Step 3: Audit current surface_normal_at and object_bounds stubs

- Task IDs:
  - `TASK-148`
- Objective: Find the current `surface_normal_at` and `object_bounds` implementations and confirm they are stubs.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — host-api.wit
- OrcaSlicer refs: Check `OrcaSlicerDocumented/`
- Verification: `grep -r "surface_normal_at\|object_bounds" crates/`

### Step 4: Implement surface_normal_at

- Task IDs:
  - `TASK-148`
- Objective: Implement `surface_normal_at` using the same mesh-query backing surface as `raycast_z_down`. Return world-space surface normal or `None` when point is off-mesh.
- Files expected to change:
  - `crates/slicer-host/src/host-services/mesh_query.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — MeshIR, Transform3d
  - `docs/03_wit_and_manifest.md` — host-api.wit
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test surface_normal_at -- --nocapture`

### Step 5: Implement object_bounds

- Task IDs:
  - `TASK-148`
- Objective: Implement `object_bounds` returning the world-space AABB of all triangles in the object (apply `ObjectMesh.transform` to all vertices).
- Files expected to change:
  - `crates/slicer-host/src/host-services/mesh_query.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — MeshIR, ObjectMesh, BoundingBox3
  - `docs/03_wit_and_manifest.md` — host-api.wit
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test object_bounds -- --nocapture`

### Step 6: Add mesh query BVH for performance (if needed)

- Task IDs:
  - `TASK-147`
  - `TASK-148`
- Objective: If the mesh query implementation uses linear scan, add a BVH (bounding volume hierarchy) for O(log n) raycast and surface normal queries. Check if `crates/slicer-core/` has relevant geometry utilities.
- Files expected to change:
  - `crates/slicer-host/src/host-services/mesh_query.rs`
  - `crates/slicer-core/src/geometry/` (if BVH lives there)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Performance targets (raycast O(log n) preferred)
- OrcaSlicer refs: None
- Verification: Performance benchmark comparing linear scan vs. BVH (optional if already O(log n))

### Step 7: Add non-identity transform integration test

- Task IDs:
  - `TASK-157`
- Objective: Add a fixture-level test that uses an STL with non-identity transform (rotation, translation, non-uniform scale) and verifies world-space Z behavior is correct through the full planning pipeline.
- Files expected to change:
  - `crates/slicer-host/tests/transform_world_space_z_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — MeshIR, Transform3d
  - `docs/08_coordinate_system.md` — World-space Z rules
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test transform_world_space_z -- --nocapture`

### Step 8: Define and regression-lock world-space Z canonical surface

- Task IDs:
  - `TASK-158`
- Objective: Determine if world-space Z extent should be a first-class IR field or documented config-only behavior. If IR, add the field; if config-only, document clearly in `docs/08_coordinate_system.md`. Regression-lock with the transform test from Step 7.
- Files expected to change:
  - `crates/slicer-ir/src/` (if adding IR field) or `docs/08_coordinate_system.md` (if documenting)
- Authoritative docs:
  - `docs/08_coordinate_system.md` — Coordinate scaling and world-space Z
  - `docs/02_ir_schemas.md` — MeshIR
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test transform_world_space_z -- --nocapture` passes.

### Step 9: Full test suite verification

- Task IDs:
  - `TASK-147`
  - `TASK-148`
  - `TASK-157`
  - `TASK-158`
- Objective: Run all mesh query and transform tests and confirm they all pass.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/08_coordinate_system.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- --nocapture` — all mesh query and transform tests pass.

## Packet Completion Gate

- `raycast_z_down` implemented with hit/miss semantics, returning world-space Z.
- `surface_normal_at` implemented with correct world-space normal.
- `object_bounds` implemented with correct world-space AABB.
- Non-identity transform integration test passes with correct world-space Z behavior.
- World-space Z canonical surface defined or documented and regression-locked.
- All related tests pass.
- `docs/07_implementation_status.md` TASK-147/148/157/158 marked complete.
- `packet.spec.md` ready to move to `status: implemented`.