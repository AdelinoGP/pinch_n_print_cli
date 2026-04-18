# Implementation Plan: mesh-query-host-services

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-147, TASK-148).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add mesh_ir field to HostExecutionContext

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Add `mesh_ir: Option<MeshIR>` field to `HostExecutionContext` and update the constructor to accept it. Create a shared helper struct/module for mesh-query logic that all four WIT world implementations can call.
- Precondition: `HostExecutionContext` has no mesh data; `raycast_z_down`, `surface_normal_at`, `object_bounds` all return stubs
- Postcondition: Context has `mesh_ir` field; dispatch path passes `mesh_ir` from blackboard to context; shared mesh-query helper exists
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — add field and constructor update
  - `crates/slicer-host/src/dispatch.rs` (or equivalent) — plumb MeshIR from blackboard to context
- Authoritative docs: `docs/02_ir_schemas.md` (MeshIR, ObjectMesh, IndexedTriangleSet)
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host 2>&1 | grep -E "error|warning:.*mesh_ir"` — should compile without errors about missing field
- Exit condition: `HostExecutionContext` has `mesh_ir: Option<MeshIR>`; constructor accepts it; no yet-called dispatch wiring changes required (Step 2 will handle wiring)

---

### Step 2: Implement raycast_z_down with hit/miss semantics (world-layer)

- Task IDs: `TASK-147`
- Objective: Replace the `Ok(None)` stub for `raycast_z_down` in the `layer` world `Host` impl with live mesh query logic:
  1. Look up `ObjectMesh` by `object_id` in `mesh_ir.objects`. Return `OBJECT_NOT_FOUND` if not found.
  2. Iterate all triangles; apply world transform to vertices; compute ray-triangle intersection with ray direction (0, 0, -1) from (x, y, start_z).
  3. Track the closest hit below `start_z`.
  4. Return `Some(world_z)` on hit, `None` on miss.
- Precondition: Step 1 complete; `HostExecutionContext` has `mesh_ir` field
- Postcondition: `raycast_z_down` returns correct Z or None; tests for hit and miss both pass
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — layer world `raycast_z_down` impl
- Authoritative docs: `docs/01_system_architecture.md` (Host Services), `docs/02_ir_schemas.md` (Transform3d, IndexedTriangleSet), `docs/08_coordinate_system.md` (Z in mm)
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test raycast_z_down_hit_tdd -- --nocapture` and `cargo test -p slicer-host --test raycast_z_down_miss_tdd -- --nocapture`
- Exit condition: `raycast_z_down` returns correct `Some(world_z)` on hit and `None` on miss for the layer world

---

### Step 3: Implement surface_normal_at (world-layer)

- Task IDs: `TASK-148`
- Objective: Replace the `Ok(None)` stub for `surface_normal_at` in the `layer` world with live normal computation:
  1. Look up `ObjectMesh` by `object_id`. Return `OBJECT_NOT_FOUND` if not found.
  2. Validate `facet_index * 3 + 2 < indices.len()`. Return `FACET_INDEX_OUT_OF_BOUNDS` if out of range.
  3. Fetch 3 vertex indices, get world-space positions via transform.
  4. Compute cross product for normal, normalize to unit length.
  5. Return `Some(normal)`.
- Precondition: Step 2 complete; raycast works
- Postcondition: `surface_normal_at` returns unit-length normal perpendicular to facet plane
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — layer world `surface_normal_at` impl
- Authoritative docs: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd -- --nocapture`
- Exit condition: `surface_normal_at` returns unit-length normal (magnitude within 1e-6 of 1.0)

---

### Step 4: Implement object_bounds (world-layer)

- Task IDs: `TASK-148`
- Objective: Replace the error stub for `object_bounds` in the `layer` world with live bounds computation:
  1. Look up `ObjectMesh` by `object_id`. Return `OBJECT_NOT_FOUND` if not found.
  2. For each vertex, apply world transform; track min/max in x, y, z.
  3. Return `BoundingBox3 { min, max }` with `min_z <= max_z`.
- Precondition: Steps 2-3 complete
- Postcondition: `object_bounds` returns correct world-space bounding box including transform
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — layer world `object_bounds` impl
- Authoritative docs: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test object_bounds_transform_tdd -- --nocapture`
- Exit condition: `object_bounds` returns world-space bounds; `min_z <= max_z`

