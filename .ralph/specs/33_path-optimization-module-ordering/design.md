# Design: path-optimization-module-ordering

## Controlling Code Paths

- Algorithm port target: `modules/core-modules/path-optimization-default/src/lib.rs` — `run_path_optimization` body
- Algorithm origin (to be deleted): `crates/slicer-host/src/layer_executor.rs` — `pub fn order_entities_by_nearest_neighbor` and its two call sites in `execute_single_layer`
- Re-export to remove: `crates/slicer-host/src/lib.rs`
- Live test surface (rewritten): `crates/slicer-host/tests/path_ordering_tdd.rs`
- Pattern reference for live WASM dispatch tests: `crates/slicer-host/tests/finalization_live_tdd.rs`
- Packet-32 host helper consumed: `crates/slicer-host/src/dispatch.rs::apply_entity_order_proposal` and the `Layer::PathOptimization` dispatch arm
- Packet-32 SDK builder consumed: `slicer_sdk::LayerCollectionBuilder::set_entity_order`
- Supersession metadata: `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`
- Backlog updates: `docs/07_implementation_status.md`
- WASM rebuild: `./modules/core-modules/build-core-modules.sh`
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`

## Architecture Constraints

- Selected approach: port the packet-18 algorithm verbatim into `path-optimization-default::run_path_optimization`. The module reads the host-staged entity snapshot via `collection.get_ordered_entities()` (one call at the top of the function), computes a `Vec<(u32, bool)>` permutation keyed on `OrderedEntityView::original_index`, and calls `collection.set_entity_order(items)` exactly once. The reversal flag is always `false` in this packet; reversal opt-in is deferred to a future packet that has a concrete use case.
- The host's role shrinks to: (a) assembling raw entities via `assemble_ordered_entities`, (b) pushing the WIT resources, (c) calling the module, (d) running `apply_entity_order_proposal` if a proposal was emitted. With no proposal, the host leaves raw order in place — that is the new fallback behavior and is asserted by `no_module_proposal_leaves_raw_assembled_order`.
- The module's NN computation operates on the **full mixed entity list** the host assembled (perimeters + infill + support), in the **raw `assemble_ordered_entities` order**. Because the read snapshot exposes `original_index` directly, there is no index-space mapping problem: the module produces a permutation over `[0, N)` where `N == ordered_entities.len()` and the host's `apply_entity_order_proposal` validates exactly that bound.
- `regions: &[PerimeterRegionView]` is **not** the algorithm's input. It is left in the function signature unchanged because the existing inter-region travel-retraction logic in `run_path_optimization` (unchanged by this packet) still consumes it. Mixing `regions`-derived indices with `ordered_entities` indices is explicitly forbidden — the algorithm uses `OrderedEntityView::original_index` as its sole index space.
- The packet-18 algorithm details are preserved: start position `(0.0, 0.0)`; Euclidean distance from current position to `view.start_point`; advance current position to `view.end_point` after each pick; equality within 0.001 mm prefers `view.role == ExtrusionRole::BridgeInfill`; further ties go to lower `original_index`; `topo_order` reassigned to the post-permutation slot index by the host's `apply_entity_order_proposal`, not by the module.

## Code Change Surface

- Selected approach:
  - one new private helper inside `path-optimization-default` for the NN computation
  - one call to `collection.get_ordered_entities()` and one call to `collection.set_entity_order` per `run_path_optimization` invocation
  - delete `order_entities_by_nearest_neighbor` and its callers from `layer_executor.rs`
  - rewrite `path_ordering_tdd.rs` so each test loads `path-optimization-default.wasm` and dispatches through `WasmRuntimeDispatcher`
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`:
    - new private `fn nearest_neighbor_permutation(entities: &[OrderedEntityView]) -> Vec<(u32, bool)>` implementing the algorithm. The function uses `view.original_index`, `view.start_point`, `view.end_point`, and `view.role` from the snapshot. Reversal flag is always `false`.
    - extend `fn run_path_optimization` to: (1) call `let snapshot = collection.get_ordered_entities();` once, (2) compute `let items = nearest_neighbor_permutation(snapshot);`, (3) call `collection.set_entity_order(items)?` once. The existing inter-region travel-retraction logic that consumes `regions: &[PerimeterRegionView]` stays unchanged and continues to run after the `set_entity_order` call.
    - early-exit when `snapshot.is_empty()` (no entities — the no-proposal path; matches host fallback semantics)
  - `crates/slicer-host/src/layer_executor.rs`:
    - delete `pub fn order_entities_by_nearest_neighbor`
    - in `execute_single_layer`, change `let ordered_entities = order_entities_by_nearest_neighbor(raw_entities);` to use `raw_entities` directly (in both the pre-PathOptimization staging block and the no-PathOptimization fallback block)
    - remove the `ExtrusionRole` import that becomes unused
  - `crates/slicer-host/src/lib.rs`:
    - remove `order_entities_by_nearest_neighbor` from the `pub use layer_executor::{...}` line
  - `crates/slicer-host/tests/path_ordering_tdd.rs`:
    - rewrite each acceptance test to: (1) build a `Blackboard` with a minimal mesh, (2) build an `ExecutionPlan` with `Layer::Infill` (mock seeds infill IR with the fixture entities) followed by `Layer::PathOptimization` (real `path-optimization-default.wasm` via `WasmRuntimeDispatcher`), (3) run `execute_per_layer`, (4) assert on `LayerCollectionIR.ordered_entities` from the produced layer
    - add `no_module_proposal_leaves_raw_assembled_order`: same fixture as `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` but with a stub `LayerStageRunner` for `Layer::PathOptimization` that returns `Success` without ever pushing a proposal — assert raw-order start-x `[30.0, 0.0, 10.0]` is preserved (NOT the NN-reordered `[0.0, 10.0, 30.0]`)
    - delete the test `reordered_sequence_is_consumed_by_path_optimization_stage` outright (its host-pre-stages-NN contract no longer exists; the live-dispatch tests cover the new contract). Update the file's module-level comment if it still references that test.
    - delete the helper-driven assertions like `order_entities_by_nearest_neighbor(...)` direct calls; everything goes through dispatch
  - `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`:
    - frontmatter `status: implemented` → `status: superseded`
    - add a `## Superseded By` section pointing at `33_path-optimization-module-ordering`
  - `docs/DEVIATION_LOG.md`:
    - add a 2026-04-28 entry titled `path-optimization-module-ordering` summarizing the move and noting that the algorithm itself is unchanged from packet 18
  - `docs/14_deviation_audit_history.md`:
    - cross-link the deviation log entry
  - `docs/07_implementation_status.md`:
    - close `TASK-152g` (status `[x]` with packet-33 close note)
    - close new `TASK-152h` (status `[x]`)
    - leave `TASK-152` as `[~]` (152b/c/f remain open)
