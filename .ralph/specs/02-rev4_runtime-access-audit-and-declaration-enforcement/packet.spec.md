---
status: draft
packet: 02-rev4_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123c
  - TASK-124
backlog_source: docs/07_implementation_status.md
supersedes: 02-rev3_runtime-access-audit-and-declaration-enforcement
---

# Packet Contract: 02-rev4_runtime-access-audit-and-declaration-enforcement

## Goal

Fix the two CRIT findings from the 02-rev3 audit: (1) add a read-performing postpass test variant to `access_audits_live_path` so AC-1's positive assertion (postpass module reading `LayerCollectionIR` produces non-empty `runtime_reads`) is exercised by a real test with an actual read-performing runner, and (2) replace the simulated `collect_dispatch_audit` helper in `dag_validation_tdd` with a test-only dispatch helper that exercises real WIT view calls via the `WasmRuntimeDispatcher` and returns live `runtime_reads` data.

## Scope Boundaries

- In scope:
  - TASK-123c: Add `PostpassModuleReadingPostpassRunner` custom runner to `pipeline_tdd.rs` that simulates a postpass module reading `LayerCollectionIR` via WIT views; update `access_audits_live_path` to run both write-only and read-performing postpass variants and assert correct `runtime_reads` content for each
  - TASK-124: Replace `collect_dispatch_audit` simulation in `dag_validation_tdd` with a test helper that uses `WasmRuntimeDispatcher` dispatch internally to produce live `runtime_reads` data; the helper must actually call dispatch methods and extract reads, not hardcode them

- Out of scope:
  - WIT view instrumentation (unchanged from 02-rev1)
  - Claim Transition Matrix (already green)
  - Changes to prepass or layer tests (already correct per 02-rev3)
  - `dispatch_tdd` linker error (pre-existing)
  - Changes to prepass or layer dispatch signatures (already correct from 02-rev2)
  - Changes to `execute_postpass`, `dispatch_postpass_gcode_call`, or `dispatch_postpass_text_call` signatures (already correct from 02-rev3)

## Prerequisites and Blockers

- Depends on:
  - `02-rev3_runtime-access-audit-and-declaration-enforcement` marked `status: superseded`
  - `WasmRuntimeDispatcher` dispatch infrastructure intact from 02-rev3
- Unblocks:
  - Clean closure of TASK-123c (read-performing postpass test) and TASK-124 (live-path dag validation audit)
- Activation blockers:
  - None beyond the superseded packet update

## Acceptance Criteria

- **Given** `access_audits_live_path` runs a write-only postpass module via `NoopPostpassRunner`, **when** `run_pipeline` collects audits via `execute_postpass` and `take_runtime_reads`, **then** the audit has `runtime_reads.is_empty()` and `runtime_writes` containing `"GCodeIR"`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** `access_audits_live_path_read_performing` runs a read-performing postpass module via `PostpassModuleReadingPostpassRunner` (which returns `runtime_reads` containing `"LayerCollectionIR"`), **when** `run_pipeline` collects audits via `execute_postpass` and `take_runtime_reads`, **then** the audit has `runtime_reads` containing `"LayerCollectionIR"`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture`

- **Given** `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` uses a dispatch helper that internally calls `WasmRuntimeDispatcher` dispatch methods to produce `earlier_live_audit`, **when** the test runs, **then** the audit's `runtime_reads` are produced by actual dispatch execution, not hardcoded simulation, and the test still correctly detects undeclared-read and undeclared-write violations. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

## Negative Test Cases

- **Given** `access_audits_live_path_read_performing` uses `NoopPostpassRunner` (which returns empty `runtime_reads`) for the read-performing postpass module, **when** the test asserts `runtime_reads` is non-empty for that module, **then** the assertion fails and the test is incomplete (the test must use a read-performing runner variant). | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture` (must FAIL before `PostpassModuleReadingPostpassRunner` is added)

- **Given** `dag_validation_tdd`'s `collect_dispatch_audit` is replaced with a hardcoded simulation that does not call `WasmRuntimeDispatcher`, **when** the test runs, **then** the packet is not accepted because the postcondition ("uses a test-only dispatch helper that exercises real WIT view calls") is unmet. | Inspection of `dag_validation_tdd.rs` confirms `collect_dispatch_audit` calls `WasmRuntimeDispatcher` methods, not hardcoded data.

## Verification

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture`
- `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract
- `docs/02_ir_schemas.md` — IR field path names (MeshIR, SliceIR, LayerCollectionIR)
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- `docs/04_host_scheduler.md` — DagValidationRequest, ModuleAccessAudit

## OrcaSlicer Reference Obligations

None. This is an infra/scheduler enforcement task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
