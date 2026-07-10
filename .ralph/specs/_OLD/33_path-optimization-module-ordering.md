---
status: implemented
packet: path-optimization-module-ordering
task_ids:
  - TASK-152h
supersedes:
  - 18_path-optimization-entity-ordering
---

# 33_path-optimization-module-ordering

## Goal

Migrate the deterministic nearest-neighbor entity ordering from the host into `path-optimization-default` using the `layer-collection-builder` surface introduced by packet 32. The module reads the host-staged entity list via `collection.get_ordered_entities()` (one `OrderedEntityView` per `LayerCollectionIR.ordered_entities` entry, covering perimeter + infill + support entities — the existing `regions: &[PerimeterRegionView]` parameter only carries perimeter wall-loops and is insufficient on its own), computes the permutation (with the same NN algorithm, bridge priority, and 0.001 mm tiebreak as packet 18), and emits it through `set-entity-order`. The host then applies the validated proposal — there is no longer a host-side ordering helper. `crates/slicer-host/src/layer_executor.rs` loses `order_entities_by_nearest_neighbor` and its call sites; the host's only contribution is to assemble entities in raw IR order via `assemble_ordered_entities`, leaving any reordering to the path-optimization module. Packet 18 is marked `status: superseded`. The packet-18 acceptance assertions are preserved by replacing the host-helper-driven fixtures in `crates/slicer-host/tests/path_ordering_tdd.rs` with end-to-end fixtures that drive `path-optimization-default.wasm` through real WASM dispatch.

## Problem Statement

Packet 18 placed entity-ordering logic on the host because the WIT surface had no way to express ordering from a guest module. That decision was honest about the WIT constraint of the time and the design.md for packet 18 explicitly chose the host as a stopgap — but it left the architecture inverted: the path-optimization stage's defining responsibility (deciding *which path goes next*) lived outside the path-optimization module. `docs/01_system_architecture.md` notes that the right home for that mutation is the future `layer-collection-builder` resource.

Packet 32 introduces `layer-collection-builder` with two methods: `set-entity-order(items: list<tuple<u32, bool>>)` for emitting a permutation, and `get-ordered-entities() -> list<ordered-entity-view>` for reading the host-staged `LayerCollectionIR.ordered_entities` snapshot (one `OrderedEntityView` per entity, covering perimeter + infill + support). The read accessor is required because the existing `regions: &[PerimeterRegionView]` parameter on `run-path-optimization` only exposes perimeter wall-loops; a `set-entity-order` proposal must carry exactly one entry per existing entity in `ordered_entities`, including infill and support entities that `regions` does not surface. Packet 32 also lands the host-side validation and application logic but keeps the host fallback so the packet-18 acceptance tests stay green during 32's landing window.

