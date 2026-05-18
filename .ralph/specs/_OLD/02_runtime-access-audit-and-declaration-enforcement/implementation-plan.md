# Implementation Plan: runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Define or locate ModuleAccessAudit and DagValidationRequest.access_audits

- Task IDs:
  - `TASK-123`
  - `TASK-123a`
- Objective: Confirm whether `ModuleAccessAudit` struct exists and whether `DagValidationRequest` has an `access_audits` field. Define/add them if missing.
- Files expected to change:
  - `crates/slicer-ir/src/` (if new types needed)
  - `crates/slicer-host/src/scheduler/` (plumbing)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `grep -r "ModuleAccessAudit" crates/` and `grep -r "access_audits" crates/`

### Step 2: Implement audit collection at WIT boundary (prepass)

- Task IDs:
  - `TASK-123a`
- Objective: Record `ModuleAccessAudit` entries for every module call during prepass execution. Aggregate and pass to `DagValidationRequest.access_audits`.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (likely `execute_prepass.rs` or similar)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement
- OrcaSlicer refs: None
- Verification: Unit test that runs prepass and asserts non-empty `access_audits`.

### Step 3: Implement audit collection at WIT boundary (per-layer)

- Task IDs:
  - `TASK-123b`
- Objective: Record `ModuleAccessAudit` entries for every module call during per-layer execution. Aggregate and pass to `DagValidationRequest.access_audits`.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (likely `execute_per_layer.rs` or similar)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
- OrcaSlicer refs: None
- Verification: Unit test that runs a single-layer slice and asserts non-empty `access_audits`.

### Step 4: Implement audit collection at WIT boundary (postpass)

- Task IDs:
  - `TASK-123c`
- Objective: Record `ModuleAccessAudit` entries for every module call during postpass execution. Aggregate and pass to `DagValidationRequest.access_audits`. Add live-path regression proving populated audits reach validation.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (likely `execute_postpass.rs` or similar)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- test access_audits_live_path` (or similar).

### Step 5: Add undeclared read enforcement at WIT boundary

- Task IDs:
  - `TASK-124`
- Objective: Implement the Host-Boundary Access Enforcement table from `docs/03_wit_and_manifest.md` — deny undeclared reads and return fatal contract error with required diagnostic fields.
- Files expected to change:
  - `crates/slicer-host/src/wit/` (boundary enforcement layer)
  - `crates/slicer-host/src/scheduler/`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- OrcaSlicer refs: None
- Verification: Negative test that attempts undeclared read and gets fatal error with correct diagnostics.

### Step 6: Add undeclared write enforcement at WIT boundary

- Task IDs:
  - `TASK-124`
- Objective: Reject undeclared writes at commit time with fatal contract error. Required diagnostic fields: module id, stage id, operation (write), requested path, manifest path set used for comparison.
- Files expected to change:
  - `crates/slicer-host/src/wit/` (boundary enforcement layer)
  - `crates/slicer-host/src/scheduler/`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
- OrcaSlicer refs: None
- Verification: Negative test that attempts undeclared write and gets fatal error with correct diagnostics.

### Step 7: Enforce Claim Transition Matrix for non-transitionable claims

- Task IDs:
  - `TASK-125`
- Objective: Implement startup validation that rejects configurations where a non-transitionable claim (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`) has different holders across layers. Emit precise diagnostics naming the claim, object, and conflicting layer ranges.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (startup validation)
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` (should go green)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Allowed Claim Transition Matrix (rows 540–556)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture` — all tests pass.

### Step 8: Fix WriteConflict.orderable semantics

- Task IDs:
  - `TASK-126`
- Objective: Fix `WriteConflict.orderable` to return `true` only when ordering can actually resolve the write-write conflict (non-overlapping write paths). Add both positive and negative test cases.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/` (WriteConflict implementation)
  - `crates/slicer-host/tests/` (new test cases)
- Authoritative docs:
  - `docs/04_host_scheduler.md` — WriteConflict, orderable semantics
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- test write_conflict_orderable` (or similar).

### Step 9: Full test suite verification

- Task IDs:
  - `TASK-123`
  - `TASK-124`
  - `TASK-125`
  - `TASK-126`
- Objective: Run the full slicer-host test suite and confirm all access-audit, claim-matrix, and orderable tests are green.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host -- --nocapture` — all tests pass.

## Packet Completion Gate

- All audit plumbing steps complete and verified by live-path regression tests.
- Undeclared read/write enforcement active at WIT boundary with correct diagnostics.
- `claim_transition_matrix_tdd.rs` green.
- `WriteConflict.orderable` fix verified by positive and negative test cases.
- `docs/07_implementation_status.md` TASK-123/123a/123b/123c/124/125/126 marked complete.
- `packet.spec.md` ready to move to `status: implemented`.