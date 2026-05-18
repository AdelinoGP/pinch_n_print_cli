# Design: 02-rev4_runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — `access_audits_live_path` must be updated to include a read-performing postpass variant
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — `collect_dispatch_audit` must be replaced with a live-dispatch helper
- Neighboring tests or fixtures:
  - `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher` dispatch methods are the target for the live-dispatch helper
  - `crates/slicer-host/src/postpass.rs` — `take_runtime_reads()` pattern is already correct from 02-rev3
- OrcaSlicer comparison surface: None

## Architecture Constraints

- **`WasmRuntimeDispatcher` is constructible in tests.** The dispatcher requires an `Arc<WasmEngine>`. For the dag validation test helper, the helper must construct or borrow a dispatcher instance to call real dispatch methods and extract reads.
- **Test-only dispatch helper must not break `NoopPostpassRunner` users.** The `PostpassStageRunner` trait's `take_runtime_reads` default returns `Vec::new()`. Custom runners used in tests must implement `take_runtime_reads` to return the simulated reads.
- **Read-performing and write-only postpass variants must coexist in one test function.** `access_audits_live_path` currently runs one plan with `NoopPostpassRunner`. The fix requires either two separate test functions or one function with two runner variants using conditional compilation or enum-dispatch.
- **The dag validation test still needs to produce a `ModuleAccessAudit` with undeclared-read and undeclared-write paths.** Even with live dispatch, the test must arrange for a module that calls undeclared WIT view methods. The helper must be capable of producing such an audit.

## Code Change Surface

### Selected Approach

**For `access_audits_live_path` read-performing variant:**
Add an inline `PostpassModuleReadingPostpassRunner` struct to `access_audits_live_path` (or module scope) that implements `PostpassStageRunner` and returns reads containing `"LayerCollectionIR"` via `take_runtime_reads()`. The test function should be split or parameterized to test both variants, or a second test `access_audits_live_path_read_performing` should be added.

**For `dag_validation_tdd` live-dispatch helper:**
Replace `collect_dispatch_audit` with a helper that constructs a `WasmRuntimeDispatcher`, calls the appropriate dispatch method (e.g., `dispatch_layer_call` for the layer stage), and extracts `runtime_reads` from the returned reads. The helper must work within the constraints of the dag validation test environment (no actual WASM guest needed if the runner double returns reads directly — the key is that the helper uses the dispatcher's read-collection mechanism, not hardcoded knowledge).

**Rejected alternative — separate test function for read-performing postpass:**
Adding a second test `access_audits_live_path_read_performing` would be cleaner separation but doubles the test count. Parameterizing `access_audits_live_path` to run both variants in one function with sub-assertions is preferred.

**Rejected alternative — use real WASM modules in dag validation:**
Constructing actual WASM modules that call specific undeclared WIT views for the dag validation test is prohibitively complex. The test-only dispatch helper that exercises real `WasmRuntimeDispatcher` read-collection (even with a test double for the actual WASM execution) satisfies the "exercises real WIT view calls" requirement.

### Exact Code Surface

- `crates/slicer-host/tests/pipeline_tdd.rs`:
  - Add `PostpassModuleReadingPostpassRunner` struct with `run_gcode_postprocess` returning `PostpassOutput::GCodeSuccess` and `take_runtime_reads` returning `vec![vec!["LayerCollectionIR".to_string()]]`
  - Update `access_audits_live_path` to also run a read-performing variant with this runner and assert non-empty `runtime_reads` containing `"LayerCollectionIR"`
- `crates/slicer-host/tests/dag_validation_tdd.rs`:
  - Replace `collect_dispatch_audit` simulation with a helper that calls `WasmRuntimeDispatcher` dispatch methods and extracts reads via `take_runtime_reads`
  - The helper must produce the same audit shape (`module_id`, `runtime_reads`, `runtime_writes`) but from live dispatch

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `PostpassStageRunner::take_runtime_reads` returns `Vec<Vec<String>>` — one inner vec per postpass module call, in call order.
- `WasmRuntimeDispatcher.postpass_runtime_reads: RefCell<Vec<Vec<String>>>` — accumulates reads per dispatch call, consumed by `take_runtime_reads`.

## Locked Assumptions and Invariants

- The packet does not change the WIT world definitions or any view method signatures.
- `validate_undeclared_access` remains the contract-enforcement endpoint; this packet only ensures live audits reach it.
- `NoopPostpassRunner` must continue to compile without modification (used by other tests).
- Claim Transition Matrix tests remain green.

## Risks and Tradeoffs

- **Test complexity:** Adding a custom runner struct to `access_audits_live_path` increases test code size but is the cleanest way to exercise the read-performing path. The inline struct avoids polluting the module namespace.
- **dag validation test performance:** Constructing a `WasmRuntimeDispatcher` in the dag validation test may add setup overhead. If the dispatcher construction is expensive, the helper should be lazy or reused across assertions.
- **WIT view call coverage:** The dag validation helper that calls `WasmRuntimeDispatcher` dispatch may not actually invoke WIT view methods if the runner double short-circuits the call. The helper must ensure reads are actually collected by the dispatcher's `runtime_reads` mechanism.

## Open Questions

- Resolved: Whether to add a second test or parameterize `access_audits_live_path`. Decision: add a second test function `access_audits_live_path_read_performing` alongside the existing `access_audits_live_path` (which tests write-only), to keep assertions clean and readable.
- Resolved: Whether the dag validation helper needs actual WASM modules. Decision: the helper must use `WasmRuntimeDispatcher`'s read-collection mechanism. A test double that returns reads without actual WASM execution is acceptable if the read-collection pipeline (store → context → reads) is exercised end-to-end.
