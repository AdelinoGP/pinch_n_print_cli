# Implementation Plan: live-travel-retraction-policy

## Execution Rules

- One atomic step at a time.
- Land the ownership decision and focused module tests before host integration.
- Add a test-scaffolding step before Step 1 so that every verification command has a valid target from the first iteration.

## Steps

### Step 0 (precondition for all other steps): Scaffold test files

- Objective:
  Create empty test files with stub test functions so every verification command returns "test not found" (not "file not found") and compiles cleanly. This lets each subsequent step verify its own failures without scaffolding noise.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs`
- Authoritative docs: the packet's `packet.spec.md` verification section
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd -- --list 2>&1 | grep -c "external_travel_emits_matched_retract_and_unretract"` → must output `1` (test is registered)
  - `cargo test -p path-optimization-default --test travel_policy_tdd -- --list 2>&1 | grep -c "internal_travel_suppresses_retraction"` → must output `1`
  - `cargo test -p slicer-host --test live_travel_policy_tdd -- --list 2>&1 | grep -c "retracting_travel_populates_matching_z_hop_and_retract_pair"` → must output `1`
  - `cargo test -p slicer-host --test live_travel_policy_tdd -- --list 2>&1 | grep -c "travel_policy_is_deterministic_across_repeated_runs"` → must output `1`
  - `cargo test -p slicer-host --test live_travel_policy_tdd -- --list 2>&1 | grep -c "no_retract_policy_emits_no_orphan_retracts_or_z_hops"` → must output `1`
  - `cargo build -p path-optimization-default --tests` → must compile without error
  - `cargo build -p slicer-host --tests` → must compile without error
- Exit condition:
  All five test names are registered (`--list` succeeds) and both test crates compile without error. No test body implements the actual policy logic yet — stubs may assert `true` (skipped) or `compile_error!` until Step 1 fills them in.

### Step 1: Freeze travel-policy ownership and add failing module test bodies

- Task IDs:
  - `TASK-120d1`
- Objective:
  Replace the Step 0 stub test bodies with real failing assertions. The module still acts primarily as a marker emitter; after Step 1 it must emit Retract/Unretract commands for external travel and suppress them for internal travel — but the tests verify this without implementing it yet.
- Precondition:
  Step 0 scaffolding is complete: all five test names are registered and both test crates compile.
- Postcondition:
  `travel_policy_tdd.rs` contains real test bodies that assert the expected Retract/Unretract behavior and fail because the module does not yet emit those decisions.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/RetractWhenCrossingPerimeters.hpp`
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture 2>&1` → must fail (module hasn't implemented decisions yet)
  - `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture 2>&1` → must fail (same reason)
  - `cargo build -p path-optimization-default --tests` → must still compile (test compiles, module behavior not yet injected)
- Exit condition:
  Both tests are compiled and **fail** — but `cargo build --tests` still passes. The failure reason must be "assertion failed" (missing Retract/Unretract in output), NOT "undefined symbol" or "link error".

### Step 2: Implement retract/no-retract decisions on the module surface

- Task IDs:
  - `TASK-120d`
  - `TASK-120d1`
- Objective:
  Make `path-optimization-default` emit matched retract/unretract decisions for external travel and suppress them for internal travel.
- Precondition:
  Step 1 tests are in place and fail because the module does not yet emit Retract/Unretract.
- Postcondition:
  The module-level travel-policy tests pass. `path-optimization-default` now calls `push_retract`, `push_unretract`, and `push_move(e=None)` on the GcodeOutputBuilder for external travels, and emits nothing for internal travels.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture` → PASS
  - `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture` → PASS
- Exit condition:
  Both AC-ext and AC-int tests pass. The module emits the correct Retract/Unretract decisions for external travel and suppresses them for internal travel.

### Step 3: Wire Z-hop and matched pairing through the live host path

- Task IDs:
  - `TASK-120d2`
- Objective:
  Prove the live host path carries retract/unretract decisions and aligned `ZHop` entries together. Create the host integration test file with real fixtures that expose the end-to-end behavior. Separately assert the negative no-retract case.
- Precondition:
  Step 2 is green (module-level travel-policy tests pass).
- Postcondition (positive):
  Host integration tests prove matched retract/unretract plus aligned `ZHop` behavior on the live path. `live_travel_policy_tdd.rs` exists with real test fixtures.
- Postcondition (negative):
  The no-retract fixture produces no orphan retracts and no stray `ZHop` entries.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_travel_policy_tdd retracting_travel_populates_matching_z_hop_and_retract_pair -- --exact --nocapture` → PASS
  - `cargo test -p slicer-host --test live_travel_policy_tdd no_retract_policy_emits_no_orphan_retracts_or_z_hops -- --exact --nocapture` → PASS
- Exit condition:
  Both the positive Z-hop test and the negative no-retract test pass. The host correctly routes Retract/Unretract/ZHop decisions from the module through to `LayerCollectionIR`.
- Dependency note:
  Step 4 appends a determinism test to the same `live_travel_policy_tdd.rs` file. Step 3 must complete before Step 4 starts.

### Step 4: Add a deterministic live travel regression

- Task IDs:
  - `TASK-120d`
  - `TASK-120d2`
- Objective:
  Lock the policy with one repeated-run determinism guard appended to the Step 3 test file.
- Precondition:
  Steps 2 and 3 are green. `live_travel_policy_tdd.rs` exists.
- Postcondition:
  A determinism test that runs the same fixture twice and asserts byte-identical retract/unretract/Z-hop decisions across both runs.
- Files expected to change:
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs` (append test, same file as Step 3)
- Authoritative docs:
  - `docs/04_host_scheduler.md`
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_travel_policy_tdd travel_policy_is_deterministic_across_repeated_runs -- --exact --nocapture` → PASS
- Exit condition:
  Determinism guard passes. No changes to any source files outside the test file.
- Dependency note:
  Step 3 must complete (file created and all its tests green) before this step starts. Both steps write `live_travel_policy_tdd.rs`; serialization prevents merge conflicts.

## Packet Completion Gate

- All steps complete (Step 0 through Step 4).
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-120d`, `TASK-120d1`, and `TASK-120d2` only if task notes or states actually changed — otherwise record `no docs/07 delta`.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm no policy logic migrated into `gcode_emit.rs`.
- Record any remaining packet-local risk before status changes.