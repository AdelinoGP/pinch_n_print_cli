# Implementation Plan: non-planar-z-envelope

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Write TDD test file `z_envelope_contract_tdd.rs`

- Task IDs:
  - `TASK-127`
- Objective:
  Write a TDD test file that enumerates all acceptance criteria from `packet.spec.md` as failing tests, so that implementation can be driven by tests.
- Precondition:
  `crates/slicer-host/tests/z_envelope_contract_tdd.rs` does not exist.
- Postcondition:
  The file exists with 6+ test cases covering: z_below_layer_z_floor (fatal), z_above_layer_z_ceiling (fatal), catchup_layer_pass (valid), perim_only_pass (valid), z_at_floor_boundary (valid), z_at_ceiling_boundary (valid). Each test uses the existing dispatch infrastructure to drive a per-layer module call with controlled Z values.
- Files expected to change:
  - `crates/slicer-host/tests/z_envelope_contract_tdd.rs` (new file)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules
  - `docs/02_ir_schemas.md` — GlobalLayer struct
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` (model)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test z_envelope_contract_tdd 2>&1 | head -50` (tests compile and run, all fail as expected)
- Exit condition:
  `cargo test -p slicer-host --test z_envelope_contract_tdd 2>&1 | grep -E "test result|FAILED" | head -10` shows all tests failing.

### Step 2: Add envelope fields to `HostExecutionContext` and plumb from dispatch

- Task IDs:
  - `TASK-127`
- Objective:
  Add `layer_z`, `effective_layer_height`, and `catchup_z_bottom` fields to `HostExecutionContext` and update all call sites in `dispatch.rs` to pass them.
- Precondition:
  `HostExecutionContext::new` accepts only `module_id: String`. `dispatch_layer_call` has `layer_index` and `layer_z` but not `effective_layer_height` or catch-up parameters.
- Postcondition:
  Every `HostExecutionContext::new` call in dispatch paths passes `layer_z`, `effective_layer_height`, and `catchup_z_bottom: Option<f32>` sourced from the `GlobalLayer` being executed. All call sites compile.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — `HostExecutionContext` struct (add 3 fields), `HostExecutionContext::new` (update signature)
  - `crates/slicer-host/src/dispatch.rs` — all `HostExecutionContext::new` call sites updated
- Authoritative docs:
  - `docs/02_ir_schemas.md` — GlobalLayer fields
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_layer_call`
- Verification:
  - `cargo build --package slicer-host 2>&1 | grep -E "error|warning" | head -20`
- Exit condition:
  `cargo build --package slicer-host` succeeds with zero warnings.

### Step 3: Add `check_z_envelope` helper and wire into all Z-bearing `push_*` methods

- Task IDs:
  - `TASK-127`
- Objective:
  Add the `check_z_envelope` validation function to `HostExecutionContext` and call it at the top of every Z-bearing `push_*` method. Return `Err(Z_ENVELOPE_VIOLATION)` message on violation.
- Precondition:
  `push_sparse_path`, `push_solid_path`, `push_ironing_path`, `push_wall_loop`, `push_seam_candidate`, `push_support_path`, `push_interface_path`, `push_raft_path` do no Z validation.
- Postcondition:
  All 8 methods call `self.check_z_envelope(path.first().z)` or equivalent before pushing. Violations return `Ok(Err(msg))` where `msg` starts with `"Z_ENVELOPE_VIOLATION: "`. Tests from Step 1 pass.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — add `fn check_z_envelope(&self, z: f32) -> Result<(), String>`, wire into all 8 methods
- Authoritative docs:
  - `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules
  - `crates/slicer-host/src/wit_host.rs` — `HostInfillOutputBuilder`, `HostPerimeterOutputBuilder`, `HostSupportOutputBuilder` impls
- Verification:
  - `cargo test -p slicer-host --test z_envelope_contract_tdd -- --nocapture 2>&1 | grep -E "test result|ok|FAILED"`
- Exit condition:
  `cargo test -p slicer-host --test z_envelope_contract_tdd 2>&1 | grep "test result" | head -3` shows all tests passing.

### Step 4: Run workspace gates

- Task IDs:
  - `TASK-127`
- Objective:
  Ensure the full workspace build and clippy pass before declaring packet complete.
- Precondition:
  Package builds, but workspace may have warnings or other failures.
- Postcondition:
  `cargo build --workspace` succeeds and `cargo clippy --workspace -- -D warnings` returns zero warnings.
- Files expected to change:
  - None (verification only; any build/clippy failures indicate a step was missed)
- Authoritative docs:
  - `CLAUDE.md` — Build & Test Commands
- Verification:
  - `cargo build --workspace && cargo clippy --workspace -- -D warnings`
- Exit condition:
  Both commands succeed with zero warnings and zero errors.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green.
- `docs/07_implementation_status.md` updated: TASK-127 marked complete.
- Reopened or superseded packet status transitions reconciled.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
