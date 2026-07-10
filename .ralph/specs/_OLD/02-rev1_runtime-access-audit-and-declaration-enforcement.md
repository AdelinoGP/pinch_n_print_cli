---
status: superseded
packet: 02-rev1_runtime-access-audit-and-declaration-enforcement
task_ids:
  - TASK-123a
  - TASK-123b
  - TASK-123c
  - TASK-124
  - TASK-126
supersedes: 02_runtime-access-audit-and-declaration-enforcement
superseded_by: 02-rev2_runtime-access-audit-and-declaration-enforcement
---

# 02-rev1_runtime-access-audit-and-declaration-enforcement

## Goal

Complete the runtime read-audit instrumentation at the WIT boundary (TASK-123abc), wire `runtime_reads` through all three execution tiers so undeclared read enforcement works at runtime (TASK-124), fix `WriteConflict.orderable` semantics so `true` means the conflict is actually resolvable by topological ordering (TASK-126), and add positive `orderable` test coverage.

## Problem Statement

The original 02 packet implemented audit collection for runtime writes but failed to implement runtime read auditing. The audit trail always has `runtime_reads: Vec::new()` because WIT view resource methods are not instrumented. This means:

1. **`validate_undeclared_access`** in pass 11 cannot detect undeclared reads at runtime — `runtime_reads` is always empty.
2. **`claim_transition_matrix_tdd`** is green (TASK-125 complete).
3. **`WriteConflict.orderable`** implementation exists but only the negative case is tested. The semantics of "orderable" need clarification: `orderable=true` should mean the conflict can actually be resolved by topological ordering (a read creates a DAG dependency edge), not merely that a module declared a read.

## Architecture Constraints

- **`HostExecutionContext`** must carry read audit state per dispatch call so it can be returned alongside write audits.
- **WIT view resource methods** must record the exact IR path accessed when called. This requires modifying the generated bindings or wrapping them.
- **Read audits must use exact paths** (e.g., `SliceIR.regions.polygons`) for enforcement; top-level roots may be used for coarse reporting.
- **Undeclared access enforcement** must be fatal (no graceful degradation for contract violations).
- **`WriteConflict.orderable`** must check DAG reachability (`can_reach`) not just `ir_reads` containment.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `WriteConflict.orderable: bool` — true only when a DAG edge (reachability path) exists via the conflicting field.
- WIT view resource methods are called by guest modules — they are the read boundary.
- The host does not currently track reads per-call; this is the primary gap.

## Risks and Tradeoffs

- **Performance**: Recording every WIT view method call adds overhead. Mitigation: only record the top-level IR root (e.g., `SliceIR`) for coarse reporting; exact paths only for modules under enforcement.
- **Generated WIT bindings**: The `wasmtime::component::bindgen!` macro generates types but host implementations must be provided via callbacks. We must ensure the read-audit callbacks survive the generated-code boundaries.
- **Path canonicalization**: Exact paths must match manifest declaration format exactly (e.g., `SliceIR` vs `SliceIR.regions.polygons`). Use path prefix matching.
