# Implementation Plan: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Inventory the real dispatch and audit surfaces

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`
- Objective: Capture the exact prepass, layer, and postpass call sites that currently lose `runtime_reads`, and record the tests that will become the packet acceptance gates.
- Precondition: Packet docs and authoritative code paths have been loaded.
- Postcondition: The packet records that prepass and layer already return `HostExecutionContext`, while postpass currently hides read paths behind its runner boundary.
- Files expected to change:
  - `.ralph/specs/02-rev2_runtime-access-audit-and-declaration-enforcement/design.md`
  - `.ralph/specs/02-rev2_runtime-access-audit-and-declaration-enforcement/task-map.md`
- Authoritative docs: N/A
- OrcaSlicer refs: None
- Verification: `grep -rn "dispatch_prepass_call\|dispatch_layer_call\|dispatch_postpass_gcode_call\|dispatch_postpass_text_call\|runtime_reads: Vec::new()" crates/slicer-host/src/`
- Exit condition: The inventory names the `dispatch.rs`, `prepass.rs`, `layer_executor.rs`, and `postpass.rs` surfaces explicitly and no additional `src/` callers remain unaccounted for.

### Step 2: Expose failing tests with exact read-path assertions

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`, `TASK-124`
- Objective: Strengthen `pipeline_tdd` and `dag_validation_tdd` so they assert the exact read-path content required by the packet and fail on the current `Vec::new()` behavior.
- Precondition: Step 1 inventory is recorded.
- Postcondition: The targeted tests assert `"MeshIR"`, `"SliceIR.regions.polygons"`, `"LayerCollectionIR"`, and the undeclared-read path `"SliceIR.regions.undeclared"` as appropriate.
- Files expected to change:
  - `crates/slicer-host/tests/pipeline_tdd.rs`
  - `crates/slicer-host/tests/dag_validation_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- Exit condition: The targeted tests fail for the current implementation because a read-performing module still produces `runtime_reads: Vec::new()` or because undeclared-read enforcement still depends on manual audit injection.

### Step 3: Preserve prepass and per-layer runtime_reads during harvesting

- Task IDs: `TASK-123a`, `TASK-123b`
- Objective: Refactor the prepass and layer runner flow so harvesting typed output does not discard `runtime_reads` before audit construction.
- Precondition: Step 2 tests are failing with exact assertions.
- Postcondition: Read-performing prepass and per-layer modules carry their collected read paths into `ModuleAccessAudit`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/prepass.rs`
  - `crates/slicer-host/src/layer_executor.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- Exit condition: The targeted dag-validation test proves that prepass and per-layer audits now carry non-empty `runtime_reads` with the expected exact path strings.

### Step 4: Surface postpass runtime_reads through the runner boundary

- Task IDs: `TASK-123c`
- Objective: Change the postpass dispatch or runner surface so read-performing postpass modules can contribute `LayerCollectionIR` read paths to their audits without losing current `PostpassOutput` behavior.
- Precondition: Prepass and per-layer runtime reads survive harvesting.
- Postcondition: `execute_postpass` can distinguish read-performing postpass modules from write-only modules when building `ModuleAccessAudit`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/postpass.rs`
  - `crates/slicer-host/src/main.rs` (if postpass trait stubs need updating)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Module Access Contract
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test pipeline_tdd -- access_audits_live_path --nocapture`
- Exit condition: `access_audits_live_path` proves that postpass audits include `"LayerCollectionIR"` for read-performing modules and still leave write-only modules with empty `runtime_reads`.

### Step 5: Replace manual undeclared-read audit injection with a live-path check

- Task IDs: `TASK-124`
- Objective: Convert the undeclared-read coverage from a manually injected `ModuleAccessAudit` fixture into a live execution path that proves the runtime audit reaches `validate_undeclared_access`.
- Precondition: All three tiers can now produce live `runtime_reads`.
- Postcondition: The dag-validation test asserts the exact undeclared path and `AccessKind::Read` using live audit data.
- Files expected to change:
  - `crates/slicer-host/tests/dag_validation_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Host-Boundary Access Enforcement table
  - `docs/04_host_scheduler.md` — DagValidationRequest
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test dag_validation_tdd -- validates_undeclared_runtime_access_and_cross_stage_dependency_rules --nocapture`
- Exit condition: The undeclared-read assertion no longer depends on constructing `DagValidationRequest.access_audits` by hand.

### Step 6: Packet acceptance ceremony and regression sweep

- Task IDs: `TASK-123a`, `TASK-123b`, `TASK-123c`, `TASK-124`
- Objective: Re-run the packet commands, confirm claim-matrix behavior is still green, and verify the packet acceptance criteria now match live code behavior.
- Precondition: Steps 1 through 5 are complete.
- Postcondition: All packet verification commands are green and no acceptance criterion relies on vague or manual-only evidence.
- Files expected to change: None
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo build --package slicer-host`
  - `cargo test --package slicer-host --test dag_validation_tdd -- --nocapture`
  - `cargo test --package slicer-host --test pipeline_tdd -- --nocapture`
  - `cargo test --package slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- Exit condition: Every pipe-suffixed packet command is green, and the packet is ready to move from `draft` to `implemented` once activated and executed.

## Packet Completion Gate

- Prepass and per-layer harvesting preserve `runtime_reads` instead of dropping them during output extraction.
- Postpass audits surface `"LayerCollectionIR"` reads for read-performing modules.
- `access_audits_live_path` asserts exact read-vs-write-only audit behavior rather than only module counts.
- Live-path undeclared-read enforcement proves the full chain from WIT call to validation error.
- `cargo build --package slicer-host` and the packet's targeted tests are green.
- `claim_transition_matrix_tdd.rs` still green (not regressed).
- `02-rev1_runtime-access-audit-and-declaration-enforcement/packet.spec.md` is marked `status: superseded`.
- This packet's `packet.spec.md` is ready to move to `status: implemented`.
