# Implementation Plan: 24_runtime-write-audit-plumbing

## Step 1 — Add `runtime_writes` field to `HostExecutionContext`

**Task IDs**: TASK-123b
**Objective**: Add `runtime_writes: Vec<String>` field to `HostExecutionContext` and initialize in `new`.
**Precondition**: None.
**Postcondition**: `HostExecutionContext::new` initializes `runtime_writes: Vec::new()`; field is `pub` (same visibility as `runtime_reads`).
**Files**: `crates/slicer-host/src/wit_host.rs`
**Verification**: `cargo build -p slicer-host 2>&1 | grep -i error | head -5 || echo "NO_ERRORS"`
**Exit**: Build passes.
**OrcaSlicer refs**: None.

## Step 2 — Add `record_write` helper and instrument perimeter builder methods

**Task IDs**: TASK-123b, TASK-124
**Objective**: Add `record_write(&mut self, path: &'static str)` method to `HostExecutionContext`. Instrument `push_wall_loop`, `push_reordered_wall_loop`, and `push_resolved_seam` to call it with canonical paths.
**Precondition**: Step 1 complete.
**Postcondition**:
- `HostExecutionContext::record_write("PerimeterIR.regions.walls")` called in `push_wall_loop` and `push_reordered_wall_loop`.
- `HostExecutionContext::record_write("PerimeterIR.resolved-seam")` called in `push_resolved_seam`.
**Files**: `crates/slicer-host/src/wit_host.rs`
**Verification**: `grep -n 'record_write\|PerimeterIR.regions.walls\|PerimeterIR.resolved-seam' crates/slicer-host/src/wit_host.rs | head -20`
**Exit**: All three methods record correct paths; build passes.
**OrcaSlicer refs**: None.

## Step 3 — Instrument Infill/Support coarse writes

**Task IDs**: TASK-123b
**Objective**: Instrument `HostInfillOutputBuilder` and `HostSupportOutputBuilder` to record coarse root writes (`"InfillIR"`, `"SupportIR"`) for symmetry.
**Precondition**: Step 2 complete.
**Postcondition**: `HostInfillOutputBuilder` methods record `"InfillIR"`; `HostSupportOutputBuilder` methods record `"SupportIR"`.
**Files**: `crates/slicer-host/src/wit_host.rs`
**Verification**: `grep -n 'record_write.*InfillIR\|record_write.*SupportIR' crates/slicer-host/src/wit_host.rs | head -10`
**Exit**: Both builders record coarse paths; build passes.
**OrcaSlicer refs**: None.

## Step 4 — Update `LayerStageRunner::run_stage` trait signature

**Task IDs**: TASK-123b
**Objective**: Update `LayerStageRunner::run_stage` to return `(LayerStageOutput, Vec<String>, Vec<String>)` (output, reads, writes) instead of `(LayerStageOutput, Vec<String>)`.
**Precondition**: Steps 1–3 complete (field exists, instrumented).
**Postcondition**: Trait signature updated; all implementations (production `WasmRuntimeDispatcher`, test fakes in dispatch_tdd.rs, live_seam_path_tdd.rs, etc.) return the 3-tuple.
**Files**: `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/src/dispatch.rs`, all test files with `LayerStageRunner` impls
**Verification**: `cargo build -p slicer-host 2>&1 | grep 'run_stage\|LayerStageRunner' | head -10`
**Exit**: Build passes; no impls return old 2-tuple.
**OrcaSlicer refs**: None.

## Step 5 — Update `WasmRuntimeDispatcher::dispatch_layer_call` to extract and return `runtime_writes`

**Task IDs**: TASK-123b
**Objective**: In the `dispatch_layer_call` return, include `ctx.runtime_writes.clone()` alongside `ctx.runtime_reads.clone()` in the 3-tuple.
**Precondition**: Step 4 complete (trait updated).
**Postcondition**: `dispatch_layer_call` returns `(LayerStageOutput, runtime_reads, runtime_writes)`.
**Files**: `crates/slicer-host/src/dispatch.rs`
**Verification**: `grep -n 'runtime_writes' crates/slicer-host/src/dispatch.rs | head -20`
**Exit**: `runtime_writes` appears in dispatch return path; build passes.
**OrcaSlicer refs**: None.

## Step 6 — Update `execute_single_layer` audit construction

