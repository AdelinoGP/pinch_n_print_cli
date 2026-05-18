# Implementation Plan: 02-rev3_runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add failing `runtime_reads` assertions to `access_audits_live_path`

- Task IDs: `TASK-123c`
- Objective: Add exact `runtime_reads` field content assertions to `access_audits_live_path` so the test fails before the postpass fix and passes after.
- Precondition: `access_audits_live_path` only checks audit count and module IDs, not `runtime_reads` content.
- Postcondition: The test now asserts that read-performing postpass modules have `runtime_reads` containing `"LayerCollectionIR"` and that write-only modules have `runtime_reads` with `len() == 0`.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/02_ir_schemas.md` — LayerCollectionIR field path
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- Exit condition: The test fails with an assertion that `runtime_reads` is non-empty for a postpass module that reads `LayerCollectionIR`, proving the postfix is needed.

### Step 2: Refactor `dispatch_postpass_gcode_call` to return `runtime_reads`

- Task IDs: `TASK-123c`
- Objective: Change `dispatch_postpass_gcode_call` return type from `Result<(), DispatchError>` to `(Result<(), DispatchError>, Vec<String>)`. Clone `runtime_reads` from the store context before it is dropped.
- Precondition: Step 1 failing test confirms postpass wiring is missing. `dispatch_postpass_gcode_call` currently returns only `Result<()>`.
- Postcondition: `dispatch_postpass_gcode_call` returns `(Result<()>, Vec<String>)` where the second element is the collected `runtime_reads`. The context is preserved and reads are extracted before dropping.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DispatchError, DispatchPhase
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host`
- Exit condition: The dispatch method compiles with the new return type and returns reads for read-performing modules.

### Step 3: Refactor `dispatch_postpass_text_call` to return `runtime_reads`

- Task IDs: `TASK-123c`
- Objective: Change `dispatch_postpass_text_call` return type from `Result<String, DispatchError>` to `(Result<String, DispatchError>, Vec<String>)`. Clone `runtime_reads` from the store context before it is dropped.
- Precondition: Step 2 complete. `dispatch_postpass_text_call` currently returns only `Result<String>`.
- Postcondition: `dispatch_postpass_text_call` returns `(Result<String>, Vec<String>)` where the second element is the collected `runtime_reads`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host`
- Exit condition: The dispatch method compiles with the new return type and returns reads for read-performing modules.

### Step 4: Update `WasmRuntimeDispatcher`'s `PostpassStageRunner` impl to thread reads

- Task IDs: `TASK-123c`
- Objective: Update `run_gcode_postprocess` and `run_text_postprocess` in `WasmRuntimeDispatcher` to use the new dispatch return types and pass reads into `execute_postpass`.
- Precondition: Steps 2 and 3 complete. Dispatch methods now return reads.
- Postcondition: The runner impl threads reads from dispatch into `execute_postpass` audit construction.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/postpass.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host`
- Exit condition: `execute_postpass` receives and uses `runtime_reads` from postpass dispatch.

### Step 5: Add `prepass_audits_live_path` test

- Task IDs: `TASK-123a`
- Objective: Add a live-path test that runs a read-performing prepass module through `run_pipeline` and asserts `"MeshIR"` appears in `prepass_audits[].runtime_reads`.
- Precondition: Prepass dispatch already preserves `runtime_reads` (02-rev2). The gap is lack of a test asserting the live path.
- Postcondition: New test `prepass_audits_live_path` asserts `"MeshIR"` in `prepass_audits` for a read-performing prepass module.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- prepass_audits_live_path --nocapture`
- Exit condition: The test passes and asserts `"MeshIR"` in collected `prepass_audits`.

### Step 6: Add `layer_audits_live_path` test

- Task IDs: `TASK-123b`
- Objective: Add a live-path test that runs a read-performing per-layer module through `run_pipeline` and asserts `"SliceIR.regions.polygons"` appears in `layer_audits[].runtime_reads`.
- Precondition: Per-layer dispatch already preserves `runtime_reads` (02-rev2). The gap is lack of a test asserting the live path.
- Postcondition: New test `layer_audits_live_path` asserts `"SliceIR.regions.polygons"` in `layer_audits` for a read-performing per-layer module.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- layer_audits_live_path --nocapture`
- Exit condition: The test passes and asserts `"SliceIR.regions.polygons"` in collected `layer_audits`.

### Step 7: Replace manual audit construction in `dag_validation_tdd`

- Task IDs: `TASK-124`
- Objective: Replace the manually constructed `earlier_live_audit` with live-path execution evidence in `validates_undeclared_runtime_access_and_cross_stage_dependency_rules`.
- Precondition: Step 5 and Step 6 provide live-path evidence for prepass and layer audits separately. The dag-validation test still uses manual audit construction.
- Postcondition: `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` either (a) runs a live dispatch path and uses the resulting audit, or (b) uses a test-only dispatch helper that exercises real WIT view calls and returns an audit with live `runtime_reads`. The test no longer constructs `DagValidationRequest.access_audits` by hand for the audit under test.
- Files expected to change:
  - `crates/slicer-host/tests/dag_validation_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- Exit condition: The test uses live audit data and no longer calls `ModuleAccessAudit { .. }` directly for the module under test.

### Step 8: Packet acceptance ceremony and regression sweep

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`, `TASK-124`
- Objective: Re-run all packet verification commands and confirm no regression in claim-matrix tests.
- Precondition: Steps 1–7 are complete.
- Postcondition: All pipe-suffixed acceptance criteria pass, and `claim_transition_matrix_tdd` remains green.
- Files expected to change: None
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo build --package slicer-host`
  - `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- prepass_audits_live_path --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- layer_audits_live_path --nocapture`
  - `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
  - `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- Exit condition: Every verification command is green. `02-rev2_runtime-access-audit-and-declaration-enforcement/packet.spec.md` is marked `status: superseded`. This packet's `packet.spec.md` is ready to move to `status: implemented`.

## Packet Completion Gate

- Postpass dispatch returns `runtime_reads` alongside call results.
- `execute_postpass` populates `ModuleAccessAudit.runtime_reads` from dispatch-returned reads.
- `access_audits_live_path` asserts `runtime_reads` content for read-performing and write-only modules.
- `prepass_audits_live_path` asserts `"MeshIR"` in collected `prepass_audits`.
- `layer_audits_live_path` asserts `"SliceIR.regions.polygons"` in collected `layer_audits`.
- `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` uses live-path execution, not manual audit construction.
- `cargo build --package slicer-host` and all targeted tests are green.
- `claim_transition_matrix_tdd` still green (no regression).
- `02-rev2_runtime-access-audit-and-declaration-enforcement/packet.spec.md` marked `status: superseded`.
- This packet's `packet.spec.md` is ready to move to `status: implemented`.