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
  - TASK-123a: Preserve `HostExecutionContext.runtime_reads` while harvesting prepass output and wire the resulting read paths to `ModuleAccessAudit`
  - TASK-123b: Preserve `HostExecutionContext.runtime_reads` from layer dispatch through `execute_single_layer` audit construction
  - TASK-123c: Surface postpass read paths from the postpass dispatch boundary and wire them to `ModuleAccessAudit`
  - TASK-124: Add live-path integration test proving undeclared-read enforcement fires on actual execution

- Out of scope:
  - WIT view method instrumentation (already done in 02-rev1)
  - `WriteConflict.orderable` semantics (already fixed in 02-rev1)
  - Claim Transition Matrix (already green)
  - `dispatch_tdd` linker error (pre-existing)

## Prerequisites and Blockers

- Depends on:
  - 02-rev1 WIT view instrumentation in `crates/slicer-host/src/wit_host.rs` remaining intact so view methods continue to push exact runtime read paths.
  - 02-rev1 `WriteConflict.orderable` fix staying unchanged; TASK-126 is not reopened here.
- Unblocks:
  - Clean closure of `TASK-123a`, `TASK-123b`, `TASK-123c`, and `TASK-124` without another reopen packet.
- Activation blockers:
  - None in packet content after this retrofit. The packet remains `draft` until explicitly activated.

## Acceptance Criteria

- **Given** the live prepass guest reads mesh data through prepass WIT views, **when** `execute_prepass` records the audit for that module, **then** the audit's `runtime_reads` is non-empty and includes `"MeshIR"` instead of `Vec::new()`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** the live per-layer guest reads slice geometry through layer WIT views, **when** `execute_single_layer` records the audit for that module, **then** the audit's `runtime_reads` is non-empty and includes `"SliceIR.regions.polygons"` instead of `Vec::new()`. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

- **Given** the live postpass guest reads layer-collection data through postpass WIT views, **when** `execute_postpass` records the audit for that module, **then** the audit's `runtime_reads` is non-empty and includes `"LayerCollectionIR"` while still preserving the expected `runtime_writes` entry for `"GCodeIR"`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** `access_audits_live_path` runs the packet's live postpass fixtures through `run_pipeline`, **when** the audits are collected, **then** read-performing modules no longer record `runtime_reads: Vec::new()` and explicitly write-only modules still record an empty `runtime_reads` vector. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

## Negative Test Cases

- **Given** a live execution path where a module reads the undeclared path `"SliceIR.regions.undeclared"`, **when** the runtime audit reaches `validate_undeclared_access`, **then** the resulting diagnostic contains `SchedulerError::UndeclaredAccess { access: Read, path: "SliceIR.regions.undeclared", .. }` for that module and stage. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`

## Verification

- `cargo build --package slicer-host`
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
