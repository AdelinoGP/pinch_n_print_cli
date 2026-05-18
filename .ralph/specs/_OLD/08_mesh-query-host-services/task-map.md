# Task Map: mesh-query-host-services

Use this file because the packet spans two task IDs and has a cross-packet dependency.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-147`, `TASK-148` | Step 1 (create the 7 TDD files) | `wit/host-api.wit`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `crates/slicer-host/tests/*.rs` | None | Create the seven dedicated mesh-query TDD files before host logic changes. |
| `TASK-147`, `TASK-148` | Step 2 (mesh_ir field + dispatch plumbing) | `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/blackboard.rs` | None | Add `mesh_ir: Option<Arc<MeshIR>>` to `HostExecutionContext` and thread `Blackboard::mesh()` through every host-service dispatch entrypoint. |
| `TASK-147` | Step 3 (raycast_z_down hit/miss) | `docs/01_system_architecture.md`, `docs/08_coordinate_system.md` | `crates/slicer-host/src/wit_host.rs` | None | Replace `Ok(None)` stub with shared live mesh query. Returns `Some(world_z)` on hit, `None` on miss. |
| `TASK-148` | Step 4 (surface_normal_at coordinate query) | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs` | None | Replace `Ok(None)` stub. Returns unit-length normal for queried on-surface world-space points and `None` when the point is off-surface. |
| `TASK-148` | Step 5 (object_bounds shared backing surface) | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs` | None | Replace error stub. Returns world-space `BoundingBox3` after transform. Invalid object → `OBJECT_NOT_FOUND`. |
| `TASK-147`, `TASK-148` | Step 6 (transform and invalid-object coverage across all worlds) | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs`, `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs`, `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs` | None | Prove world-space Z for transformed meshes, invalid-object diagnostics, and off-surface `surface_normal_at` behavior while confirming all four WIT worlds use the shared helper. |
| `TASK-147`, `TASK-148` | Step 7 (workspace gate) | — | Workspace-wide | None | `cargo build --workspace && cargo clippy --workspace -- -D warnings`. |
