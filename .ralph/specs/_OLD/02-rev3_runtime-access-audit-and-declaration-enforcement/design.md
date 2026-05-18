# Design: 02-rev3_runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_postpass_gcode_call` and `dispatch_postpass_text_call` must be refactored to return `runtime_reads` alongside the call result
  - `crates/slicer-host/src/postpass.rs` — `execute_postpass` must use the returned reads to populate `ModuleAccessAudit.runtime_reads`
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — `access_audits_live_path`, new `prepass_audits_live_path`, new `layer_audits_live_path`
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — `validates_undeclared_runtime_access_and_cross_stage_dependency_rules`
- OrcaSlicer comparison surface: None

## Architecture Constraints

- **`WasmRuntimeDispatcher` owns `HostExecutionContext` lifetime.** Refactor dispatch methods to extract `runtime_reads` (via clone) before the context is dropped, and return those reads alongside the result.
- **Postpass runner trait (`PostpassStageRunner`) must remain backward-compatible.** The `run_gcode_postprocess` and `run_text_postprocess` signatures cannot change because `NoopPostpassRunner` in `main.rs` and test doubles depend on them. The dispatch refactor must not change the trait.
- **Prepass and layer dispatch signatures are unchanged (already correct from 02-rev2).** Do not touch `dispatch.rs` lines 1085 or 1214.
- **All WIT view methods remain unchanged (02-rev1).** `wit_host.rs` lines 2291-2309 already record `"LayerCollectionIR"` correctly.
- **`validate_undeclared_access` in `validation.rs` is unchanged.** It correctly uses `runtime_reads`.

## Code Change Surface

### Selected Approach

**For postpass dispatch:** Change `dispatch_postpass_gcode_call` to return `(Result<(), DispatchError>, Vec<String>)` and `dispatch_postpass_text_call` to return `(Result<String, DispatchError>, Vec<String>)`. Extract `runtime_reads` via `ctx.runtime_reads.clone()` before the store/context is dropped. Update `WasmRuntimeDispatcher`'s `PostpassStageRunner` impl to thread reads from dispatch into `execute_postpass`.

**For `execute_postpass`:** Change signature to accept an out-parameter or additional return value that carries the collected reads from all postpass modules. Populate `ModuleAccessAudit.runtime_reads` from these reads instead of `Vec::new()`.

**For tests:** Add explicit `runtime_reads` assertions in `access_audits_live_path`. Add `prepass_audits_live_path` and `layer_audits_live_path` that run live prepass/layer modules and assert on collected audit content.

### Exact Code Surface

- `dispatch.rs`:
  - `dispatch_postpass_gcode_call` — change return from `Result<(), DispatchError>` to `(Result<(), DispatchError>, Vec<String>)`
  - `dispatch_postpass_text_call` — change return from `Result<String, DispatchError>` to `(Result<String, DispatchError>, Vec<String>)`
  - Both methods: clone `runtime_reads` from the store context before dropping
- `postpass.rs`:
  - `execute_postpass` — update signature and audit population to use returned reads
- `pipeline.rs`:
  - `PipelineStageRunners` — no change needed (trait impl changes internally)
- `pipeline_tdd.rs`:
  - `access_audits_live_path` — add `runtime_reads` content assertions
  - Add `prepass_audits_live_path` test
  - Add `layer_audits_live_path` test
- `dag_validation_tdd.rs`:
  - `validates_undeclared_runtime_access_and_cross_stage_dependency_rules` — replace manual `earlier_live_audit` construction with live-path execution (see implementation-plan.md Step 4 for exact approach)

### Rejected Alternatives

- **Do not change the `PostpassStageRunner` trait signatures** — doing so would break `NoopPostpassRunner` and all test doubles. The dispatch-level refactor is preferred.
- **Do not add a new trait method for returning reads** — threading through the existing dispatch is cleaner than introducing parallel interfaces.
- **Do not change prepass or layer dispatch** — they already work correctly.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `HostExecutionContext.runtime_reads` is populated by WIT view methods in `wit_host.rs` (unchanged).
- `wit_host.rs` already records `"LayerCollectionIR"` reads for postpass views at lines 2291-2309.
- The dispatch functions create a fresh `HostExecutionContext` per call (`dispatch.rs:626`, `dispatch.rs:688`); reads collected in that context must be extracted before the function returns.

## Locked Assumptions and Invariants

- The packet does not change the WIT world definitions or any view method signatures.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- TASK-126 behavior stays untouched.
- `NoopPostpassRunner` must continue to compile without modification.

## Risks and Tradeoffs

- **Breaking postpass trait consumers:** If the trait changes, `NoopPostpassRunner` and postpass test doubles must be updated in the same step. The selected approach avoids trait changes.
- **Return type explosion:** Changing two dispatch methods to return tuples may cascade to callers. The `PostpassStageRunner` impl on `WasmRuntimeDispatcher` is the only caller of those methods in the relevant execution path; impact is contained.
- **Thread safety:** `HostExecutionContext` is not `Sync` — extracted read paths must be copied out (via clone) before the context is consumed.

## Open Questions

- Resolved: The postpass dispatch functions discard context after the call. The fix requires returning reads alongside the result. No alternative approach is viable without changing the postpass execution architecture.
- Resolved: `NoopPostpassRunner` must remain compile-stable. The dispatch-level refactor (not trait change) preserves this.
- Resolved: The `dag_validation_tdd` test validates dag validation logic, not live dispatch. The question is whether it should also provide live-path evidence for AC-1/AC-2, or whether `pipeline_tdd` tests cover that separately. This packet assigns live-path prepass/layer coverage to new `pipeline_tdd` tests, leaving `dag_validation_tdd` focused on validation logic but with manual audit injection replaced per Step 5.