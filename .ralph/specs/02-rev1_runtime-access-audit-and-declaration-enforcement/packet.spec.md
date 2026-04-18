---
status: draft
packet: 02-rev1_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
  - TASK-126
backlog_source: docs/07_implementation_status.md
supersedes: 02_runtime-access-audit-and-declaration-enforcement
---

# Packet Contract: 02-rev1_runtime-access-audit-and-declaration-enforcement

## Goal

Complete the runtime read-audit instrumentation at the WIT boundary (TASK-123abc), wire `runtime_reads` through all three execution tiers so undeclared read enforcement works at runtime (TASK-124), fix `WriteConflict.orderable` semantics so `true` means the conflict is actually resolvable by topological ordering (TASK-126), and add positive `orderable` test coverage.

## Scope Boundaries

- In scope:
  - Reopen TASK-123a: Record prepass execution read audits and plumb into `DagValidationRequest.access_audits`
  - Reopen TASK-123b: Record per-layer execution read audits and plumb into `DagValidationRequest.access_audits`
  - Reopen TASK-123c: Record postpass execution read audits and plumb into `DagValidationRequest.access_audits`
  - Reopen TASK-124: Verify undeclared read enforcement fires at runtime with correct diagnostics
  - Reopen TASK-126: Fix `WriteConflict.orderable` semantics; add positive semantics test

- Out of scope:
  - Claim Transition Matrix enforcement (already complete in 02 — TASK-125 done)
  - Manifest population (TASK-121/122 — separate packet)
  - `dispatch_tdd` linker error (pre-existing, separate issue)

## Acceptance Criteria

- **Given** a prepass execution run, **when** any module reads an IR field via a WIT view method, **then** a `ModuleAccessAudit` entry records the exact IR path in `runtime_reads` and reaches `DagValidationRequest.access_audits`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a prepass execution run, **when** any module writes an IR field, **then** a `ModuleAccessAudit` entry records the exact IR path in `runtime_writes` and reaches `DagValidationRequest.access_audits`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a per-layer execution run, **when** any module reads an IR field via a WIT view method, **then** a `ModuleAccessAudit` entry records the exact IR path in `runtime_reads` and reaches `DagValidationRequest.access_audits`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a postpass execution run, **when** any module reads an IR field via a WIT view method, **then** a `ModuleAccessAudit` entry records the exact IR path in `runtime_reads` and reaches `DagValidationRequest.access_audits`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** a live execution path, **when** a module attempts an undeclared read, **then** the host returns a fatal contract error with module id, stage id, operation (read), and requested path. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a live execution path, **when** a module attempts an undeclared write, **then** the host rejects the commit with a fatal contract error with module id, stage id, operation (write), and requested path. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** a `WriteConflict` where neither module reads the conflicting field, **when** `orderable` is called, **then** it returns `false`. | `cargo test --package slicer-host --test dag_validation_tdd -- write_conflict_orderable_is_false_when_neither_module_reads_conflicting_field --nocapture`

- **Given** a `WriteConflict` where one module reads the conflicting field and there is a DAG reachability edge from the writer to the reader, **when** `orderable` is called, **then** it returns `true`. | `cargo test --package slicer-host --test dag_validation_tdd -- write_conflict_orderable_is_true_when_read_establishes_dag_edge --nocapture`

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
