# Implementation Plan: finalization-aware-travel-coordination

## Execution Rules

- One atomic step at a time.
- Add host-side tests before reconciliation helper changes.

## Steps

### Step 1: Add failing brim-aware and no-op reconciliation tests

- Task IDs:
  - `TASK-152f`
- Objective:
  Freeze the brim-aware first-travel transition and the no-op boundary when no finalization geometry exists.
- Precondition:
  Packets `15` and `16` have already defined the relevant travel and finalization surfaces.
- Postcondition:
  `finalization_aware_travel_tdd.rs` exists with failing brim-aware and no-op assertions.
- Files expected to change:
  - `crates/slicer-host/tests/finalization_aware_travel_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`
- Verification:
  - `cargo test -p slicer-host --test finalization_aware_travel_tdd brim_geometry_changes_first_model_travel_transition -- --exact --nocapture`
  - `cargo test -p slicer-host --test finalization_aware_travel_tdd no_finalization_geometry_is_a_reconciliation_no_op -- --exact --nocapture`
- Exit condition:
  The focused host tests exist and fail only because no reconciliation helper exists yet.

### Step 2: Implement brim-aware and wipe-aware travel reconciliation

- Task IDs:
  - `TASK-152`
  - `TASK-152f`
- Objective:
  Add the host-side reconciliation pass that incorporates brim and wipe geometry into travel transitions.
- Precondition:
  Step 1 tests are in place, and packets `16` and `17` have already made the geometry available.
- Postcondition:
  Brim-aware and wipe-aware reconciliation tests are green.
- Files expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/src/postpass.rs`
  - `crates/slicer-host/tests/finalization_aware_travel_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test finalization_aware_travel_tdd brim_geometry_changes_first_model_travel_transition -- --exact --nocapture`
  - `cargo test -p slicer-host --test finalization_aware_travel_tdd wipe_tower_geometry_is_included_in_travel_reconciliation -- --exact --nocapture`
- Exit condition:
  Both focused reconciliation tests are green.

### Step 3: Add the preserve-order negative regression

- Task IDs:
  - `TASK-152f`
- Objective:
  Prove the reconciliation pass never reorders model extrusion entities.
- Precondition:
  Step 2 is green.
- Postcondition:
  Preserve-order negative regression is green.
- Files expected to change:
  - `crates/slicer-host/tests/finalization_aware_travel_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order -- --exact --nocapture`
- Exit condition:
  Preserve-order regression is green.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for TASK-152 / TASK-152f.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm model extrusion order is unchanged while travel transitions update.
- Record any remaining packet-local risk before status changes.