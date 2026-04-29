---
status: implemented
packet: path-optimization-module-ordering
task_ids:
  - TASK-152h
backlog_source: docs/07_implementation_status.md
supersedes:
  - 18_path-optimization-entity-ordering
---

# Packet Contract: path-optimization-module-ordering

## Goal

Migrate the deterministic nearest-neighbor entity ordering from the host into `path-optimization-default` using the `layer-collection-builder` surface introduced by packet 32. The module reads the host-staged entity list via `collection.get_ordered_entities()` (one `OrderedEntityView` per `LayerCollectionIR.ordered_entities` entry, covering perimeter + infill + support entities — the existing `regions: &[PerimeterRegionView]` parameter only carries perimeter wall-loops and is insufficient on its own), computes the permutation (with the same NN algorithm, bridge priority, and 0.001 mm tiebreak as packet 18), and emits it through `set-entity-order`. The host then applies the validated proposal — there is no longer a host-side ordering helper. `crates/slicer-host/src/layer_executor.rs` loses `order_entities_by_nearest_neighbor` and its call sites; the host's only contribution is to assemble entities in raw IR order via `assemble_ordered_entities`, leaving any reordering to the path-optimization module. Packet 18 is marked `status: superseded`. The packet-18 acceptance assertions are preserved by replacing the host-helper-driven fixtures in `crates/slicer-host/tests/path_ordering_tdd.rs` with end-to-end fixtures that drive `path-optimization-default.wasm` through real WASM dispatch.

## Scope Boundaries

