---
status: superseded
packet: live-seam-placement-and-consumption
task_ids:
  - TASK-120c
  - TASK-151
backlog_source: docs/07_implementation_status.md
superseded_by: 14-rev1_live-seam-placement-and-consumption
---

# Packet Contract: live-seam-placement-and-consumption

## Goal

Restore seam placement on real wall-loop seam candidates and teach the live path-optimization surface to consume `PerimeterRegion.resolved_seam` so it replays seam-started wall loops instead of remaining a comment-only slot filler.

## Scope Boundaries

- In scope:
  - host and module coverage proving real wall-loop seam candidates reach `seam-placer`
  - commitment of `PerimeterIR.regions[*].resolved_seam` on the live wall-postprocess path
  - narrow expansion of `path-optimization-default` so it consumes resolved seam output and emits replayed wall-loop moves instead of only marker comments
  - deterministic regressions for seam-started wall-loop replay
- Out of scope:
  - generic travel ordering, retract/no-retract policy, or Z-hop planning (packet `15` and packet `18`)
  - Orca-facing GCode text emission for seam-started loops (packet `11`)
  - broader nearest-neighbor or cross-object path ordering (packet `18`)

## Prerequisites and Blockers

- Depends on:
  - live perimeter generators already producing `PerimeterIR.regions[*].seam_candidates`
  - `modules/core-modules/seam-placer` and `modules/core-modules/path-optimization-default`
- Unblocks:
  - TASK-135 seam evidence on the Benchy path
  - TASK-120d travel policy work, which assumes real wall-loop replay rather than comment-only output
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a live wall-postprocess fixture whose `PerimeterIR.regions[0].seam_candidates` contains multiple real wall-loop candidates, **when** `seam-placer` runs through the real `Layer::WallPostProcess` dispatch path, **then** `PerimeterIR.regions[0].resolved_seam` becomes `Some(SeamPosition)` and `resolved_seam.point.z` matches the source wall loop layer Z exactly. | `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- **Given** a `PerimeterRegionView` fixture with `resolved_seam=Some(...)` on two wall loops, **when** `path-optimization-default` runs, **then** the captured `GcodeOutputBuilder.commands()` contains at least one `GCodeCommand::Move` and the first move of each replayed wall loop begins at the corresponding `resolved_seam.point` rather than at the original unsplit loop start. | `cargo test -p path-optimization-default --test seam_consumption_tdd path_optimization_replays_wall_loops_from_resolved_seams -- --exact --nocapture`
- **Given** the same resolved-seam fixture executed twice, **when** `path-optimization-default` replays the wall loops, **then** the emitted `Move` sequence is byte-identical across both runs. | `cargo test -p path-optimization-default --test seam_consumption_tdd seam_started_wall_replay_is_deterministic -- --exact --nocapture`
- **Given** a live host dispatch that includes perimeter generation, seam placement, and path optimization for one layer, **when** the stage chain completes, **then** the output for `Layer::PathOptimization` is no longer only the marker comment and the resulting replay includes at least one wall-loop move derived from the seam-resolved perimeter input. | `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`

## Negative Test Cases

- **Given** a perimeter region whose `resolved_seam` is `None`, **when** `path-optimization-default` runs, **then** it leaves the original wall-loop order unchanged and does not fabricate a seam split or synthetic seam-start move. | `cargo test -p path-optimization-default --test seam_consumption_tdd missing_resolved_seam_leaves_wall_loop_order_unchanged -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd path_optimization_replays_wall_loops_from_resolved_seams -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd seam_started_wall_replay_is_deterministic -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd missing_resolved_seam_leaves_wall_loop_order_unchanged -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — seam placement and per-layer stage ownership
- `docs/02_ir_schemas.md` — `PerimeterIR`, `SeamCandidate`, and `SeamPosition`
- `docs/04_host_scheduler.md` — `Layer::WallPostProcess` and `Layer::PathOptimization` execution order
- `docs/07_implementation_status.md` — TASK-120c and TASK-151 scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`