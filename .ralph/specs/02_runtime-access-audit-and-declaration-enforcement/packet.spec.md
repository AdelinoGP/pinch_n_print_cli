---
status: superseded
packet: runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
  - TASK-125
superseded_by: 02-rev1_runtime-access-audit-and-declaration-enforcement
  - TASK-126
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: runtime-access-audit-and-declaration-enforcement

## Goal

Feed `ModuleAccessAudit` from every live execution path into `DagValidationRequest.access_audits`, enforce undeclared runtime read/write faults at the WIT boundary (DEV-003), enforce the Claim Transition Matrix for non-transitionable claims (DEV-004), and fix `WriteConflict.orderable` to report `true` only when ordering can actually resolve the pair (scheduler cleanup for docs/04 contract).

## Scope Boundaries

- In scope:
  - TASK-123a: Record prepass execution audits and plumb into `DagValidationRequest.access_audits`
  - TASK-123b: Record per-layer execution audits and plumb into `DagValidationRequest.access_audits`
  - TASK-123c: Record postpass execution audits and add a live-path regression proving populated audits reach validation
  - TASK-124: Enforce undeclared runtime read/write faults at the WIT boundary; add negative harness for layer-time undeclared access
  - TASK-125: Enforce Claim Transition Matrix for `perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer` — must turn `claim_transition_matrix_tdd.rs` green
  - TASK-126: Fix `WriteConflict.orderable` so it reports `true` only when ordering can actually resolve the pair; add both positive and negative semantics tests

- Out of scope:
  - TASK-121/TASK-122 (manifest population — separate packet)
  - TASK-144/145/146 (WIT consolidation — separate packet)
  - TASK-149/150 (custom type widening — separate packet)

## Acceptance Criteria

- **Given** a prepass execution run, **when** any module reads or writes IR, **then** a `ModuleAccessAudit` entry is recorded and reaches `DagValidationRequest.access_audits`.
- **Given** a per-layer execution run, **when** any module reads or writes IR, **then** a `ModuleAccessAudit` entry is recorded and reaches `DagValidationRequest.access_audits`.
- **Given** a postpass execution run, **when** any module reads or writes IR, **then** a `ModuleAccessAudit` entry is recorded and reaches `DagValidationRequest.access_audits`.
- **Given** a live execution path, **when** a module attempts an undeclared read, **then** the host returns a fatal contract error with module id, stage id, operation, and requested path.
- **Given** a live execution path, **when** a module attempts an undeclared write, **then** the host rejects the commit with a fatal contract error with module id, stage id, operation, and requested path.
- **Given** a layer-varying claim holder for a non-transitionable claim (perimeter-generator, seam-placer, layer-planner, mesh-analyzer), **when** the host evaluates the configuration, **then** startup validation fails with a precise diagnostic naming the claim, object, and conflicting layer ranges.
- **Given** `WriteConflict` between two modules, **when** `orderable` is called, **then** it returns `true` only when topological ordering can actually resolve the write-write conflict.
- **Given** the claim transition matrix test harness, **when** it runs, **then** `claim_transition_matrix_tdd.rs` passes completely.

## Verification

- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- Live-path regression test proving `access_audits` populated on a full slice run
- Negative harness: attempting undeclared access triggers fatal error (tested in host test suite)

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract, Claim System, Allowed Claim Transition Matrix
- `docs/02_ir_schemas.md` — IR field paths
- `docs/03_wit_and_manifest.md` — WIT boundary access enforcement table
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None. This is an infra/scheduler enforcement task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`