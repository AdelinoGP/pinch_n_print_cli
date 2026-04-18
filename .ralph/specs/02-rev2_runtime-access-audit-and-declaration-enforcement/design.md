# Design: 02-rev2_runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_prepass_call`, `dispatch_layer_call`, `dispatch_postpass_gcode_call`, `dispatch_postpass_text_call`, and the `PrepassStageRunner`, `LayerStageRunner`, and `PostpassStageRunner` impls on `WasmRuntimeDispatcher`
  - `crates/slicer-host/src/prepass.rs` — `execute_prepass` audit construction
  - `crates/slicer-host/src/layer_executor.rs` — `execute_single_layer` audit construction
  - `crates/slicer-host/src/postpass.rs` — `execute_postpass` audit construction
- Neighboring tests:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — `access_audits_live_path`
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — `validates_undeclared_runtime_access_and_cross_stage_dependency_rules`
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — manifest-level contract

## Architecture Constraints

- **`WasmRuntimeDispatcher`** owns the `HostExecutionContext` lifetime. The design must preserve `runtime_reads` before the context is consumed by harvest helpers.
- **Prepass and layer dispatch already return `HostExecutionContext`.** The packet must not invent a return-type change there unless the code actually needs it.
- **Postpass currently erases read data at the runner boundary.** Any design must surface postpass read paths without losing the existing `PostpassOutput` behavior.
- **Backward compatibility**: `NoopPrepassRunner`, `NoopLayerRunner`, and `NoopPostpassRunner` in `main.rs` must continue to compile. If a trait changes, the packet must update those stubs explicitly.
- **All WIT view methods are already instrumented** (02-rev1) — they push IR paths to `ctx.runtime_reads`. No changes needed to view method implementations.
- **`validate_undeclared_access`** in `validation.rs` already correctly uses `runtime_reads`. No changes needed.

## Code Change Surface

### Selected Approach

Keep the existing prepass and layer dispatch signatures, because they already return `HostExecutionContext`. Refactor the harvest or runner boundary so the typed output can be extracted without dropping `runtime_reads`, then thread those read paths into `ModuleAccessAudit`.

For postpass, change the dispatch or runner boundary so `run_gcode_postprocess` / `run_text_postprocess` can surface both the current `PostpassOutput` and the read paths collected by `wit_host.rs`.

### Exact Code Surface

- `dispatch.rs`
  - Preserve `runtime_reads` before `harvest_layer_plan_ir`, `harvest_mesh_segmentation_ir`, `harvest_paint_segmentation_ir`, `harvest_mesh_analysis_auxiliary`, or `commit_layer_outputs` consume the context.
  - Change `dispatch_postpass_gcode_call` / `dispatch_postpass_text_call` or the `PostpassStageRunner` return path so postpass read paths survive.
- `prepass.rs`, `layer_executor.rs`, `postpass.rs`
  - Replace `runtime_reads: Vec::new()` with the collected runtime read paths for read-performing modules.
- `pipeline_tdd.rs`, `dag_validation_tdd.rs`
  - Add exact assertions for `"MeshIR"`, `"SliceIR.regions.polygons"`, `"LayerCollectionIR"`, and `"SliceIR.regions.undeclared"` as appropriate.

### Rejected Alternatives

- Do not keep the current packet's earlier assumption that prepass and layer dispatch must start returning `(Output, HostExecutionContext)`. The source already returns `HostExecutionContext`; the problem is preserving reads through harvesting, not adding a second return value.
- Do not change `wit_host.rs` instrumentation again. The recorded paths already exist and are the basis for the retrofit.

### Step-by-Step Change Map

1. Add or tighten failing tests so they assert the exact read paths and stop accepting `Vec::new()`.
2. Preserve prepass and per-layer `runtime_reads` while harvesting typed outputs from `HostExecutionContext`.
3. Surface postpass read paths through the postpass dispatch or runner boundary.
4. Replace manual undeclared-read audit injection with a live-path execution assertion.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `HostExecutionContext.runtime_reads` is populated by WIT view methods in `wit_host.rs` (unchanged from 02-rev1).
- `wit_host.rs` already records postpass reads as `"LayerCollectionIR"`; postpass audits must not assume reads are always empty.
- The `execute_*` functions are the only places that construct `ModuleAccessAudit`; audit population changes are local to those functions and the dispatcher helpers that feed them.

## Locked Assumptions and Invariants

- The packet preserves the current WIT host read-path strings rather than renaming them.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- `TASK-126` behavior stays untouched.

## Risks and Tradeoffs

- **Breaking postpass trait consumers**: if the postpass runner surface changes, `NoopPostpassRunner` and any postpass test doubles must be updated in the same step.
- **Partial fixes that only change tests**: the packet must not stop after adding assertions; the runtime audit flow must become live for all three tiers.
- **Thread safety**: `HostExecutionContext` is not `Sync` — extracted read paths must be copied out before the context is consumed.

## Open Questions

- Resolved: `dispatch_prepass_call` and `dispatch_layer_call` are consumed by the `PrepassStageRunner` and `LayerStageRunner` impls in `dispatch.rs`; postpass uses `dispatch_postpass_gcode_call` and `dispatch_postpass_text_call` inside the `PostpassStageRunner` impl. No additional `src/` callers were found in the retrofit survey.
- Resolved: postpass read auditing is real work, not a theoretical edge case. `wit_host.rs` already records `"LayerCollectionIR"` reads for postpass views.
- Resolved: `dispatch_tdd` remains out of scope and non-blocking for this packet because none of the packet verification commands depend on that test file.
