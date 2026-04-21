# Implementation Plan: orca-gcode-emission-contract

## Execution Rules

- One atomic step at a time.
- Each step maps back to the TASK-119 family.
- TDD first, then implementation, then the narrowest text assertion.

## Steps

### Step 1: Freeze the Orca header and role-label contract in focused emitter tests

- Task IDs:
  - `TASK-119`
  - `TASK-119a`
- Objective:
  Add failing tests that enumerate the exact header ordering and role-boundary labels the host must emit.
- Precondition:
  `DefaultGCodeEmitter` still emits mostly raw move lines without a canonical Orca contract.
- Postcondition:
  `gcode_emit_tdd.rs` names exact header lines and role labels for a synthetic fixture layer stream.
- Files expected to change:
  - `crates/slicer-host/tests/gcode_emit_tdd.rs`
  - `crates/slicer-host/src/gcode_emit.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.cpp`
- Verification:
  - `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_layer_headers_before_first_extrusion -- --exact --nocapture`
  - `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_type_comments_at_role_boundaries -- --exact --nocapture`
- Exit condition:
  Both tests exist and fail only because the live host emit path does not yet produce the expected lines.

### Step 2: Emit canonical layer headers and contiguous role boundaries from the real host path

- Task IDs:
  - `TASK-119a`
  - `TASK-119b`
- Objective:
  Implement canonical header and role-label emission in `DefaultGCodeEmitter` and serializer.
- Precondition:
  Step 1 tests are in place and failing on the live emit path.
- Postcondition:
  The host emits exact header and `;TYPE:` lines from the real postpass path without test-only formatting helpers.
- Files expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/tests/gcode_emit_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.hpp`
- Verification:
  - `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_layer_headers_before_first_extrusion -- --exact --nocapture`
  - `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_type_comments_at_role_boundaries -- --exact --nocapture`
- Exit condition:
  Both tests pass and the emitted lines are produced from the host implementation itself.

### Step 3: Serialize seam-preserving wall starts and canonical retract/travel/Z-hop ordering

- Task IDs:
  - `TASK-119b`
- Objective:
  Preserve seam-started wall loops and serialize retract/unretract/travel/Z-hop commands in the canonical emitted order.
- Precondition:
  Header and role-label emission is already green.
- Postcondition:
  Focused text tests prove seam-started loops are preserved and travel-related commands serialize in a deterministic order.
- Files expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-sdk/src/postpass_builders.rs`
  - `crates/slicer-host/tests/gcode_emit_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test gcode_emit_tdd preserves_seam_started_wall_loop_order_in_output -- --exact --nocapture`
  - `cargo test -p slicer-host --test gcode_emit_tdd serializes_retract_travel_and_z_hop_in_canonical_order -- --exact --nocapture`
  - `cargo test -p slicer-host --test gcode_emit_tdd omits_absent_role_labels_and_retraction_lines -- --exact --nocapture`
- Exit condition:
  All three focused text tests pass, including the negative omission guard.

### Step 4: Add a whole-postpass contract regression

- Task IDs:
  - `TASK-119c`
- Objective:
  Lock the full host postpass round-trip with a synthetic fixture that exercises comments, raw lines, tool changes, retracts, and role labels together.
- Precondition:
  Steps 2 and 3 are green on focused emitter tests.
- Postcondition:
  A whole-postpass regression proves the canonical contract survives `execute_postpass()` end-to-end and remains deterministic across repeats.
- Files expected to change:
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs`
  - `crates/slicer-host/src/postpass.rs`

  Note: `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` is a neighboring WASM-module boundary test and is not owned by this packet; it was incorrectly listed in an earlier draft.
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/tests/fff_print/test_gcode.cpp`
- Verification:
  - `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd full_postpass_pipeline_preserves_orca_emission_contract -- --exact --nocapture`
- Exit condition:
  The whole-postpass regression passes and is deterministic across repeated runs.

## Packet Completion Gate

- All four steps complete.
- Every step exit condition is met.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for TASK-119 / TASK-119a / TASK-119b / TASK-119c.
- `packet.spec.md` is ready to move from `draft` to `implemented` once the acceptance ceremony is complete.

## Acceptance Ceremony

- Re-run every pipe-suffixed command from `packet.spec.md`.
- Re-run the whole-postpass regression twice to confirm deterministic output.
- Record any remaining packet-local risk before status changes.