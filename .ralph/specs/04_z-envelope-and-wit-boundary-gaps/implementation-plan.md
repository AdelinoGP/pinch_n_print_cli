# Implementation Plan: z-envelope-and-wit-boundary-gaps

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit existing Z-envelope enforcement

- Task IDs:
  - `TASK-127`
- Objective: Find any existing Z-envelope validation in the codebase. Confirm it is missing or incomplete.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules
  - `docs/04_host_scheduler.md` — Proactive Validation Points
- OrcaSlicer refs: None
- Verification: `grep -r "z.envelope\|z_envelope\|layer.z\|effective_layer_height" crates/slicer-host/src/` — report what exists.

### Step 2: Implement Z-envelope enforcement at output-commit

- Task IDs:
  - `TASK-127`
- Objective: Add proactive Z-envelope validation at output-commit for any module that writes path Z. Validate against `[layer.z, layer.z + effective_layer_height]`. Emit fatal contract error with required diagnostics on violation.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (output commit validation)
  - Likely `crates/slicer-host/src/layer_ir.rs` or similar (per-layer arena)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules
  - `docs/04_host_scheduler.md` — Proactive Validation Points
- OrcaSlicer refs: None
- Verification: Test that a module writing out-of-envelope Z gets fatal error with correct diagnostics.

### Step 3: Audit dispatch_postpass_gcode_call

- Task IDs:
  - `TASK-129a`
- Objective: Find `dispatch_postpass_gcode_call` and determine whether it currently receives real GCode command lists or stub data.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — world-postpass.wit
  - `docs/04_host_scheduler.md` — PostPass Execution
- OrcaSlicer refs: None
- Verification: `grep -r "dispatch_postpass_gcode_call" crates/`

### Step 4: Wire real GCode command lists into dispatch_postpass_gcode_call

- Task IDs:
  - `TASK-129a`
- Objective: Ensure real `GCodeCommand` list content from `PostPass::GCodeEmit` is passed to postpass modules, not placeholders.
- Files expected to change:
  - `crates/slicer-host/src/postpass/` or `crates/slicer-host/src/scheduler/postpass.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — GCodeIR, GCodeCommand
  - `docs/03_wit_and_manifest.md` — world-postpass.wit
- OrcaSlicer refs: None
- Verification: Test that postpass receives correct per-command content through WIT boundary.

### Step 5: Add layer-world deep-copy boundary coverage

- Task IDs:
  - `TASK-129b`
- Objective: Audit layer-world deep-copy paths. Add live-path tests proving IR fields are preserved through deep-copy vs. native fallback.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (deep-copy logic)
  - `crates/slicer-host/tests/` (boundary coverage tests)
- Authoritative docs:
  - `docs/04_host_scheduler.md` — LayerCollectionIR Lifecycle
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- test layer_world_deep_copy` — pass.

### Step 6: Add finalization-world deep-copy boundary coverage

- Task IDs:
  - `TASK-129c`
- Objective: Audit finalization-world deep-copy paths. Add live-path tests proving IR fields are preserved through deep-copy vs. native fallback.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (deep-copy logic)
  - `crates/slicer-host/tests/` (boundary coverage tests)
- Authoritative docs:
  - `docs/04_host_scheduler.md` — LayerCollectionIR Lifecycle & Memory Strategy
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- test finalization_world_deep_copy` — pass.

### Step 7: Full test suite verification

- Task IDs:
  - `TASK-127`
  - `TASK-129`
- Objective: Run full slicer-host test suite and confirm all Z-envelope, postpass GCode, and deep-copy tests pass.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- --nocapture` — all tests pass.

## Packet Completion Gate

- Z-envelope enforcement active at output-commit with correct fatal diagnostics on violation.
- Real postpass GCode command lists wired to `dispatch_postpass_gcode_call`.
- Layer-world deep-copy has live-path boundary coverage.
- Finalization-world deep-copy has live-path boundary coverage.
- All related tests pass.
- `docs/07_implementation_status.md` TASK-127/129/129a/129b/129c marked complete.
- `packet.spec.md` ready to move to `status: implemented`.