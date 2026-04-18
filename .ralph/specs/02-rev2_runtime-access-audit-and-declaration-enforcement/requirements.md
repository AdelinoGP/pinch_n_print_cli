# Requirements: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs (reopened from 02-rev1):
  - `TASK-123a` — Prepass `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-123b` — Per-layer `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-123c` — Postpass `runtime_reads` extraction not wired (always `Vec::new()`)
  - `TASK-124` — Live-path undeclared-read enforcement not validated end-to-end
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `02-rev1_runtime-access-audit-and-declaration-enforcement` (status: implemented — incomplete)

## Problem Statement

The 02-rev1 packet added `runtime_reads` to `HostExecutionContext` and instrumented all WIT view methods to populate it. However, the `HostExecutionContext` is consumed inside `WasmRuntimeDispatcher` dispatch calls and `runtime_reads` is never extracted and passed to `ModuleAccessAudit`. All three execution tiers (`execute_prepass`, `execute_single_layer`, `execute_postpass`) create `ModuleAccessAudit` entries with `runtime_reads: Vec::new()`.

The architectural root cause splits by tier:

1. `dispatch_prepass_call` and `dispatch_layer_call` already return `HostExecutionContext`, but the tier-specific harvest and commit flow consumes that context without preserving `runtime_reads` before `ModuleAccessAudit` is created.
2. Postpass dispatch currently hides runtime reads behind `dispatch_postpass_gcode_call` / `dispatch_postpass_text_call`, so the postpass audit path never sees read data even though `wit_host.rs` records `LayerCollectionIR` reads.

Result:

1. Positive audit assertions fail because all three tiers still write `runtime_reads: Vec::new()` on the host side.
2. Undeclared-read enforcement only has confidence through a manually injected `ModuleAccessAudit`; the live execution path is untested.
3. `access_audits_live_path` in `pipeline_tdd` passes but doesn't assert `runtime_reads` is non-empty — it only checks audit count and module IDs.

## In Scope

- Preserve `runtime_reads` through prepass harvest and audit construction
- Preserve `runtime_reads` through per-layer commit and audit construction
- Surface `runtime_reads` through the postpass dispatch or runner boundary into `ModuleAccessAudit`
- Add live-path test asserting non-empty `runtime_reads` for modules that perform reads
- Add live-path integration test for undeclared-read enforcement

## Out of Scope

- WIT view method instrumentation (already done in 02-rev1)
- `WriteConflict.orderable` fix (already done in 02-rev1)
- Claim Transition Matrix enforcement (already green)
- `dispatch_tdd` linker error (pre-existing)

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- `docs/04_host_scheduler.md` — WriteConflict, orderable semantics, DagValidationRequest

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- Positive cases:
  - Prepass audits for read-performing guests include `"MeshIR"` in `runtime_reads`.
  - Per-layer audits for read-performing guests include `"SliceIR.regions.polygons"` in `runtime_reads`.
  - Postpass audits for read-performing guests include `"LayerCollectionIR"` in `runtime_reads` and keep the expected `"GCodeIR"` write audit.
  - `access_audits_live_path` distinguishes read-performing modules from write-only modules instead of accepting `Vec::new()` everywhere.
- Negative cases:
  - Live undeclared reads produce `SchedulerError::UndeclaredAccess { access: Read, path: "SliceIR.regions.undeclared", .. }` rather than relying on a manually injected audit fixture.
- Measurable outcomes:
  - No touched tier may leave a read-performing module with `runtime_reads: Vec::new()`.
  - Packet verification includes both targeted tests and `cargo build --package slicer-host`.
- Cross-packet impact:
  - This packet supersedes the `TASK-123abc` and `TASK-124` closure previously claimed by 02-rev1.

## Verification Commands

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Step Completion Expectations

- Precondition: the step names the exact guest, audit path, or helper surface it is changing.
- Postcondition: the step leaves behind either a failing targeted test with exact path assertions or a passing implementation plus evidence.
- Falsifying check: a targeted command still shows `runtime_reads: Vec::new()` for a read-performing module or still relies on manual audit injection for the undeclared-read case.
