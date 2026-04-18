# Implementation Plan: 02-rev1_runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add `runtime_reads` field to `HostExecutionContext`

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Add a `runtime_reads: Vec<String>` field to `HostExecutionContext` in `crates/slicer-host/src/wit_host.rs`. This field will accumulate exact IR paths read during a module call.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — Add field and initialize in `HostExecutionContext::new()`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — IR field path names
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- OrcaSlicer refs: None
- Verification: `grep -n "runtime_reads" crates/slicer-host/src/wit_host.rs`

### Step 2: Instrument WIT view resource methods to record reads

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Modify each WIT view resource method (`slice-region-view`, `perimeter-region-view`, etc.) to append the IR path to `HostExecutionContext::runtime_reads` when called. The exact paths must match `docs/02_ir_schemas.md` naming.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — Instrument view methods in the layer and prepass WIT worlds
- Authoritative docs:
  - `docs/02_ir_schemas.md` — IR field path names
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- OrcaSlicer refs: None
- Verification: Unit test that calls a WIT view method and asserts it appears in `runtime_reads`.

### Step 3: Wire read audits through prepass execution

- Task IDs: `TASK-123a`
- Objective: Modify `execute_prepass` to extract `ctx.runtime_reads` from `HostExecutionContext` after each module call and populate the `ModuleAccessAudit.runtime_reads` field. Return read audits alongside write audits.
- Files expected to change:
  - `crates/slicer-host/src/prepass.rs` — Extract read audits from dispatch context
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture` — read audit entries are non-empty.

### Step 4: Wire read audits through per-layer execution

- Task IDs: `TASK-123b`
- Objective: Modify `execute_single_layer` to extract `ctx.runtime_reads` and populate `ModuleAccessAudit.runtime_reads` for each layer module.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs` — Extract read audits from per-layer dispatch context
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: Integration test that runs per-layer execution and asserts non-empty `runtime_reads` in layer audits.

### Step 5: Wire read audits through postpass execution

- Task IDs: `TASK-123c`
- Objective: Modify `execute_postpass` to extract read audits from postpass dispatch context and populate `ModuleAccessAudit.runtime_reads`. (Postpass modules typically don't read IR, but any IR reads should be tracked.)
- Files expected to change:
  - `crates/slicer-host/src/postpass.rs` — Extract read audits from postpass dispatch context
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture` — postpass audits include both reads and writes.

### Step 6: Verify undeclared read enforcement fires at runtime

- Task IDs: `TASK-124`
- Objective: Run the negative test harness that attempts an undeclared read. With `runtime_reads` now populated, `validate_undeclared_access` should detect the violation and produce a fatal `SchedulerError::UndeclaredAccess` with the correct diagnostic fields (module id, stage id, operation: Read, path).
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture` — both read and write undeclared errors appear.

### Step 7: Fix `WriteConflict.orderable` semantics

- Task IDs: `TASK-126`
- Objective: Fix `validate_write_conflicts` in `crates/slicer-host/src/validation.rs` to set `orderable=true` only when there is a reachability path via the conflicting field. The correct check is: does one module's read of the field establish a DAG edge to the other module?
- Files expected to change:
  - `crates/slicer-host/src/validation.rs` — Fix `orderable` computation
- Authoritative docs:
  - `docs/04_host_scheduler.md` — WriteConflict, orderable semantics
- OrcaSlicer refs: None
- Verification: Both the existing negative test (`dag_validation_tdd: write_conflict_orderable_is_false_when_neither_module_reads_conflicting_field`) AND the new positive test from Step 8 pass.

### Step 8: Add positive `orderable` test case

- Task IDs: `TASK-126`
- Objective: Add a test in `crates/slicer-host/tests/dag_validation_tdd.rs` that creates a `WriteConflict` where module A writes field F, module B reads F and writes F, and there is a reachability path A→B. Assert `orderable == true`.
- Files expected to change:
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — Add positive orderable test
- Authoritative docs:
  - `docs/04_host_scheduler.md` — WriteConflict, orderable semantics
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- orderable --nocapture`

### Step 9: Full test suite verification

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`, `TASK-124`, `TASK-126`
- Objective: Run the full targeted test suite and confirm all access-audit, claim-matrix, and orderable tests are green.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo test --package slicer-host --test dag_validation_tdd -- --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- --nocapture`
  - `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Packet Completion Gate

- `runtime_reads` populated with exact paths in all three execution tiers.
- `validate_undeclared_access` receives non-empty `runtime_reads` and produces correct errors for undeclared reads.
- `claim_transition_matrix_tdd.rs` still green (not regressed).
- `dag_validation_tdd`: both `orderable == true` and `orderable == false` test cases pass.
- `docs/07_implementation_status.md` TASK-123abc/124/126 updated to reflect completion of the reopened items.
- `packet.spec.md` ready to move to `status: implemented`.