**Task IDs**: TASK-123b, TASK-124
**Objective**: In `execute_single_layer`, destructure the 3-tuple from `runner.run_stage` and use `runtime_writes` in `ModuleAccessAudit` construction instead of `vec![path]` from `ir_path_for_layer_stage`. Keep `ir_path_for_layer_stage` as fallback for non-instrumented stages.
**Precondition**: Step 5 complete.
**Postcondition**: `ModuleAccessAudit.runtime_writes` in layer execution comes from the collected writes, not from `ir_path_for_layer_stage`.
**Files**: `crates/slicer-host/src/layer_executor.rs`
**Verification**: `grep -n 'runtime_writes.*ir_path_for_layer_stage\|ir_path_for_layer_stage.*runtime_writes' crates/slicer-host/src/layer_executor.rs`
**Exit**: Build passes; `ir_path_for_layer_stage` still exists but is not the primary source for layer module audits.
**OrcaSlicer refs**: None.

## Step 7 — Add pipeline regression tests for the new `runtime_writes` behavior

**Task IDs**: TASK-123b
**Objective**: Add tests to `pipeline_tdd.rs` asserting that `push_wall_loop`, `push_reordered_wall_loop`, and `push_resolved_seam` record the correct canonical paths.
**Precondition**: Steps 1–6 complete.
**Postcondition**: 3 new tests in `pipeline_tdd.rs` pass:
- `push_wall_loop_records_runtime_write`
- `push_reordered_wall_loop_records_runtime_write`
- `push_resolved_seam_records_runtime_write`
Plus 1 fallback test:
- `infill_coarse_fallback_audit`
**Files**: `crates/slicer-host/tests/pipeline_tdd.rs`
**Verification**: `cargo test -p slicer-host --test pipeline_tdd push_wall_loop_records_runtime_write push_reordered_wall_loop_records_runtime_write push_resolved_seam_records_runtime_write infill_coarse_fallback_audit -- --nocapture 2>&1 | tail -20`
**Exit**: All new tests pass.
**OrcaSlicer refs**: None.

## Step 8 — Add live seam narrow-write audit regression

**Task IDs**: TASK-124
**Objective**: Add a test to `core_module_ir_access_contract_tdd.rs` proving that `seam-placer`'s narrow manifest write `PerimeterIR.resolved-seam` validates correctly when live access audits are consumed.
**Precondition**: Steps 1–6 complete.
**Postcondition**: Test `seam_placer_narrow_manifest_write_validates` passes.
**Files**: `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
**Verification**: `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_placer_narrow_manifest_write_validates -- --nocapture 2>&1 | tail -20`
**Exit**: Test passes.
**OrcaSlicer refs**: None.

## Step 9 — Add negative regression for missing instrumentation

**Task IDs**: TASK-124
**Objective**: Add a test proving that a module that calls `push_wall_loop` but has the runtime_writes instrumentation missing fails the audit assertion (guards against future regressions where instrumentation is accidentally removed).
**Precondition**: Steps 1–8 complete.
**Postcondition**: Test `missing_runtime_writes_fails` passes (or the positive counterpart `push_wall_loop_records_runtime_write` serves as the regression guard — if the skill of this test is covered by Step 7's positive test, it may be omitted with comment justification).
**Files**: `crates/slicer-host/tests/pipeline_tdd.rs` or `crates/slicer-host/tests/acceptance_gate_gaps_tdd.rs`
**Verification**: `cargo test -p slicer-host --test pipeline_tdd missing_runtime_writes_fails -- --nocapture 2>&1 | tail -20`
**Exit**: Test passes or is justifiedly omitted.
**OrcaSlicer refs**: None.

## Step 10 — Packet completion gate

**Objective**: Run the focused test matrix for Packet 24 and confirm workspace build/clippy.
**Precondition**: Steps 1–9 complete.
**Postcondition**: `cargo test -p slicer-host --test pipeline_tdd -- --nocapture` passes; `cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture` passes; `cargo build --workspace` exits 0; `cargo clippy --workspace -- -D warnings` exits 0 with no warnings.
**Files**: All changed files.
**Verification**:
```
cargo test -p slicer-host --test pipeline_tdd -- --nocapture 2>&1 | tail -5
cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture 2>&1 | tail -5
cargo build --workspace 2>&1 | tail -3
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
```
**Exit**: All four commands succeed.
**OrcaSlicer refs**: None.
