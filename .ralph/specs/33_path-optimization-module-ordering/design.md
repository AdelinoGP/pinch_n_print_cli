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

- Selected approach: port the packet-18 algorithm verbatim into `path-optimization-default::run_path_optimization`. The module reads its `regions: &[PerimeterRegionView]` slice, computes a `Vec<(u32, bool)>` permutation, and calls `collection.set_entity_order(items)` exactly once. The reversal flag is always `false` in this packet; reversal opt-in is deferred to a future packet that has a concrete use case.
- The host's role shrinks to: (a) assembling raw entities via `assemble_ordered_entities`, (b) pushing the WIT resources, (c) calling the module, (d) running `apply_entity_order_proposal` if a proposal was emitted. With no proposal, the host leaves raw order in place — that is the new fallback behavior and is asserted by `no_module_proposal_leaves_raw_assembled_order`.
- The module's NN computation must operate on the **same logical entity set** the host assembled, in the **same raw order**. The module receives `regions: &[PerimeterRegionView]`, so for now the algorithm walks regions and wall loops in iteration order to derive an entity index that matches the host's `assemble_ordered_entities` ordering. The mapping between the module's "region/loop" iteration and the host's `ordered_entities` index must be stable.
- The packet-18 algorithm details are preserved: start position `(0.0, 0.0)`; Euclidean distance from current position to `path.points[0]`; advance to `path.points.last()` after each pick; equality within 0.001 mm prefers `BridgeInfill`; further ties go to lower original index; `topo_order` reassigned to the post-permutation slot index.

## Code Change Surface

- Selected approach:
  - one new private helper inside `path-optimization-default` for the NN computation
  - one call to `collection.set_entity_order` per `run_path_optimization` invocation
  - delete `order_entities_by_nearest_neighbor` and its callers from `layer_executor.rs`
  - rewrite `path_ordering_tdd.rs` so each test loads `path-optimization-default.wasm` and dispatches through `WasmRuntimeDispatcher`
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`:
    - new private `fn nearest_neighbor_permutation(regions: &[PerimeterRegionView]) -> Vec<(u32, bool)>` implementing the algorithm
    - extend `fn run_path_optimization` to call the helper, then `collection.set_entity_order(items)?`. The existing inter-region travel-retraction logic stays unchanged.
  - `crates/slicer-host/src/layer_executor.rs`:
    - delete `pub fn order_entities_by_nearest_neighbor`
    - in `execute_single_layer`, change `let ordered_entities = order_entities_by_nearest_neighbor(raw_entities);` to use `raw_entities` directly (in both the pre-PathOptimization staging block and the no-PathOptimization fallback block)
    - remove the `ExtrusionRole` import that becomes unused
  - `crates/slicer-host/src/lib.rs`:
    - remove `order_entities_by_nearest_neighbor` from the `pub use layer_executor::{...}` line
  - `crates/slicer-host/tests/path_ordering_tdd.rs`:
    - rewrite each acceptance test to: (1) build a `Blackboard` with a minimal mesh, (2) build an `ExecutionPlan` with `Layer::Infill` (mock seeds infill IR with the fixture entities) followed by `Layer::PathOptimization` (real `path-optimization-default.wasm` via `WasmRuntimeDispatcher`), (3) run `execute_per_layer`, (4) assert on `LayerCollectionIR.ordered_entities` from the produced layer
    - add `no_module_proposal_leaves_raw_assembled_order`: same fixture as `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` but with a stub `LayerStageRunner` for `Layer::PathOptimization` that returns `Success` without ever pushing a proposal — assert raw-order start-x `[30.0, 0.0, 10.0]` is preserved (NOT the NN-reordered `[0.0, 10.0, 30.0]`)
    - delete the helper-driven assertions like `order_entities_by_nearest_neighbor(...)` direct calls; everything goes through dispatch
  - `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`:
    - frontmatter `status: implemented` → `status: superseded`
    - add a `## Superseded By` section pointing at `33_path-optimization-module-ordering`
  - `docs/DEVIATION_LOG.md`:
    - add a 2026 entry titled `path-optimization-module-ordering` summarizing the move and noting that the algorithm itself is unchanged from packet 18
  - `docs/14_deviation_audit_history.md`:
    - cross-link the deviation log entry
  - `docs/07_implementation_status.md`:
    - close `TASK-152g` (status `[x]` with packet-33 close note)
    - close new `TASK-152h` (status `[x]`)
    - leave `TASK-152` as `[~]` (152b/c/f remain open)
- Rejected alternatives:
  - keeping `order_entities_by_nearest_neighbor` as a Rust library function in a non-host crate (e.g., `slicer-helpers`) and calling it from the module via FFI: rejected because the algorithm runs on `PerimeterRegionView` (a WIT-bound type) inside the WASM guest, so the host crate is no longer the right owner. The simplest path is a private helper inside `path-optimization-default`.
  - keeping the host fallback alongside the module-side ordering as a "double safety net": rejected because two ordering surfaces silently competing makes regression diagnosis harder. The fallback removal proof (`no_module_proposal_leaves_raw_assembled_order`) is the *contract* this packet asserts.
  - keeping the helper in `layer_executor.rs` "for now in case packet 33 is reverted": rejected — superseding packet 18 is the explicit goal of this packet, and the helper's continued existence would invite accidental fallback re-use.

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

- The module's `regions: &[PerimeterRegionView]` iteration matches the host's `assemble_ordered_entities` ordering for perimeter entities. (Infill and support entities are not present in `regions` — the current `path-optimization-default` is wall-loop-driven; this packet's algorithm derives indices from wall loops in iteration order, which corresponds to the host's perimeter entity slot range. Mixed perimeter+infill+support fixtures must therefore be carefully constructed so that all entities live in `regions`. In practice, the test fixtures already use the wall-loop fixture path.)
- The reversal flag stays `false` for every entry. This is a future-proofing affordance — packet 32's WIT carries it, but no packet-33 test sets it true.
- Once `order_entities_by_nearest_neighbor` is deleted, no new code in `crates/slicer-host/` may reintroduce host-side ordering.

## Risks and Tradeoffs

- Risk: the module's algorithm derives entity indices differently from `assemble_ordered_entities`, producing a permutation that doesn't match the host's index space. Mitigation: the test rewrites pin the exact expected post-dispatch start-x sequences; a mismatch surfaces immediately. Implementation-plan Step 2 calls out the index-space alignment as the central concern.
- Risk: removing the host fallback breaks an existing test whose author assumed packet-18 host behavior. Mitigation: the rewrite of `path_ordering_tdd.rs` is exhaustive — every assertion either drives the live module or asserts the new raw-order fallback. Run the full slicer-host test suite as part of the acceptance ceremony.
- Risk: rebuilding `path-optimization-default.wasm` fails on a contributor's machine without the `wasm32-wasi` target installed. Mitigation: documented in CLAUDE.md (`./modules/core-modules/build-core-modules.sh` requires `wasm32` target). Acceptance ceremony runs the script.
- Risk: the deviation log entry is forgotten. Mitigation: explicit acceptance criterion `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md` blocks the packet otherwise.

## Open Questions

- None. The algorithm is preserved verbatim from packet 18, the WIT surface from packet 32 is consumed, and the fallback-removal proof is encoded as an explicit acceptance criterion.
