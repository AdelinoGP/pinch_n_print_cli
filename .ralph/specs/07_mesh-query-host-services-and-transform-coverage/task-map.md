# Task Map: mesh-query-host-services-and-transform-coverage

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-147` | Steps 1–2, 6 | `docs/02_ir_schemas.md` (MeshIR) | `crates/slicer-host/src/host-services/mesh_query.rs` | Check OrcaSlicerDocumented/ | raycast_z_down live wiring |
| `TASK-148` | Steps 3–5, 6 | `docs/02_ir_schemas.md` (MeshIR) | `crates/slicer-host/src/host-services/mesh_query.rs` | None | surface_normal_at and object_bounds |
| `TASK-157` | Step 7 | `docs/08_coordinate_system.md` | `crates/slicer-host/tests/transform_world_space_z_tdd.rs` | None | Non-identity transform integration |
| `TASK-158` | Step 8 | `docs/08_coordinate_system.md` | `crates/slicer-ir/src/` or `docs/08_coordinate_system.md` | None | World-space Z canonical surface |