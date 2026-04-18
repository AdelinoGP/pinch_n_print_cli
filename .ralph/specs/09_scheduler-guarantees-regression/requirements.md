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

1. **`resolve_active_regions` complexity** (docs/04 lines 492-510): Must be O(1) lookup via `module_region_index` map, not a linear rescan over all regions or all global layers. Without a guard, a future code change could silently reintroduce O(n) behavior and degrade per-layer performance below the budget thresholds in docs/12.

2. **RegionMap overflow diagnostics** (docs/04 lines 512-530): The host enforces a 1000-entry cap on RegionMapIR. Without structured overflow coverage, the cap exists but its diagnostics, top-contributor attribution, and remediation hints are absent — making debugging regrowth scenarios difficult.

3. **Instance pool serialization** (docs/01 lines 46-49): Modules with `layer_parallel_safe = false` must serialize WASM instance acquisition across rayon threads. The contract is documented but not tested — a future pool refactor could accidentally parallelize sequential modules.

4. **Catch-up layer field propagation** (docs/01 lines 117-136 + docs/02 lines 274-278): When objects have different layer heights, catch-up layers carry `is_catchup_layer=true`, `catchup_z_bottom`, and `effective_layer_height`. These must survive unchanged through all nine per-layer stages (Slice through PathOptimization). Without a propagation test, a stage could silently drop or overwrite these fields.

If this packet reopens, supersedes, or narrows a prior packet, name the earlier packet and the exact gap it left behind.

This packet does not supersede any prior packet. It is the first guard for these four scheduler guarantees.

## In Scope

- `resolve_active_regions` O(1) contract regression guard (TASK-131)
- RegionMap overflow coverage with 1000-entry cap, top-contributor tuples, and remediation messaging (TASK-132)
- `layer_parallel_safe = false` pool-behavior serialization test (TASK-133)
- Catch-up layer field propagation test across all nine per-layer stages (TASK-134)
- Negative cases: empty-region resolution returns empty slice; exactly-1000-entry RegionMap succeeds without overflow

## Out of Scope

- PrePass segmentation boundary gaps (TASK-128a/128b)
- PostPass WIT gaps (TASK-129a/129b/129c)
- Mesh-query host services (TASK-147/148)
- Benchy parity (TASK-120 series)
- Runtime-budget evidence collection (TASK-156)
- Non-planar Z envelope enforcement (TASK-127) — separate packet
- Claim transition matrix enforcement (TASK-125) — already covered by `claim_transition_matrix_tdd.rs`

## Authoritative Docs

- `docs/01_system_architecture.md` — WASM instance pool behavior (lines 46-49), Catch-Up Layer Semantics (lines 117-136)
- `docs/02_ir_schemas.md` — LayerPlanIR `ActiveRegion` fields `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height` (lines 274-278); `RegionMapIR` structure
- `docs/04_host_scheduler.md` — `resolve_active_regions` O(1) contract (lines 492-510); RegionMapIR Memory Budget Contract (lines 512-530); WASM Host-Call Batching Contract
- `docs/12_architecture_gate_metrics.md` — performance thresholds

## OrcaSlicer Reference Obligations

- None — all four guarantees are internal scheduler contracts with no OrcaSlicer behavioral reference.

## Acceptance Summary

### Positive Cases

- TASK-131: `resolve_active_regions` called with a `GlobalLayer` and `CompiledModule` against a RegionMap with up to 1000 entries completes in O(1) time and does not iterate over all regions or all global layers.
- TASK-132: When a slice job would produce more than 1000 RegionMap entries, the host fails with a fatal error containing the computed entry count, the configured cap (1000), top-contributor tuples `(object_id, region_count, layer_count)`, and a remediation hint.
- TASK-133: When a module with `layer_parallel_safe = false` is used in a multi-threaded context, concurrent layer processing tasks serialize WASM instance acquisition — only one acquisition succeeds at a time.
- TASK-134: A catch-up layer passing through all nine per-layer stages (Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization) preserves `is_catchup_layer=true`, `catchup_z_bottom=B`, and `effective_layer_height=H` unchanged in each stage's output IR.

### Negative Cases

- `resolve_active_regions` with a `module_id` that has no active regions for the given layer returns an empty slice, not an error.
- A RegionMap with exactly 1000 entries (at the cap) succeeds without overflow when the job is otherwise valid.

### Measurable Outcomes

- `resolve_active_regions_o1_contract_tdd`: Test constructs a RegionMap with N entries (N up to 1000) and a `GlobalLayer`/`CompiledModule` pair, calls `resolve_active_regions`, and asserts lookup is via `module_region_index` map get (not iteration). Regex assertion on O(1) pattern.
- `region_map_overflow_tdd`: Test configures a job producing 1001+ entries, runs host startup, and asserts the fatal error contains entry count, cap 1000, top-contributor tuples, and remediation hint.
- `layer_parallel_safe_false_serialization_tdd`: Test uses rayon thread pool, spawns two concurrent layer tasks targeting the same sequential module, and asserts exclusive acquisition via mutex or equivalent serialization primitive.
- `catchup_layer_propagation_tdd`: Test creates a catch-up `GlobalLayer` with `is_catchup_layer=true`, `catchup_z_bottom=0.3`, `effective_layer_height=0.3`, runs it through all nine per-layer stage executors, and asserts each output IR preserves all three fields unchanged.
- `resolve_active_regions_empty_tdd`: Test calls `resolve_active_regions` with a `module_id` that has no active regions and asserts return value is an empty slice.
- `region_map_at_cap_tdd`: Test configures a valid job with exactly 1000 RegionMap entries and asserts it succeeds without overflow error.

## Verification Commands

- `cargo test -p slicer-host --test resolve_active_regions_o1_contract_tdd`
- `cargo test -p slicer-host --test region_map_overflow_tdd`
- `cargo test -p slicer-host --test layer_parallel_safe_false_serialization_tdd`
- `cargo test -p slicer-host --test catchup_layer_propagation_tdd`
- `cargo test -p slicer-host --test resolve_active_regions_empty_tdd`
- `cargo test -p slicer-host --test region_map_at_cap_tdd`
- `cargo clippy --workspace -- -D warnings` (workspace gate before commit)

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: What must be true before the step begins.
- Postcondition: What the step achieves; how to confirm it is done without full integration.
- Falsifying check: The narrowest command or assertion that would fail if the step goal is not met.

Steps do not depend on each other — all four tasks can be implemented and tested independently. The negative cases (empty-resolution, at-cap) can be added in the same step as their positive counterpart or as separate sub-steps at the implementer's discretion.

## Cross-Packet Impact

- TASK-131 unblocks TASK-156 (runtime-budget evidence collection) — the O(1) guard provides the performance baseline needed for budget assertions.
- TASK-132 provides RegionMap overflow evidence for DEV-026 closure in the architecture acceptance gate.
- TASK-133 provides instance-pool concurrency evidence for docs/04 contract documentation.
- TASK-134 guards catch-up layer propagation needed for multi-layer-height object support, which is a prerequisite for Benchy parity on heterogeneous build plates.