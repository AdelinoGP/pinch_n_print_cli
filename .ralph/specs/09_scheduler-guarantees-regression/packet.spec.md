---
status: implemented
packet: scheduler-guarantees-regression
task_ids:
  - TASK-131
  - TASK-132
  - TASK-133
  - TASK-134
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: scheduler-guarantees-regression

## Goal

Close the gap between the documented scheduler guarantees and the current host implementation by materializing the missing host-side `(global_layer_index, module_id)` active-region lookup surface, enriching RegionMap overflow failures with structured contributor/remediation diagnostics, locking serialized pool acquisition on the canonical instance-pool surface, and adding catch-up metadata regression coverage on the host and IR surfaces that actually carry it during per-layer execution.

## Scope Boundaries

- In scope:
  - host-side precomputed `(global_layer_index, module_id)` lookup plus regression guard for the documented `resolve_active_regions` O(1) contract (TASK-131)
  - RegionMap overflow diagnostics with 1000-entry cap, top-contributor tuples, and remediation messaging on the real startup/region-mapping paths (TASK-132)
  - concurrent acquisition coverage for `layer_parallel_safe = false` on the canonical `src/instance_pool.rs` surface (TASK-133)
  - catch-up metadata coverage that verifies every per-layer stage sees unchanged source `ActiveRegion` catch-up fields, and every downstream IR surface that defines `effective_layer_height` preserves that value unchanged (TASK-134)
  - Negative cases for empty-region resolution and exactly-at-cap RegionMap
- Out of scope:
  - PrePass segmentation boundary gaps (TASK-128a/128b)
  - PostPass WIT gaps (TASK-129a/129b/129c)
  - Mesh-query host services (TASK-147/148)
  - Benchy parity (TASK-120 series)
  - Runtime-budget evidence collection (TASK-156) — separate packet
  - WIT or IR widening to add `is_catchup_layer` / `catchup_z_bottom` fields to downstream per-layer IR structs that do not currently define them

## Prerequisites and Blockers

- Depends on: TASK-120 (Benchy parity) is not a blocker for this packet; this packet operates at the scheduler/IR layer and does not require end-to-end Benchy completion.
- Unblocks: TASK-156 (runtime-budget evidence collection) consumes the `resolve_active_regions` O(1) guard; DEV-026 evidence for RegionMap overflow; instance-pool concurrency documentation in docs/04.
- Activation blockers: None — this draft now selects concrete host-side lookup, diagnostics, and catch-up coverage surfaces without widening the IR schema.

## Acceptance Criteria

- **Given** an `ExecutionPlan` with layer `3` containing regions `7` and `9` bound to module `com.example.perimeters`, **when** the host resolves active regions for `(3, com.example.perimeters)`, **then** it returns region IDs `[7, 9]` from a precomputed `(global_layer_index, module_id)` lookup and does not fall back to a full scan of `global_layers` or `region_plans`. | `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_uses_precomputed_index -- --exact --nocapture`
- **Given** a slice job would produce more than `1000` RegionMap entries, **when** the host aborts startup planning, **then** the fatal error contains `entry_count`, `cap=1000`, at least one top-contributor tuple `(object_id, region_count, layer_count)`, and one remediation hint among `reduce region granularity`, `raise cap`, or `split job`. | `cargo test -p slicer-host --test region_mapping_tdd region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation -- --exact --nocapture`
- **Given** a module declares `layer_parallel_safe = false`, **when** two concurrent acquisition attempts target the same pool, **then** the second acquisition blocks until the first lease is dropped and both leases observe slot `0` on the serialized pool. | `cargo test -p slicer-host --test wasm_instance_pool_tdd serialized_pools_block_other_leasers_until_release -- --exact --nocapture`
- **Given** a catch-up `ActiveRegion` with `is_catchup_layer=true` and `catchup_z_bottom=0.3`, **when** a layer containing that region runs through Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization, **then** every stage sees unchanged source `ActiveRegion.is_catchup_layer` and `ActiveRegion.catchup_z_bottom` values. | `cargo test -p slicer-host --test layer_executor_tdd catchup_metadata_remains_stable_across_all_per_layer_stages -- --exact --nocapture`
- **Given** a catch-up `ActiveRegion` with `effective_layer_height=0.3`, **when** the host-built slice/output surfaces are assembled for that layer, **then** every downstream IR type that defines `effective_layer_height` preserves the value `0.3` unchanged. | `cargo test -p slicer-host --test layer_slice_tdd layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions -- --exact --nocapture`

## Negative Test Cases

- **Given** the host resolves active regions for `(3, com.example.support)` and no regions are bound to that module on layer `3`, **when** it executes, **then** it returns an empty slice/list rather than an error. | `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_returns_empty_when_module_has_no_regions -- --exact --nocapture`
- **Given** RegionMap construction produces exactly `1000` entries (at the cap), **when** startup planning continues, **then** it succeeds without overflow error. | `cargo test -p slicer-host --test region_mapping_tdd region_mapping_at_cap_is_accepted -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_uses_precomputed_index -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd resolve_active_regions_returns_empty_when_module_has_no_regions -- --exact --nocapture`
- `cargo test -p slicer-host --test region_mapping_tdd region_mapping_cap_exceeded_surfaces_top_contributors_and_remediation -- --exact --nocapture`
- `cargo test -p slicer-host --test region_mapping_tdd region_mapping_at_cap_is_accepted -- --exact --nocapture`
- `cargo test -p slicer-host --test wasm_instance_pool_tdd serialized_pools_block_other_leasers_until_release -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_executor_tdd catchup_metadata_remains_stable_across_all_per_layer_stages -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_slice_tdd layer_slice_builtin_preserves_effective_layer_height_for_catchup_regions -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings` must pass before packet completion.

## Authoritative Docs

- `docs/01_system_architecture.md` — WASM instance pool behavior; Catch-Up Layer Semantics
- `docs/02_ir_schemas.md` — `ActiveRegion` catch-up metadata; `RegionMapIR` and `SlicedRegion` field surfaces
- `docs/04_host_scheduler.md` — `resolve_active_regions` complexity contract; RegionMapIR Memory Budget Contract with 1000-entry cap; WASM host-call batching contract
- `docs/12_architecture_gate_metrics.md` — performance thresholds

## OrcaSlicer Reference Obligations

- None — all four guarantees are internal scheduler contracts with no OrcaSlicer behavioral reference.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`