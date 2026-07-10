---
status: implemented
packet: 02-rev4_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123c
  - TASK-124
supersedes: 02-rev3_runtime-access-audit-and-declaration-enforcement
---

# 02-rev4_runtime-access-audit-and-declaration-enforcement

## Goal

Fix the two CRIT findings from the 02-rev3 audit: (1) add a read-performing postpass test variant to `access_audits_live_path` so AC-1's positive assertion (postpass module reading `LayerCollectionIR` produces non-empty `runtime_reads`) is exercised by a real test with an actual read-performing runner, and (2) replace the simulated `collect_dispatch_audit` helper in `dag_validation_tdd` with a test-only dispatch helper that exercises real WIT view calls via the `WasmRuntimeDispatcher` and returns live `runtime_reads` data.

## Problem Statement

The 02-rev3 audit identified two CRIT findings that must be addressed:

1. **CRIT-1 — AC-1's positive assertion untested (TASK-123c):** `access_audits_live_path` only tests write-only postpass modules via `NoopPostpassRunner`, which returns empty `runtime_reads`. The test docstring promises to verify "Read-performing postpass modules...produce non-empty `runtime_reads`," but no test variant exercises this. AC-1 requires: "Given a postpass module that reads `LayerCollectionIR`...then the audit's `runtime_reads` is non-empty." This is never exercised by the current test.

2. **CRIT-2 — Simulation instead of live dispatch (TASK-124):** `dag_validation_tdd`'s `collect_dispatch_audit` (lines 282-316) is a simulation helper that hardcodes expected read paths based on dispatch knowledge. The spec's Step 7 postcondition requires "a test-only dispatch helper that exercises real WIT view calls." The helper encodes knowledge rather than executing dispatch, so the test can pass even if the real dispatch wiring is broken.

## Architecture Constraints

- **`WasmRuntimeDispatcher` is constructible in tests.** The dispatcher requires an `Arc<WasmEngine>`. For the dag validation test helper, the helper must construct or borrow a dispatcher instance to call real dispatch methods and extract reads.
- **Test-only dispatch helper must not break `NoopPostpassRunner` users.** The `PostpassStageRunner` trait's `take_runtime_reads` default returns `Vec::new()`. Custom runners used in tests must implement `take_runtime_reads` to return the simulated reads.
- **Read-performing and write-only postpass variants must coexist in one test function.** `access_audits_live_path` currently runs one plan with `NoopPostpassRunner`. The fix requires either two separate test functions or one function with two runner variants using conditional compilation or enum-dispatch.
- **The dag validation test still needs to produce a `ModuleAccessAudit` with undeclared-read and undeclared-write paths.** Even with live dispatch, the test must arrange for a module that calls undeclared WIT view methods. The helper must be capable of producing such an audit.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `PostpassStageRunner::take_runtime_reads` returns `Vec<Vec<String>>` — one inner vec per postpass module call, in call order.
- `WasmRuntimeDispatcher.postpass_runtime_reads: RefCell<Vec<Vec<String>>>` — accumulates reads per dispatch call, consumed by `take_runtime_reads`.

## Locked Assumptions and Invariants

- The packet does not change the WIT world definitions or any view method signatures.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- `NoopPostpassRunner` must continue to compile without modification (used by other tests).
- Claim Transition Matrix tests remain green.

## Risks and Tradeoffs

- **Test complexity:** Adding a custom runner struct to `access_audits_live_path` increases test code size but is the cleanest way to exercise the read-performing path. The inline struct avoids polluting the module namespace.
- **dag validation test performance:** Constructing a `WasmRuntimeDispatcher` in the dag validation test may add setup overhead. If the dispatcher construction is expensive, the helper should be lazy or reused across assertions.
- **WIT view call coverage:** The dag validation helper that calls `WasmRuntimeDispatcher` dispatch may not actually invoke WIT view methods if the runner double short-circuits the call. The helper must ensure reads are actually collected by the dispatcher's `runtime_reads` mechanism.
