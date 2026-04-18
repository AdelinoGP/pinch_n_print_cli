# Requirements: 02-rev1_runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs (reopened from 02):
  - `TASK-123a` — Prepass read audit plumbing was incomplete (only writes collected)
  - `TASK-123b` — Per-layer read audit plumbing was incomplete (only writes collected)
  - `TASK-123c` — Postpass read audit plumbing was incomplete (only writes collected)
  - `TASK-124` — Undeclared read enforcement cannot work without runtime read auditing
  - `TASK-126` — `WriteConflict.orderable` positive test case missing; semantics unclear
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `02_runtime-access-audit-and-declaration-enforcement` (status: superseded)

## Problem Statement

The original 02 packet implemented audit collection for runtime writes but failed to implement runtime read auditing. The audit trail always has `runtime_reads: Vec::new()` because WIT view resource methods are not instrumented. This means:

1. **`validate_undeclared_access`** in pass 11 cannot detect undeclared reads at runtime — `runtime_reads` is always empty.
2. **`claim_transition_matrix_tdd`** is green (TASK-125 complete).
3. **`WriteConflict.orderable`** implementation exists but only the negative case is tested. The semantics of "orderable" need clarification: `orderable=true` should mean the conflict can actually be resolved by topological ordering (a read creates a DAG dependency edge), not merely that a module declared a read.

## In Scope

- Add `runtime_reads` tracking to `HostExecutionContext` and instrument all WIT view resource methods to record exact IR paths accessed per call.
- Wire `runtime_reads` through prepass, per-layer, and postpass audit collection.
- Fix `WriteConflict.orderable` semantics: `orderable=true` only when the conflict is actually resolvable by topological ordering (the read creates a DAG edge between the conflicting modules).
- Add positive test case for `orderable=true`.
- Add per-criterion pipe-suffix verification commands to all acceptance criteria in `packet.spec.md`.

## Out of Scope

- `dispatch_tdd` linker error (pre-existing, separate issue).
- Claim Transition Matrix enforcement (already complete in 02 — not reopened).
- Manifest population (TASK-121/122 — separate packet).

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- `runtime_reads` is populated with exact IR paths for every module call across all three execution tiers.
- Undeclared read attempts at runtime produce fatal contract errors with required diagnostic fields.
- `WriteConflict.orderable` returns `true` only when topological ordering can actually resolve the pair.
- Positive and negative `orderable` test cases both pass.
- All per-criterion verification commands pass.
