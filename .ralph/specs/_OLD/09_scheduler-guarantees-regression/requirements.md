# Requirements: scheduler-guarantees-regression

## Packet Metadata

- Grouped task IDs:
  - `TASK-131` — regression guard for `resolve_active_regions` O(1) contract
  - `TASK-132` — structured RegionMap overflow coverage for 1000-entry cap
  - `TASK-133` — pool-behavior test for `layer_parallel_safe = false` serialization
  - `TASK-134` — catch-up layer field propagation test across all per-layer stages
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The scheduler makes four behavioral contracts that are documented but not regression-guarded:

1. **`resolve_active_regions` complexity** (docs/04 `resolve_active_regions` contract): The docs promise an O(1) lookup by `(global_layer_index, module_id)`, but the current host only freezes `ExecutionPlan.region_plans` / `RegionMapIR.entries` and does not materialize the documented host-side lookup surface. This packet must add the missing host-side index/helper and lock it with direct tests instead of assuming the function already exists.

2. **RegionMap overflow diagnostics** (docs/04 RegionMapIR Memory Budget Contract): The host already enforces a 1000-entry cap, but the current structured errors only carry the count/cap. The documented top-contributor tuples and remediation hints are still missing on the real region-mapping / execution-plan startup paths.

3. **Instance pool serialization** (docs/01 WASM instance pool contract): `src/instance_pool.rs` already forces non-parallel-safe modules into a serialized single-slot pool, and existing tests cover the basic blocking lease behavior. This packet keeps the backlog slice honest by tightening the canonical scheduler-contract coverage on that existing surface rather than inventing a second pool abstraction.

4. **Catch-up metadata propagation** (docs/01 Catch-Up Layer Semantics + docs/02 `ActiveRegion`): Catch-up state lives on `ActiveRegion`, while downstream per-layer IRs only guarantee `effective_layer_height`. The packet must therefore guard the real contract: every stage must observe unchanged source `ActiveRegion.is_catchup_layer` / `catchup_z_bottom`, and every downstream IR surface that actually defines `effective_layer_height` must preserve that value unchanged.

If this packet reopens, supersedes, or narrows a prior packet, name the earlier packet and the exact gap it left behind.

This packet does not supersede any prior packet. It is the first guard for these four scheduler guarantees.

## In Scope

- host-side precomputed `(global_layer_index, module_id)` lookup plus regression guard for the documented `resolve_active_regions` O(1) contract (TASK-131)
- RegionMap overflow diagnostics with 1000-entry cap, top-contributor tuples, and remediation messaging on the real startup/region-mapping paths (TASK-132)
- canonical pool-behavior coverage for `layer_parallel_safe = false` serialization on `src/instance_pool.rs` (TASK-133)
- catch-up metadata coverage across all nine per-layer stages, scoped to the source `ActiveRegion` surface and the downstream IR types that actually define `effective_layer_height` (TASK-134)
- Negative cases: empty-region resolution returns empty slice; exactly-1000-entry RegionMap succeeds without overflow

## Out of Scope

- PrePass segmentation boundary gaps (TASK-128a/128b)
- PostPass WIT gaps (TASK-129a/129b/129c)
- Mesh-query host services (TASK-147/148)
- Benchy parity (TASK-120 series)
- Runtime-budget evidence collection (TASK-156)
- Non-planar Z envelope enforcement (TASK-127) — separate packet
- Claim transition matrix enforcement (TASK-125) — already covered by `claim_transition_matrix_tdd.rs`
- WIT or IR widening to add `is_catchup_layer` / `catchup_z_bottom` fields to downstream per-layer IR types that do not currently define them

## Authoritative Docs

- `docs/01_system_architecture.md` — WASM instance pool behavior (lines 46-49), Catch-Up Layer Semantics (lines 117-136)
- `docs/02_ir_schemas.md` — LayerPlanIR `ActiveRegion` fields `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` (lines 274-278); `RegionMapIR` structure
- `docs/04_host_scheduler.md` — `resolve_active_regions` O(1) contract (lines 492-510); RegionMapIR Memory Budget Contract (lines 512-530); WASM Host-Call Batching Contract
- `docs/12_architecture_gate_metrics.md` — performance thresholds

## OrcaSlicer Reference Obligations

- None — all four guarantees are internal scheduler contracts with no OrcaSlicer behavioral reference.

