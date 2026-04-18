# Requirements: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs (reopened from 02-rev1):
  - `TASK-123a` — Prepass `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-123b` — Per-layer `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-123c` — Postpass `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-124` — Live-path undeclared-read enforcement not validated end-to-end
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `02-rev1_runtime-access-audit-and-declaration-enforcement` (status: implemented — incomplete)

## Problem Statement

The 02-rev1 packet added `runtime_reads` to `HostExecutionContext` and instrumented all WIT view methods to populate it. However, the `HostExecutionContext` is consumed inside `WasmRuntimeDispatcher` dispatch calls and `runtime_reads` is never extracted and passed to `ModuleAccessAudit`. All three execution tiers (`execute_prepass`, `execute_single_layer`, `execute_postpass`) create `ModuleAccessAudit` entries with `runtime_reads: Vec::new()`.

The architectural root cause: `dispatch_prepass_call` returns `Result<HostExecutionContext, DispatchError>` but the `impl PrepassStageRunner for WasmRuntimeDispatcher::run_stage` discards the `HostExecutionContext` after extracting only the typed output (e.g., `LayerPlanIR`). The context is consumed, `runtime_reads` is lost.

Result:
1. AC-1/AC-3/AC-4 (prepass/per-layer/postpass read audits) fail — all `runtime_reads` are empty.
2. AC-5 (undeclared read enforcement) passes only because the test manually injects `ModuleAccessAudit`; the live execution path is untested.
3. `access_audits_live_path` in `pipeline_tdd` passes but doesn't assert `runtime_reads` is non-empty — it only checks audit count and module IDs.

## In Scope

- Refactor dispatch calls to return `HostExecutionContext` alongside typed output so `runtime_reads` can be extracted
- Wire `runtime_reads` from prepass dispatch into `ModuleAccessAudit`
- Wire `runtime_reads` from per-layer dispatch into `ModuleAccessAudit`
- Wire `runtime_reads` from postpass dispatch into `ModuleAccessAudit`
- Add live-path test asserting non-empty `runtime_reads` for modules that perform reads
- Add live-path integration test for undeclared-read enforcement

## Out of Scope

- WIT view method instrumentation (already done in 02-rev1)
- `WriteConflict.orderable` fix (already done in 02-rev1)
- Claim Transition Matrix enforcement (already green)
- `dispatch_tdd` linker error (pre-existing)

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- `dispatch_prepass_call`, `dispatch_layer_call`, and `dispatch_postpass_call` return `HostExecutionContext` alongside typed output
- `execute_prepass` produces `ModuleAccessAudit` with non-empty `runtime_reads` for modules that invoke WIT view methods
- `execute_single_layer` produces `ModuleAccessAudit` with non-empty `runtime_reads` for modules that invoke WIT view methods
- `execute_postpass` produces `ModuleAccessAudit` with non-empty `runtime_reads` for modules that invoke WIT view methods
- `access_audits_live_path` asserts `runtime_reads` content is non-empty
- Live-path undeclared-read enforcement test proves the full chain works
