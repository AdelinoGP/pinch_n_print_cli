# Implementation Plan: scheduler-contract-regression-guards

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Implement resolve_active_regions O(1) regression guard

- Task IDs:
  - `TASK-131`
- Objective: Add a performance test that verifies `resolve_active_regions` completes in O(1) time. Measure call time for small and large region counts and assert constant time.
- Files expected to change:
  - `crates/slicer-host/tests/scheduler_contract_guards_tdd.rs` (new or extended)
- Authoritative docs:
  - `docs/04_host_scheduler.md` — `resolve_active_regions` Complexity Contract (rows 492–510)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test resolve_active_regions_o1_contract -- --nocapture`

### Step 2: Implement RegionMap overflow structured diagnostics test

- Task IDs:
  - `TASK-132`
- Objective: Add a test that exceeds the RegionMap 1000-entry cap and asserts the error contains: computed entry count, configured cap, top contributing tuples, and remediation hint.
- Files expected to change:
  - `crates/slicer-host/tests/scheduler_contract_guards_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — RegionMapIR Memory Budget Contract (rows 512–530)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test region_map_overflow_coverage -- --nocapture`

### Step 3: Implement layer_parallel_safe serialization test

- Task IDs:
  - `TASK-133`
- Objective: Add a concurrency test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Verify pool size is 1 and acquisition is serialized.
- Files expected to change:
  - `crates/slicer-host/tests/scheduler_contract_guards_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — WASM Instance Pool behavior, `layer_parallel_safe` semantics
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test layer_parallel_safe_serialization -- --nocapture`

### Step 4: Implement catch-up layer propagation test

- Task IDs:
  - `TASK-134`
- Objective: Add a test that creates a catch-up layer scenario (two objects with different layer heights), runs the full per-layer pipeline, and asserts `is_catchup_layer`, `catchup_z_bottom`, and `effective_layer_height` survive through every per-layer stage.
- Files expected to change:
  - `crates/slicer-host/tests/scheduler_contract_guards_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — catch-up layer semantics
  - `docs/01_system_architecture.md` — Catch-Up Layer Semantics (rows 117–136)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test catch_up_layer_propagation -- --nocapture`

### Step 5: Full test suite verification

- Task IDs:
  - `TASK-131`
  - `TASK-132`
  - `TASK-133`
  - `TASK-134`
- Objective: Run all four regression guard tests and confirm they all pass.
- Files expected to change: None (verification only)
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test scheduler_contract_guards_tdd -- --nocapture` — all four subtests pass.

## Packet Completion Gate

- `resolve_active_regions` O(1) regression guard in place and passing.
- RegionMap overflow test passes with complete diagnostics.
- `layer_parallel_safe = false` serialization test passes.
- Catch-up layer propagation test passes with all three fields surviving.
- All four regression guards green.
- `docs/07_implementation_status.md` TASK-131/132/133/134 marked complete.
- `packet.spec.md` ready to move to `status: implemented`.