---

### Step 5: Wire all four WIT world implementations

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Update the `prepass`, `finalization`, and `postpass` world `Host` trait implementations to use the same mesh-query logic as the layer world. All four worlds share the sameWIT type signatures for these functions, so the implementations should be identical (or call a shared helper).
- Precondition: Steps 2-4 complete for layer world
- Postcondition: All four WIT worlds return correct results from mesh queries
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — prepass, finalization, postpass world impls
- Authoritative docs: `crates/slicer-host/src/wit_host.rs` (search for `impl.*hs::Host.*for.*HostExecutionContext` to find all four impls)
- OrcaSlicer refs: None
- Verification: All seven TDD tests pass
- Exit condition: All four world implementations use live mesh-query logic

---

### Step 6: Add transformed-object test coverage

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Add test proving that `raycast_z_down` returns world-space Z (accounting for object transform) not object-local Z. Create a mesh with known triangle at local Z=5.0mm but world Z=15.0mm (e.g., object transform has +10mm Z translation). Verify raycast from above returns 15.0mm not 5.0mm.
- Precondition: Steps 2-5 complete
- Postcondition: Transformed-object test passes
- Files expected to change:
  - `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs` (new file)
- Authoritative docs: `docs/02_ir_schemas.md` (Transform3d column-major format)
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test raycast_z_down_transformed_object_tdd -- --nocapture`
- Exit condition: Transformed mesh returns world-space Z, not local Z

---

### Step 7: Add error code coverage tests

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Add two negative tests:
  1. `raycast_z_down_invalid_object_tdd`: call with invalid object_id, verify fatal error with `OBJECT_NOT_FOUND`
  2. `surface_normal_at_oob_tdd`: call with facet_index >= triangle_count, verify fatal error with `FACET_INDEX_OUT_OF_BOUNDS`
- Precondition: Steps 2-5 complete
- Postcondition: Both error tests pass
- Files expected to change:
  - `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs` (new file)
  - `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs` (new file)
- Authoritative docs: `docs/03_wit_and_manifest.md` (error handling at WIT boundary)
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test raycast_z_down_invalid_object_tdd -- --nocapture`; `cargo test -p slicer-host --test surface_normal_at_oob_tdd -- --nocapture`
- Exit condition: Both error code tests pass

---

### Step 8: Workspace build and clippy gate

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Run full workspace build and clippy to confirm no regressions.
- Precondition: Steps 1-7 complete; all 7 TDD tests pass
- Postcondition: `cargo build --workspace` succeeds; `cargo clippy --workspace -- -D warnings` passes with zero warnings
- Files expected to change: None (verification only)
- Authoritative docs: None
- OrcaSlicer refs: None
- Verification: `cargo build --workspace && cargo clippy --workspace -- -D warnings`
- Exit condition: Full workspace build and clippy pass

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- All 7 TDD tests pass:
  - `raycast_z_down_hit_tdd`
  - `raycast_z_down_miss_tdd`
  - `surface_normal_at_unit_length_tdd`
  - `object_bounds_transform_tdd`
  - `raycast_z_down_transformed_object_tdd`
  - `raycast_z_down_invalid_object_tdd`
  - `surface_normal_at_oob_tdd`
- `cargo build --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes with zero warnings.
- All acceptance criteria from `packet.spec.md` verified.
- `docs/07_implementation_status.md` updated: TASK-147, TASK-148 marked complete.
- `packet.spec.md` status updated to `implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all 7 TDD tests pass with correct output assertions.
- Confirm full workspace build and clippy are green.
- Confirm error code tests return the correct fatal error codes.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
