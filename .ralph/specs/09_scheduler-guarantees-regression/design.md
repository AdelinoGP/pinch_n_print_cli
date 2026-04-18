# Design: scheduler-guarantees-regression

## Controlling Code Paths

- Primary code path for TASK-131 and TASK-132: `crates/slicer-host/src/scheduler/` — `resolve_active_regions` function and `RegionMap` construction/budget enforcement.
- Primary code path for TASK-133: `crates/slicer-host/src/wasm_instance_pool.rs` — instance acquisition and the `layer_parallel_safe` gating in the pool.
- Primary code path for TASK-134: `crates/slicer-host/src/layer_executor.rs` — per-layer stage runner and the nine stage executors (Slice, SlicePostProcess, Perimeters, PerimetersPostProcess, Infill, InfillPostProcess, Support, SupportPostProcess, PathOptimization).
- Neighboring tests or fixtures: `crates/slicer-host/tests/` — existing TDD tests use `#[tokio::test]` or `#[test]` with `slicer_host_test_fixtures` or `wasmtime` bindings. Existing `resolve_active_regions` callers in `dispatch_tdd.rs` and `layer_executor_tdd.rs` show the invocation pattern.
- OrcaSlicer comparison surface: None — all four are internal scheduler contracts.

## Architecture Constraints

- `resolve_active_regions` must use `module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegionRef>>` for O(1) lookup. Any implementation that filters `region_map.entries.iter()` on each call is non-compliant per docs/04.
- RegionMap overflow must be caught at startup (Phase 3 DAG validation), not lazily at per-layer execution, per docs/04 Memory Budget Contract.
- Instance pool serialization must be enforced at the pool level, not by requiring modules to be single-threaded — a `Mutex<WasmInstance>` inside the pool for sequential modules is the correct implementation.
- Catch-up field propagation must survive each stage's output IR transformation. Stages that produce new IR structs (e.g., `SliceIR`, `PerimeterIR`) must preserve the three `ActiveRegion` fields from the input `GlobalLayer` into the output IR's `effective_layer_height`, `is_catchup_layer`, `catchup_z_bottom` fields.

## Code Change Surface

- Selected approach:
  - TASK-131: Add `resolve_active_regions_o1_contract_tdd.rs`. Use a mock `RegionMap` with N entries and verify the implementation calls `module_region_index.get()` directly, not an iteration over `entries`. Assert the output matches expected active regions.
  - TASK-132: Add `region_map_overflow_tdd.rs`. Build a `RegionMap` with 1001 entries using a test helper, attempt host initialization, and assert the returned error contains entry count, cap 1000, top-contributor tuples, and remediation string. Also add `region_map_at_cap_tdd.rs` for the boundary case.
  - TASK-133: Add `layer_parallel_safe_false_serialization_tdd.rs`. Use `rayon::spawn` with a shared sequential module pool, spawn two concurrent acquisition tasks, and assert via an `Arc<Mutex<u32>>` counter that only one acquisition is in-flight at a time.
  - TASK-134: Add `catchup_layer_propagation_tdd.rs`. Construct a catch-up `GlobalLayer` with the three fields set, run it through each of the nine stage executors in sequence, and assert each output IR preserves the three fields. Stage executors can be called directly via `SliceExecutor::run`, `SlicePostProcessExecutor::run`, etc. using test fixtures from the existing layer executor tests.

- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - New: `crates/slicer-host/tests/resolve_active_regions_o1_contract_tdd.rs`
  - New: `crates/slicer-host/tests/region_map_overflow_tdd.rs`
  - New: `crates/slicer-host/tests/region_map_at_cap_tdd.rs`
  - New: `crates/slicer-host/tests/layer_parallel_safe_false_serialization_tdd.rs`
  - New: `crates/slicer-host/tests/catchup_layer_propagation_tdd.rs`
  - No existing source files need modification. The tests are additive regression guards. If an existing implementation already satisfies the contract, the test validates the existing behavior without requiring changes.

- Rejected alternatives that were considered and why they were not chosen:
  - Adding performance benchmarks for TASK-131 instead of a contract test: Benchmarks are noisy and platform-dependent. A contract test that asserts the implementation uses `module_region_index.get()` is deterministic and reproducible.
  - Putting overflow check inside per-layer loop for TASK-132: The docs/04 contract requires catch at startup/Phase 3, not lazily. A per-layer check would miss the overflow until runtime and could produce partial output before failing.
  - Using a semaphore for TASK-133 instead of a mutex: A semaphore with count=1 is functionally equivalent to a mutex for serialization. A `Mutex<WasmInstance>` is simpler and already used in the existing pool implementation pattern.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `RegionMapIR` (docs/02) — budget cap at 1000 entries
  - `ActiveRegion` (docs/02 lines 274-278) — `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height`
  - `GlobalLayer` (docs/02) — carries catch-up metadata from planning into per-layer execution
  - `layer_parallel_safe` manifest field (docs/03) — gates pool behavior

- WIT boundary considerations: None for this packet. All four tests operate at the host/scheduler layer and do not cross the WIT boundary.

- Determinism or scheduler constraints: `resolve_active_regions` must be deterministic — same `(layer, module)` pair always returns the same slice. The test must cover repeated calls with the same inputs.

## Locked Assumptions and Invariants

- The `module_region_index` map is precomputed once during RegionMap construction and never modified during per-layer execution. This is required for the O(1) guarantee.
- RegionMap overflow is a fatal planning error (not a warning or a retry). The test must assert the host terminates with an error, not just logs a warning.
- Sequential module pools use a single `Mutex<WasmInstance>` wrapping one instance. The mutex ensures only one thread can hold the instance at a time.
- Catch-up layer metadata originates in `PrePass::LayerPlanning` and is stored in `GlobalLayer`. Each stage executor receives a `GlobalLayer` and must propagate relevant fields into its output IR.

## Risks and Tradeoffs

- Risk: The TASK-134 propagation test could be brittle if stage executors produce output IR with different type signatures than currently documented. Mitigation: Use the existing `LayerPlanIR` fixture from `layer_executor_tdd.rs` as the test harness, and assert only on the three named fields that are documented in docs/02.
- Risk: TASK-133 serialization test could be flaky under heavy CI load if rayon thread scheduling is unpredictable. Mitigation: Use a barrier (`std::sync::Barrier`) to ensure both tasks attempt acquisition simultaneously, and assert the counter never exceeds 1. The test completes in under 1 second.

## Open Questions

- None. All four tasks have clear authoritative sources, clear test strategies, and no ambiguous scope boundaries. The packet can proceed to implementation without additional discovery.

- If an open question would change scope, interfaces, or verification strategy, the packet must remain `draft` until it is answered.