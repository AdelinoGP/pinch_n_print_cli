# Requirements: live-seam-placement-and-consumption

## Packet Metadata

- Grouped task IDs:
  - `TASK-120c` — restore seam placement on real wall-loop seam candidates
  - `TASK-151` — teach `path-optimization-default` to consume seam-placement output on real wall loops
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Seam candidates exist upstream, and `seam-placer` has unit coverage, but the live Workstream 3 path still lacks two linked behaviors: a real committed `resolved_seam` on production wall loops and a path-optimization stage that does something useful with that seam output. Right now the default path-optimization module emits only a marker comment. This packet narrows the gap to the seam slice only: make real wall-loop seam candidates land in `resolved_seam`, then replay seam-started wall loops on the path-optimization surface without pulling in the broader travel-ordering and retraction work.

## In Scope

- live commitment of `PerimeterIR.regions[*].resolved_seam`
- seam-started wall-loop replay from `path-optimization-default`
- deterministic module and host regressions for the seam slice

## Out of Scope

- generic path-ordering heuristics
- retraction, Z-hop, and travel-policy ownership
- Orca-facing GCode formatting

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`

## Acceptance Summary

### Positive Cases

- `resolved_seam` is committed on the live wall-postprocess path.
- `path-optimization-default` consumes resolved seam output and emits wall-loop `Move` commands rather than only a marker comment.
- Replayed seam-started wall loops are deterministic across repeated runs.
- The host stage chain proves seam placement and consumption work together end-to-end for one layer.

### Negative Cases

- Missing `resolved_seam` leaves the original wall-loop order unchanged and does not fabricate a split.

### Measurable Outcomes

- Tests assert exact `PerimeterIR.regions[*].resolved_seam` presence and exact first-move coordinates for replayed loops.
- The path-optimization surface is considered fixed only when a real `GCodeCommand::Move` is emitted for wall-loop replay.

### Cross-Packet Impact

- Packet `15` assumes this packet has turned the path-optimization surface into something richer than a comment-only stage.
- Packet `21` uses this packet to add seam evidence to the Benchy acceptance path.

## Verification Commands

- `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd path_optimization_replays_wall_loops_from_resolved_seams -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd seam_started_wall_replay_is_deterministic -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_path_end_to_end_emits_wall_loop_moves_after_resolution -- --exact --nocapture`
- `cargo test -p path-optimization-default --test seam_consumption_tdd missing_resolved_seam_leaves_wall_loop_order_unchanged -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the seam commitment or replay surface is isolated to one stage boundary
- Postcondition: one exact seam contract surface is observable on the real path
- Falsifying check: a focused assertion fails if seam placement is missing or path optimization remains comment-only