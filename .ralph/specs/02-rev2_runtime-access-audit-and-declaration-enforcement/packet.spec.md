---
status: draft
packet: 02-rev2_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
backlog_source: docs/07_implementation_status.md
supersedes: 02-rev1_runtime-access-audit-and-declaration-enforcement
---

# Packet Contract: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Goal

Complete the `runtime_reads` wiring from `HostExecutionContext` through all three execution tiers into `ModuleAccessAudit`, so that live prepass, per-layer, and postpass execution paths produce populated read-audit entries — and add live-path test coverage proving the full chain from WIT view call to `DagValidationRequest.access_audits` works end-to-end.

## Scope Boundaries

- In scope:
  - TASK-123a: Extract `runtime_reads` from `HostExecutionContext` in prepass dispatch and wire to `ModuleAccessAudit`
  - TASK-123b: Extract `runtime_reads` from `HostExecutionContext` in per-layer dispatch and wire to `ModuleAccessAudit`
  - TASK-123c: Extract `runtime_reads` from `HostExecutionContext` in postpass dispatch and wire to `ModuleAccessAudit`
  - TASK-124: Add live-path integration test proving undeclared-read enforcement fires on actual execution

- Out of scope:
  - WIT view method instrumentation (already done in 02-rev1)
  - `WriteConflict.orderable` semantics (already fixed in 02-rev1)
  - Claim Transition Matrix (already green)
  - `dispatch_tdd` linker error (pre-existing)

## Acceptance Criteria

- **Given** a prepass module call via `dispatch_prepass_call`, **when** the module invokes any WIT view method, **then** the returned `HostExecutionContext.runtime_reads` contains the exact IR path(s) and `execute_prepass` propagates them into `ModuleAccessAudit.runtime_reads`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a per-layer module call via `dispatch_layer_call`, **when** the module invokes any WIT view method, **then** the returned `HostExecutionContext.runtime_reads` contains the exact IR path(s) and `execute_single_layer` propagates them into `ModuleAccessAudit.runtime_reads`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a postpass module call, **when** the module invokes any WIT view method, **then** the returned `HostExecutionContext.runtime_reads` contains the exact IR path(s) and `execute_postpass` propagates them into `ModuleAccessAudit.runtime_reads`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** a live prepass execution run with a module that reads an undeclared IR path via a WIT view method, **when** the module calls the WIT view method at runtime, **then** `validate_undeclared_access` produces a fatal `UndeclaredAccess` error with `access: Read` and the exact path. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a live execution path, **when** `run_pipeline` collects audits from all three tiers and passes them to `validate_startup_dag`, **then** the resulting `DagValidationReport` shows non-empty `runtime_reads` for the modules that performed reads. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

## Verification

- `cargo test --package slicer-host --test dag_validation_tdd -- --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract, Claim System
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None. This is an infra/scheduler enforcement task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
