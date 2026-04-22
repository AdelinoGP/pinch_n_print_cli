# Design: 24_runtime-write-audit-plumbing

## Controlling Code Paths

1. **`HostExecutionContext::new`** (`wit_host.rs:1127`) — initializer; add `runtime_writes: Vec::new()` here.
2. **`HostPerimeterOutputBuilder` trait impl** (`wit_host.rs:2573`) — three methods to instrument:
   - `push_wall_loop` → record `"PerimeterIR.regions.walls"`
   - `push_reordered_wall_loop` → record `"PerimeterIR.regions.walls"`
   - `push_resolved_seam` → record `"PerimeterIR.resolved-seam"`
3. **`HostInfillOutputBuilder`** and **`HostSupportOutputBuilder`** — decide coarse root writes (`"InfillIR"`, `"SupportIR"`) for symmetry; leave prepass/finalization/postpass out of scope.
4. **`LayerStageRunner::run_stage`** trait (`layer_executor.rs:172`) — update return from `Result<(LayerStageOutput, Vec<String>), LayerStageError>` to `Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError>` (output, reads, writes).
5. **`WasmRuntimeDispatcher::dispatch_layer_call`** (`dispatch.rs`) — extract `ctx.runtime_writes` at dispatch return and include in the tuple.
6. **`execute_single_layer`** (`layer_executor.rs:282`) — change audit construction at line ~363:
   - Before: `runtime_writes: vec![path]` (where `path = ir_path_for_layer_stage(...)`)
   - After: `runtime_writes: collected_writes` (from the runner return tuple)
7. **`ir_path_for_layer_stage`** — keep as fallback only; it should not be the primary source for layer module audits.

## Architecture Constraints

- `HostExecutionContext` is per-call and re-created for each WASM call; `runtime_writes` must not persist across calls.
- The write-path vocabulary must match the manifest/docs naming style from `docs/04_host_scheduler.md §Manifest ↔ Runtime Naming Map`: `PerimeterIR.regions.walls` not `PerimeterIR.wall-loops`.
- Negative case: if `seam-placer` manifest says `PerimeterIR.resolved-seam` but runtime collects `PerimeterIR`, `validate_undeclared_access` must fail (coarse write against narrow declared path is an error).

## Implementation Approach

### Step 1: Add `runtime_writes` field

Add `runtime_writes: Vec<String>` to `HostExecutionContext` and initialize in `new`. This mirrors the existing `runtime_reads` pattern exactly.

### Step 2: Centralized write-path helper

```rust
impl HostExecutionContext {
    /// Record a canonical write path for runtime audit.
    fn record_write(&mut self, path: &'static str) {
        self.runtime_writes.push(String::from(path));
    }
}
```

Using a `&'static str` input forces literals to be canonical constants, not arbitrary strings.

### Step 3: Instrument perimeter builder methods

```rust
fn push_wall_loop(&mut self, ...) -> ... {
    // existing logic unchanged
    self.record_write("PerimeterIR.regions.walls");
    Ok(Ok(()))
}

fn push_reordered_wall_loop(&mut self, ...) -> ... {
    self.record_write("PerimeterIR.regions.walls");
    Ok(Ok(()))
}

fn push_resolved_seam(&mut self, ...) -> ... {
    self.record_write("PerimeterIR.resolved-seam");
    Ok(Ok(()))
}
```

### Step 4: Infill/Support coarse writes

Instrument `HostInfillOutputBuilder` and `HostSupportOutputBuilder` to record `"InfillIR"` and `"SupportIR"` respectively. This is a low-risk symmetry addition that future-proofs the audit.

### Step 5: Update `LayerStageRunner::run_stage`

Change the trait signature and both implementations (production `WasmRuntimeDispatcher` and any test fakes) to carry writes:

```rust
fn run_stage(
    &self,
    stage_id: &StageId,
    layer: &GlobalLayer,
    module: &CompiledModule,
    blackboard: &Blackboard,
    arena: &mut LayerArena,
) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError>;
```

### Step 6: Update `dispatch_layer_call`

In `dispatch_layer_call`, after the WASM call returns, extract `ctx.runtime_writes` alongside `ctx.runtime_reads` and include both in the return tuple.

### Step 7: Update `execute_single_layer`

Change the audit construction to use the writes from the runner return:
```rust
let (stage_result, runtime_reads, runtime_writes) = match run_result { ... };
// ...
LayerAccessAudit {
    module_id: module.module_id.clone(),
    runtime_reads,
    runtime_writes, // now from runner, not from ir_path_for_layer_stage
}
```

Remove the dependency on `ir_path_for_layer_stage` for successful layer-module audits; keep it only as a fallback for stages not yet instrumented.

### Step 8: Regression tests

- `pipeline_tdd.rs`: 3 tests for `push_wall_loop`, `push_reordered_wall_loop`, `push_resolved_seam` recording; 1 test for infill coarse fallback.
- `core_module_ir_access_contract_tdd.rs`: 1 live seam audit regression proving narrow `PerimeterIR.resolved-seam` write validates against manifest.
- Negative test: prove missing instrumentation fails (in `acceptance_gate_gaps_tdd.rs` or a new test in `pipeline_tdd.rs`).

## Data and Contract Notes

- `ModuleAccessAudit.runtime_writes` is `Vec<String>` (same as `runtime_reads`).
- Canonical paths use the manifest naming style (dot-notation, kebab-case field names): `PerimeterIR.regions.walls`, `PerimeterIR.resolved-seam`.
- The `ir_path_for_layer_stage` function is NOT deleted — it remains as a fallback for stages whose builder methods are not yet instrumented.

## Risks and Tradeoffs

- **Risk**: Changing `LayerStageRunner::run_stage` signature is a breaking API change for all implementors. All call sites must be updated.
  - Mitigation: Update all implementations (WasmRuntimeDispatcher + test fakes) in the same atomic change.
- **Risk**: If any perimeter builder method is not instrumented and a module uses it, the audit will be silently incomplete.
  - Mitigation: Add a negative test that fails when the write is missing from the audit.
- **Tradeoff**: Recording coarse `InfillIR`/`SupportIR` instead of subfield paths is a simplification. If a future module writes a subfield of InfillIR, it will need a new instrumented method.
  - Accepted: This is the same coarse pattern used by `ir_path_for_layer_stage` today; the same coarse fallback is preserved for uninstrumented stages.

## Open Questions

- Q1: Should `HostInfillOutputBuilder` also record narrow subfield paths? The plan says "decide" — the answer should be recorded in this packet's design doc before implementation begins.
  - **Resolution**: Use coarse `InfillIR` for now; add subfield instrumentation when a module actually needs it.
- Q2: Should prepass output builders be instrumented in this packet?
  - **Resolution**: No — prepass uses a different dispatch path (`dispatch_prepass_call`) and is out of scope.

## Locked Assumptions

1. `HostExecutionContext` is per-call; `runtime_writes` resets each call.
2. Write-path vocabulary uses manifest naming style: `PerimeterIR.regions.walls`, `PerimeterIR.resolved-seam`.
3. `ir_path_for_layer_stage` remains as fallback only, not deleted.
4. `LayerStageRunner::run_stage` signature change is atomic across all implementations.
