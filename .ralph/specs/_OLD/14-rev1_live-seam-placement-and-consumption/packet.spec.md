---
status: superseded
packet: 14-rev1_live-seam-placement-and-consumption
task_ids:
  - TASK-120c
  - TASK-151
backlog_source: docs/07_implementation_status.md
supersedes: 14_live-seam-placement-and-consumption
superseded_by: 22_live-seam-contract-repair
---

# Packet Contract: 14-rev1_live-seam-placement-and-consumption

## Goal

Implement seam-started wall loops so that `resolved_seam` is a **directive** — the wall loop's `path.points` sequence must start at the seam vertex. `Layer::PerimetersPostProcess` (seam-placer) emits rotated wall loops via a new `push-reordered-wall-loop` WIT method on `perimeter-output-builder`. `Layer::PathOptimization` reads already-rotated wall loops from `PerimeterIR` and emits only a per-layer marker comment — no replay needed.

**Seam-first contract:** Printing a layer begins at the seam and ends at the seam. The `WallLoop.path.points[0]` is the first vertex emitted, and the last emitted vertex joins cleanly to `points[0]` to close the loop.

## Scope Boundaries

- In scope:
  - New WIT method `push-reordered-wall-loop` on `perimeter-output-builder` at `Layer::WallPostProcess`
  - `seam-placer` rotates wall loop points so seam vertex comes first and emits via `push-reordered-wall-loop`
  - `PerimeterIR` stores wall loops with `path.points[0]` as the seam point
  - `path-optimization-default` reverts to comment-only output (marker emission only)
  - Existing `resolved_seam` WIT field remains on `perimeter-region-view` for diagnostic reads
  - `perimeter_region_to_data` in `wit_host.rs` maps rotated wall loop geometry through the WIT boundary
  - Deterministic regression suite for seam-rotated wall loop geometry
- Out of scope:
  - Generic travel ordering, retract/no-retract policy, Z-hop planning (packet `15`)
  - PathOptimization entity reordering beyond seam-first loop storage (packet `18`)
  - Orca-facing GCode text emission for seam-started loops (packet `11`)
  - WIT changes at `Layer::PathOptimization` (no `push-move` or new output methods needed)

## Prerequisites and Blockers

- Depends on:
  - `Layer::Perimeters` generating `PerimeterIR.regions[*].seam_candidates` (upstream perimeter generators must produce candidates)
  - `modules/core-modules/seam-placer` manifest already claims `seam-placer` hold and `writes = ["PerimeterIR.resolved-seam"]`
- Unblocks:
  - TASK-135 seam evidence on the Benchy path
  - Packet `15` travel policy, which can now assume `PerimeterIR` contains seam-started wall geometry (no replay needed)
- Activation blockers:
  - This packet requires updating `docs/02_ir_schemas.md` to explicitly document that `WallLoop.path.points[0]` is the seam-first vertex — the current schema is ambiguous on this point (see design.md Open Questions). The packet must remain `draft` until this ambiguity is resolved in the documentation.

## Acceptance Criteria

- **Given** a `PerimeterRegionView` fixture with two wall loops and one `resolved_seam` pointing to wall index `0` at `(x=5.0, y=0.0, z=0.2)`, **when** `seam-placer` runs through the real `Layer::WallPostProcess` dispatch path, **then** `PerimeterIR.regions[0].walls[0].path.points[0]` equals the seam position `(5.0, 0.0, 0.2)` and `points[0].width` equals the local extrusion width from that wall loop. | `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_rotates_wall_loop_points_to_seam_first -- --exact --nocapture`
- **Given** the same seam-placer fixture executed twice with identical wall loop geometry and identical `resolved_seam`, **when** seam-placer commits rotated wall loops both times, **then** `PerimeterIR` output is byte-identical across both runs. | `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_wall_loop_rotate_is_deterministic -- --exact --nocapture`
- **Given** a `PerimeterRegionView` with `resolved_seam` pointing to wall index `99` but only `3` wall loops exist, **when** seam-placer attempts to rotate, **then** `PerimeterIR` is committed with the original wall loop order unchanged and no fatal error is raised. | `cargo test -p slicer-host --test live_seam_path_tdd out_of_bounds_seam_wall_index_preserves_original_loop -- --exact --nocapture`
- **Given** `Layer::PathOptimization` dispatch against a `PerimeterIR` whose wall loops are already seam-first rotated, **when** `path-optimization-default` runs, **then** it emits only the per-layer marker comment and does not emit any `GCodeMoveCmd` via `push_move`. | `cargo test -p path-optimization-default --test seam_consumption_tdd no_move_commands_emitted_when_perimeter_already_rotated -- --exact --nocapture`
- **Given** a `PerimeterRegionView` with `resolved_seam = None` (no seam set), **when** seam-placer runs, **then** `PerimeterIR` is committed with the original wall loop order unchanged and `PerimeterIR.regions[0].resolved_seam = None`. | `cargo test -p slicer-host --test live_seam_path_tdd no_resolved_seam_preserves_original_wall_order -- --exact --nocapture`

## Negative Test Cases

- **Given** `seam-placer` writes a wall loop whose rotated `path.points` length does not match the original wall loop's `width_profile.widths` length, **when** `PerimeterIR` is committed, **then** the commit is rejected with a cardinality mismatch error and no `PerimeterIR` is set in the arena. | `cargo test -p slicer-host --test live_seam_path_tdd rotated_points_cardinality_mismatch_rejected -- --exact --nocapture`
- **Given** a seam position whose Z coordinate exceeds `layer.z + effective_layer_height`, **when** `push-reordered-wall-loop` is called, **then** the host returns an error and the perimeter output is not committed. | `cargo test -p slicer-host --test live_seam_path_tdd seam_z_outside_layer_envelope_rejected -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — Stage I/O contract, seam placement definition, paint propagation
- `docs/02_ir_schemas.md` — `PerimeterIR`, `WallLoop`, `PerimeterRegion`, `ExtrusionPath3D`, `Point3WithWidth`, `SeamPosition`
- `docs/03_wit_and_manifest.md` — `perimeter-output-builder` WIT resource, IR access path format
- `docs/04_host_scheduler.md` — `Layer::WallPostProcess` execution order, commit path

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — reference seam selection logic and wall loop rotation behavior
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — interface and constants

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
