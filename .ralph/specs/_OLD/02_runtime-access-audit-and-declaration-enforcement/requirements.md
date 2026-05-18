# Requirements: runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs:
  - `TASK-123` / `TASK-123a` / `TASK-123b` / `TASK-123c` — Feed `ModuleAccessAudit` from every live execution path into `DagValidationRequest.access_audits`. Covers DEV-003.
  - `TASK-124` — Enforce undeclared runtime read/write faults at the WIT boundary. Continues DEV-003 after TASK-123 lands.
  - `TASK-125` — Enforce Claim Transition Matrix for non-transitionable claims. Covers DEV-004. Must turn `claim_transition_matrix_tdd.rs` green.
  - `TASK-126` — Fix `WriteConflict.orderable` so it reports `true` only when ordering can actually resolve the pair. Scheduler conflict-ordering cleanup for docs/04 contract.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-003: Runtime IR-access enforcement is dormant because access audits are not fed from live execution paths into validation. The host cannot distinguish between a module that legitimately reads a path vs. one that violates its declaration.

DEV-004: Claim Transition Matrix is not enforced for non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`). A module could hold one of these claims on layer N and a different holder on layer N+1, breaking wall continuity and seam scoring assumptions.

TASK-126: `WriteConflict.orderable` currently returns `true` for any conflict pair, but ordering can only resolve a write-write conflict when the writes touch non-overlapping IR paths. This blocks correct scheduler behavior.

## In Scope

- Plumb `ModuleAccessAudit` records from prepass, per-layer, and postpass execution into `DagValidationRequest.access_audits`.
- Add host-side enforcement that rejects undeclared reads/writes at the WIT boundary with precise diagnostics.
- Add negative test harness for layer-time undeclared access.
- Enforce the Allowed Claim Transition Matrix from `docs/01_system_architecture.md` for non-transitionable claims at startup validation.
- Fix `WriteConflict.orderable` semantics; add both positive and negative test coverage.
- Make `claim_transition_matrix_tdd.rs` and `core_module_ir_access_contract_tdd.rs` green.

## Out of Scope

- Manifest population (TASK-121/TASK-122 — separate packet).
- WIT consolidation (TASK-144/145/146 — separate packet).
- Custom type widening (TASK-149/150 — separate packet).
- Python postpass live-path decision (TASK-137 series — separate workstream).

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285), Claim System (rows 485–539), Allowed Claim Transition Matrix (rows 540–556)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49), Valid Reads/Writes section
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- `ModuleAccessAudit` records are collected from prepass, per-layer, and postpass live execution paths and flow into `DagValidationRequest.access_audits`.
- Undeclared read/write attempts at the WIT boundary produce fatal contract errors with required diagnostic fields (module id, stage id, operation, requested path).
- Non-transitionable claim transitions are rejected at startup with precise diagnostics.
- `WriteConflict.orderable` returns `true` only when ordering can resolve the conflict.
- `claim_transition_matrix_tdd.rs` passes completely.

## Verification Commands

- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo test --package slicer-host -- --nocapture` (full host test suite)
- Negative harness test: undeclared access triggers fatal error