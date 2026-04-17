# Design: scheduler-contract-regression-guards

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/scheduler/` (resolve_active_regions, RegionMap building, instance pool management)
- Neighboring tests or fixtures: `crates/slicer-host/tests/scheduler_contract_guards_tdd.rs` (to be added with four subtests)
- OrcaSlicer comparison surface: None

## Architecture Constraints

- O(1) lookup means `resolve_active_regions` must use a precomputed index (HashMap or similar), not iterate over all regions.
- RegionMap overflow diagnostics require tracking top contributors during the counting pass, not just failing with a number.
- `layer_parallel_safe = false` means the instance pool for that module has size 1 and acquires/releases serially, not concurrently.
- Catch-up layer fields must be propagated through all per-layer stages without being dropped or reset.

## Proposed Changes

### TASK-131 — resolve_active_regions O(1) Regression Guard

1. **Implement performance test**: Create a scenario with many active regions (e.g., 500+ regions). Call `resolve_active_regions` in a loop and measure time per call. Assert per-call time is constant regardless of region count.
2. **Assert O(1) behavior**: Compare average call time for small region count vs. large region count. If times differ significantly, fail the test.

### TASK-132 — RegionMap Overflow Structured Diagnostics

3. **Implement overflow test**: Create a scenario that would exceed the 1000-entry cap. Assert the error contains: computed entry count, configured cap, top contributing `(object_id, region_count, layer_count)` tuples, remediation hint.
4. **Verify top-contributor sorting**: Ensure the diagnostic tuples are sorted by region_count descending (most-contributing first).

### TASK-133 — layer_parallel_safe Serialization Test

5. **Implement concurrency test**: Load a module with `layer_parallel_safe = false`. Exercise concurrent calls to that module. Assert that calls are serialized (only one executes at a time) by checking instance pool state or timing.
6. **Assert pool size 1**: Verify the instance pool for the module has exactly one instance and that acquisition is serialized.

### TASK-134 — Catch-Up Layer Propagation Test

7. **Implement catch-up scenario**: Create two objects with different layer heights (e.g., 0.2mm and 0.3mm) so a catch-up layer is needed at the LCM sync point. Run the full per-layer pipeline.
8. **Assert field survival**: At each per-layer stage, verify `is_catchup_layer = true`, `catchup_z_bottom` has the correct value (bottom of the catch-up range), and `effective_layer_height` matches the catching-up object's layer height.
9. **Cover all per-layer stages**: The test must pass the catch-up layer through every stage (Slice, SlicePostProcess, Perimeters, Infill, Support, PathOptimization).

## Data and Contract Notes

- `resolve_active_regions` must use `blackboard.region_map.module_region_index.get(&(layer.index, module.module_id.clone()))` not a linear scan.
- RegionMap overflow diagnostics minimum: computed entry count, configured cap (1000 default), top contributing tuples (object_id, region_count, layer_count), remediation hint.
- Catch-up layer fields: `is_catchup_layer: bool`, `catchup_z_bottom: f32`, `effective_layer_height: f32` from `ActiveRegion`.

## Risks and Tradeoffs

- O(1) performance test may be sensitive to system load. Run multiple iterations and use median or 90th percentile to avoid false positives.
- Catch-up layer test requires a multi-object scenario. Use the test fixtures if available, or construct a synthetic one.

## Open Questions

- Does the test infrastructure already support multi-object scenarios? Check `crates/slicer-host/tests/` for existing multi-object tests. If not, add them.
- Is the RegionMap overflow path already exercised anywhere in the test suite, or is it completely untested?