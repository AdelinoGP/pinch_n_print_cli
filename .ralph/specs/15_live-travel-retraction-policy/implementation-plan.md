# Implementation Plan: live-travel-retraction-policy

## Execution Rules

- One atomic step at a time.
- Land the ownership decision and focused module tests before host integration.

## Steps

### Step 1: Freeze travel-policy ownership and add failing module tests

- Task IDs:
  - `TASK-120d1`
- Objective:
  Capture the chosen ownership model and add focused failing tests for external retract and internal suppression on `path-optimization-default`.
- Precondition:
  The module still acts primarily as a marker emitter.
- Postcondition:
  `travel_policy_tdd.rs` exists with failing external and internal travel assertions.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/RetractWhenCrossingPerimeters.hpp`
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture`
  - `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture`
- Exit condition:
  The tests exist and fail only because the live module does not yet emit the expected decisions.

### Step 2: Implement retract/no-retract decisions on the module surface

- Task IDs:
  - `TASK-120d`
  - `TASK-120d1`
- Objective:
  Make `path-optimization-default` emit matched retract/unretract decisions for external travel and suppress them for internal travel.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  The module-level travel-policy tests pass.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd -- --nocapture`
- Exit condition:
  External and internal travel-policy tests are green.

### Step 3: Wire Z-hop and matched pairing through the live host path

- Task IDs:
  - `TASK-120d2`
- Objective:
  Prove the live host path carries retract/unretract decisions and aligned `ZHop` entries together.
- Precondition:
  Module-level travel-policy tests are green.
- Postcondition:
  Host integration tests prove matched retract/unretract plus aligned `ZHop` behavior on the real path.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_travel_policy_tdd retracting_travel_populates_matching_z_hop_and_retract_pair -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_travel_policy_tdd no_retract_policy_emits_no_orphan_retracts_or_z_hops -- --exact --nocapture`
- Exit condition:
  Host integration tests pass for matched pairs and the negative no-retract case.

### Step 4: Add a deterministic live travel regression

- Task IDs:
  - `TASK-120d`
  - `TASK-120d2`
- Objective:
  Lock the policy with one repeated-run determinism guard.
- Precondition:
  Steps 2 and 3 are green.
- Postcondition:
  Live travel decisions are byte-identical across repeated runs.
- Files expected to change:
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_travel_policy_tdd travel_policy_is_deterministic_across_repeated_runs -- --exact --nocapture`
- Exit condition:
  Determinism guard passes.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-120d`, `TASK-120d1`, and `TASK-120d2`.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm no policy logic migrated into `gcode_emit.rs`.
- Record any remaining packet-local risk before status changes.