## Acceptance Summary

### Positive Cases

- TASK-131: `ExecutionPlan` exposes a precomputed `(global_layer_index, module_id)` lookup surface that returns the expected region IDs for a module/layer pair without rescanning all `global_layers` or `region_plans` on each call.
- TASK-132: When a slice job would produce more than 1000 RegionMap entries, the host fails with a fatal error containing the computed entry count, the configured cap (1000), top-contributor tuples `(object_id, region_count, layer_count)`, and a remediation hint.
- TASK-133: When a module with `layer_parallel_safe = false` is used in a multi-threaded context, concurrent acquisition attempts serialize on the canonical pool surface — the second call blocks until the first lease releases slot `0`.
- TASK-134: A catch-up `ActiveRegion` passing through all nine per-layer stages keeps `is_catchup_layer=true` and `catchup_z_bottom=B` unchanged on the source layer surface seen by every stage, and all downstream IR types that define `effective_layer_height` preserve `H` unchanged.

### Negative Cases

- `resolve_active_regions` with a `module_id` that has no active regions for the given layer returns an empty slice/list, not an error.
- A RegionMap with exactly 1000 entries (at the cap) succeeds without overflow when the job is otherwise valid.

### Measurable Outcomes

- `resolve_active_regions_uses_precomputed_index`: Test builds an `ExecutionPlan` with layer `3` / module `com.example.perimeters` bound to region IDs `[7, 9]`, resolves active regions twice, and asserts the helper returns `[7, 9]` from the precomputed lookup surface rather than rescanning the whole plan.
- `region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation`: Test constructs `1001+` entries, runs the real startup/region-mapping path, and asserts the fatal error contains `entry_count`, `cap=1000`, at least one `(object_id, region_count, layer_count)` tuple, and one remediation hint.
- `serialized_pools_block_other_leasers_until_release`: Extend or reuse the canonical `wasm_instance_pool_tdd.rs` contention test so it remains the scheduler-contract guard for `layer_parallel_safe = false` serialization.
- `catchup_metadata_remains_stable_across_all_per_layer_stages`: Test records the `GlobalLayer.active_regions` surface observed by every per-layer stage and asserts `is_catchup_layer=true` and `catchup_z_bottom=0.3` remain unchanged across all nine stages.
- `layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions`: Test runs `execute_layer_slice` for a catch-up region and asserts `SliceIR.regions[*].effective_layer_height == 0.3` on the supported downstream IR surface.
- `resolve_active_regions_returns_empty_when_module_has_no_regions`: Test resolves a module/layer pair with no bound regions and asserts the return value is empty.
- `region_mapping_at_cap_is_accepted`: Test configures a valid job with exactly 1000 RegionMap entries and asserts it succeeds without overflow error.

## Verification Commands

- `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_uses_precomputed_index -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_returns_empty_when_module_has_no_regions -- --exact --nocapture`
- `cargo test -p slicer-host --test region_mapping_tdd region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation -- --exact --nocapture`
- `cargo test -p slicer-host --test region_mapping_tdd region_mapping_at_cap_is_accepted -- --exact --nocapture`
- `cargo test -p slicer-host --test wasm_instance_pool_tdd serialized_pools_block_other_leasers_until_release -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_executor_tdd catchup_metadata_remains_stable_across_all_per_layer_stages -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_slice_tdd layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings` (workspace gate before commit)

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: What must be true before the step begins.
- Postcondition: What the step achieves; how to confirm it is done without full integration.
- Falsifying check: The narrowest command or assertion that would fail if the step goal is not met.

Steps 1 and 2 share `src/execution_plan.rs` / `src/region_mapping.rs` and should be serialized. Step 3 stays on the canonical instance-pool surface and can proceed independently. Step 4 may proceed independently once its supported metadata surfaces are locked. The negative cases stay with the same parent step as their positive counterpart.

## Cross-Packet Impact

- TASK-131 unblocks TASK-156 (runtime-budget evidence collection) — the O(1) guard provides the performance baseline needed for budget assertions.
- TASK-132 provides RegionMap overflow evidence for DEV-026 closure in the architecture acceptance gate.
- TASK-133 provides instance-pool concurrency evidence for docs/04 contract documentation.
- TASK-134 guards catch-up metadata stability needed for multi-layer-height object support while deferring any broader WIT/IR surface widening to later packets.