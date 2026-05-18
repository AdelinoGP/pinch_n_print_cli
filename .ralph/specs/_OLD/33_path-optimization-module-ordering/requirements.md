# Requirements: path-optimization-module-ordering

## Packet Metadata

- Grouped task IDs:
  - `TASK-152h` — move the deterministic nearest-neighbor entity ordering from `crates/slicer-host/src/layer_executor.rs::order_entities_by_nearest_neighbor` into `modules/core-modules/path-optimization-default/src/lib.rs`, using the `layer-collection-builder` surface introduced by packet 32. Mark packet 18 superseded. Close `TASK-152g` (its surface is now consumed end-to-end).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `18_path-optimization-entity-ordering`

## Problem Statement

Packet 18 placed entity-ordering logic on the host because the WIT surface had no way to express ordering from a guest module. That decision was honest about the WIT constraint of the time and the design.md for packet 18 explicitly chose the host as a stopgap — but it left the architecture inverted: the path-optimization stage's defining responsibility (deciding *which path goes next*) lived outside the path-optimization module. `docs/01_system_architecture.md` notes that the right home for that mutation is the future `layer-collection-builder` resource.

Packet 32 introduces `layer-collection-builder` with two methods: `set-entity-order(items: list<tuple<u32, bool>>)` for emitting a permutation, and `get-ordered-entities() -> list<ordered-entity-view>` for reading the host-staged `LayerCollectionIR.ordered_entities` snapshot (one `OrderedEntityView` per entity, covering perimeter + infill + support). The read accessor is required because the existing `regions: &[PerimeterRegionView]` parameter on `run-path-optimization` only exposes perimeter wall-loops; a `set-entity-order` proposal must carry exactly one entry per existing entity in `ordered_entities`, including infill and support entities that `regions` does not surface. Packet 32 also lands the host-side validation and application logic but keeps the host fallback so the packet-18 acceptance tests stay green during 32's landing window.

