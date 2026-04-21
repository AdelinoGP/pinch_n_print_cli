# Implementation Plan: path-optimization-tool-order-and-cooling-policy

## Execution Rules

- One atomic step at a time.
- Land mixed-tool tests before queue changes, then update the docs rejection path.

## Steps

### Step 1: Add failing mixed-tool ordering tests

- Task IDs:
  - `TASK-152b`
- Objective:
  Freeze the exact grouped tool order and deferred tool-change sequence on the live host path.
- Precondition:
  Mixed-tool ordering is not yet locked by focused tests.
- Postcondition:
  `tool_ordering_tdd.rs` exists with failing mixed-tool, single-tool, and redundant-change suppression assertions.
- Files expected to change:
  - `crates/slicer-host/tests/tool_ordering_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- Verification:
  - `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
  - `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
  - `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- Exit condition:
  The focused tool-order tests exist and fail only because the live path does not yet emit the expected sequence.

### Step 2: Implement grouped mixed-tool ordering and deferred `ToolChange` emission

- Task IDs:
  - `TASK-152`
  - `TASK-152b`
- Objective:
  Add grouped tool ordering on the live path and emit the exact expected deferred `ToolChange` sequence.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  The tool-ordering acceptance tests are green.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `crates/slicer-host/tests/tool_ordering_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- Verification:
  - `cargo test -p slicer-host --test tool_ordering_tdd -- --nocapture`
- Exit condition:
  Mixed-tool and redundant-change suppression tests are green.

### Step 3: Close cooling overrides explicitly on the documentation rejection path

- Task IDs:
  - `TASK-152c`
- Objective:
  Update the docs surfaces so they explicitly say live cooling overrides are intentionally unsupported on `Layer::PathOptimization`.
- Precondition:
  Tool-ordering implementation is green.
- Postcondition:
  Both docs surfaces contain the exact rejection text and TASK-152c can close on that basis.
- Files expected to change:
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md`
- Authoritative docs:
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`
- Verification:
  - `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- Exit condition:
  The exact rejection text appears in both docs surfaces.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for TASK-152 / TASK-152b / TASK-152c.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm no new cooling/fan live-path API was added in this packet.
- Record any remaining packet-local risk before status changes.