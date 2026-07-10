---
status: superseded
packet: 02-rev3_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
supersedes: 02-rev2_runtime-access-audit-and-declaration-enforcement
---

# 02-rev3_runtime-access-audit-and-declaration-enforcement

## Goal

Fix the postpass `runtime_reads` wiring (CRIT-1), add `runtime_reads` assertions to `access_audits_live_path` (CRIT-2), and replace manual audit injection with live-path evidence in `dag_validation_tdd` (CRIT-3). The prepass and per-layer dispatch wiring from 02-rev2 is preserved intact.

## Problem Statement

The 02-rev2 packet correctly preserved `runtime_reads` through the prepass and per-layer harvest boundaries (`dispatch.rs:1085`, `dispatch.rs:1214`), but three gaps remain:

1. **Postpass wiring missing (CRIT-1):** `dispatch_postpass_gcode_call` and `dispatch_postpass_text_call` return `Result<()>` / `Result<String>` respectively and discard the `HostExecutionContext` after the call. `execute_postpass` hardcodes `runtime_reads: Vec::new()` at lines 178 and 218. Even though `wit_host.rs` correctly records `"LayerCollectionIR"` reads (lines 2291-2309), those reads never reach `ModuleAccessAudit`.

2. **`access_audits_live_path` assertions missing (CRIT-2):** The test only checks audit count (3) and module ID presence. It never asserts that any audit's `runtime_reads` field is non-empty or contains `"LayerCollectionIR"`. This means AC-3 and AC-4 from 02-rev2's spec are unverified.

3. **Manual audit construction not replaced (CRIT-3):** `dag_validation_tdd`'s `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` still manually constructs `earlier_live_audit` at lines 288-298 instead of using live execution. Step 5's postcondition ("no longer depends on constructing `DagValidationRequest.access_audits` by hand") is unmet.

## Architecture Constraints

- **`WasmRuntimeDispatcher` owns `HostExecutionContext` lifetime.** Refactor dispatch methods to extract `runtime_reads` (via clone) before the context is dropped, and return those reads alongside the result.
- **Postpass runner trait (`PostpassStageRunner`) must remain backward-compatible.** The `run_gcode_postprocess` and `run_text_postprocess` signatures cannot change because `NoopPostpassRunner` in `main.rs` and test doubles depend on them. The dispatch refactor must not change the trait.
- **Prepass and layer dispatch signatures are unchanged (already correct from 02-rev2).** Do not touch `dispatch.rs` lines 1085 or 1214.
- **All WIT view methods remain unchanged (02-rev1).** `wit_host.rs` lines 2291-2309 already record `"LayerCollectionIR"` correctly.
- **`validate_undeclared_access` in `validation.rs` is unchanged.** It correctly uses `runtime_reads`.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `HostExecutionContext.runtime_reads` is populated by WIT view methods in `wit_host.rs` (unchanged).
- `wit_host.rs` already records `"LayerCollectionIR"` reads for postpass views at lines 2291-2309.
- The dispatch functions create a fresh `HostExecutionContext` per call (`dispatch.rs:626`, `dispatch.rs:688`); reads collected in that context must be extracted before the function returns.

## Locked Assumptions and Invariants

- The packet does not change the WIT world definitions or any view method signatures.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- TASK-126 behavior stays untouched.
- `NoopPostpassRunner` must continue to compile without modification.

## Risks and Tradeoffs

- **Breaking postpass trait consumers:** If the trait changes, `NoopPostpassRunner` and postpass test doubles must be updated in the same step. The selected approach avoids trait changes.
- **Return type explosion:** Changing two dispatch methods to return tuples may cascade to callers. The `PostpassStageRunner` impl on `WasmRuntimeDispatcher` is the only caller of those methods in the relevant execution path; impact is contained.
- **Thread safety:** `HostExecutionContext` is not `Sync` — extracted read paths must be copied out (via clone) before the context is consumed.
