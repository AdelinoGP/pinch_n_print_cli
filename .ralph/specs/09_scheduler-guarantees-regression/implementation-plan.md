# Implementation Plan: scheduler-guarantees-regression

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add `resolve_active_regions_o1_contract_tdd.rs` and `resolve_active_regions_empty_tdd.rs`

- Task IDs:
  - `TASK-131`
- Objective:
  Add a regression guard proving `resolve_active_regions` uses `module_region_index` map lookup (O(1)) and does not iterate over all entries or all layers. Also add the negative case for empty-region resolution.
- Precondition:
  `crates/slicer-host/src/scheduler/` contains a `resolve_active_regions` function with at least one caller in the dispatch/executor path. The `RegionMap` struct has a `module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegionRef>>` field.
- Postcondition:
  Two new test files exist in `crates/slicer-host/tests/`. `resolve_active_regions_o1_contract_tdd` asserts O(1) lookup. `resolve_active_regions_empty_tdd` asserts empty slice returned when no regions match.
- Files expected to change:
  - `crates/slicer-host/tests/resolve_active_regions_o1_contract_tdd.rs` (new)
  - `crates/slicer-host/tests/resolve_active_regions_empty_tdd.rs` (new)
- Authoritative docs:
  - `docs/04_host_scheduler.md` (lines 492-510)
  - `docs/02_ir_schemas.md` (RegionMapIR structure)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test resolve_active_regions_o1_contract_tdd 2>&1 | grep -E "O.1|O\\(1\\)|constant.*time"`
  - `cargo test -p slicer-host --test resolve_active_regions_empty_tdd 2>&1 | grep -E "empty.*slice|ok"`
- Exit condition:
  Both tests pass and the O(1) test confirms the implementation calls `module_region_index.get()` not `region_map.entries.iter()`.

---

### Step 2: Add `region_map_overflow_tdd.rs` and `region_map_at_cap_tdd.rs`

- Task IDs:
  - `TASK-132`
- Objective:
  Add structured overflow coverage proving that when a slice job exceeds 1000 RegionMap entries, the host fails with a fatal error containing entry count, configured cap, top-contributor tuples, and a remediation hint. Also add the boundary case at exactly 1000 entries.
- Precondition:
  `RegionMap` construction happens in Phase 3 (DAG validation) before execution begins. There is an existing budget cap field or configuration for the 1000-entry limit.
- Postcondition:
  `region_map_overflow_tdd.rs` triggers overflow with 1001 entries and asserts the error message contains entry count, cap 1000, top-contributor tuples, and remediation hint. `region_map_at_cap_tdd.rs` passes with exactly 1000 entries and asserts no overflow error.
- Files expected to change:
  - `crates/slicer-host/tests/region_map_overflow_tdd.rs` (new)
  - `crates/slicer-host/tests/region_map_at_cap_tdd.rs` (new)
- Authoritative docs:
  - `docs/04_host_scheduler.md` (lines 512-530)
  - `docs/02_ir_schemas.md` (RegionMapIR)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test region_map_overflow_tdd 2>&1 | grep -E "overflow|cap.*1000|top.contributor|remediation"`
  - `cargo test -p slicer-host --test region_map_at_cap_tdd 2>&1 | grep -E "at.cap|succeeds|1000.*entries"`
- Exit condition:
  Both tests pass. The overflow test confirms the error message contains all four required fields. The at-cap test confirms a valid job at exactly 1000 entries does not error.

---

### Step 3: Add `layer_parallel_safe_false_serialization_tdd.rs`

- Task IDs:
  - `TASK-133`
- Objective:
  Add a regression guard proving that a module with `layer_parallel_safe = false` serializes concurrent WASM instance acquisition across rayon threads.
- Precondition:
  The WASM instance pool in `crates/slicer-host/src/wasm_instance_pool.rs` has separate paths for parallel-safe and sequential modules. Sequential modules use a `Mutex<WasmInstance>` or equivalent to enforce exclusive access.
- Postcondition:
  `layer_parallel_safe_false_serialization_tdd.rs` creates two rayon tasks that concurrently attempt to acquire an instance from a sequential module's pool. The test asserts via a shared counter that at most one acquisition is in-flight at any moment.
- Files expected to change:
  - `crates/slicer-host/tests/layer_parallel_safe_false_serialization_tdd.rs` (new)
- Authoritative docs:
  - `docs/01_system_architecture.md` (lines 46-49)
  - `docs/04_host_scheduler.md` (instance pool behavior)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test layer_parallel_safe_false_serialization_tdd 2>&1 | grep -E "serialized|one.at.a.time|exclusive"`
- Exit condition:
  The test passes and confirms that concurrent acquisitions are serialized, not parallelized.

---

### Step 4: Add `catchup_layer_propagation_tdd.rs`

- Task IDs:
  - `TASK-134`
- Objective:
  Add a regression guard proving that `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive unchanged through all nine per-layer stages: Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization.
- Precondition:
  `GlobalLayer` carries `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` fields. Each stage executor receives a `GlobalLayer` and produces an output IR. Existing stage executor types are `SliceExecutor`, `SlicePostProcessExecutor`, `PerimetersExecutor`, etc. in `crates/slicer-host/src/layer_executor.rs`.
- Postcondition:
  `catchup_layer_propagation_tdd.rs` constructs a catch-up `GlobalLayer` with `is_catchup_layer=true`, `catchup_z_bottom=0.3`, `effective_layer_height=0.3`. It runs it through each of the nine stage executors in sequence and asserts each output IR preserves all three fields unchanged.
- Files expected to change:
  - `crates/slicer-host/tests/catchup_layer_propagation_tdd.rs` (new)
- Authoritative docs:
  - `docs/01_system_architecture.md` (lines 117-136)
  - `docs/02_ir_schemas.md` (lines 274-278 — `ActiveRegion` fields)
  - `docs/04_host_scheduler.md` (per-layer stage list)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test catchup_layer_propagation_tdd 2>&1 | grep -E "catchup.*layer.*propagat|is_catchup_layer.*preserved"`
- Exit condition:
  The test passes and confirms all nine stages preserve the three catch-up fields.

---

## Packet Completion Gate

- All four steps complete.
- Every step exit condition is met.
- All six tests pass:
  - `resolve_active_regions_o1_contract_tdd`
  - `resolve_active_regions_empty_tdd`
  - `region_map_overflow_tdd`
  - `region_map_at_cap_tdd`
  - `layer_parallel_safe_false_serialization_tdd`
  - `catchup_layer_propagation_tdd`
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated to mark TASK-131, TASK-132, TASK-133, TASK-134 as complete.
- Reopened or superseded packet status transitions reconciled (N/A — no prior packets).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all six test grep patterns match the expected output.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.