- In scope:
  - port the nearest-neighbor algorithm (bridge priority + 0.001 mm tiebreak + lower-original-index stable tiebreak) from `crates/slicer-host/src/layer_executor.rs` into `modules/core-modules/path-optimization-default/src/lib.rs`
  - module reads the host-staged entity snapshot via `collection.get_ordered_entities()` (returns `&[OrderedEntityView]` with `original_index`, `region_key`, `role`, `start_point`, `end_point`, `point_count`) and computes the permutation from that snapshot plus a fresh start position of `(0.0, 0.0)`. The existing `regions: &[PerimeterRegionView]` parameter is left untouched for the algorithm — it is still needed by the existing inter-region travel-retraction logic in the same module
  - module calls `collection.set_entity_order(items)` with the computed `Vec<(u32, bool)>` (reversal flag remains `false` for every entry in this packet — reversal is supported by the WIT but unused by the default module's NN)
  - delete `pub fn order_entities_by_nearest_neighbor` and its imports from `crates/slicer-host/src/layer_executor.rs`
  - delete the helper's call sites: the pre-PathOptimization staging block and the post-loop fallback block both fall back to raw `assemble_ordered_entities` order
  - update `crates/slicer-host/src/lib.rs` to remove the `order_entities_by_nearest_neighbor` re-export
  - rewrite `crates/slicer-host/tests/path_ordering_tdd.rs` so each acceptance test drives `path-optimization-default.wasm` through `WasmRuntimeDispatcher` instead of calling the deleted host helper
  - delete the existing test `reordered_sequence_is_consumed_by_path_optimization_stage` (its contract — "host pre-stages NN order before PathOptimization runs" — is removed by this packet; its successor is the new `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` live-dispatch test plus the new `no_module_proposal_leaves_raw_assembled_order` fallback-removal proof)
  - mark `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md` `status: superseded` and document the move in `docs/DEVIATION_LOG.md`
  - rebuild `path-optimization-default.wasm` via `./modules/core-modules/build-core-modules.sh`
  - update `docs/07_implementation_status.md`: close `TASK-152g` (its packet-32 surface is now actually consumed) and close the new `TASK-152h` (this packet's migration); leave `TASK-152` `[~]` since 152b/c/f remain
- Out of scope:
  - any reversal use case (the flag stays `false` in this packet's algorithm; downstream packets can opt in)
  - tool-change ordering (TASK-152b → packet 19), cooling overrides (TASK-152c → packet 19), finalization travel coordination (TASK-152f → packet 20)
  - any new methods on `layer-collection-builder` (only `set-entity-order` is used)
  - changes to the host validation logic introduced in packet 32 (it stays exactly as packet 32 landed it)
  - changes to seam placement, retraction, or z-hop policy (covered by packets 15 and 23)

## Prerequisites and Blockers

- Depends on:
  - packet 32 (`32_layer-collection-builder-wit-surface`) — provides the `layer-collection-builder` resource (both `set_entity_order` and `get_ordered_entities`), the `OrderedEntityView` SDK type, host validation, the `project_ordered_entities` host helper, dispatch wiring, SDK builder, and macro plumbing this packet relies on
  - packet 18 (`18_path-optimization-entity-ordering`) — the algorithm being ported originated here
- Unblocks:
  - packet 19 (mixed-tool ordering) — stable module-side ordering is now the foundation for tool-change sequencing
  - packet 21 (Benchy evidence beyond comment markers) — module-driven ordering is observable end-to-end
- Activation blockers:
  - packet 32 must be `implemented` before this packet activates. If packet 32 is still `draft`, this packet stays `draft`.

## Acceptance Criteria

- **Given** the canonical `path-optimization-default.wasm` driven through `WasmRuntimeDispatcher::run_stage` for a layer with raw assembled entity start-x sequence `[0.0, 30.0, 10.0]` (all same `region_key.object_id`), **when** the module's `run_path_optimization` returns and the host applies the captured proposal, **then** `LayerCollectionIR.ordered_entities` start-x sequence is exactly `[0.0, 10.0, 30.0]` and `topo_order` values are `[0, 1, 2]`. | `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- **Given** a 4-entity mixed-object layer whose raw start points are `A1(0,0), A2(0,100), B1(1,0), B2(1,1)`, **when** the live `path-optimization-default.wasm` dispatches and the host applies the proposal, **then** `LayerCollectionIR.ordered_entities[*].region_key.object_id` is exactly `["A","B","B","A"]`. | `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- **Given** one `ExtrusionRole::BridgeInfill` and one `ExtrusionRole::SparseInfill` entity whose `path.points[0]` are equidistant within 0.001 mm of the start position, **when** the live `path-optimization-default.wasm` dispatches, **then** the resulting `ordered_entities[0].role` is `ExtrusionRole::BridgeInfill` and `ordered_entities[1].role` is `ExtrusionRole::SparseInfill`. | `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- **Given** an identical layer fixture executed twice through the live module, **when** both runs complete, **then** the `LayerCollectionIR.ordered_entities` from each run are byte-identical (`assert_eq!(run1, run2)`). | `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
- **Given** a 1-entity layer or an already-NN-optimal 3-entity sequence, **when** the live `path-optimization-default.wasm` dispatches, **then** `ordered_entities` is unchanged from raw assembly order (start-x sequence `[0.0, 10.0, 30.0]` for the 3-entity case; single entity untouched). | `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- **Given** a layer whose `Layer::PathOptimization` runs no module that emits `set-entity-order` (e.g., the runner skips path-optimization entirely or runs a stub guest that does not call the builder), **when** the layer finalizes, **then** `LayerCollectionIR.ordered_entities` matches the raw `assemble_ordered_entities` order (start-x sequence `[30.0, 0.0, 10.0]` for the packet-18 fixture, NOT the NN-reordered sequence). This proves the host fallback has been removed and the host now leaves entities untouched without an emitted proposal. | `cargo test -p slicer-host --test path_ordering_tdd no_module_proposal_leaves_raw_assembled_order -- --exact --nocapture`
- **Given** `modules/core-modules/path-optimization-default/src/lib.rs` after this packet's edits, **when** the source is grepped, **then** the module body invokes `collection.get_ordered_entities()` exactly once (proving the algorithm reads the full mixed entity list, not just `regions`) and `collection.set_entity_order(` exactly once. | `grep -c "collection.get_ordered_entities()" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$" && grep -c "collection.set_entity_order(" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$"`
- **Given** the slicer-host crate after this packet's edits, **when** the source is grepped for the deleted helper, **then** zero matches are returned. | `! grep -RIn "order_entities_by_nearest_neighbor" crates/slicer-host/`
- **Given** the slicer-host test crate after this packet's edits, **when** the source is grepped, **then** the obsolete packet-32 host-fallback test is no longer present. | `! grep -RIn "reordered_sequence_is_consumed_by_path_optimization_stage" crates/slicer-host/tests/`
- **Given** `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`, **when** the file is read, **then** its frontmatter has `status: superseded` and a top-level `## Superseded By` section names `33_path-optimization-module-ordering`. | `grep -E "^status:\\s*superseded" .ralph/specs/18_path-optimization-entity-ordering/packet.spec.md && grep -E "33_path-optimization-module-ordering" .ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`
- **Given** `docs/DEVIATION_LOG.md`, **when** the file is read, **then** it contains a 2026-* entry referencing `path-optimization-module-ordering` and explaining the move from host helper to module-side ordering. | `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md`

## Negative Test Cases

This packet changes host behavior in the no-module-proposal case (the previous host-side NN fallback in `order_entities_by_nearest_neighbor` is deleted), so the activation-gate's negative-criterion requirement applies. The gate-required negative criterion is **acceptance criterion #6** (`no_module_proposal_leaves_raw_assembled_order`) — it is the *rejection* criterion, not just a positive assertion: with no module proposal emitted, the host must NOT silently re-introduce NN ordering, and `LayerCollectionIR.ordered_entities` MUST stay in raw `assemble_ordered_entities` order. The negative case is therefore not duplicated here — see acceptance criterion #6 above for the full Given/When/Then and run command (`cargo test -p slicer-host --test path_ordering_tdd no_module_proposal_leaves_raw_assembled_order -- --exact --nocapture`).

## Verification

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
- `cargo test -p slicer-host --test layer_collection_builder_tdd 2>&1 | grep "test result: ok"`  (packet-32 host validation tests must still pass)
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Authoritative Docs

- `docs/01_system_architecture.md` — entity-ordering ownership now sits in the path-optimization module; host falls back to raw assembly order
- `docs/02_ir_schemas.md` — `LayerCollectionIR.ordered_entities`, `PrintEntity.topo_order`, `OrderedEntityView`, `PerimeterRegionView`
- `docs/04_host_scheduler.md` — PathOptimization stage scheduling (unchanged structurally; only the source of ordering moves)
- `docs/05_module_sdk.md` — SDK builder usage (`LayerCollectionBuilder::get_ordered_entities`, `LayerCollectionBuilder::set_entity_order`, `OrderedEntityView`)
- `docs/07_implementation_status.md` — close `TASK-152g`, close new `TASK-152h`
- `docs/14_deviation_audit_history.md`, `docs/DEVIATION_LOG.md` — record the supersession

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` — same NN heuristic shape carried over; module's algorithm is a Rust port of `chain_segments_closest_point` minus segment reversal
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — bridge-priority tiebreak preserved from packet 18

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
