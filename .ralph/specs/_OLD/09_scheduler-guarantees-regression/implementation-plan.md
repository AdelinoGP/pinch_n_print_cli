# Implementation Plan: scheduler-guarantees-regression

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Steps 1 and 2 are serialized because they share `src/execution_plan.rs` / `src/region_mapping.rs`.

## Steps

### Step 1: Materialize the precomputed module-region lookup and add O(1) guards

- Task IDs:
  - `TASK-131`
- Objective:
  Add the missing host-side `(global_layer_index, module_id)` lookup surface on `ExecutionPlan`, expose the canonical `resolve_active_regions` helper there, and add both the positive and empty-result regression guards.
- Precondition:
  `crates/slicer-host/src/execution_plan.rs` already freezes `global_layers` and `region_plans`, but no host-side `(global_layer_index, module_id)` lookup or canonical `resolve_active_regions` helper exists yet.
- Postcondition:
  `ExecutionPlan` exposes an immutable precomputed module-region lookup and `execution_plan_tdd.rs` contains both the positive and empty-result tests for that helper.
- Files expected to change:
  - `crates/slicer-host/src/execution_plan.rs`
  - `crates/slicer-host/tests/execution_plan_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` (`resolve_active_regions` complexity contract)
  - `docs/02_ir_schemas.md` (`RegionMapIR` / `ActiveRegion` surfaces)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_uses_precomputed_index -- --exact --nocapture`
  - `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_returns_empty_when_module_has_no_regions -- --exact --nocapture`
- Exit condition:
  Both tests pass and the positive test proves the helper reads the precomputed module-region lookup instead of rescanning all `global_layers` or all `region_plans`.

---

### Step 2: Enrich RegionMap cap diagnostics on the shared startup paths

- Task IDs:
  - `TASK-132`
- Objective:
  Enrich the real startup/region-mapping overflow diagnostics so they include the contributor/remediation fields promised by docs/04, then lock both the overflow and at-cap boundary behavior with tests.
- Precondition:
  The `DEFAULT_REGION_MAP_CAP` check already exists on `region_mapping.rs` / `execution_plan.rs`, and Step 1 changes are staged because both steps share `src/execution_plan.rs`.
- Postcondition:
  The overflow error emitted by the real startup path includes `entry_count`, `cap=1000`, contributor tuples, and remediation text; the at-cap path succeeds unchanged.
- Files expected to change:
  - `crates/slicer-host/src/region_mapping.rs`
  - `crates/slicer-host/src/execution_plan.rs`
  - `crates/slicer-host/tests/region_mapping_tdd.rs`
  - `crates/slicer-host/tests/execution_plan_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` (RegionMapIR Memory Budget Contract)
  - `docs/02_ir_schemas.md` (`RegionMapIR`)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test region_mapping_tdd region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation -- --exact --nocapture`
  - `cargo test -p slicer-host --test region_mapping_tdd region_mapping_at_cap_is_accepted -- --exact --nocapture`
- Exit condition:
  Both tests pass. The overflow test confirms the real startup error contains all required fields; the at-cap test confirms exactly 1000 entries does not error.

---

### Step 3: Tighten canonical instance-pool serialization coverage

- Task IDs:
  - `TASK-133`
- Objective:
  Keep TASK-133 on the canonical `src/instance_pool.rs` / `wasm_instance_pool_tdd.rs` surface by extending or tightening the existing contention coverage for `layer_parallel_safe = false`.
- Precondition:
  `crates/slicer-host/src/instance_pool.rs` already forces `layer_parallel_safe = false` modules into `InstancePoolMode::Serialized` with a single blocking slot.
- Postcondition:
  The canonical contention test proves that the second acquisition blocks until the first lease is released, and the scheduler contract is covered on the real pool implementation surface.
- Files expected to change:
  - `crates/slicer-host/tests/wasm_instance_pool_tdd.rs`
  - `crates/slicer-host/src/instance_pool.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` (WASM instance pool behavior)
  - `docs/04_host_scheduler.md` (instance-pool behavior)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test wasm_instance_pool_tdd serialized_pools_block_other_leasers_until_release -- --exact --nocapture`
- Exit condition:
  The canonical contention test passes and proves serialized acquisition on the real instance-pool surface.

---

### Step 4: Add catch-up metadata coverage on the supported per-layer surfaces

- Task IDs:
  - `TASK-134`
- Objective:
  Add a regression guard proving that all nine per-layer stages see unchanged source `ActiveRegion.is_catchup_layer` / `catchup_z_bottom`, and that downstream IR surfaces preserve `effective_layer_height` wherever that field actually exists.
- Precondition:
  `GlobalLayer.active_regions` carries `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height`. The downstream IR types do not all define the catch-up flags, so the test must stay on the source layer surface for those fields and on the supported IR surfaces for `effective_layer_height`.
- Postcondition:
  `layer_executor_tdd.rs` proves that every per-layer stage sees unchanged source catch-up flags, and `layer_slice_tdd.rs` proves the supported downstream IR surface preserves `effective_layer_height=0.3` for catch-up regions.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/layer_slice.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/layer_executor_tdd.rs`
  - `crates/slicer-host/tests/layer_slice_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` (Catch-Up Layer Semantics)
  - `docs/02_ir_schemas.md` (`ActiveRegion` and `SlicedRegion` field surfaces)
  - `docs/04_host_scheduler.md` (per-layer stage list)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test layer_executor_tdd catchup_metadata_remains_stable_across_all_per_layer_stages -- --exact --nocapture`
  - `cargo test -p slicer-host --test layer_slice_tdd layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions -- --exact --nocapture`
- Exit condition:
  Both tests pass, no assertion relies on non-existent downstream catch-up fields, and the real stage path still sees the original catch-up metadata unchanged.

---

## Packet Completion Gate

- All four steps complete.
- Every step exit condition is met.
- All seven targeted tests pass:
  - `resolve_active_regions_uses_precomputed_index`
  - `resolve_active_regions_returns_empty_when_module_has_no_regions`
  - `region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation`
  - `region_mapping_at_cap_is_accepted`
  - `serialized_pools_block_other_leasers_until_release`
  - `catchup_metadata_remains_stable_across_all_per_layer_stages`
  - `layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions`
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated to mark TASK-131, TASK-132, TASK-133, TASK-134 as complete.
- Reopened or superseded packet status transitions reconciled (N/A — no prior packets).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all seven targeted test commands pass with the expected assertions.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.