---
status: implemented
packet: scheduler-guarantees-regression
task_ids:
  - TASK-131
  - TASK-132
  - TASK-133
  - TASK-134
---

# 09_scheduler-guarantees-regression

## Goal

Close the gap between the documented scheduler guarantees and the current host implementation by materializing the missing host-side `(global_layer_index, module_id)` active-region lookup surface, enriching RegionMap overflow failures with structured contributor/remediation diagnostics, locking serialized pool acquisition on the canonical instance-pool surface, and adding catch-up metadata regression coverage on the host and IR surfaces that actually carry it during per-layer execution.

## Problem Statement

The scheduler makes four behavioral contracts that are documented but not regression-guarded:

1. **`resolve_active_regions` complexity** (docs/04 `resolve_active_regions` contract): The docs promise an O(1) lookup by `(global_layer_index, module_id)`, but the current host only freezes `ExecutionPlan.region_plans` / `RegionMapIR.entries` and does not materialize the documented host-side lookup surface. This packet must add the missing host-side index/helper and lock it with direct tests instead of assuming the function already exists.

2. **RegionMap overflow diagnostics** (docs/04 RegionMapIR Memory Budget Contract): The host already enforces a 1000-entry cap, but the current structured errors only carry the count/cap. The documented top-contributor tuples and remediation hints are still missing on the real region-mapping / execution-plan startup paths.

3. **Instance pool serialization** (docs/01 WASM instance pool contract): `src/instance_pool.rs` already forces non-parallel-safe modules into a serialized single-slot pool, and existing tests cover the basic blocking lease behavior. This packet keeps the backlog slice honest by tightening the canonical scheduler-contract coverage on that existing surface rather than inventing a second pool abstraction.

4. **Catch-up metadata propagation** (docs/01 Catch-Up Layer Semantics + docs/02 `ActiveRegion`): Catch-up state lives on `ActiveRegion`, while downstream per-layer IRs only guarantee `effective_layer_height`. The packet must therefore guard the real contract: every stage must observe unchanged source `ActiveRegion.is_catchup_layer` / `catchup_z_bottom`, and every downstream IR surface that actually defines `effective_layer_height` must preserve that value unchanged.

If this packet reopens, supersedes, or narrows a prior packet, name the earlier packet and the exact gap it left behind.

This packet does not supersede any prior packet. It is the first guard for these four scheduler guarantees.

## Architecture Constraints

- The O(1) lookup must come from a precomputed host-side index built once at startup. Any implementation that filters all `global_layers`, all `RegionMapIR.entries`, or all `ExecutionPlan.region_plans` on each call is non-compliant with docs/04.
- Selected approach for TASK-131: add a derived host-only lookup table to `ExecutionPlan` keyed by `(global_layer_index, module_id)` and expose a canonical `resolve_active_regions` helper on that surface. Do not widen `RegionMapIR` in `slicer-ir` for this packet.
- RegionMap overflow must be caught at startup (planning / region-mapping time), not lazily at per-layer execution, per docs/04 Memory Budget Contract.
- Selected approach for TASK-132: enrich the startup error shape with contributor tuples and remediation text on the real `region_mapping.rs` / `execution_plan.rs` paths. If both paths can fail, they must share one diagnostics shape or one formatting helper so the emitted fields stay identical.
- Instance pool serialization must remain enforced at the canonical pool level through `InstancePoolMode::Serialized` and `SlotAvailability` (`Mutex<SlotAvailabilityState>` + `Condvar`). Do not introduce a second concurrency primitive or a fake test-only pool implementation.
- Catch-up metadata is defined on `ActiveRegion`; downstream per-layer IRs only guarantee `effective_layer_height`. Tests must therefore assert `is_catchup_layer` / `catchup_z_bottom` on the source `GlobalLayer.active_regions` surface across all nine stages, and assert `effective_layer_height` only on the IR types that actually define it.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `ExecutionPlan` host runtime state — gains the precomputed module-region lookup surface for TASK-131
  - `RegionMapIR` / `region_plans` startup budget cap at 1000 entries
  - `RegionMappingError` / `ExecutionPlanError` contributor/remediation diagnostics
  - `ActiveRegion` — `is_catchup_layer`, `catchup_z_bottom`, `effective_layer_height`
  - `SlicedRegion.effective_layer_height` — downstream IR field that must preserve the source height
  - `layer_parallel_safe` manifest field (docs/03) — gates pool behavior

- WIT boundary considerations: No WIT schema change is planned. If Step 4 uncovers that the typed layer dispatch path is the controlling metadata surface, keep any fix narrowly inside host-side dispatch/context wiring and do not widen the world definitions in this packet.

- Determinism or scheduler constraints: the module-region lookup must be deterministic — the same `(layer, module)` pair always returns the same region ordering; overflow contributor ordering must also be stable enough for exact test assertions.

## Locked Assumptions and Invariants

- `ExecutionPlan.region_plans` remains the canonical per-region plan storage; the new module-region lookup is derived once during startup and never mutated during per-layer execution.
- RegionMap overflow is a fatal planning error (not a warning or a retry). The test must assert the host terminates with an error, not just logs a warning.
- Sequential module pools use one serialized slot and block contenders until release. The backlog slice should keep proving that canonical behavior rather than re-specifying the pool abstraction.
- Catch-up metadata originates in `PrePass::LayerPlanning` and is stored on `GlobalLayer.active_regions`. Each stage executor receives the source `GlobalLayer`; downstream IRs only need to preserve the fields they actually define.

## Risks and Tradeoffs

- Risk: TASK-131 may expose that no runtime consumer currently calls the lookup helper. Mitigation: land the host-side lookup and direct tests first, then wire one canonical consumer only if the task still lacks an executable proof.
- Risk: TASK-132 touches the same startup surfaces as TASK-131. Mitigation: keep Steps 1 and 2 serialized and reuse one diagnostics helper/shape rather than forking the startup error contract.
- Risk: TASK-133 contention coverage could still be timing-sensitive on CI. Mitigation: keep the existing blocking-lease test shape and only strengthen it with deterministic coordination primitives already used in the test suite.
- Risk: TASK-134 can become brittle if it asserts non-existent fields on downstream IRs. Mitigation: keep the assertions on `GlobalLayer.active_regions` for catch-up flags and on `effective_layer_height` only for the IR types that declare it.
