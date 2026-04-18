# Task Map: mesh-query-host-services

Use this file because the packet spans two task IDs and has a cross-packet dependency.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-147`, `TASK-148` | Step 1 (mesh_ir field + helper) | `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs` — `HostExecutionContext` struct + constructor; shared mesh-query helper module | None | Add `mesh_ir: Option<MeshIR>` field and shared query logic. Self-contained — no WIT changes needed. |
| `TASK-147` | Step 2 (raycast_z_down hit/miss — world-layer) | `docs/01_system_architecture.md`, `docs/08_coordinate_system.md` | `crates/slicer-host/src/wit_host.rs` — layer world `raycast_z_down` impl (~line 1185) | None | Replace `Ok(None)` stub with live mesh query. Returns `Some(world_z)` on hit, `None` on miss. |
| `TASK-148` | Step 3 (surface_normal_at — world-layer) | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs` — layer world `surface_normal_at` impl (~line 1200) | None | Replace `Ok(None)` stub. Returns unit-length normal perpendicular to facet. OOB → `FACET_INDEX_OUT_OF_BOUNDS`. |
| `TASK-148` | Step 4 (object_bounds — world-layer) | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs` — layer world `object_bounds` impl (~line 1211) | None | Replace error stub. Returns world-space BoundingBox3 after transform. Invalid object → `OBJECT_NOT_FOUND`. |
| `TASK-147`, `TASK-148` | Step 5 (all four WIT worlds) | `crates/slicer-host/src/wit_host.rs` | `crates/slicer-host/src/wit_host.rs` — prepass, finalization, postpass world impls | None | Same mesh-query logic wired into all four `Host` trait implementations. |
| `TASK-147` | Step 6 (transformed-object test) | `docs/02_ir_schemas.md` (Transform3d) | `crates/slicer-host/tests/raycast_z_down_transformed_object_tdd.rs` | None | Prove world-space Z is returned, not object-local Z. |
| `TASK-147`, `TASK-148` | Step 7 (error code tests) | `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/raycast_z_down_invalid_object_tdd.rs`, `crates/slicer-host/tests/surface_normal_at_oob_tdd.rs` | None | `OBJECT_NOT_FOUND` and `FACET_INDEX_OUT_OF_BOUNDS` error codes. |
| `TASK-147`, `TASK-148` | Step 8 (workspace gate) | — | Workspace-wide | None | `cargo build --workspace && cargo clippy --workspace -- -D warnings`. |
