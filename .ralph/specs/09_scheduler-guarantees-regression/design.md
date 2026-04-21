# Design: scheduler-guarantees-regression

## Controlling Code Paths

- Primary code path for TASK-131: `crates/slicer-host/src/execution_plan.rs` — frozen runtime execution metadata already carries `global_layers` and `region_plans`, and is the narrowest host-owned surface where the documented `(global_layer_index, module_id)` lookup can be materialized without widening the IR schema.
- Primary code path for TASK-132: `crates/slicer-host/src/region_mapping.rs` and `crates/slicer-host/src/execution_plan.rs` — both participate in startup-time RegionMap bounds enforcement and must surface the same structured contributor/remediation diagnostics.
- Primary code path for TASK-133: `crates/slicer-host/src/instance_pool.rs` — instance acquisition and the `layer_parallel_safe` gating in the canonical pool implementation.
- Primary code path for TASK-134: `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/src/layer_slice.rs`, and adjacent typed layer dispatch in `crates/slicer-host/src/dispatch.rs` — these surfaces carry the source `GlobalLayer.active_regions` metadata and the downstream `effective_layer_height` copies.
- Neighboring tests or fixtures: `crates/slicer-host/tests/execution_plan_tdd.rs`, `crates/slicer-host/tests/region_mapping_tdd.rs`, `crates/slicer-host/tests/wasm_instance_pool_tdd.rs`, `crates/slicer-host/tests/acceptance_gate_gaps_tdd.rs`, `crates/slicer-host/tests/layer_executor_tdd.rs`, `crates/slicer-host/tests/layer_slice_tdd.rs`, and `crates/slicer-host/tests/scenario_traces_tdd.rs`.
- OrcaSlicer comparison surface: None — all four are internal scheduler contracts.

## Architecture Constraints

- The O(1) lookup must come from a precomputed host-side index built once at startup. Any implementation that filters all `global_layers`, all `RegionMapIR.entries`, or all `ExecutionPlan.region_plans` on each call is non-compliant with docs/04.
- Selected approach for TASK-131: add a derived host-only lookup table to `ExecutionPlan` keyed by `(global_layer_index, module_id)` and expose a canonical `resolve_active_regions` helper on that surface. Do not widen `RegionMapIR` in `slicer-ir` for this packet.
- RegionMap overflow must be caught at startup (planning / region-mapping time), not lazily at per-layer execution, per docs/04 Memory Budget Contract.
- Selected approach for TASK-132: enrich the startup error shape with contributor tuples and remediation text on the real `region_mapping.rs` / `execution_plan.rs` paths. If both paths can fail, they must share one diagnostics shape or one formatting helper so the emitted fields stay identical.
- Instance pool serialization must remain enforced at the canonical pool level through `InstancePoolMode::Serialized` and `SlotAvailability` (`Mutex<SlotAvailabilityState>` + `Condvar`). Do not introduce a second concurrency primitive or a fake test-only pool implementation.
- Catch-up metadata is defined on `ActiveRegion`; downstream per-layer IRs only guarantee `effective_layer_height`. Tests must therefore assert `is_catchup_layer` / `catchup_z_bottom` on the source `GlobalLayer.active_regions` surface across all nine stages, and assert `effective_layer_height` only on the IR types that actually define it.

## Code Change Surface

- Selected approach:
  - TASK-131: extend `ExecutionPlan` with a precomputed module-region lookup and a canonical lookup helper; add direct coverage in `execution_plan_tdd.rs` for the positive and empty-result cases.
  - TASK-132: enrich the startup overflow diagnostics on `region_mapping.rs` / `execution_plan.rs`, then extend `region_mapping_tdd.rs` (and `execution_plan_tdd.rs` only if needed for the plan-build path) to assert the exact contributor/remediation fields.
  - TASK-133: extend or tighten the canonical contention coverage in `wasm_instance_pool_tdd.rs` so the backlog item is satisfied on the real `src/instance_pool.rs` surface. Source changes are only expected if the current blocking path needs small diagnostic or observability adjustments.
  - TASK-134: extend `layer_executor_tdd.rs` with a recording runner that proves each stage sees unchanged source `ActiveRegion` catch-up metadata, and extend `layer_slice_tdd.rs` to assert `effective_layer_height` propagation on the supported downstream IR surface. If the real typed dispatch path proves to be the controlling surface for the regression, `dispatch.rs` may need a narrow fix to thread non-zero layer metadata into `HostExecutionContext::new`.

- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/execution_plan.rs`
  - `crates/slicer-host/src/region_mapping.rs`
  - `crates/slicer-host/src/instance_pool.rs` (only if Step 3 needs a narrow observability/diagnostic change)
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/layer_slice.rs`
  - `crates/slicer-host/src/dispatch.rs` (only if Step 4 proves the typed layer dispatch path is the controlling metadata surface)
  - `crates/slicer-host/tests/execution_plan_tdd.rs`
  - `crates/slicer-host/tests/region_mapping_tdd.rs`
  - `crates/slicer-host/tests/wasm_instance_pool_tdd.rs`
  - `crates/slicer-host/tests/layer_executor_tdd.rs`
  - `crates/slicer-host/tests/layer_slice_tdd.rs`

- Rejected alternatives that were considered and why they were not chosen:
  - Adding performance benchmarks for TASK-131 instead of a direct lookup contract test: benchmarks are noisy and platform-dependent. A host-owned lookup helper plus deterministic tests is the right guardrail.
  - Widening `RegionMapIR` with a serialized `module_region_index` field: the documented lookup can be implemented as host-only runtime state on `ExecutionPlan` without changing the IR schema.
  - Putting overflow checks inside the later per-layer loop for TASK-132: the docs/04 contract requires catch at startup/Phase 3, not lazily during execution.
  - Replacing the current serialized pool implementation with a second `Mutex<WasmInstance>` wrapper just for tests: `src/instance_pool.rs` already encodes the canonical serialized behavior through slot availability and should remain the single implementation surface.
  - Asserting `is_catchup_layer` / `catchup_z_bottom` on every downstream IR: the downstream IR types do not define those fields, so the test must stay on the source `ActiveRegion` surface and on the IRs that actually define `effective_layer_height`.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `ExecutionPlan` host runtime state — gains the precomputed module-region lookup surface for TASK-131
  - `RegionMapIR` / `region_plans` startup budget cap at 1000 entries
  - `RegionMappingError` / `ExecutionPlanError` contributor/remediation diagnostics
  - `ActiveRegion` — `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height`
  - `SlicedRegion.effective_layer_height` — downstream IR field that must preserve the source height
  - `layer_parallel_safe` manifest field (docs/03) — gates pool behavior

- WIT boundary considerations: No WIT schema change is planned. If Step 4 uncovers that the typed layer dispatch path is the controlling metadata surface, keep any fix narrowly inside host-side dispatch/context wiring and do not widen the world definitions in this packet.

- Determinism or scheduler constraints: the module-region lookup must be deterministic — the same `(layer, module)` pair always returns the same region ordering; overflow contributor ordering must also be stable enough for exact test assertions.

## Locked Assumptions and Invariants

- `ExecutionPlan.region_plans` remains the canonical per-region plan storage; the new module-region lookup is derived once during startup and never mutated during per-layer execution.
- RegionMap overflow is a fatal planning error (not a warning or a retry). The test must assert the host terminates with an error, not just logs a warning.
- Sequential module pools use one serialized slot and block contenders until release. The backlog slice should keep proving that canonical behavior rather than re-specifying the pool abstraction.
- Catch-up metadata originates in `PrePass::LayerPlanning` and is stored on `GlobalLayer.active_regions`. Each stage executor receives the source `GlobalLayer`; downstream IRs only need to preserve the fields they actually define.

## Risks and Tradeoffs

- Risk: TASK-131 may expose that no runtime consumer currently calls the lookup helper. Mitigation: land the host-side lookup and direct tests first, then wire one canonical consumer only if the task still lacks an executable proof.
- Risk: TASK-132 touches the same startup surfaces as TASK-131. Mitigation: keep Steps 1 and 2 serialized and reuse one diagnostics helper/shape rather than forking the startup error contract.
- Risk: TASK-133 contention coverage could still be timing-sensitive on CI. Mitigation: keep the existing blocking-lease test shape and only strengthen it with deterministic coordination primitives already used in the test suite.
- Risk: TASK-134 can become brittle if it asserts non-existent fields on downstream IRs. Mitigation: keep the assertions on `GlobalLayer.active_regions` for catch-up flags and on `effective_layer_height` only for the IR types that declare it.

## Open Questions

- None. All four tasks have clear authoritative sources, clear test strategies, and no ambiguous scope boundaries. The packet can proceed to implementation without additional discovery.

- If an open question would change scope, interfaces, or verification strategy, the packet must remain `draft` until it is answered.