This packet finishes the migration. The NN algorithm moves into `path-optimization-default`. The module reads `collection.get_ordered_entities()` to obtain the full mixed entity list (with `original_index`, `region_key`, `role`, `start_point`, `end_point`, `point_count`), computes its proposal, calls `set_entity_order` once with `Vec<(u32, bool)>` (all `false` for now — reversal is supported by the WIT but unused by the default module's NN), and returns. The host applies the validated proposal. `order_entities_by_nearest_neighbor` is deleted from `crates/slicer-host/src/layer_executor.rs`. Without a module-emitted proposal the host now leaves `ordered_entities` in raw `assemble_ordered_entities` order — that is the fallback-removal proof.

The packet-18 acceptance tests are preserved as live end-to-end fixtures: they now drive `path-optimization-default.wasm` through `WasmRuntimeDispatcher` and assert on the `LayerCollectionIR.ordered_entities` produced after dispatch. This validates the round trip: module → WIT boundary (read snapshot) → algorithm → WIT boundary (write proposal) → host validation → application → final IR. The packet-32 host-fallback test `reordered_sequence_is_consumed_by_path_optimization_stage` is deleted because the contract it asserted ("the host pre-stages NN ordering before `Layer::PathOptimization` runs") is removed in this packet; the live-dispatch successor tests cover the new contract.

## In Scope

- port the NN algorithm into `path-optimization-default/src/lib.rs` (start at `(0.0, 0.0)`, Euclidean distance to each entity's `start_point`, BridgeInfill priority within 0.001 mm, lower-`original_index` stable tiebreak). The algorithm reads the entity list from `collection.get_ordered_entities()` (a single call at the top of `run_path_optimization`), iterates over `&[OrderedEntityView]`, and produces `Vec<(u32, bool)>` keyed on `OrderedEntityView::original_index`
- module emits `Vec<(u32, bool)>` with the reversal flag always `false` (placeholder for future packets that may opt into reversal)
- delete `pub fn order_entities_by_nearest_neighbor` from `crates/slicer-host/src/layer_executor.rs`
- delete the helper's two call sites in `execute_single_layer`
- remove the helper's re-export from `crates/slicer-host/src/lib.rs`
- rewrite `crates/slicer-host/tests/path_ordering_tdd.rs` to drive `path-optimization-default.wasm` through real WASM dispatch (mirrors the pattern in `crates/slicer-host/tests/finalization_live_tdd.rs`)
- delete the obsolete test `reordered_sequence_is_consumed_by_path_optimization_stage` (its host-pre-stages-NN contract is removed by this packet)
- add a new test `no_module_proposal_leaves_raw_assembled_order` proving the host fallback is gone
- update `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md` to `status: superseded` with a `## Superseded By` block
- add a deviation log entry in `docs/DEVIATION_LOG.md`
- close `TASK-152g` and `TASK-152h` in `docs/07_implementation_status.md`
- rebuild `path-optimization-default.wasm` via `./modules/core-modules/build-core-modules.sh`

## Out of Scope

- reversal usage — flag stays `false` for now; opting in is a future packet
- tool-change ordering (TASK-152b → packet 19)
- cooling/fan policy decisions (TASK-152c → packet 19)
- finalization travel coordination (TASK-152f → packet 20)
- changes to the host validation logic from packet 32 (it stays exactly as packet 32 landed it)
- any new methods on `layer-collection-builder`
- any change to `Layer::Perimeters`, `Layer::Infill`, `Layer::Support`, seam placement, retraction policy, or z-hop policy

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` — same heuristic preserved on the module side
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — bridge-priority tiebreak preserved

## Acceptance Summary

### Positive Cases

- Same-object NN: live `path-optimization-default.wasm` reorders raw start-x `[0.0, 30.0, 10.0]` to `[0.0, 10.0, 30.0]`.
- Cross-object NN: raw `[A1(0,0), A2(0,100), B1(1,0), B2(1,1)]` produces `object_id` sequence `["A","B","B","A"]`.
- Bridge priority: equidistant BridgeInfill + SparseInfill yields BridgeInfill first.
- Determinism: two repeated dispatches produce byte-identical `ordered_entities`.
- Single / already-optimal sequence: unchanged.
- Module-driven path is end-to-end: tests dispatch through `WasmRuntimeDispatcher`, not via the deleted host helper.

### Negative Cases

- No-module-proposal fallback removal: when no module emits a proposal, `ordered_entities` reflects raw assembly order — not the previous host NN. This is required because this packet changes host behavior in the no-proposal case.

### Measurable Outcomes

- `! grep -RIn "order_entities_by_nearest_neighbor" crates/slicer-host/` returns zero matches (clean delete).
- `! grep -RIn "reordered_sequence_is_consumed_by_path_optimization_stage" crates/slicer-host/tests/` returns zero matches (obsolete packet-32 host-fallback test is gone).
- `modules/core-modules/path-optimization-default/src/lib.rs` invokes `collection.get_ordered_entities()` exactly once and `collection.set_entity_order(` exactly once.
- Each acceptance test asserts on either start-x values, `object_id` sequence, `role` ordering, or byte-equality of two `LayerCollectionIR` instances. No vague "shorter travel" prose.
- `path-optimization-default.wasm` mtime is newer than the start of the packet's CI run.
- `crates/slicer-host/tests/layer_collection_builder_tdd.rs` (introduced in packet 32, including the read-projection tests) still passes — host validation and projection logic is unchanged.

### Cross-Packet Impact

- Packet 18 → `status: superseded` (with `## Superseded By` pointing at this packet). Packet 18's task closures stay closed; the algorithm location moved, the algorithm itself did not.
- Packet 32 stays `implemented`; this packet is the consumer that proves the surface is end-to-end useful.
- Packet 19 (mixed-tool) and packet 21 (Benchy evidence) build on this stable module-side ordering.

## Verification Commands

- `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd no_module_proposal_leaves_raw_assembled_order -- --exact --nocapture`
- `! grep -RIn "order_entities_by_nearest_neighbor" crates/slicer-host/`
- `! grep -RIn "reordered_sequence_is_consumed_by_path_optimization_stage" crates/slicer-host/tests/`
- `grep -c "collection.get_ordered_entities()" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$"`
- `grep -c "collection.set_entity_order(" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$"`
- `grep -E "^status:\\s*superseded" .ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`
- `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md`
- `cargo test -p slicer-host --test layer_collection_builder_tdd 2>&1 | grep "test result: ok"`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the packet-32 surface is in place and the prior step has been verified
- Postcondition: one observable migration milestone (algorithm ported, helper deleted, tests rewired, packet 18 marked superseded)
- Falsifying check: a focused targeted assertion fails if the step's contribution is missing
