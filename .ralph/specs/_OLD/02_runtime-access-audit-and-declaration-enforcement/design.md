# Design: runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/scheduler/` — where `ModuleAccessAudit` is collected and `DagValidationRequest` is constructed
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs`
  - `crates/slicer-host/tests/` (negative harness)
- OrcaSlicer comparison surface: None

## Architecture Constraints

- `ModuleAccessAudit` must be collected at the WIT boundary — the point where a module call enters or exits the host runtime.
- The audit trail must be structured so it can be passed into `DagValidationRequest.access_audits` for post-execution validation.
- Undeclared access enforcement must be fatal (no graceful degradation for contract violations).
- Claim Transition Matrix enforcement must occur at startup (before any slice begins), not at runtime per layer.
- `WriteConflict.orderable` must consider write-path overlap: if two modules write the same IR field, ordering cannot resolve the conflict.

## Proposed Changes

### TASK-123a/123b/123c — Access Audit Plumbing

1. **Identify audit collection points**: At each WIT boundary crossing (module entry and exit), record:
   - Module ID
   - Stage ID
   - Operation (read/write)
   - IR path accessed
   - Timestamp

2. **Plumb audits into DagValidationRequest**: After each execution tier (prepass/per-layer/postpass), aggregate collected audits and pass them into `DagValidationRequest.access_audits` for the validation step.

3. **Add live-path regression test**: A test that runs a full slice and asserts that `access_audits` is non-empty and covers the expected modules/stages.

### TASK-124 — Undeclared Access Enforcement

4. **Implement enforcement at WIT boundary**: Before allowing a module to read a path, verify the path is in the module's declared `[ir-access].reads`. If not, return fatal contract error.
5. **Implement write enforcement at commit**: Before allowing a module to commit a write, verify the path is in the module's declared `[ir-access].writes`. If not, reject commit with fatal error.
6. **Add negative test harness**: A test module that attempts undeclared access and is rejected with the required diagnostic fields.

### TASK-125 — Claim Transition Matrix Enforcement

7. **Implement startup validation for non-transitionable claims**: At startup, for each `(object_id, claim)` where the claim is non-transitionable (perimeter-generator, seam-placer, layer-planner, mesh-analyzer), verify that the holder is consistent across all layers.
8. **Add precise diagnostics**: When a violation is found, name the claim, object, conflicting layer ranges, and the two conflicting module IDs.

### TASK-126 — WriteConflict.orderable Fix

9. **Fix orderable semantics**: `WriteConflict.orderable` should return `true` only when:
   - The two modules write to different IR paths (no overlap), OR
   - The write order can be topological-sorted to eliminate the conflict
10. **Add positive and negative test coverage**: A test that creates `WriteConflict` pairs with known orderable/non-orderable outcomes and asserts `orderable` returns the correct boolean.

## Data and Contract Notes

- `ModuleAccessAudit` struct fields: `module_id`, `stage_id`, `operation: AccessOperation`, `path: String`, `timestamp_us: u64`.
- `DagValidationRequest.access_audits: Vec<ModuleAccessAudit>`.
- Claim Transition Matrix non-transitionable claims: `perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`.
- `WriteConflict.orderable` must check write-path overlap, not just existence of conflict.

## Risks and Tradeoffs

- Collecting access audits on every WIT boundary crossing may add overhead. Ensure audits are cheap to record (simple struct copy, not async).
- Enforcing at WIT boundary means the host must have the module's declared access paths available at call time. This requires loading manifests before execution.
- Claim matrix enforcement at startup requires building a per-object, per-claim layer-holder map before execution begins.

## Open Questions

- Does `ModuleAccessAudit` already exist in the codebase, or does it need to be defined? Check `crates/slicer-ir/` or `crates/slicer-host/src/`. Already Exists
- Does `DagValidationRequest` already have an `access_audits` field? Check `docs/04_host_scheduler.md`.
- Is there an existing negative test harness infrastructure, or does one need to be built for TASK-124?
- Does the existing `claim_transition_matrix_tdd.rs` test already cover the full matrix, or does it need additional cases?