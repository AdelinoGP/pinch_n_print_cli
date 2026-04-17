---
status: draft
packet: scheduler-contract-regression-guards
task_ids:
  - TASK-131
  - TASK-132
  - TASK-133
  - TASK-134
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: scheduler-contract-regression-guards

## Goal

Add regression guards for four critical scheduler contracts: the O(1) `resolve_active_regions` contract (performance guard for runtime-budget evidence), RegionMap overflow coverage for the 1000-entry cap with top-contributor and remediation messaging, `layer_parallel_safe = false` serialization proof, and catch-up-layer propagation test proving `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive every per-layer stage.

## Scope Boundaries

- In scope:
  - TASK-131: Add a regression guard for the documented `resolve_active_regions` O(1) contract. Scheduler performance guard needed for runtime-budget evidence.
  - TASK-132: Add structured RegionMap overflow coverage for the 1000-entry cap, including top-contributor and remediation messaging. Hardens the existing bounds path needed for DEV-026 evidence.
  - TASK-133: Add a pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Scheduler concurrency guard for the docs/04 instance-pool contract.
  - TASK-134: Add a catch-up-layer propagation test that verifies `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive every per-layer stage. Guards the documented catch-up-layer propagation contract across every per-layer stage.

- Out of scope:
  - Z-envelope enforcement (TASK-127 — separate packet)
  - Prepass segmentation alignment (TASK-128 series — separate packet)
  - Mesh query host services (TASK-147/148 — separate packet)
  - Manifest population / runtime audit (Workstream 1)

## Acceptance Criteria

- **Given** the scheduler implementation, **when** `resolve_active_regions` is called, **then** it completes in O(1) time (constant-time lookup) regardless of the number of active regions, and the regression guard fails if the implementation degrades to O(n).
- **Given** a `RegionMapIR` entry count that would exceed the 1000-entry cap, **when** the cap is exceeded, **then** the host returns a fatal planning error with computed entry count, configured cap, top contributing `(object_id, region_count, layer_count)` tuples, and remediation hint.
- **Given** a module with `layer_parallel_safe = false`, **when** the scheduler handles concurrent layer requests, **then** the module's WASM instance is serialized (only one instance acquired at a time) and the concurrency behavior matches the instance-pool contract in docs/04.
- **Given** a catch-up layer scenario with multiple objects of different layer heights, **when** each per-layer stage processes the catch-up layer, **then** `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` retain their correct values through all stages.
- **Given** the four regression guards, **when** they run, **then** all four tests pass.

## Verification

- `cargo test --package slicer-host --test resolve_active_regions_o1_contract -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test region_map_overflow_coverage -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test layer_parallel_safe_serialization -- --nocapture` (test to be added)
- `cargo test --package slicer-host --test catch_up_layer_propagation -- --nocapture` (test to be added)

## Authoritative Docs

- `docs/04_host_scheduler.md` — `resolve_active_regions` Complexity Contract (rows 492–510), RegionMapIR Memory Budget Contract (rows 512–530), WASM Instance Pool behavior, `layer_parallel_safe` semantics, catch-up layer semantics

## OrcaSlicer Reference Obligations

None. This is a scheduler contract verification task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`