# Requirements: mesh-query-host-services-and-transform-coverage

## Packet Metadata

- Grouped task IDs:
  - `TASK-147` — Implement `raycast_z_down` live mesh-data wiring and cover hit/miss semantics across WIT worlds. Covers DEV-015.
  - `TASK-148` — Implement `surface_normal_at` and `object_bounds` on the same backing surface, replacing stubs with tested results. Continues DEV-015.
  - `TASK-157` — Add fixture-level integration coverage for non-identity object transforms proving correct world-space Z behavior. Covers DEV-027.
  - `TASK-158` — Promote world-space Z extent to canonical derived contract surface and regression-lock transformed-object behavior. Continues DEV-027.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-015: Mesh query host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`) remain stubs. They must return real geometry results from `MeshIR`, not trap or return dummy values.

DEV-027: Non-identity object transforms (rotation, translation, non-uniform scale) lack fixture-level integration coverage. World-space Z behavior through planning is unverified for transformed STL/3MF inputs.

## In Scope

- TASK-147: Wire `raycast_z_down` to real mesh data from `MeshIR`. Implement hit/miss semantics: `Some(z)` on hit, `None` on miss. Cover hit/miss across WIT worlds.
- TASK-148: Implement `surface_normal_at` and `object_bounds` on the same mesh-query backing surface as `raycast_z_down`, replacing stubs.
- TASK-157: Add integration test with non-identity transformed STL/3MF input. Prove world-space Z behavior is correct through the full planning pipeline.
- TASK-158: Define or document the world-space Z canonical contract surface. If first-class IR, add it; if config-only, document clearly. Regression-lock transformed-object behavior.

## Out of Scope

- Z-envelope enforcement (TASK-127), prepass segmentation (TASK-128 series), scheduler regression guards (TASK-131-134), manifest population / runtime audit (Workstream 1)

## Authoritative Docs

- `docs/01_system_architecture.md` — Data Ownership Rules (host-services API for mesh queries)
- `docs/02_ir_schemas.md` — MeshIR, ObjectMesh, Transform3d, BoundingBox3
- `docs/03_wit_and_manifest.md` — host-api.wit
- `docs/08_coordinate_system.md` — Coordinate scaling and world-space Z rules

## OrcaSlicer Reference Obligations

- Check `OrcaSlicerDocumented/` for OrcaSlicer raycast / surface normal behavior. Cite paths if found.

## Acceptance Summary

- `raycast_z_down` returns `Some(z)` on hit, `None` on miss with correct world-space Z.
- `surface_normal_at` returns correct normal or `None` when off-mesh.
- `object_bounds` returns correct world-space bounding box.
- Non-identity transform integration test passes with correct world-space Z behavior.
- World-space Z canonical surface defined or documented and regression-locked.

## Verification Commands

- `cargo test --package slicer-host --test raycast_z_down_hit_miss -- --nocapture`
- `cargo test --package slicer-host --test surface_normal_at -- --nocapture`
- `cargo test --package slicer-host --test object_bounds -- --nocapture`
- `cargo test --package slicer-host --test transform_world_space_z -- --nocapture`