# Implementation Plan: 27_phase-h-final-validation

## Step 1 — Rebuild WASM artifacts

**Task IDs**: TASK-120
**Objective**: Run `modules/core-modules/build-core-modules.sh` to rebuild all checked-in WASM artifacts after binding/manifest changes from Packets 24/25.
**Precondition**: Packets 24 and 25 are complete.
**Postcondition**: All WASM artifacts are rebuilt; `build-core-modules.sh` exits 0.
**Files**: `modules/core-modules/`
**Verification**: `./modules/core-modules/build-core-modules.sh 2>&1 | tail -10; echo "EXIT: $?"`
**Exit**: Exit 0.
**OrcaSlicer refs**: None.

## Step 2 — Run `core_module_ir_access_contract_tdd`

**Task IDs**: TASK-120, TASK-124
**Objective**: Run the IR access contract test suite.
**Precondition**: Step 1 complete.
**Postcondition**: All tests in `core_module_ir_access_contract_tdd.rs` pass.
**Files**: `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
**Verification**: `cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 3 — Run `pipeline_tdd`

**Task IDs**: TASK-120, TASK-123b
**Objective**: Run the pipeline test suite including the new `runtime_writes` regression tests.
**Precondition**: Step 2 complete.
**Postcondition**: All tests in `pipeline_tdd.rs` pass.
**Files**: `crates/slicer-host/tests/pipeline_tdd.rs`
**Verification**: `cargo test -p slicer-host --test pipeline_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 4 — Run `wit_drift_detection_tdd`

**Task IDs**: TASK-120, TASK-145
**Objective**: Run the WIT drift detection test suite including the expanded signature assertions.
**Precondition**: Step 3 complete.
**Postcondition**: All tests in `wit_drift_detection_tdd.rs` pass.
**Files**: `crates/slicer-host/tests/wit_drift_detection_tdd.rs`
**Verification**: `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 5 — Run `live_support_generation_tdd`

**Task IDs**: TASK-120, TASK-120b
**Objective**: Run the live support generation test suite including the new real live-dispatch tier.
**Precondition**: Step 4 complete.
**Postcondition**: All tests in `live_support_generation_tdd.rs` pass, both tiers.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 6 — Run `live_seam_path_tdd`

**Task IDs**: TASK-120, TASK-120c (excluded from this plan, but test must still pass)
**Objective**: Run the live seam path test suite (unchanged by this packet; must remain green).
**Precondition**: Step 5 complete.
**Postcondition**: All tests in `live_seam_path_tdd.rs` pass.
**Files**: `crates/slicer-host/tests/live_seam_path_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 7 — Run `benchy_end_to_end_tdd`

**Task IDs**: TASK-120, TASK-120b
**Objective**: Run the Benchy acceptance test suite including the new support-enabled acceptance tests.
**Precondition**: Step 6 complete.
**Postcondition**: All tests in `benchy_end_to_end_tdd.rs` pass.
**Files**: `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
**Verification**: `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All tests pass.
**OrcaSlicer refs**: None.

## Step 8 — Workspace build

**Task IDs**: TASK-120
**Objective**: Run `cargo build --workspace` and confirm it exits 0.
**Precondition**: Steps 1–7 complete.
**Postcondition**: `cargo build --workspace` exits 0.
**Files**: All changed files.
**Verification**: `cargo build --workspace 2>&1 | tail -5`
**Exit**: Exit 0.
**OrcaSlicer refs**: None.

## Step 9 — Workspace clippy

**Task IDs**: TASK-120
**Objective**: Run `cargo clippy --workspace -- -D warnings` and confirm it exits 0 with no warnings.
**Precondition**: Step 8 complete.
**Postcondition**: `cargo clippy --workspace -- -D warnings` exits 0 with no warnings.
**Files**: All changed files.
**Verification**: `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
**Exit**: Exit 0, no warnings.
**OrcaSlicer refs**: None.

## Step 10 — Update `docs/07_implementation_status.md`

**Task IDs**: TASK-120, TASK-120b
**Objective**: Update TASK-120b entry to cite the real live support-module evidence from Packet 26. Add TASK-120b1 (or equivalent) to track the true Benchy-with-tree-support acceptance harness. Update Workstream 3 to explicitly track the acceptance check.
**Precondition**: Steps 1–9 complete.
**Postcondition**: TASK-120b entry cites new real evidence; Workstream 3 tracks the acceptance check.
**Files**: `docs/07_implementation_status.md`
**Verification**: `grep -B2 -A8 'TASK-120' docs/07_implementation_status.md | head -40`
**Exit**: Doc updated.
**OrcaSlicer refs**: None.

## Step 11 — Packet completion gate

**Objective**: Confirm all four packets are validated and the Phase H acceptance gate is ready.
**Precondition**: Steps 1–10 complete.
**Postcondition**: All acceptance criteria across all four packets are satisfied; Phase H closure is unblocked.
**Files**: All changed files.
**Verification**: All 9 commands from Steps 2–9 succeeded.
**Exit**: All gates green.
**OrcaSlicer refs**: None.
