# Task Map: transform-aware-world-z

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This file is required because the packet spans more than one task ID (TASK-157 and TASK-158) and reopens/supersedes/continues DEV-027.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Notes |
|---|---|---|---|---|
| `TASK-157` | `Step 1` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/model_loader.rs` | Discovery: inventory transform call sites and confirm all use world-space Z |
| `TASK-157` | `Step 2` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/translated_object_z_floor_tdd.rs` (new) | Translated object integration test: `translate(0,0,10mm)` → first layer z >= 10.0 |
| `TASK-157` | `Step 3` | `docs/02_ir_schemas.md`, `crates/slicer-host/src/mesh_analysis.rs` | `crates/slicer-host/tests/rotated_object_world_extent_tdd.rs` (new) | Rotated object integration test: `rotate_x(90deg)` world extent correctness |
| `TASK-157` | `Step 4` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/transformed_model_world_z_tdd.rs` (new) | General transformed model fixture test through full planning path |
| `TASK-157` | `Step 5` | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/multi_object_transform_world_z_tdd.rs` (new) | Multi-object LCM synchronization with different transforms |
| `TASK-158` | `Step 6` | `docs/02_ir_schemas.md` | `crates/slicer-ir/src/` or `docs/02_ir_schemas.md` | Canonical surface: Option A (IR field `ObjectMesh.world_z_extent`) or Option B (config-only documentation of `object_height:{id}` supply) |
| `TASK-158` | `Step 7` | `docs/02_ir_schemas.md` | `crates/slicer-host/tests/world_z_canonical_surface_tdd.rs` (new) | Regression lock proving canonical surface is used consistently |
| `TASK-157` / `TASK-158` | `Step 8` | `docs/08_coordinate_system.md` | `crates/slicer-host/tests/non_uniform_scale_tdd.rs` (new), `crates/slicer-host/src/model_loader.rs` | Negative test: `NON_UNIFORM_SCALE_UNSUPPORTED` at load time |
| `TASK-157` / `TASK-158` | `Step 9` | `docs/08_coordinate_system.md` | `crates/slicer-host/tests/world_z_below_floor_tdd.rs` (new), `crates/slicer-host/src/model_loader.rs` | Negative test: `WORLD_Z_BELOW_FLOOR` when world Z < 0 |
| `TASK-157` / `TASK-158` | `Step 10` | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md` | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md` | Workspace gate, clippy, DEV-027 closure, TASK-157/158 completion |

## DEV-027 Continuation Summary

AUDIT-21 confirmed that transform is now applied in both `mesh_analysis.rs` and `model_loader.rs`, but identified two remaining gaps:

1. **No integration fixture test** for non-identity transforms — owned by TASK-157 (Steps 2–5)
2. **World-space Z not a first-class IR field** — owned by TASK-158 (Steps 6–7)

The negative test cases (Steps 8–9) are shared between both task IDs as they validate the boundary conditions of the transform-aware system.

## Why This Step Distribution Is Sufficient Evidence for Each Task ID

- **TASK-157** ("fixture-level integration coverage for non-identity object transforms"): Steps 2–5 produce 5 integration tests that prove transformed STL/3MF inputs (via constructed `MeshIR` fixtures) exhibit correct world-space Z behavior through `LayerPlanIR` generation. This is the literal definition of fixture-level integration coverage for the transform-aware system.

- **TASK-158** ("promote world-space Z extent to one canonical derived contract surface"): Steps 6–7 implement the canonical surface (Option A or B) and add a regression lock test proving no silent use of object-local Z. This directly satisfies "promote to canonical derived contract surface."
