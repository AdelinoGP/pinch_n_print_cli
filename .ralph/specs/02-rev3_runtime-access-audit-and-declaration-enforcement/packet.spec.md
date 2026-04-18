---
status: implemented
packet: 02-rev3_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
backlog_source: docs/07_implementation_status.md
supersedes: 02-rev2_runtime-access-audit-and-declaration-enforcement
---

# Packet Contract: 02-rev3_runtime-access-audit-and-declaration-enforcement

## Goal

Fix the postpass `runtime_reads` wiring (CRIT-1), add `runtime_reads` assertions to `access_audits_live_path` (CRIT-2), and replace manual audit injection with live-path evidence in `dag_validation_tdd` (CRIT-3). The prepass and per-layer dispatch wiring from 02-rev2 is preserved intact.

## Scope Boundaries

- In scope:
  - TASK-123c: Refactor postpass dispatch boundary so `HostExecutionContext.runtime_reads` reaches `ModuleAccessAudit` for read-performing postpass modules
  - TASK-123c: Add `runtime_reads` assertions to `access_audits_live_path` proving non-empty reads for LayerCollectionIR
  - TASK-123a/TASK-123b: Add live-path assertions to prove prepass and per-layer audits carry exact IR paths (MeshIR, SliceIR.regions.polygons) through to collected audits
  - TASK-124: Replace manual `earlier_live_audit` construction in `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` with live execution path evidence

- Out of scope:
  - WIT view instrumentation (unchanged from 02-rev1)
  - `WriteConflict.orderable` semantics (unchanged from 02-rev1)
  - Claim Transition Matrix (already green)
  - `dispatch_tdd` linker error (pre-existing)
  - Changes to prepass or layer dispatch signatures or harvest logic (already correct per 02-rev2)

## Prerequisites and Blockers

- Depends on:
  - `02-rev2_runtime-access-audit-and-declaration-enforcement` marked `status: superseded` (this packet completes its work)
  - 02-rev1 WIT view instrumentation in `crates/slicer-host/src/wit_host.rs` remaining intact
- Unblocks:
  - Clean closure of TASK-123a/123b/123c (final wiring) and TASK-124 (live-path replacement)
- Activation blockers:
  - None beyond the superseded packet update

## Acceptance Criteria

- **Given** a postpass module that reads `LayerCollectionIR` through WIT views, **when** `execute_postpass` records the audit for that module, **then** the audit's `runtime_reads` is non-empty and contains `"LayerCollectionIR"` while the same module's `runtime_writes` entry for `"GCodeIR"` is preserved. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** a postpass module that does not read any IR (write-only gcode/text processing), **when** `execute_postpass` records the audit for that module, **then** the audit's `runtime_reads` is empty (`len() == 0`). | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** `access_audits_live_path` runs live postpass fixtures through `run_pipeline`, **when** the audits are collected, **then** the test asserts both read-performing and write-only audit entries have the correct `runtime_reads` field content. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** a read-performing postpass module is dispatched via `WasmRuntimeDispatcher`, **when** the module's WIT call returns, **then** `dispatch_postpass_gcode_call` / `dispatch_postpass_text_call` return the collected `runtime_reads` alongside the call result so `execute_postpass` can populate `ModuleAccessAudit`. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`

- **Given** a live prepass module reads mesh data through prepass WIT views, **when** the collected `prepass_audits` are asserted, **then** at least one audit entry has `runtime_reads` containing `"MeshIR"`. | `cargo test --package slicer-host --test pipeline_tdd -- prepass_audits_live_path --nocapture`

- **Given** a live per-layer module reads slice geometry through layer WIT views, **when** the collected `layer_audits` are asserted, **then** at least one audit entry has `runtime_reads` containing `"SliceIR.regions.polygons"`. | `cargo test --package slicer-host --test pipeline_tdd -- layer_audits_live_path --nocapture`

## Negative Test Cases

- **Given** a live execution path where a postpass module reads the undeclared path `"LayerCollectionIR"` through WIT views but the dispatch boundary discards `runtime_reads`, **when** the resulting audit is collected, **then** the audit has `runtime_reads: Vec::new()` and the positive assertion fails. | `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture` (must FAIL before fix)

- **Given** `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` constructs `DagValidationRequest` with a manually built `ModuleAccessAudit` instead of using live execution, **when** the test runs, **then** the test is incomplete and the packet cannot be accepted. | `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture` (must FAIL before fix)

## Verification

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- prepass_audits_live_path --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- layer_audits_live_path --nocapture`
- `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract, Claim System
- `docs/02_ir_schemas.md` — IR field path names (MeshIR, SliceIR, LayerCollectionIR)
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- `docs/04_host_scheduler.md` — DagValidationRequest, ModuleAccessAudit, WriteConflict

## OrcaSlicer Reference Obligations

None. This is an infra/scheduler enforcement task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`