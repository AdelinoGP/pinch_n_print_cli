# Implementation Plan: mesh-query-host-services

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-147, TASK-148).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Create the 7 mesh-query TDD files

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Create the seven dedicated `crates/slicer-host/tests/*_tdd.rs` files referenced by the packet so implementation can proceed TDD-first against real acceptance commands.
- Precondition: The packet references seven dedicated TDD files, but they do not exist on disk.
- Postcondition: All seven test files exist, compile, and each names the exact acceptance or negative case it will guard.
- Files expected to change:
  - `crates/slicer-host/tests/raycast_z_down_hit_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_miss_tdd.rs`
  - `crates/slicer-host/tests/surface_normal_at_unit_length_tdd.rs`
  - `crates/slicer-host/tests/object_bounds_transform_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs`
  - `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs`
- Authoritative docs: `wit/host-api.wit`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --tests --no-run`
- Exit condition: The seven files exist under `crates/slicer-host/tests/` and compile into test binaries.

---

### Step 2: Add mesh_ir field to HostExecutionContext and thread blackboard mesh through dispatch

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Add `mesh_ir: Option<Arc<MeshIR>>` field to `HostExecutionContext`, update the constructor to accept it, and thread `blackboard.mesh().clone()` into every dispatch path that constructs a host execution context. Create a shared helper for mesh-query logic that all four WIT world implementations can call.
- Precondition: `HostExecutionContext` has no mesh data; `raycast_z_down`, `surface_normal_at`, and `object_bounds` all return stubs
- Postcondition: Context has `mesh_ir` field; `dispatch_layer_call`, `dispatch_prepass_call`, `dispatch_finalization_call`, `dispatch_postpass_gcode_call`, and `dispatch_postpass_text_call` all construct `HostExecutionContext` with blackboard mesh data; shared mesh-query helper exists
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — add field and constructor update
  - `crates/slicer-host/src/dispatch.rs` — plumb `Blackboard::mesh()` from every runner boundary into context construction
- Authoritative docs: `docs/02_ir_schemas.md` (MeshIR, ObjectMesh, IndexedTriangleSet)
- OrcaSlicer refs: None
- Verification: `rg -n "mesh_ir: Option<Arc<MeshIR>>|pub fn new\(" crates/slicer-host/src/wit_host.rs` and `rg -n "HostExecutionContext::new" crates/slicer-host/src/dispatch.rs`
- Exit condition: `HostExecutionContext` stores `Option<Arc<MeshIR>>`, the constructor accepts it, and every dispatch entrypoint is ready to supply mesh data.

---

### Step 3: Implement raycast_z_down with hit/miss semantics across the shared helper

- Task IDs: `TASK-147`
- Objective: Replace the `Ok(None)` stub for `raycast_z_down` with live shared mesh query logic:
  1. Look up `ObjectMesh` by `object_id` in `mesh_ir.objects`. Return `OBJECT_NOT_FOUND` if not found.
  2. Iterate all triangles; apply world transform to vertices; compute ray-triangle intersection with ray direction `(0, 0, -1)` from `(x, y, start_z)`.
  3. Track the closest hit below `start_z`.
  4. Return `Some(world_z)` on hit, `None` on miss.
- Precondition: Steps 1-2 complete; `HostExecutionContext` has `mesh_ir` field
- Postcondition: `raycast_z_down` returns correct Z or `None` through the shared helper; tests for hit and miss both pass
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — shared helper and world `raycast_z_down` impls
- Authoritative docs: `docs/01_system_architecture.md` (Host Services), `docs/02_ir_schemas.md` (Transform3d, IndexedTriangleSet), `docs/08_coordinate_system.md` (Z in mm)
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test raycast_z_down_hit_tdd -- --nocapture` and `cargo test -p slicer-host --test raycast_z_down_miss_tdd -- --nocapture`
- Exit condition: `raycast_z_down` returns correct `Some(world_z)` on hit and `None` on miss for the shared host-service surface.

---

### Step 4: Implement surface_normal_at for coordinate-based world-space queries

- Task IDs: `TASK-148`
- Objective: Replace the `Ok(None)` stub for `surface_normal_at` with live normal computation for coordinate-based world-space queries:
  1. Look up `ObjectMesh` by `object_id`. Return `OBJECT_NOT_FOUND` if not found.
  2. Iterate transformed triangles and find one whose plane and barycentric footprint contain the queried point `(x, y, z)` within a small epsilon.
  3. Compute cross product for the matching triangle normal, normalize to unit length.
  4. Return `Some(normal)` for a match or `None` when the point is off-surface.
- Precondition: Steps 1-3 complete; raycast works
- Postcondition: `surface_normal_at` returns a unit-length normal for on-surface points and `None` for off-surface points
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — shared helper and world `surface_normal_at` impls
- Authoritative docs: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd -- --nocapture` and `cargo test -p slicer-host --test surface_normal_at_oob_tdd -- --nocapture`
- Exit condition: `surface_normal_at` returns unit-length normal (magnitude within `1e-6` of `1.0`) and returns `None` for off-surface coordinates.

---

### Step 5: Implement object_bounds on the same shared backing surface

- Task IDs: `TASK-148`
- Objective: Replace the error stub for `object_bounds` with live bounds computation:
  1. Look up `ObjectMesh` by `object_id`. Return `OBJECT_NOT_FOUND` if not found.
  2. For each vertex, apply world transform; track min/max in x, y, z.
  3. Return `BoundingBox3 { min, max }` with `min_z <= max_z`.
- Precondition: Steps 1-4 complete
- Postcondition: `object_bounds` returns correct world-space bounding box including transform
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — shared helper and world `object_bounds` impls
- Authoritative docs: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test object_bounds_transform_tdd -- --nocapture`
- Exit condition: `object_bounds` returns world-space bounds; `min_z <= max_z`

---

### Step 6: Prove transformed-object and invalid-object behavior across the wired worlds

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Finish validation coverage for transformed-object raycast behavior and invalid-object diagnostics, and confirm the same shared helper is used from `layer`, `prepass`, `finalization`, and `postpass` host-service implementations.
- Precondition: Steps 1-5 complete
- Postcondition: Transform-aware raycast and invalid-object coverage pass, and all four WIT worlds dispatch through the shared helper surface
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — prepass, finalization, postpass world impls
  - `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs`
  - `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs`
  - `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs`
- Authoritative docs: `crates/slicer-host/src/wit_host.rs`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo test -p slicer-host --test raycast_z_down_transformed_object_tdd -- --nocapture` and `cargo test -p slicer-host --test raycast_z_down_invalid_object_tdd -- --nocapture`
- Exit condition: Transform-aware raycast behavior and invalid-object diagnostics are verified, and no WIT world remains on the stub path.

---

### Step 7: Workspace build and clippy gate

- Task IDs: `TASK-147`, `TASK-148`
- Objective: Run full workspace build and clippy to confirm no regressions.
- Precondition: Steps 1-6 complete; all 7 TDD tests pass
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
- Confirm invalid-object diagnostics and off-surface `None` behavior match the packet contract.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
