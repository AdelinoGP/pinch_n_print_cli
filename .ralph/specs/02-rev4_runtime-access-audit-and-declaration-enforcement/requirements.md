# Requirements: 02-rev4_runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs (reopened from 02-rev3):
  - `TASK-123c` — postpass read-performing test gap not exercised by a real test
  - `TASK-124` — dag validation audit simulation not replaced with live dispatch
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `02-rev3_runtime-access-audit-and-declaration-enforcement` (status: implemented — audit found 2 CRIT gaps)

## Problem Statement

The 02-rev3 audit identified two CRIT findings that must be addressed:

1. **CRIT-1 — AC-1's positive assertion untested (TASK-123c):** `access_audits_live_path` only tests write-only postpass modules via `NoopPostpassRunner`, which returns empty `runtime_reads`. The test docstring promises to verify "Read-performing postpass modules...produce non-empty `runtime_reads`," but no test variant exercises this. AC-1 requires: "Given a postpass module that reads `LayerCollectionIR`...then the audit's `runtime_reads` is non-empty." This is never exercised by the current test.

2. **CRIT-2 — Simulation instead of live dispatch (TASK-124):** `dag_validation_tdd`'s `collect_dispatch_audit` (lines 282-316) is a simulation helper that hardcodes expected read paths based on dispatch knowledge. The spec's Step 7 postcondition requires "a test-only dispatch helper that exercises real WIT view calls." The helper encodes knowledge rather than executing dispatch, so the test can pass even if the real dispatch wiring is broken.

## In Scope

- Add `PostpassModuleReadingPostpassRunner` custom runner in `pipeline_tdd.rs` that simulates a postpass module reading `LayerCollectionIR` via WIT views (returns `runtime_reads` containing `"LayerCollectionIR"`)
- Update `access_audits_live_path` to run both write-only and read-performing postpass module variants and assert correct `runtime_reads` content for each
- Replace `collect_dispatch_audit` simulation in `dag_validation_tdd` with a dispatch helper that internally uses `WasmRuntimeDispatcher` dispatch to produce live `runtime_reads` data
- Verify both test variants pass and claim matrix tests remain green

## Out of Scope

- WIT view instrumentation (02-rev1, unchanged)
- Claim Transition Matrix (already green)
- `dispatch_tdd` linker error (pre-existing)
- Changes to prepass or layer dispatch signatures (already correct from 02-rev2)
- Changes to `execute_postpass`, `dispatch_postpass_gcode_call`, or `dispatch_postpass_text_call` signatures (already correct from 02-rev3)

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- `docs/04_host_scheduler.md` — DagValidationRequest, WriteConflict

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- Positive cases:
  - `access_audits_live_path` runs a read-performing postpass module variant and asserts `runtime_reads` contains `"LayerCollectionIR"`.
  - `access_audits_live_path` runs a write-only postpass module variant and asserts `runtime_reads.is_empty()`.
  - `dag_validation_tdd`'s dispatch helper calls `WasmRuntimeDispatcher` methods to produce live audit data, not hardcoded simulation.
  - Claim matrix tests remain green (no regression).
- Negative cases:
  - Read-performing variant with `NoopPostpassRunner` causes assertion failure (proving the read-performing runner is necessary).
  - Simulation helper instead of live dispatch causes packet rejection per Step 7 postcondition.
- Measurable outcomes:
  - `access_audits_live_path` has two runner variants: write-only (empty reads) and read-performing (non-empty `"LayerCollectionIR"` reads).
  - `dag_validation_tdd` uses `WasmRuntimeDispatcher` dispatch in its helper, not hardcoded IR paths.
  - All targeted tests pass.
- Cross-packet impact:
  - Supersedes 02-rev3 (marked `status: superseded` upon this packet's activation)
  - Closes TASK-123c (read-performing postpass test) and TASK-124 (live-path dag validation) definitively

## Verification Commands

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Step Completion Expectations

- Precondition: the step names the exact runner, helper surface, or assertion it is changing.
- Postcondition: the step leaves behind either a failing targeted test proving the gap exists, or a passing implementation proving the gap is closed.
- Falsifying check: `access_audits_live_path` passes with only `NoopPostpassRunner` (no read-performing variant), or `collect_dispatch_audit` still hardcodes reads without calling dispatch.
