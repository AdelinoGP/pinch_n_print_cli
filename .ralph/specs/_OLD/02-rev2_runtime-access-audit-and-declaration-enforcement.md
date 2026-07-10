---
status: superseded
packet: 02-rev2_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
supersedes: 02-rev1_runtime-access-audit-and-declaration-enforcement
---

# 02-rev2_runtime-access-audit-and-declaration-enforcement

## Goal

Complete the `runtime_reads` wiring from `HostExecutionContext` through all three execution tiers into `ModuleAccessAudit`, so that live prepass, per-layer, and postpass execution paths produce populated read-audit entries — and add live-path test coverage proving the full chain from WIT view call to `DagValidationRequest.access_audits` works end-to-end.

## Problem Statement

The 02-rev1 packet added `runtime_reads` to `HostExecutionContext` and instrumented all WIT view methods to populate it. However, the `HostExecutionContext` is consumed inside `WasmRuntimeDispatcher` dispatch calls and `runtime_reads` is never extracted and passed to `ModuleAccessAudit`. All three execution tiers (`execute_prepass`, `execute_single_layer`, `execute_postpass`) create `ModuleAccessAudit` entries with `runtime_reads: Vec::new()`.

The architectural root cause splits by tier:

1. `dispatch_prepass_call` and `dispatch_layer_call` already return `HostExecutionContext`, but the tier-specific harvest and commit flow consumes that context without preserving `runtime_reads` before `ModuleAccessAudit` is created.
2. Postpass dispatch currently hides runtime reads behind `dispatch_postpass_gcode_call` / `dispatch_postpass_text_call`, so the postpass audit path never sees read data even though `wit_host.rs` records `LayerCollectionIR` reads.

Result:

1. Positive audit assertions fail because all three tiers still write `runtime_reads: Vec::new()` on the host side.
2. Undeclared-read enforcement only has confidence through a manually injected `ModuleAccessAudit`; the live execution path is untested.
3. `access_audits_live_path` in `pipeline_tdd` passes but doesn't assert `runtime_reads` is non-empty — it only checks audit count and module IDs.

## Architecture Constraints

- **`WasmRuntimeDispatcher`** owns the `HostExecutionContext` lifetime. The design must preserve `runtime_reads` before the context is consumed by harvest helpers.
- **Prepass and layer dispatch already return `HostExecutionContext`.** The packet must not invent a return-type change there unless the code actually needs it.
- **Postpass currently erases read data at the runner boundary.** Any design must surface postpass read paths without losing the existing `PostpassOutput` behavior.
- **Backward compatibility**: `NoopPrepassRunner`, `NoopLayerRunner`, and `NoopPostpassRunner` in `main.rs` must continue to compile. If a trait changes, the packet must update those stubs explicitly.
- **All WIT view methods are already instrumented** (02-rev1) — they push IR paths to `ctx.runtime_reads`. No changes needed to view method implementations.
- **`validate_undeclared_access`** in `validation.rs` already correctly uses `runtime_reads`. No changes needed.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `HostExecutionContext.runtime_reads` is populated by WIT view methods in `wit_host.rs` (unchanged from 02-rev1).
- `wit_host.rs` already records postpass reads as `"LayerCollectionIR"`; postpass audits must not assume reads are always empty.
- The `execute_*` functions are the only places that construct `ModuleAccessAudit`; audit population changes are local to those functions and the dispatcher helpers that feed them.

## Locked Assumptions and Invariants

- The packet preserves the current WIT host read-path strings rather than renaming them.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- `TASK-126` behavior stays untouched.

## Risks and Tradeoffs

- **Breaking postpass trait consumers**: if the postpass runner surface changes, `NoopPostpassRunner` and any postpass test doubles must be updated in the same step.
- **Partial fixes that only change tests**: the packet must not stop after adding assertions; the runtime audit flow must become live for all three tiers.
- **Thread safety**: `HostExecutionContext` is not `Sync` — extracted read paths must be copied out before the context is consumed.
