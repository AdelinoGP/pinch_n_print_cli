# Requirements: mesh-query-host-services

## Packet Metadata

- Grouped task IDs:
  - `TASK-147`
  - `TASK-148`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The mesh-query host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`) are currently stubs that return `None` or trap with a diagnostic message. They need to be wired to live `MeshIR` data so modules can query mesh geometry at runtime. This is blocking non-planar surface projection, seam placement heuristics, and any module that needs surface normal or bounds information.

The three functions share a common backing surface: they all must look up an `ObjectMesh` by `ObjectId` in `MeshIR.objects`, apply the world transform, and then perform geometry queries on the `IndexedTriangleSet`.

## In Scope

- Wiring `raycast_z_down` to live mesh data with correct hit/miss semantics across all four WIT worlds
- Returning `Some(world_z: f32)` on hit and `None` on miss, matching the current `option<f32>` WIT signature
- Wiring `surface_normal_at(object_id, x, y, z)` to live mesh data, returning unit-length normals for queried world-space surface points
- Wiring `object_bounds` to live mesh data, returning world-space bounding box
- Handling non-identity object transforms (rotation, translation) in all three functions
- Error code `OBJECT_NOT_FOUND` for invalid object lookup and deterministic `None` for off-surface coordinate queries
- Seven dedicated TDD test files for all positive and negative cases, created before host logic changes

## Out of Scope

- Prepass segmentation SDK inputs (TASK-128)
- Postpass WIT gap coverage (TASK-129)
- Modifier volume queries
- Non-planar Z envelope enforcement (TASK-127)
- Fixture-level transform integration (TASK-157, TASK-158)
- Facet-index-returning mesh-query APIs or WIT signature changes

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md`
- `docs/08_coordinate_system.md`

## OrcaSlicer Reference Obligations

None. This is internal host-service wiring, not geometry algorithm porting.

## Acceptance Summary

### Positive Cases

- `raycast_z_down` returns `Some(world_z)` when triangles exist below the origin Z
- `raycast_z_down` returns `None` when no triangles exist below the origin Z
- `surface_normal_at` returns a `Point3` with unit length (within `1e-6`) and perpendicular to the containing triangle plane at the queried world-space point
- `object_bounds` returns `BoundingBox3` where `min_z <= max_z` after transform
- Transformed objects return world-space Z values (not object-local)

### Negative Cases

- Invalid `object_id` returns fatal error with code `OBJECT_NOT_FOUND`
- `surface_normal_at` returns `None` for a point outside the transformed mesh surface

### Measurable Outcomes

- 7 new TDD test files are created in `crates/slicer-host/tests/` before host mesh-query logic changes:
  - `raycast_z_down_hit_tdd`
  - `raycast_z_down_miss_tdd`
  - `surface_normal_at_unit_length_tdd`
  - `object_bounds_transform_tdd`
  - `raycast_z_down_transformed_object_tdd`
  - `raycast_z_down_invalid_object_tdd`
  - `surface_normal_at_oob_tdd`
- All 7 tests pass
- `cargo build --workspace` succeeds
- `cargo clippy --workspace -- -D warnings` passes

### Cross-Packet Impact

- Unblocks TASK-157 (transform-aware fixture integration)
- Unblocks TASK-128a (usable MeshSegmentation inputs via real mesh queries)

## Verification Commands

- `cargo test -p slicer-host --test raycast_z_down_hit_tdd 2>&1 | grep -E "raycast.*hit|world_z|Some\("`
- `cargo test -p slicer-host --test raycast_z_down_miss_tdd 2>&1 | grep -E "raycast.*miss|None|no.*hit"`
- `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd 2>&1 | grep -E "unit.*length|normal.*1\.0|magnitude"`
- `cargo test -p slicer-host --test object_bounds_transform_tdd 2>&1 | grep -E "BoundingBox|world.*transform|min_z.*max_z"`
- `cargo test -p slicer-host --test raycast_z_down_transformed_object_tdd 2>&1 | grep -E "world.*space|transformed.*raycast"`
- `cargo test -p slicer-host --test raycast_z_down_invalid_object_tdd 2>&1 | grep -E "OBJECT_NOT_FOUND|fatal"`
- `cargo test -p slicer-host --test surface_normal_at_oob_tdd 2>&1 | grep -E "outside.*surface|None|no.*normal"`
- `cargo build --workspace && cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

### Step 1: MeshQuery backing surface
- Precondition: `HostExecutionContext` has no mesh data field
- Postcondition: `HostExecutionContext` has a `mesh_ir: Option<Arc<MeshIR>>` field and a constructor that accepts it, and all dispatcher entrypoints thread blackboard mesh into the context
- Falsifying check: Structural search does not show the `mesh_ir` field or constructor parameter in `wit_host.rs`, or dispatch entrypoints still construct `HostExecutionContext` without mesh data

### Step 2: raycast_z_down hit/miss wiring
- Precondition: Function returns `Ok(None)` unconditionally
- Postcondition: Function returns `Ok(Some(world_z))` on hit, `Ok(None)` on miss
- Falsifying check: TDD test for miss returns `Some` instead of `None`

### Step 3: surface_normal_at wiring
- Precondition: Function returns `Ok(None)` unconditionally
- Postcondition: Function returns `Ok(Some(Point3))` with unit-length normal for a queried world-space point on the transformed surface
- Falsifying check: Normal magnitude is not within `1e-6` of `1.0`

### Step 4: object_bounds wiring
- Precondition: Function returns `Err(wasmtime::Error::msg(...not yet wired...))`
- Postcondition: Function returns `Ok(BoundingBox3)` with correct world-space bounds
- Falsifying check: Bounds do not account for transform

### Step 5: All WIT world coverage
- Precondition: Only one host world implementation is updated
- Postcondition: All four world implementations (`world_layer`, `world_prepass`, `world_finalization`, `world_postpass`) return correct results through the shared helper
- Falsifying check: Any non-layer world still returns stub behavior

### Step 6: Error and off-surface coverage
- Precondition: Invalid object lookup returns the wrong error or `surface_normal_at` fabricates normals away from the mesh
- Postcondition: `OBJECT_NOT_FOUND` is returned correctly and off-surface coordinate queries return `None`
- Falsifying check: Wrong error code, panic on invalid input, or off-surface point returns `Some(...)`

### Step 7: Workspace gate
- Precondition: Some code path has clippy warnings or build errors
- Postcondition: `cargo build --workspace && cargo clippy --workspace -- -D warnings` passes
- Falsifying check: Build failure or clippy warning
