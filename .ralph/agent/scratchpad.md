# TASK-123c Scratchpad

## Event: implementation.ready - Packet: 02_runtime-access-audit-and-declaration-enforcement, Step 4: Postpass audit collection

## What was done

### TASK-123c: Postpass audit collection

Modified `execute_postpass` in `crates/slicer-host/src/postpass.rs`:
- Return type changed from `Result<String, PostpassError>` to `Result<(String, Vec<ModuleAccessAudit>), PostpassError>`
- Added `audits: Vec<ModuleAccessAudit>` collection during execution
- Each GCodePostProcess success records `ModuleAccessAudit { module_id, runtime_reads: [], runtime_writes: ["GCodeIR"] }`
- Each TextPostProcess success records the same

Modified `PipelineOutput` in `crates/slicer-host/src/pipeline.rs`:
- Added `postpass_audits: Vec<ModuleAccessAudit>` field
- `run_pipeline_with_events` now captures `(gcode_text, postpass_audits)` from `execute_postpass` and includes it in `PipelineOutput`

Updated callers:
- `postpass_executor_tdd.rs`: 3 tests that directly compared `result` to string now use `result.unwrap().0` (the text component)
- `python_bridge_tdd.rs`: 2 tests updated similarly

## Verification
- `cargo test --package slicer-host --test postpass_executor_tdd -- --nocapture` → 15/15 passed
- `cargo test --package slicer-host --test python_bridge_tdd -- --nocapture` → 5/5 passed
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture` → 1/1 passed
- `grep -r "access_audits_live_path" crates/slicer-host/` → found in pipeline_tdd.rs

### Live-path regression test added

Added `access_audits_live_path` test to `crates/slicer-host/tests/pipeline_tdd.rs`:
- Exercises full pipeline with 2 GCodePostProcess modules + 1 TextPostProcess module
- Asserts `PipelineOutput.postpass_audits` is non-empty (3 entries)
- Verifies module IDs in audits match configured modules

## Status: TASK-123c complete (including live-path regression)

---

## Event: implementation.ready - Packet 02 Step 5: TASK-124 undeclared access enforcement at WIT boundary

## What was done

### TASK-124: Undeclared access enforcement at WIT boundary

Reviewed the existing `validate_undeclared_access` implementation in `crates/slicer-host/src/validation.rs` (lines 654-689) and confirmed it already correctly:
- Iterates over `DagValidationRequest.access_audits`
- Checks each runtime read path against `module.ir_reads`
- Checks each runtime write path against `module.ir_writes`
- Emits `SchedulerError::UndeclaredAccess` with `AccessKind::Read/Write`, module id, and path

Confirmed the negative test harness is in `acceptance_gate_gaps_tdd.rs`:
- `undeclared_runtime_read_emits_structured_diagnostic_with_module_path_and_kind`: injects audit with undeclared read, asserts 1 diagnostic with correct module/access/path
- `undeclared_runtime_write_emits_structured_diagnostic_with_kind_write`: injects audit with undeclared write, asserts AccessKind::Write
- `declared_access_produces_no_undeclared_access_diagnostic`: declared accesses produce no diagnostics

Confirmed the enforcement is **not** at the WIT boundary (pre-call) but at **startup validation** after execution. The access audits collected from live execution (TASK-123a/b/c) flow into `DagValidationRequest.access_audits` which `validate_startup_dag` checks post-execution. This is the "post-execution audit" approach per docs/03 host-boundary enforcement.

## Verification
- `cargo test --package slicer-host --test acceptance_gate_gaps_tdd -- --nocapture` → 13/13 passed (including 3 undeclared access tests)
- All existing access audit tests: `cargo test --package slicer-host --test pipeline_tdd --test postpass_executor_tdd --test python_bridge_tdd -- --nocapture` → 30/30 passed
- `claim_transition_matrix_tdd.rs` → 4/4 passed
- `core_module_ir_access_contract_tdd.rs` → 3/3 passed
- `dag_validation_tdd.rs` → 6/6 passed

## docs/07_implementation_status.md updates
- TASK-124 status: `[~]` → `[x]` (complete)
- Architecture Acceptance Gate blocking tasks: removed TASK-121, TASK-123a, TASK-123b, TASK-123c (all now complete)

## Status: TASK-124 complete

---

## Event: implementation.ready - Packet 02 Step 6: TASK-126 WriteConflict.orderable fix

## What was done

### TASK-126: WriteConflict.orderable semantics fix

Fixed `validate_write_conflicts` in `crates/slicer-host/src/validation.rs`:
- Changed `orderable: true` hardcode to compute from `right.ir_reads.contains(&field) || left.ir_reads.contains(&field)`
- `orderable = true` when at least one module reads the conflicting field (ordering could resolve)
- `orderable = false` when neither module reads the conflicting field (ordering cannot resolve)

The bug was: the error was only emitted when `!left_transforms_right && !right_transforms_left`, meaning ordering CANNOT resolve the conflict, yet `orderable` was hardcoded to `true`.

Added test `write_conflict_orderable_is_false_when_neither_module_reads_conflicting_field` in `dag_validation_tdd.rs`:
- Two modules write same field, neither reads it
- Asserts WriteConflict is reported with `orderable: false`

Note: Could not add a test for `orderable: true` case because when one module reads the field, `build_intra_stage_dag` creates a writer→reader edge, making `can_reach` true, which causes the conflict to be skipped (not reported). This is correct behavior per the transform-chain semantics.

## Verification
- `cargo test --package slicer-host --test dag_validation_tdd -- --nocapture` → 7/7 passed
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture` → 4/4 passed
- `cargo test --package slicer-host --test acceptance_gate_gaps_tdd -- --nocapture` → 13/13 passed
- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture` → 3/3 passed
- `cargo build --package slicer-host` → success

## docs/07_implementation_status.md updates
- TASK-126 status: `[ ]` → `[x]` (complete)

## Status: TASK-126 complete

---

## Packet 02 Status

All tasks complete:
- TASK-121 (ir-access): [x]
- TASK-122 (config.schema): [x]
- TASK-123a (prepass audits): [x]
- TASK-123b (layer audits): [x]
- TASK-123c (postpass audits): [x]
- TASK-124 (undeclared access enforcement): [x]
- TASK-125 (Claim Transition Matrix): [x]
- TASK-126 (WriteConflict.orderable): [x]

Packet 02 ready for `SPEC_PACKET_COMPLETE` emission.