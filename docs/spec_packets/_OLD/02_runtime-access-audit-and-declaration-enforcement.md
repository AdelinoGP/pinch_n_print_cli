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
---

# 02_runtime-access-audit-and-declaration-enforcement

## Goal

Feed `ModuleAccessAudit` from every live execution path into `DagValidationRequest.access_audits`, enforce undeclared runtime read/write faults at the WIT boundary (DEV-003), enforce the Claim Transition Matrix for non-transitionable claims (DEV-004), and fix `WriteConflict.orderable` to report `true` only when ordering can actually resolve the pair (scheduler cleanup for docs/04 contract).

## Problem Statement

DEV-003: Runtime IR-access enforcement is dormant because access audits are not fed from live execution paths into validation. The host cannot distinguish between a module that legitimately reads a path vs. one that violates its declaration.

DEV-004: Claim Transition Matrix is not enforced for non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`). A module could hold one of these claims on layer N and a different holder on layer N+1, breaking wall continuity and seam scoring assumptions.

TASK-126: `WriteConflict.orderable` currently returns `true` for any conflict pair, but ordering can only resolve a write-write conflict when the writes touch non-overlapping IR paths. This blocks correct scheduler behavior.

## Architecture Constraints

- `ModuleAccessAudit` must be collected at the WIT boundary — the point where a module call enters or exits the host runtime.
- The audit trail must be structured so it can be passed into `DagValidationRequest.access_audits` for post-execution validation.
- Undeclared access enforcement must be fatal (no graceful degradation for contract violations).
- Claim Transition Matrix enforcement must occur at startup (before any slice begins), not at runtime per layer.
- `WriteConflict.orderable` must consider write-path overlap: if two modules write the same IR field, ordering cannot resolve the conflict.

## Data and Contract Notes

- `ModuleAccessAudit` struct fields: `module_id`, `stage_id`, `operation: AccessOperation`, `path: String`, `timestamp_us: u64`.
- `DagValidationRequest.access_audits: Vec<ModuleAccessAudit>`.
- Claim Transition Matrix non-transitionable claims: `perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`.
- `WriteConflict.orderable` must check write-path overlap, not just existence of conflict.

## Risks and Tradeoffs

- Collecting access audits on every WIT boundary crossing may add overhead. Ensure audits are cheap to record (simple struct copy, not async).
- Enforcing at WIT boundary means the host must have the module's declared access paths available at call time. This requires loading manifests before execution.
- Claim matrix enforcement at startup requires building a per-object, per-claim layer-holder map before execution begins.
