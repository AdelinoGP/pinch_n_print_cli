# Requirements: 02-rev3_runtime-access-audit-and-declaration-enforcement

## Packet Metadata

- Grouped task IDs (reopened from 02-rev2):
  - `TASK-123c` — Postpass `runtime_reads` still not wired to `ModuleAccessAudit`
  - `TASK-123a` — Prepass live-path assertions need strengthening
  - `TASK-123b` — Per-layer live-path assertions need strengthening
  - `TASK-124` — Manual audit injection not yet replaced with live-path evidence
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `02-rev2_runtime-access-audit-and-declaration-enforcement` (status: implemented — incomplete)

## Problem Statement

The 02-rev2 packet correctly preserved `runtime_reads` through the prepass and per-layer harvest boundaries (`dispatch.rs:1085`, `dispatch.rs:1214`), but three gaps remain:

1. **Postpass wiring missing (CRIT-1):** `dispatch_postpass_gcode_call` and `dispatch_postpass_text_call` return `Result<()>` / `Result<String>` respectively and discard the `HostExecutionContext` after the call. `execute_postpass` hardcodes `runtime_reads: Vec::new()` at lines 178 and 218. Even though `wit_host.rs` correctly records `"LayerCollectionIR"` reads (lines 2291-2309), those reads never reach `ModuleAccessAudit`.

2. **`access_audits_live_path` assertions missing (CRIT-2):** The test only checks audit count (3) and module ID presence. It never asserts that any audit's `runtime_reads` field is non-empty or contains `"LayerCollectionIR"`. This means AC-3 and AC-4 from 02-rev2's spec are unverified.

3. **Manual audit construction not replaced (CRIT-3):** `dag_validation_tdd`'s `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` still manually constructs `earlier_live_audit` at lines 288-298 instead of using live execution. Step 5's postcondition ("no longer depends on constructing `DagValidationRequest.access_audits` by hand") is unmet.

## In Scope

- Refactor postpass dispatch to return `runtime_reads` alongside call results
- Update `execute_postpass` to use returned reads for audit population
- Add `runtime_reads` content assertions to `access_audits_live_path`
- Add `prepass_audits_live_path` test asserting `"MeshIR"` in collected prepass audits
- Add `layer_audits_live_path` test asserting `"SliceIR.regions.polygons"` in collected layer audits
- Replace manual audit construction in `dag_validation_tdd` with live-path execution evidence

## Out of Scope

- WIT view instrumentation (02-rev1, unchanged)
- `WriteConflict.orderable` fix (02-rev1, unchanged)
- Claim Transition Matrix (already green)
- `dispatch_tdd` linker error (pre-existing)
- Changes to prepass or layer dispatch signatures (already correct from 02-rev2)

## Authoritative Docs

- `docs/01_system_architecture.md` — Module Access Contract (rows 276–285)
- `docs/02_ir_schemas.md` — IR field path names
- `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table (rows 26–49)
- `docs/04_host_scheduler.md` — DagValidationRequest, WriteConflict, orderable semantics

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- Positive cases:
  - Postpass audits for read-performing modules include `"LayerCollectionIR"` in `runtime_reads`.
  - Postpass audits for write-only modules have empty `runtime_reads`.
  - `access_audits_live_path` asserts exact `runtime_reads` content for both read-performing and write-only modules.
  - `prepass_audits_live_path` asserts `"MeshIR"` in collected `prepass_audits`.
  - `layer_audits_live_path` asserts `"SliceIR.regions.polygons"` in collected `layer_audits`.
  - `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` uses live-path audit data, not hand-constructed `ModuleAccessAudit`.
- Negative cases:
  - Postpass module that reads `LayerCollectionIR` but produces `runtime_reads: Vec::new()` causes test failure.
  - Manual audit construction causes test failure (Step 5 postcondition enforced).
- Measurable outcomes:
  - No postpass module may leave `runtime_reads: Vec::new()` when it performs reads.
  - All three live-path audit tests assert exact path strings.
  - Manual audit construction replaced.
- Cross-packet impact:
  - Supersedes 02-rev2 (which is marked `status: superseded` upon this packet's activation)
  - Closes TASK-123a, TASK-123b, TASK-123c, TASK-124 definitively

## Verification Commands

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- prepass_audits_live_path --nocapture`
- `cargo test --package slicer-host --test pipeline_tdd -- layer_audits_live_path --nocapture`
- `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`

## Step Completion Expectations

- Precondition: the step names the exact guest, audit path, or helper surface it is changing.
- Postcondition: the step leaves behind either a failing targeted test with exact path assertions or a passing implementation plus evidence.
- Falsifying check: a targeted command still shows `runtime_reads: Vec::new()` for a read-performing postpass module, or a test still checks only audit count without asserting `runtime_reads` field content.