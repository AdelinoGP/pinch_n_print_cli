---
status: implemented
packet: mesh-query-host-services
task_ids:
  - TASK-147
  - TASK-148
backlog_source: docs/07_implementation_status.md
copy_note: Copy this file into ./.ralph/specs/mesh-query-host-services/ and change status to draft or active.
---

# Packet Contract: mesh-query-host-services

## Goal

Implement live mesh-data wiring for `raycast_z_down`, `surface_normal_at`, and `object_bounds` on one shared mesh-query backing surface, replacing the current stub/trap behavior with tested results. Covers DEV-015.

## Scope Boundaries

- In scope:
  - TASK-147: Wire `raycast_z_down` to live `IndexedTriangleSet` mesh data from `MeshIR` so hit/miss semantics return real world-space Z values under the current `option<f32>` WIT signature
  - TASK-147: Cover hit/miss semantics across all WIT worlds (`world-layer`, `world-prepass`, `world-finalization`, `world-postpass`)
  - TASK-148: Wire `surface_normal_at(object_id, x, y, z)` to live mesh data, returning a unit-length world-space normal for a queried point on the transformed mesh surface
  - TASK-148: Wire `object_bounds` to live mesh data, returning the world-space bounding box after applying the object's `Transform3d`
  - TASK-148: Cover invalid-object diagnostics via `OBJECT_NOT_FOUND` and off-surface coordinate handling via `None`
  - World-transform handling: all three functions must account for `ObjectMesh.transform` (column-major 4x4 matrix)

- Out of scope:
  - Prepass segmentation (TASK-128)
  - Postpass WIT gaps (TASK-129)
  - Path-optimization behavior
  - Modifier volume queries (separate surface)
  - Fixture-level transform integration tracked by TASK-157 and TASK-158
  - Facet-index-based mesh-query APIs or WIT signature changes

## Prerequisites and Blockers

- Depends on: None (self-contained mesh-query surface)
- Unblocks: TASK-157 (transform-aware fixture integration), TASK-128a (usable MeshSegmentation inputs)
- Activation blockers: None

## Acceptance Criteria

- **Given** `raycast_z_down(object_id: ObjectId, x: f32, y: f32, start_z: f32)` is called on a mesh that has triangles below `start_z`, **when** the raycast executes, **then** it returns `Some(world_z: f32)` with `world_z` being the world-space Z of the closest intersected triangle. | `cargo test -p slicer-host --test raycast_z_down_hit_tdd 2>&1 | grep -E "raycast.*hit|world_z|Some\("`

- **Given** `raycast_z_down(object_id: ObjectId, x: f32, y: f32, start_z: f32)` is called on a mesh with no triangles below `start_z`, **when** the raycast executes, **then** it returns `None`. | `cargo test -p slicer-host --test raycast_z_down_miss_tdd 2>&1 | grep -E "raycast.*miss|None|no.*hit"`

- **Given** `surface_normal_at(object_id: ObjectId, x: f32, y: f32, z: f32)` is called with a world-space point that lies on a transformed mesh triangle, **when** it executes, **then** the returned normal vector has unit length (magnitude within `1e-6` of `1.0`) and is perpendicular to that triangle's plane. | `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd 2>&1 | grep -E "unit.*length|normal.*1\.0|magnitude"`

- **Given** `object_bounds(object_id: ObjectId)` is called, **when** it executes, **then** the returned `BoundingBox3` has `min_z <= max_z` and contains all vertex positions of the mesh after applying the object's world transform. | `cargo test -p slicer-host --test object_bounds_transform_tdd 2>&1 | grep -E "BoundingBox|world.*transform|min_z.*max_z"`

- **Given** a module calls `raycast_z_down` on an object that has been transformed (rotated, translated), **when** the hit result is returned, **then** the Z value is in world space accounting for the transform, not object-local space. | `cargo test -p slicer-host --test raycast_z_down_transformed_object_tdd 2>&1 | grep -E "world.*space|transformed.*raycast"`

## Negative Test Cases

- **Given** `raycast_z_down` is called with an invalid `object_id` not in the scene, **when** it executes, **then** the host returns a fatal error with code `OBJECT_NOT_FOUND` and does not panic. | `cargo test -p slicer-host --test raycast_z_down_invalid_object_tdd 2>&1 | grep -E "OBJECT_NOT_FOUND|fatal"`

- **Given** `surface_normal_at(object_id, x, y, z)` is called with a world-space point outside the transformed mesh surface, **when** it executes, **then** the host returns `None` rather than panicking or fabricating a normal. | `cargo test -p slicer-host --test surface_normal_at_oob_tdd 2>&1 | grep -E "outside.*surface|None|no.*normal"`

## Verification

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- Per-criterion test commands listed in acceptance criteria above

## Authoritative Docs

- `docs/01_system_architecture.md` — Host Services section (Mesh raycasts, Host Services table)
- `docs/02_ir_schemas.md` — `MeshIR`, `ObjectMesh`, `IndexedTriangleSet`, `Transform3d`, coordinate system (1 unit = 100nm)
- `docs/03_wit_and_manifest.md` — `host-api.wit` (`raycast-z-down`, `surface-normal-at`, `object-bounds`)
- `docs/05_module_sdk.md` — SDK usage, host service wrappers, mesh-query API signatures
- `docs/08_coordinate_system.md` — coordinate conversion rules (1 unit = 100nm)

## OrcaSlicer Reference Obligations

None. This is an internal host-service wiring task; geometry algorithms are not being ported.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