- Rejected alternatives:
  - deriving entity indices from `regions: &[PerimeterRegionView]` (walking regions and wall loops in iteration order to recover `assemble_ordered_entities` indices): rejected because (a) `regions` does not carry infill or support entities, so any index derived from it would only cover the perimeter prefix of `ordered_entities`, and (b) tying the algorithm to perimeter-only data forecloses the bridge-priority tiebreak (BridgeInfill is an infill role, not a perimeter role). The packet-32 read accessor `get_ordered_entities` is the right input.
  - keeping `order_entities_by_nearest_neighbor` as a Rust library function in a non-host crate (e.g., `slicer-helpers`) and calling it from the module via FFI: rejected because the algorithm now runs on `OrderedEntityView` (a WIT-bound type) inside the WASM guest, so the host crate is no longer the right owner. The simplest path is a private helper inside `path-optimization-default`.
  - keeping the host fallback alongside the module-side ordering as a "double safety net": rejected because two ordering surfaces silently competing makes regression diagnosis harder. The fallback removal proof (`no_module_proposal_leaves_raw_assembled_order`) is the *contract* this packet asserts.
  - keeping the helper in `layer_executor.rs` "for now in case packet 33 is reverted": rejected — superseding packet 18 is the explicit goal of this packet, and the helper's continued existence would invite accidental fallback re-use.
  - keeping `reordered_sequence_is_consumed_by_path_optimization_stage` as a regression test: rejected. The contract it asserted ("the host's NN ordering is pre-staged and observable from a `Layer::PathOptimization` runner via the arena's `LayerCollectionIR`") is the exact contract this packet removes. Keeping it would either fail (good!) or silently lock in stale behavior.

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

## Read-only Context

