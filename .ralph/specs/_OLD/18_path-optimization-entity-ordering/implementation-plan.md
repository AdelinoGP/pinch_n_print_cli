# Implementation Plan: path-optimization-entity-ordering

## Execution Rules

- One atomic step at a time.
- Land host ordering tests before helper changes.

## Steps

### Step 1: Add failing same-object and no-op ordering tests

- Task IDs:
  - `TASK-152a`
- Objective:
  Freeze same-object nearest-neighbor ordering and the unchanged no-op boundary case.
- Precondition:
  The host still mostly preserves assembled order.
- Postcondition:
  `path_ordering_tdd.rs` exists with failing same-object and no-op assertions.
- Files expected to change:
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- Exit condition:
  `path_ordering_tdd.rs` contains the two tests and compiles against a passthrough stub helper (one that returns the input slice unchanged); both tests fail their ordering assertions because the stub does not reorder anything.

### Step 2: Implement the same-object ordering helper and extend it to cross-object travel ordering

- Task IDs:
  - `TASK-152a`
  - `TASK-152d`
- Objective:
  Add the host-side ordering helper that resequences entities by travel cost within and across objects.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  Same-object and cross-object ordering tests pass.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- Exit condition:
  Both ordering tests are green.

### Step 3: Add bridge-sensitive ordering priority

- Task IDs:
  - `TASK-152e`
- Objective:
  Extend the ordering helper so bridge-sensitive entities outrank generic infill when candidate cost is comparable.
- Precondition:
  Step 2 is green.
- Postcondition:
  The bridge-priority test passes without regressing the earlier ordering rules.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- Exit condition:
  The bridge-priority test is green.

### Step 4: Lock determinism and expose the reordered sequence to the live stage chain

- Task IDs:
  - `TASK-152`
  - `TASK-152a`
  - `TASK-152d`
  - `TASK-152e`
- Objective:
  Prove the reordered sequence is the one the live path consumes and remains deterministic across repeated runs.
- Precondition:
  Steps 2 and 3 are green.
- Postcondition:
  Determinism and live-stage visibility tests are green.
- Files expected to change:
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/tests/fff_print/test_extrusion_entity.cpp`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- Exit condition:
  Both the determinism test and the live-stage integration test are green; the integration test confirms the path-optimization module receives the host-reordered entity list, not the original assembled order.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for TASK-152 / TASK-152a / TASK-152d / TASK-152e.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm the host owns canonical ordering before path optimization.
- Record any remaining packet-local risk before status changes.