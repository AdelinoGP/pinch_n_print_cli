# Implementation Plan: live-seam-placement-and-consumption

## Execution Rules

- One atomic step at a time.
- Land seam commitment before seam consumption.

## Steps

### Step 1: Add a live host regression for `resolved_seam` commitment

- Task IDs:
  - `TASK-120c`
- Objective:
  Prove the real wall-postprocess stage commits `PerimeterIR.regions[*].resolved_seam` on a live host path.
- Precondition:
  `seam-placer` unit tests exist, but host dispatch does not yet prove the value lands in `PerimeterIR`.
- Postcondition:
  `live_seam_path_tdd.rs` contains a failing host regression for committed `resolved_seam`.
- Files expected to change:
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- Exit condition:
  The focused host regression exists and fails only because the live path is not yet committing the seam value.

### Step 2: Restore live seam commitment

- Task IDs:
  - `TASK-120c`
- Objective:
  Make the real wall-postprocess path commit `resolved_seam` from `seam-placer`.
- Precondition:
  Step 1 regression is in place.
- Postcondition:
  The host seam-commit test passes and `resolved_seam.point.z` matches the source loop layer Z.
- Files expected to change:
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- Verification:
  - `cargo test -p slicer-host --test live_seam_path_tdd wall_postprocess_commits_resolved_seam_to_perimeter_ir -- --exact --nocapture`
- Exit condition:
  The live host regression passes.

### Step 3: Add failing seam-consumption replay tests on `path-optimization-default`

- Task IDs:
  - `TASK-151`
- Objective:
  Freeze the seam-consumption expectations on the path-optimization module before widening its output surface.
- Precondition:
  Live seam commitment is green.
- Postcondition:
  `seam_consumption_tdd.rs` contains failing assertions for seam-started wall-loop replay and the no-fabrication negative case.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Verification:
  - `cargo test -p path-optimization-default --test seam_consumption_tdd path_optimization_replays_wall_loops_from_resolved_seams -- --exact --nocapture`
  - `cargo test -p path-optimization-default --test seam_consumption_tdd missing_resolved_seam_leaves_wall_loop_order_unchanged -- --exact --nocapture`
- Exit condition:
  Both module tests exist and fail only because the module is still comment-only.

### Step 4: Widen the path-optimization seam slice just enough to replay wall loops

- Task IDs:
  - `TASK-151`
- Objective:
  Make `path-optimization-default` consume seam output and replay seam-started wall-loop moves on the real path, without taking on broader travel ordering.
- Precondition:
  Step 3 tests are in place.
- Postcondition:
  The module emits seam-started wall-loop moves deterministically, and the host end-to-end seam-path test proves the stage is no longer comment-only.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs`
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Verification:
  - `cargo test -p path-optimization-default --test seam_consumption_tdd path_optimization_replays_wall_loops_from_resolved_seams -- --exact --nocapture`
  - `cargo test -p path-optimization-default --test seam_consumption_tdd seam_started_wall_replay_is_deterministic -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_path_end_to_end_emits_wall_loop_moves_after_resolution -- --exact --nocapture`
- Exit condition:
  All seam-consumption and host end-to-end tests pass.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-120c` and `TASK-151`.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm the path-optimization surface emits real wall-loop moves for the seam slice.
- Record any remaining packet-local risk before status changes.