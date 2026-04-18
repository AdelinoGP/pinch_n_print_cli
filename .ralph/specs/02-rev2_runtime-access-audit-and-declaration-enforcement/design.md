# Design: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/src/dispatch.rs` — Change dispatch calls to return `(Output, HostExecutionContext)` so callers can extract `runtime_reads`
  - `crates/slicer-host/src/prepass.rs` — Extract `runtime_reads` from dispatch result and populate `ModuleAccessAudit`
  - `crates/slicer-host/src/layer_executor.rs` — Extract `runtime_reads` from dispatch result and populate `ModuleAccessAudit`
  - `crates/slicer-host/src/postpass.rs` — Extract `runtime_reads` from dispatch result and populate `ModuleAccessAudit`
- Neighboring tests:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — `access_audits_live_path`
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — `validates_undeclared_runtime_access_and_cross_stage_dependency_rules`
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — manifest-level contract

## Architecture Constraints

- **`WasmRuntimeDispatcher`** owns the `HostExecutionContext` lifetime. It must return `runtime_reads` to callers without leaking the context.
- **Runner trait signatures** (`PrepassStageRunner`, `LayerStageRunner`) cannot change without breaking all test stubs. Instead, `runtime_reads` must be returned through the dispatcher implementation directly.
- **Backward compatibility**: `NoopPrepassRunner`, `NoopLayerRunner`, etc. in `main.rs` must not break. These are no-op stubs that return `None` or empty output — they are unaffected.
- **All WIT view methods are already instrumented** (02-rev1) — they push IR paths to `ctx.runtime_reads`. No changes needed to view method implementations.
- **`validate_undeclared_access`** in `validation.rs` already correctly uses `runtime_reads`. No changes needed.

## Proposed Changes

### Dispatch Return Value Refactor

Currently `dispatch_prepass_call` returns `Result<HostExecutionContext, DispatchError>` but `impl PrepassStageRunner for WasmRuntimeDispatcher` discards the context after extracting typed output.

**Approach A (preferred)**: Change `dispatch_prepass_call` and `dispatch_layer_call` to return `(Output, HostExecutionContext)`. Update all call sites to unpack the tuple.

**Approach B**: Return `HostExecutionContext` from the dispatch call and extract typed output from it at the call site, rather than returning typed output separately.

The cleanest approach is to return `(HostExecutionContext, Output)` from dispatch calls and unpack at the `execute_*` layer:

```rust
// In dispatch.rs — change return type
pub fn dispatch_prepass_call(...) -> Result<(HostExecutionContext, PrepassStageOutput), DispatchError> {
    let ctx = /* existing call logic */;
    let output = /* extract typed output from ctx */;
    Ok((ctx, output))
}
```

### Step-by-Step Change Map

**Step 1 — Prepdispatch returns context + output**
- `dispatch.rs`: Change `dispatch_prepass_call` to return `Result<(HostExecutionContext, PrepassStageOutput), DispatchError>`
- `dispatch.rs`: Change `dispatch_layer_call` similarly
- `dispatch.rs`: Change `dispatch_postpass_call` similarly
- Update all internal call sites within dispatch.rs

**Step 2 — Wire prepass read audits**
- `prepass.rs`: In `impl PrepassStageRunner for WasmRuntimeDispatcher::run_stage`, unpack `(ctx, output)` from dispatch call
- Extract `ctx.runtime_reads` and pass it alongside `runtime_writes` to `ModuleAccessAudit`
- Update `execute_prepass` to accept and forward `runtime_reads`

**Step 3 — Wire per-layer read audits**
- `layer_executor.rs`: Similar refactor for per-layer dispatch
- Extract `ctx.runtime_reads` from `dispatch_layer_call` result
- Forward to `ModuleAccessAudit`

**Step 4 — Wire postpass read audits**
- `postpass.rs`: Extract `ctx.runtime_reads` from `dispatch_postpass_call` result
- Forward to `ModuleAccessAudit`

**Step 5 — Update access_audits_live_path test**
- `pipeline_tdd.rs`: Add assertions that `runtime_reads` is non-empty for modules that perform reads
- Also assert that modules performing only writes have empty `runtime_reads`

**Step 6 — Add live-path undeclared-read integration test**
- `dag_validation_tdd.rs`: Add or extend a test that runs actual prepass/layer/postpass execution and verifies `UndeclaredAccess` fires

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `HostExecutionContext.runtime_reads` is populated by WIT view methods in `wit_host.rs` (unchanged from 02-rev1).
- The `execute_*` functions are the only places that construct `ModuleAccessAudit` — all changes are local to those functions.

## Risks and Tradeoffs

- **Breaking the noop runners**: `NoopPrepassRunner`, `NoopLayerRunner`, etc. in `main.rs` implement the runner traits. Since we are not changing trait signatures (only the `WasmRuntimeDispatcher` impl), these remain unaffected.
- **Breaking existing callers of `dispatch_prepass_call`**: If there are callers outside dispatch.rs, they need to be updated to unpack the tuple return. Check with `grep -n "dispatch_prepass_call" crates/slicer-host/src/`.
- **Thread safety**: `HostExecutionContext` is not `Sync` — it must not outlive the dispatch call. The tuple return makes the ownership clear.

## Open Questions

- **Q1**: Does any code besides `execute_prepass` / `execute_single_layer` / `execute_postpass` call the dispatch functions? Need to verify before Step 1. If so, those callers need tuple unpacking too.
- **Q2**: Should postpass modules actually read IR? Per the design doc, postpass modules typically emit GCode rather than read IR. If true, `runtime_reads` for postpass may always be empty. Verify with `grep -n "dispatch_postpass_call"`.
- **Q3**: Does the dispatch_tdd test (with linker error) affect our ability to verify this packet? Should be a pre-existing issue unrelated to this work.