These files inform the implementation but must NOT be edited by this packet:

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` — algorithm reference (NN heuristic shape, `chain_segments_closest_point` minus segment reversal)
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — bridge-priority tiebreak reference
- `crates/slicer-host/src/dispatch.rs` — packet-32 host helpers `apply_entity_order_proposal` and `project_ordered_entities`, plus the `Layer::PathOptimization` dispatch arm (consumed unchanged)
- `crates/slicer-sdk` — `LayerCollectionBuilder::set_entity_order`, `LayerCollectionBuilder::get_ordered_entities`, and the `OrderedEntityView` SDK type (signatures only — already landed by packet 32)
- `crates/slicer-host/tests/layer_collection_builder_tdd.rs` — packet-32 validation suite (read for fixture patterns; must remain green after packet 33)
- `crates/slicer-host/tests/finalization_live_tdd.rs` — canonical pattern for live-WASM-dispatch tests (`Blackboard`, `ExecutionPlan`, `WasmRuntimeDispatcher`, `execute_per_layer`)
- `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` — authoritative architecture/IR/scheduler/SDK docs (consulted, not edited)
- `.ralph/specs/32_layer-collection-builder-wit-surface/` — predecessor packet (consulted for surface details, not edited)

## Out-of-bounds Files

This packet must NOT modify any of the following — drift here means the packet's scope has grown and the change belongs to a follow-up packet:

- `wit/deps/ir-types.wit` and any other WIT package files — packet 32 owns the `layer-collection-builder` resource definition (`set-entity-order`, `get-ordered-entities`, `ordered-entity-view`)
- `crates/slicer-host/src/dispatch.rs` — host validation/application logic (`apply_entity_order_proposal`, `project_ordered_entities`, dispatch arm) stays exactly as packet 32 landed it
- `crates/slicer-sdk/` — SDK builder, SDK types, and the `#[slicer_module]` macro plumbing belong to packet 32
- any handler for `Layer::Perimeters`, `Layer::Infill`, or `Layer::Support` — out of scope per `packet.spec.md`
- seam placement, retraction policy, or Z-hop policy code — covered by packets 15 and 23
- `path-optimization-default`'s existing inter-region travel-retraction logic that consumes `regions: &[PerimeterRegionView]` — left structurally unchanged; the NN code added by this packet must not read or mutate that block
- any backlog row in `docs/07_implementation_status.md` other than `TASK-152` (parent), `TASK-152g` (closed), and `TASK-152h` (added + closed)
- packet 18's `requirements.md`, `design.md`, `implementation-plan.md`, or `task-map.md` — only packet 18's `packet.spec.md` is touched (frontmatter flip + new `## Superseded By` section)
- the NN algorithm itself — preserved bit-identically from packet 18; if any byte-level algorithmic change is needed, scope has drifted

## Risks and Tradeoffs

- Risk: the module assumes the snapshot index is `OrderedEntityView::original_index` but accidentally uses the slice index `i` from `iter().enumerate()` after a partial sort. Mitigation: the algorithm builds the proposal entirely off `view.original_index`; do not write code that closes over `i` from a temporarily-sorted slice. The test rewrites pin the exact expected post-dispatch start-x sequences; a mismatch surfaces immediately.
- Risk: removing the host fallback breaks an existing test whose author assumed packet-18 host behavior. Mitigation: the rewrite of `path_ordering_tdd.rs` is exhaustive — every assertion either drives the live module or asserts the new raw-order fallback. Step 5 explicitly deletes the obsolete `reordered_sequence_is_consumed_by_path_optimization_stage` test alongside the host helper. Run the full slicer-host test suite as part of the acceptance ceremony.
- Risk: rebuilding `path-optimization-default.wasm` fails on a contributor's machine without the `wasm32-wasi` target installed. Mitigation: documented in CLAUDE.md (`./modules/core-modules/build-core-modules.sh` requires `wasm32` target). Acceptance ceremony runs the script.
- Risk: the deviation log entry is forgotten. Mitigation: explicit acceptance criterion `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md` blocks the packet otherwise.
- Risk: a future change to the `OrderedEntityView` field set silently changes the algorithm's input. Mitigation: the WIT drift-detection regression (packet 32, Step 9) covers the record's field set; any field-rename or field-removal triggers a test failure in CI before this packet's algorithm can drift.

## Open Questions

- None. The algorithm is preserved verbatim from packet 18, both the read (`get_ordered_entities`) and write (`set_entity_order`) sides of the packet-32 WIT surface are consumed, the obsolete packet-32 host-fallback test is explicitly deleted, and the fallback-removal proof is encoded as an explicit acceptance criterion.
