---
status: draft
packet: mesh-query-host-services-and-transform-coverage
task_ids:
  - TASK-147
  - TASK-148
  - TASK-157
  - TASK-158
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: mesh-query-host-services-and-transform-coverage

## Goal

Implement the mesh query host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`) that are currently stubs, replacing the stub/trap behavior with tested real results (DEV-015). Add fixture-level integration coverage for non-identity object transforms so transformed STL/3MF inputs prove correct world-space Z behavior through planning (DEV-027). Promote world-space Z extent to a canonical derived contract surface and regression-lock transformed-object behavior (DEV-027).

## Scope Boundaries

- In scope:
  - TASK-147: Implement live mesh-data wiring for `raycast_z_down` and cover hit/miss semantics across the WIT worlds. Covers DEV-015.
  - TASK-148: Implement `surface_normal_at` and `object_bounds` on the same mesh-query backing surface, replacing the current stub/trap behavior with tested results. Continues DEV-015.
  - TASK-157: Add fixture-level integration coverage for non-identity object transforms so transformed STL/3MF inputs prove correct world-space Z behavior through planning. Covers DEV-027.
  - TASK-158: Promote world-space Z extent to one canonical derived contract surface, either first-class IR or explicitly documented config-only behavior, then regression-lock transformed-object behavior. Continues DEV-027.

- Out of scope:
  - Z-envelope enforcement (TASK-127 — separate packet)
  - Prepass segmentation alignment (TASK-128 series — separate packet)
  - Scheduler regression guards (TASK-131-134 — separate packet)
  - Manifest population / runtime audit (Workstream 1)

## Acceptance Criteria

- **Given** a module that calls `raycast_z_down(object-id, x, y, start-z)`, **when** the ray hits the mesh, **then** the host returns `Some(z)` where z is the intersection point in world space; **when** the ray misses, **then** `None` is returned. Hit/miss semantics are documented and tested.
- **Given** a module that calls `surface_normal_at(object-id, x, y, z)`, **when** the point is on the mesh surface, **then** the host returns `Some(point3)` with the correct surface normal; **when** the point is not on the mesh, **then** `None` is returned.
- **Given** a module that calls `object_bounds(object-id)`, **when** the object exists, **then** the host returns the correct bounding box in world space.
- **Given** a non-identity object transform (rotation, translation, non-uniform scale), **when** the object is loaded and sliced, **then** all world-space Z behavior (layer Z, catch-up Z, effective_layer_height) is correct and consistent with the transformed mesh.
- **Given** the world-space Z canonical surface, **when** it is defined as a derived IR or documented config-only behavior, **then** transformed-object regression tests prove the canonical surface holds.
- **Given** the mesh query implementations and transform coverage, **when** tests run, **then** all related tests pass.

## Verification

- `cargo test --package slicer-host --test raycast_z_down_hit_miss -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test surface_normal_at -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test object_bounds -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test transform_world_space_z -- --nocapture` (test to be added)

## Authoritative Docs

- `docs/01_system_architecture.md` — Data Ownership Rules (mesh query via host-services API), Catch-Up Layer Semantics
- `docs/02_ir_schemas.md` — MeshIR, ObjectMesh, Transform3d, BoundingBox3
- `docs/03_wit_and_manifest.md` — host-api.wit (mesh query functions), WIT boundary
- `docs/08_coordinate_system.md` — Coordinate scaling and porting rules, world-space Z behavior

## OrcaSlicer Reference Obligations

- Check `OrcaSlicerDocumented/` for OrcaSlicer mesh query behavior (raycast, surface normal). If found, cite specific paths.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`