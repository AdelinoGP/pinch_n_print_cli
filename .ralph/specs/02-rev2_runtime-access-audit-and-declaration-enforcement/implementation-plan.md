# Implementation Plan: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Survey all callers of dispatch functions

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Verify exactly which functions call `dispatch_prepass_call`, `dispatch_layer_call`, and `dispatch_postpass_call`. No callers should be missed.
- Files expected to change: None
- Authoritative docs: N/A
- OrcaSlicer refs: None
- Verification: `grep -rn "dispatch_prepass_call\|dispatch_layer_call\|dispatch_postpass_call" crates/slicer-host/src/`

### Step 2: Change dispatch call return types to include HostExecutionContext

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Change `dispatch_prepass_call`, `dispatch_layer_call`, and `dispatch_postpass_call` to return `(HostExecutionContext, Output)` so callers can extract `runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs` — update return types and body
- Authoritative docs:
  - `docs/04_host_scheduler.md` — dispatch semantics
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` (verify no type errors from return-type change)

### Step 3: Wire runtime_reads through prepass execution

- Task IDs: `TASK-123a`
- Objective: In `impl PrepassStageRunner for WasmRuntimeDispatcher::run_stage`, unpack the returned `HostExecutionContext` and forward `ctx.runtime_reads` into `ModuleAccessAudit.runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/src/prepass.rs` — extract `runtime_reads` and pass to `ModuleAccessAudit`
  - `crates/slicer-host/src/dispatch.rs` — update `impl PrepassStageRunner` call site
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture` — verify prepass read audit entries are non-empty

### Step 4: Wire runtime_reads through per-layer execution

- Task IDs: `TASK-123b`
- Objective: In `impl LayerStageRunner for WasmRuntimeDispatcher::run_stage`, unpack the returned `HostExecutionContext` and forward `ctx.runtime_reads` into `ModuleAccessAudit.runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs` — extract `runtime_reads` and pass to `ModuleAccessAudit`
  - `crates/slicer-host/src/dispatch.rs` — update `impl LayerStageRunner` call site
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture` — verify layer read audit entries are non-empty

### Step 5: Wire runtime_reads through postpass execution

- Task IDs: `TASK-123c`
- Objective: In `execute_postpass`, unpack `HostExecutionContext` from dispatch call and forward `ctx.runtime_reads` into `ModuleAccessAudit.runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/src/postpass.rs` — extract `runtime_reads` and pass to `ModuleAccessAudit`
  - `crates/slicer-host/src/dispatch.rs` — update postpass dispatch call site
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture` — postpass audits include non-empty `runtime_reads`

### Step 6: Enhance access_audits_live_path to verify runtime_reads content

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Update `access_audits_live_path` in `pipeline_tdd.rs` to assert that `runtime_reads` is non-empty for modules that should have performed reads, and that modules performing only writes have empty `runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — enhance `access_audits_live_path` assertions
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

### Step 7: Add live-path undeclared-read enforcement test

- Task IDs: `TASK-124`
- Objective: Add an integration test that runs a prepass or layer module that reads an undeclared path via WIT view, then verifies `validate_undeclared_access` fires with the correct diagnostics (module id, stage id, operation: Read, path).
- Files expected to change:
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — add or extend live-path test
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

### Step 8: Full test suite verification

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`, `TASK-124`
- Objective: Run the full targeted test suite and confirm all access-audit, claim-matrix, and pipeline tests are green.
- Files expected to change: None
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo test --package slicer-host --test dag_validation_tdd -- --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- --nocapture`
  - `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Packet Completion Gate

- `dispatch_prepass_call`, `dispatch_layer_call`, and `dispatch_postpass_call` return `HostExecutionContext` alongside typed output.
- All three execution tiers produce `ModuleAccessAudit` with non-empty `runtime_reads` for modules performing WIT reads.
- `access_audits_live_path` asserts `runtime_reads` content is non-empty.
- Live-path undeclared-read enforcement test proves full chain from WIT call to validation error.
- `claim_transition_matrix_tdd.rs` still green (not regressed).
- `docs/07_implementation_status.md` TASK-123abc/124 updated to reflect completion.
- `02-rev1_runtime-access-audit-and-declaration-enforcement/packet.spec.md` marked `status: superseded`.
- This packet's `packet.spec.md` ready to move to `status: implemented`.
