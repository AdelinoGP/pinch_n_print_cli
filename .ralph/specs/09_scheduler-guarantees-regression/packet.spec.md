---
status: draft
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

Add regression guards for four scheduler guarantees: O(1) `resolve_active_regions` lookup, RegionMap overflow diagnostics for the 1000-entry cap, `layer_parallel_safe = false` serialization of WASM instance acquisition, and catch-up layer field propagation across all nine per-layer stages.

## Scope Boundaries

- In scope:
  - `resolve_active_regions` O(1) contract regression guard (TASK-131)
  - RegionMap overflow coverage with 1000-entry cap, top-contributor tuples, and remediation messaging (TASK-132)
  - `layer_parallel_safe = false` pool-behavior serialization test (TASK-133)
  - Catch-up layer field propagation test across Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization (TASK-134)
  - Negative cases for empty-region resolution and exactly-at-cap RegionMap
- Out of scope:
  - PrePass segmentation boundary gaps (TASK-128a/128b)
  - PostPass WIT gaps (TASK-129a/129b/129c)
  - Mesh-query host services (TASK-147/148)
  - Benchy parity (TASK-120 series)
  - Runtime-budget evidence collection (TASK-156) — separate packet

## Prerequisites and Blockers

- Depends on: TASK-120 (Benchy parity) is not a blocker for this packet; this packet operates at the scheduler/IR layer and does not require end-to-end Benchy completion.
- Unblocks: TASK-156 (runtime-budget evidence collection) consumes the `resolve_active_regions` O(1) guard; DEV-026 evidence for RegionMap overflow; instance-pool concurrency documentation in docs/04.
- Activation blockers: None — the four tasks are self-contained and do not depend on each other.

## Acceptance Criteria

- **Given** `resolve_active_regions` is called with a `GlobalLayer` and a `CompiledModule`, **when** the RegionMap has N entries (N up to 1000), **then** the call completes in O(1) time — specifically it must not iterate over all regions or all global layers. | `cargo test -p slicer-host --test resolve_active_regions_o1_contract_tdd 2>&1 | grep -E "O.1|O\\(1\\)|constant.*time"`
- **Given** a slice job would produce more than 1000 RegionMap entries, **when** the host processes it, **then** startup fails with a fatal error containing: the computed entry count, the configured cap (1000), top-contributor tuples `(object_id, region_count, layer_count)`, and a remediation hint. | `cargo test -p slicer-host --test region_map_overflow_tdd 2>&1 | grep -E "overflow|cap.*1000|top.contributor|remediation"`
- **Given** a module declares `layer_parallel_safe = false` and the host has multiple rayon threads, **when** two layer processing tasks both try to acquire a WASM instance from that module's pool concurrently, **then** only one acquisition succeeds at a time — the second blocks until the first releases. | `cargo test -p slicer-host --test layer_parallel_safe_false_serialization_tdd 2>&1 | grep -E "serialized|one.at.a.time|exclusive"`
- **Given** a catch-up layer at global Z=Zc with `is_catchup_layer=true`, `catchup_z_bottom=B`, `effective_layer_height=H`, **when** the layer passes through Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization, **then** each stage's output IR preserves `is_catchup_layer=true`, `catchup_z_bottom=B`, and `effective_layer_height=H` unchanged. | `cargo test -p slicer-host --test catchup_layer_propagation_tdd 2>&1 | grep -E "catchup.*layer.*propagat|is_catchup_layer.*preserved"`

## Negative Test Cases

- **Given** `resolve_active_regions` is called with a `module_id` that has no active regions for the given layer, **when** it executes, **then** it returns an empty slice (not an error). | `cargo test -p slicer-host --test resolve_active_regions_empty_tdd 2>&1 | grep -E "empty.*slice|ok"`
- **Given** `RegionMap` has exactly 1000 entries (at the cap), **when** a valid job with 1000 entries is processed, **then** it succeeds without overflow error. | `cargo test -p slicer-host --test region_map_at_cap_tdd 2>&1 | grep -E "at.cap|succeeds|1000.*entries"`

## Verification

- `cargo test -p slicer-host -- resolve_active_regions_o1_contract_tdd region_map_overflow_tdd layer_parallel_safe_false_serialization_tdd catchup_layer_propagation_tdd resolve_active_regions_empty_tdd region_map_at_cap_tdd`
- `cargo clippy --workspace -- -D warnings` must pass before packet completion.

## Authoritative Docs

- `docs/01_system_architecture.md` — WASM instance pool behavior (lines 46-49), Catch-Up Layer Semantics (lines 117-136)
- `docs/02_ir_schemas.md` — LayerPlanIR `ActiveRegion` fields `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` (lines 274-278); `RegionMapIR` structure
- `docs/04_host_scheduler.md` — `resolve_active_regions` O(1) contract (lines 492-510); RegionMapIR Memory Budget Contract with 1000-entry cap (lines 512-530); WASM Host-Call Batching Contract
- `docs/12_architecture_gate_metrics.md` — performance thresholds

## OrcaSlicer Reference Obligations

- None — all four guarantees are internal scheduler contracts with no OrcaSlicer behavioral reference.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`