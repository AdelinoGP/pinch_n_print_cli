---
status: implemented
packet: transform-aware-world-z
task_ids:
  - TASK-157
  - TASK-158
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: transform-aware-world-z

## Goal

Add fixture-level integration coverage for non-identity object transforms proving correct world-space Z behavior through `PrePass::LayerPlanning`, and promote world-space Z extent to one canonical derived contract surface (either as a first-class IR field on `ObjectMesh` or as explicitly documented config-only behavior). Regression-lock transformed-object behavior. Closes DEV-027.

## Scope Boundaries

- In scope:
  - TASK-157: Integration test fixtures and TDD harness proving world-space Z correctness for translated, rotated, and scaled objects through the full `LayerPlanIR` generation path
  - TASK-157: `transformed_model_world_z_tdd`, `translated_object_z_floor_tdd`, `rotated_object_world_extent_tdd`, `multi_object_transform_world_z_tdd` test files under `crates/slicer-host/tests/`
  - TASK-158: Canonical world-space Z contract surface — either `ObjectMesh.world_z_extent: Option<(f32, f32)>` as a derived IR field (computed once at load time, cached), or documented config-only behavior with `object_height:{id}` keys as the canonical supply
  - TASK-158: Regression-lock fixture proving no silent use of object-local Z anywhere in the planning path
  - Negative test cases for non-uniform scale rejection (`NON_UNIFORM_SCALE_UNSUPPORTED`) and world-space Z below floor (`WORLD_Z_BELOW_FLOOR`)

- Out of scope:
  - Mesh-query host services (`raycast_z_down`, `surface_normal_at`) — tracked in TASK-147/148
  - Path-optimization behavior
  - Benchy parity — tracked in TASK-120 series
  - Changes to the 3MF/STL parser to apply transforms to mesh geometry at load time (transform is stored on `ObjectMesh.transform`, applied at query time)

## Prerequisites and Blockers

- Depends on: TASK-144 (canonical WIT source), TASK-147/148 (mesh query host services stubs to real)
- Unblocks: TASK-127 (non-planar Z envelope), Phase H Benchy acceptance when objects have non-identity transforms
- Activation blockers: None beyond the dependencies above

## Acceptance Criteria

- **Given** a model file (STL/3MF) with a non-identity transform (rotation, translation, scale) is loaded, **when** the host computes the global Z-plane sequence via `PrePass::LayerPlanning`, **then** the resulting `LayerPlanIR.global_layers[*].z` values are in world-space and correctly account for the object's transform (translation in Z, rotation affecting which Z planes intersect the mesh). | `cargo test -p slicer-host --test transformed_model_world_z_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|world.*space.*Z|transformed.*Z"`

- **Given** an object with `transform = translate(0, 0, 10mm)` is sliced with 0.2mm layer height, **when** the first global layer z is computed, **then** `global_layers[0].z >= 10.0` (world-space floor accounting for the translation). | `cargo test -p slicer-host --test translated_object_z_floor_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|z.*>=.*10|Z.*floor.*translated"`

- **Given** an object with `transform = rotate_x(90deg)` (lay-flat transform) is sliced, **when** world-space Z values are computed, **then** the host computes correct projection and the resulting Z planes span the object's world-space extent (not its local extent). | `cargo test -p slicer-host --test rotated_object_world_extent_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|rotate.*90|world.*extent|lay.flat"`

- **Given** multiple objects with different transforms are in the same scene, **when** `LayerPlanIR` is built, **then** global layer indices are computed from the LCM of layer heights and each object's Z range is correctly projected to world space. | `cargo test -p slicer-host --test multi_object_transform_world_z_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|multi.object|LCM.*layer.*height|world.*space.*multiple"`

- **Given** the world-space Z canonical surface is defined (either as a new IR field or documented config-only behavior), **when** a transformed object is sliced, **then** the canonical surface is used consistently and regression tests prove no silent use of object-local Z. | `cargo test -p slicer-host --test world_z_canonical_surface_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|canonical.*surface|world.*Z.*contract|regression"`

## Negative Test Cases

- **Given** an object has a non-uniform scale (e.g., `scale_x != scale_y != scale_z`), **when** it is sliced, **then** this is handled correctly (host applies scale to mesh before slicing) — if the host does not support non-uniform scale, a fatal error with code `NON_UNIFORM_SCALE_UNSUPPORTED` is returned at load time. | `cargo test -p slicer-host --test non_uniform_scale_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|NON_UNIFORM_SCALE|non.uniform"`

- **Given** a model has a transform that results in world-space Z < 0 (below print volume floor), **when** slicing is attempted, **then** a diagnostic is emitted and slicing fails with `WORLD_Z_BELOW_FLOOR`. | `cargo test -p slicer-host --test world_z_below_floor_tdd -- --nocapture 2>&1 | grep -E "PASS|FAIL|WORLD_Z_BELOW_FLOOR|below.*floor"`

## Verification

- `cargo test -p slicer-host -- transformed_model_world_z_tdd translated_object_z_floor_tdd rotated_object_world_extent_tdd multi_object_transform_world_z_tdd world_z_canonical_surface_tdd non_uniform_scale_tdd world_z_below_floor_tdd`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — transform-aware behavior expectations, coordinate system
- `docs/02_ir_schemas.md` — `MeshIR.ObjectMesh.transform` (column-major f64 `Transform3d`), `LayerPlanIR.GlobalLayer.z` (mm float), coordinate system (1 unit = 100nm)
- `docs/04_host_scheduler.md` — layer planning with transforms, `PrePass::LayerPlanning`
- `docs/08_coordinate_system.md` — coordinate conversion rules, transform handling
- `docs/05_module_sdk.md` — host services with transform awareness
- `docs/07_implementation_status.md` — DEV-027 status, TASK-157/158 backlog entries

## OrcaSlicer Reference Obligations

None. Transform handling is internal architecture; no direct OrcaSlicer reference comparison is required for this packet.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
