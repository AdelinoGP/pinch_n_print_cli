# Requirements: scheduler-contract-regression-guards

## Packet Metadata

- Grouped task IDs:
  - `TASK-131` — Add regression guard for `resolve_active_regions` O(1) contract. Performance guard for runtime-budget evidence.
  - `TASK-132` — Add structured RegionMap overflow coverage for 1000-entry cap with top-contributor and remediation messaging. DEV-026 evidence.
  - `TASK-133` — Add pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Instance-pool contract guard.
  - `TASK-134` — Add catch-up-layer propagation test for `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` across all per-layer stages. Catch-up propagation contract guard.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

These four scheduler contracts lack regression coverage:
1. `resolve_active_regions` must be O(1) but has no guard against O(n) degradation.
2. RegionMap overflow lacks structured diagnostics (entry count, cap, top contributors, remediation).
3. `layer_parallel_safe = false` serialization has no proof.
4. Catch-up layer propagation (`is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height`) has no test that verifies they survive every per-layer stage.

## In Scope

- TASK-131: Add a performance test that creates a large number of regions and measures `resolve_active_regions` call time. Assert it is O(1). Fail if it degrades.
- TASK-132: Add RegionMap overflow test that exceeds the cap and asserts the error contains all required diagnostic fields.
- TASK-133: Add concurrency test that exercises `layer_parallel_safe = false` and proves serialization behavior.
- TASK-134: Add catch-up layer propagation test that passes `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` through all per-layer stages and verifies they survive correctly.

## Out of Scope

- Z-envelope enforcement, prepass segmentation alignment, mesh query services, manifest population, runtime audit — all separate packets.

## Authoritative Docs

- `docs/04_host_scheduler.md` — `resolve_active_regions` Complexity Contract (rows 492–510), RegionMapIR Memory Budget Contract (rows 512–530), WASM Instance Pool, `layer_parallel_safe`, catch-up layer semantics (rows 117–136 in docs/01)

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- `resolve_active_regions` O(1) regression guard in place and passing.
- RegionMap overflow test passes with complete diagnostics.
- `layer_parallel_safe = false` serialization test passes.
- Catch-up layer propagation test passes with all three fields surviving correctly.
- All four regression guards green.

## Verification Commands

- `cargo test --package slicer-host --test resolve_active_regions_o1_contract -- --nocapture`
- `cargo test --package slicer-host --test region_map_overflow_coverage -- --nocapture`
- `cargo test --package slicer-host --test layer_parallel_safe_serialization -- --nocapture`
- `cargo test --package slicer-host --test catch_up_layer_propagation -- --nocapture`