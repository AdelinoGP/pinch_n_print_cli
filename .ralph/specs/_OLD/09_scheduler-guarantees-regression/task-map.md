# Task Map: scheduler-guarantees-regression

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This file is required when the packet spans more than one task ID.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-131` | Step 1 | `docs/04_host_scheduler.md` (`resolve_active_regions` contract) | `crates/slicer-host/src/execution_plan.rs`, `crates/slicer-host/tests/execution_plan_tdd.rs` | None | Materialize the missing host-side `(global_layer_index, module_id)` lookup on `ExecutionPlan` and guard it directly. Keeps the O(1) contract out of the IR schema while unblocking TASK-156. |
| `TASK-132` | Step 2 | `docs/04_host_scheduler.md` (RegionMapIR Memory Budget Contract) | `crates/slicer-host/src/region_mapping.rs`, `crates/slicer-host/src/execution_plan.rs`, `crates/slicer-host/tests/region_mapping_tdd.rs`, `crates/slicer-host/tests/execution_plan_tdd.rs` | None | Structured RegionMap overflow diagnostics with 1000-entry cap, contributor tuples, and remediation hints on the real startup paths. Provides DEV-026 evidence for the architecture acceptance gate. |
| `TASK-133` | Step 3 | `docs/01_system_architecture.md` (WASM instance pool behavior) | `crates/slicer-host/src/instance_pool.rs`, `crates/slicer-host/tests/wasm_instance_pool_tdd.rs` | None | Keep the serialization guard on the canonical pool implementation surface instead of inventing a second test-only pool abstraction. |
| `TASK-134` | Step 4 | `docs/01_system_architecture.md` (Catch-Up Layer Semantics), `docs/02_ir_schemas.md` (`ActiveRegion`, `SlicedRegion`) | `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/src/layer_slice.rs`, `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/tests/layer_executor_tdd.rs`, `crates/slicer-host/tests/layer_slice_tdd.rs` | None | Guard the real catch-up metadata contract: source `ActiveRegion` flags survive all nine stages, and supported downstream IR surfaces preserve `effective_layer_height`. |
| `TASK-131` (negative) | Step 1 | `docs/04_host_scheduler.md` (`resolve_active_regions` contract) | `crates/slicer-host/tests/execution_plan_tdd.rs` | None | Empty-region resolution returns an empty slice/list, not an error. |
| `TASK-132` (negative) | Step 2 | `docs/04_host_scheduler.md` (RegionMapIR Memory Budget Contract) | `crates/slicer-host/tests/region_mapping_tdd.rs` | None | At-cap boundary test: exactly 1000 entries succeeds without overflow. |