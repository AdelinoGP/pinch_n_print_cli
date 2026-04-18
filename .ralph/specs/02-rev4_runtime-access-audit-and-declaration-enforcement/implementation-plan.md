# Implementation Plan: 02-rev4_runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add failing `access_audits_live_path_read_performing` test

- Task IDs:
  - `TASK-123c`
- Objective: Add a new test function that runs a postpass module with a read-performing runner and asserts `runtime_reads` contains `"LayerCollectionIR"`. The test must FAIL with the current implementation because `NoopPostpassRunner` is used, proving the gap exists.
- Precondition: `access_audits_live_path` only tests write-only postpass modules. No read-performing postpass test exists.
- Postcondition: New test `access_audits_live_path_read_performing` fails with assertion that `runtime_reads` is empty for a read-performing module.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture`
- Exit condition: The test fails because the runner (`NoopPostpassRunner`) returns empty `runtime_reads`, proving the read-performing variant is needed.

### Step 2: Add `PostpassModuleReadingPostpassRunner` and make test pass

- Task IDs:
  - `TASK-123c`
- Objective: Implement `PostpassModuleReadingPostpassRunner` (inline struct in `pipeline_tdd.rs` test module) that simulates a postpass module reading `LayerCollectionIR` via WIT views. The runner must implement `PostpassStageRunner`, return `PostpassOutput::GCodeSuccess` from `run_gcode_postprocess`, and return `vec![vec!["LayerCollectionIR".to_string()]]` from `take_runtime_reads`.
- Precondition: `access_audits_live_path_read_performing` is failing because no read-performing runner exists.
- Postcondition: `access_audits_live_path_read_performing` passes, asserting `runtime_reads` contains `"LayerCollectionIR"` for the read-performing postpass module.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture`
- Exit condition: The test passes with the read-performing runner, proving AC-1's positive assertion is exercised.

### Step 3: Verify `access_audits_live_path` (write-only variant) still passes

- Task IDs:
  - `TASK-123c`
- Objective: Confirm the existing write-only postpass test (`access_audits_live_path`) still passes with its `NoopPostpassRunner` variant, asserting write-only modules have `runtime_reads.is_empty()`.
- Precondition: Steps 1 and 2 complete. Both write-only and read-performing variants have separate test functions.
- Postcondition: `access_audits_live_path` passes, asserting write-only modules have empty `runtime_reads` and non-empty `runtime_writes` containing `"GCodeIR"`.
- Files expected to change: None
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- Exit condition: The write-only test still passes, proving no regression.

### Step 4: Replace `collect_dispatch_audit` simulation in `dag_validation_tdd`

- Task IDs:
  - `TASK-124`
- Objective: Replace the simulated `collect_dispatch_audit` helper (which hardcodes IR paths) with a live-dispatch helper that uses `WasmRuntimeDispatcher` to produce actual `runtime_reads` data. The helper must call dispatch methods and extract reads via `take_runtime_reads`, not return hardcoded vectors.
- Precondition: `collect_dispatch_audit` at `dag_validation_tdd.rs:282-316` is a simulation that hardcodes expected reads. The dag validation test still passes but the postcondition ("exercises real WIT view calls") is unmet.
- Postcondition: `collect_dispatch_audit` (or its replacement) calls `WasmRuntimeDispatcher` dispatch internally and produces live `runtime_reads` data. The test still correctly detects undeclared-read and undeclared-write violations.
- Files expected to change:
  - `crates/slicer-host/tests/dag_validation_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- Exit condition: The test passes using live-dispatch-produced audit data, not hardcoded simulation. Inspecting the helper shows it calls `WasmRuntimeDispatcher` or equivalent dispatch methods, not hardcoded IR paths.

### Step 5: Packet acceptance ceremony and regression sweep

- Task IDs:
  - `TASK-123c`, `TASK-124`
- Objective: Re-run all packet verification commands and confirm no regression in claim-matrix tests.
- Precondition: Steps 1–4 are complete.
- Postcondition: All pipe-suffixed acceptance criteria pass, and `claim_transition_matrix_tdd` remains green.
- Files expected to change: None
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo build --package slicer-host`
  - `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path_read_performing --nocapture`
  - `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
  - `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- Exit condition: Every verification command is green. `02-rev3_runtime-access-audit-and-declaration-enforcement/packet.spec.md` is marked `status: superseded`. This packet's `packet.spec.md` is ready to move to `status: implemented`.

## Packet Completion Gate

- `access_audits_live_path` (write-only) asserts `runtime_reads.is_empty()` and `runtime_writes` contains `"GCodeIR"` for write-only modules.
- `access_audits_live_path_read_performing` (new) asserts `runtime_reads` contains `"LayerCollectionIR"` for read-performing modules.
- `collect_dispatch_audit` in `dag_validation_tdd` uses `WasmRuntimeDispatcher` dispatch, not hardcoded simulation.
- `dag_validation_tdd`'s `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` still correctly detects undeclared-read and undeclared-write violations.
- `cargo build --package slicer-host` and all targeted tests are green.
- `claim_transition_matrix_tdd` still green (no regression).
- `02-rev3_runtime-access-audit-and-declaration-enforcement/packet.spec.md` marked `status: superseded`.
- This packet's `packet.spec.md` is ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
