---
status: draft
packet: mesh-query-host-services
task_ids:
  - TASK-147
  - TASK-148
backlog_source: docs/07_implementation_status.md
copy_note: Copy this file into ./.ralph/specs/mesh-query-host-services/ and change status to draft or active.
---

# Packet Contract: mesh-query-host-services

## Goal

Implement live mesh-data wiring for `raycast_z_down` and implement `surface_normal_at` and `object_bounds` on the same mesh-query backing surface, replacing the current stub/trap behavior with tested results. Covers DEV-015.

## Scope Boundaries

- In scope:
  - TASK-147: Wire `raycast_z_down` to live `IndexedTriangleSet` mesh data from `MeshIR` so hit/miss semantics return real Z and facet indices
  - TASK-147: Cover hit/miss semantics across all WIT worlds (`world-layer`, `world-prepass`, `world-finalization`, `world-postpass`)
  - TASK-148: Wire `surface_normal_at` to live mesh data, returning a unit-length normal perpendicular to the hit facet
  - TASK-148: Wire `object_bounds` to live mesh data, returning the world-space bounding box after applying the object's `Transform3d`
  - TASK-148: Cover invalid-object and out-of-bounds error codes (`OBJECT_NOT_FOUND`, `FACET_INDEX_OUT_OF_BOUNDS`)
  - World-transform handling: all three functions must account for `ObjectMesh.transform` (column-major 4x4 matrix)

- Out of scope:
  - Prepass segmentation (TASK-128)
  - Postpass WIT gaps (TASK-129)
  - Path-optimization behavior
  - Modifier volume queries (separate surface)
  - Non-identity transforms on object_bounds (TASK-157, TASK-158)

## Prerequisites and Blockers

- Depends on: None (self-contained mesh-query surface)
- Unblocks: TASK-157 (transform-aware fixture integration), TASK-128a (usable MeshSegmentation inputs)
- Activation blockers: None

## Acceptance Criteria

- **Given** `raycast_z_down(origin: Point3, object_id: ObjectId)` is called on a mesh that has triangles below the origin's Z, **when** the raycast executes, **then** it returns `Some(world_z: f32)` with world_z being the world-space Z of the closest intersected triangle. | `cargo test -p slicer-host --test raycast_z_down_hit_tdd 2>&1 | grep -E "raycast.*hit|world_z|Some\("`

- **Given** `raycast_z_down(origin: Point3, object_id: ObjectId)` is called on a mesh with no triangles below the origin's Z (e.g., origin is below the print volume floor), **when** the raycast executes, **then** it returns `None`. | `cargo test -p slicer-host --test raycast_z_down_miss_tdd 2>&1 | grep -E "raycast.*miss|None|no.*hit"`

- **Given** `surface_normal_at(facet_index: u32, object_id: ObjectId)` is called, **when** it executes, **then** the returned normal vector has unit length (magnitude within 1e-6 of 1.0) and is perpendicular to the facet's plane. | `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd 2>&1 | grep -E "unit.*length|normal.*1\.0|magnitude"`

- **Given** `object_bounds(object_id: ObjectId)` is called, **when** it executes, **then** the returned `BoundingBox3` has min_z <= max_z and contains all vertex positions of the mesh after applying the object's world transform. | `cargo test -p slicer-host --test object_bounds_transform_tdd 2>&1 | grep -E "BoundingBox|world.*transform|min_z.*max_z"`

- **Given** a module calls `raycast_z_down` on an object that has been transformed (rotated, translated), **when** the hit result is returned, **then** the Z value is in world space accounting for the transform, not object-local space. | `cargo test -p slicer-host --test raycast_z_down_transformed_object_tdd 2>&1 | grep -E "world.*space|transformed.*raycast"`

## Negative Test Cases

- **Given** `raycast_z_down` is called with an invalid `object_id` not in the scene, **when** it executes, **then** the host returns a fatal error with code `OBJECT_NOT_FOUND` and does not panic. | `cargo test -p slicer-host --test raycast_z_down_invalid_object_tdd 2>&1 | grep -E "OBJECT_NOT_FOUND|fatal"`

- **Given** `surface_normal_at` is called with `facet_index >= triangle_count`, **when** it executes, **then** the host returns a fatal error with code `FACET_INDEX_OUT_OF_BOUNDS`. | `cargo test -p slicer-host --test surface_normal_at_oob_tdd 2>&1 | grep -E "FACET_INDEX_OUT_OF_BOUNDS|out.of.bounds"`

## Verification

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- Per-criterion test commands listed in acceptance criteria above

## Authoritative Docs

- `docs/01_system_architecture.md` — Host Services section (Mesh raycasts, Host Services table)
- `docs/02_ir_schemas.md` — `MeshIR`, `ObjectMesh`, `IndexedTriangleSet`, `Transform3d`, coordinate system (1 unit = 100nm)
- `docs/03_wit_and_manifest.md` — `host-api.wit` (`raycast-z-down`, `surface-normal-at`, `object-bounds`)
- `docs/05_module_sdk.md` — SDK usage, host service wrappers, raycast API
- `docs/08_coordinate_system.md` — coordinate conversion rules (1 unit = 100nm)

## OrcaSlicer Reference Obligations

None. This is an internal host-service wiring task; geometry algorithms are not being ported.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