This packet finishes the migration. The NN algorithm moves into `path-optimization-default`. The module reads `collection.get_ordered_entities()` to obtain the full mixed entity list (with `original_index`, `region_key`, `role`, `start_point`, `end_point`, `point_count`), computes its proposal, calls `set_entity_order` once with `Vec<(u32, bool)>` (all `false` for now — reversal is supported by the WIT but unused by the default module's NN), and returns. The host applies the validated proposal. `order_entities_by_nearest_neighbor` is deleted from `crates/slicer-host/src/layer_executor.rs`. Without a module-emitted proposal the host now leaves `ordered_entities` in raw `assemble_ordered_entities` order — that is the fallback-removal proof.

The packet-18 acceptance tests are preserved as live end-to-end fixtures: they now drive `path-optimization-default.wasm` through `WasmRuntimeDispatcher` and assert on the `LayerCollectionIR.ordered_entities` produced after dispatch. This validates the round trip: module → WIT boundary (read snapshot) → algorithm → WIT boundary (write proposal) → host validation → application → final IR. The packet-32 host-fallback test `reordered_sequence_is_consumed_by_path_optimization_stage` is deleted because the contract it asserted ("the host pre-stages NN ordering before `Layer::PathOptimization` runs") is removed in this packet; the live-dispatch successor tests cover the new contract.

## Architecture Constraints

- Selected approach: port the packet-18 algorithm verbatim into `path-optimization-default::run_path_optimization`. The module reads the host-staged entity snapshot via `collection.get_ordered_entities()` (one call at the top of the function), computes a `Vec<(u32, bool)>` permutation keyed on `OrderedEntityView::original_index`, and calls `collection.set_entity_order(items)` exactly once. The reversal flag is always `false` in this packet; reversal opt-in is deferred to a future packet that has a concrete use case.
- The host's role shrinks to: (a) assembling raw entities via `assemble_ordered_entities`, (b) pushing the WIT resources, (c) calling the module, (d) running `apply_entity_order_proposal` if a proposal was emitted. With no proposal, the host leaves raw order in place — that is the new fallback behavior and is asserted by `no_module_proposal_leaves_raw_assembled_order`.
- The module's NN computation operates on the **full mixed entity list** the host assembled (perimeters + infill + support), in the **raw `assemble_ordered_entities` order**. Because the read snapshot exposes `original_index` directly, there is no index-space mapping problem: the module produces a permutation over `[0, N)` where `N == ordered_entities.len()` and the host's `apply_entity_order_proposal` validates exactly that bound.
- `regions: &[PerimeterRegionView]` is **not** the algorithm's input. It is left in the function signature unchanged because the existing inter-region travel-retraction logic in `run_path_optimization` (unchanged by this packet) still consumes it. Mixing `regions`-derived indices with `ordered_entities` indices is explicitly forbidden — the algorithm uses `OrderedEntityView::original_index` as its sole index space.
- The packet-18 algorithm details are preserved: start position `(0.0, 0.0)`; Euclidean distance from current position to `view.start_point`; advance current position to `view.end_point` after each pick; equality within 0.001 mm prefers `view.role == ExtrusionRole::BridgeInfill`; further ties go to lower `original_index`; `topo_order` reassigned to the post-permutation slot index by the host's `apply_entity_order_proposal`, not by the module.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities` — populated post-call by `apply_entity_order_proposal` (packet 32 helper) when the module emits a proposal; otherwise reflects raw `assemble_ordered_entities` order
  - `PrintEntity.topo_order` — reassigned to the post-permutation 0-based slot
- WIT boundary considerations:
  - none new — packet 32 already landed the WIT resource and parameter
- Determinism / scheduler constraints:
  - the module's NN must be deterministic (same input → same `Vec<(u32, bool)>`); the algorithm preserves packet-18's stable tiebreak
  - the host's `apply_entity_order_proposal` is deterministic by construction (validation + `Vec::reverse()` are deterministic)

## Locked Assumptions and Invariants

- The module's algorithm input is `&[OrderedEntityView]` returned by `collection.get_ordered_entities()`. Each view's `original_index` is the entity's slot in `LayerCollectionIR.ordered_entities` at the moment the module entered `run_path_optimization`. The proposal `Vec<(u32, bool)>` indexes into that same space; `apply_entity_order_proposal` validates length / range / uniqueness against `ordered_entities.len()`.
- The view snapshot is a *flat* projection: it carries `start_point`, `end_point`, `role`, `region_key`, and `point_count` only — not the full `path.points` list. Algorithms that need richer geometry are out of scope for this packet (a future packet can add an explicit accessor).
- `regions: &[PerimeterRegionView]` is left in `run_path_optimization`'s signature for the existing inter-region travel-retraction logic that already consumes it. The NN algorithm in this packet **does not** read `regions`; mixing `regions`-derived indices with `OrderedEntityView::original_index` is forbidden.
- Test fixtures (carried over from packet 18) build entities from `InfillIR.sparse_infill` (and similar). This is correct: the entities reach `LayerCollectionIR.ordered_entities` via the host's `assemble_ordered_entities` pipeline, the read accessor surfaces them as `OrderedEntityView`, and the module sees them with the correct `role` and `region_key`. No fixture rewrite is required to put entities in the perimeter wall-loop path.
- The reversal flag stays `false` for every entry. This is a future-proofing affordance — packet 32's WIT carries it, but no packet-33 test sets it true.
- Once `order_entities_by_nearest_neighbor` is deleted, no new code in `crates/slicer-host/` may reintroduce host-side ordering.

## Risks and Tradeoffs

- Risk: the module assumes the snapshot index is `OrderedEntityView::original_index` but accidentally uses the slice index `i` from `iter().enumerate()` after a partial sort. Mitigation: the algorithm builds the proposal entirely off `view.original_index`; do not write code that closes over `i` from a temporarily-sorted slice. The test rewrites pin the exact expected post-dispatch start-x sequences; a mismatch surfaces immediately.
- Risk: removing the host fallback breaks an existing test whose author assumed packet-18 host behavior. Mitigation: the rewrite of `path_ordering_tdd.rs` is exhaustive — every assertion either drives the live module or asserts the new raw-order fallback. Step 5 explicitly deletes the obsolete `reordered_sequence_is_consumed_by_path_optimization_stage` test alongside the host helper. Run the full slicer-host test suite as part of the acceptance ceremony.
- Risk: rebuilding `path-optimization-default.wasm` fails on a contributor's machine without the `wasm32-wasi` target installed. Mitigation: documented in CLAUDE.md (`./modules/core-modules/build-core-modules.sh` requires `wasm32` target). Acceptance ceremony runs the script.
- Risk: the deviation log entry is forgotten. Mitigation: explicit acceptance criterion `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md` blocks the packet otherwise.
- Risk: a future change to the `OrderedEntityView` field set silently changes the algorithm's input. Mitigation: the WIT drift-detection regression (packet 32, Step 9) covers the record's field set; any field-rename or field-removal triggers a test failure in CI before this packet's algorithm can drift.
