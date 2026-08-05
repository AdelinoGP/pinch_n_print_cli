---
status: implemented
packet: layer-collection-builder-wit-surface
task_ids:
  - TASK-152g
---

# 32_layer-collection-builder-wit-surface

## Goal

Introduce a new `layer-collection-builder` WIT resource (the planned host-collection mutation surface from `docs/01_system_architecture.md`) and wire it through the host bindings, SDK guest type, `#[slicer_module]` macro, and `LayerModule::run_path_optimization` trait signature. Provide two methods:

- `set-entity-order(items: list<tuple<u32, bool>>)` — the module declares a permutation of `LayerCollectionIR.ordered_entities` and an optional per-entity reversal flag (the OrcaSlicer `pair<size_t, bool>` shape). The host validates the proposal and applies it to the layer's pre-staged ordered entity list and per-entity `path.points`.
- `get-ordered-entities() -> list<ordered-entity-view>` — read accessor projecting the host-staged `LayerCollectionIR.ordered_entities` so the module can compute a permutation over the **full mixed perimeters + infill + support entity list** (which `run-path-optimization`'s existing `regions: list<perimeter-region-view>` parameter does not surface). Each view carries `original-index`, `region-key`, `role`, `start-point`, `end-point`, and `point-count`.

The packet-18 host-side `order_entities_by_nearest_neighbor` is retained as a fallback when no module emits a proposal — it is removed in packet 33, not here.

## Problem Statement

Packet 18 (`18_path-optimization-entity-ordering`) implemented entity ordering as a host helper `order_entities_by_nearest_neighbor` in `crates/slicer-host/src/layer_executor.rs`. That was a deliberate stopgap because the `Layer::PathOptimization` WIT surface had no way for the module to express a reordering: the existing `gcode-output-builder` only accepts GCode-shaped commands plus `push-z-hop`, and the `run-path-optimization` export takes a read-only `list<perimeter-region-view>` and returns nothing.

`docs/01_system_architecture.md` notes that "Reordering / mutation of `ordered_entities` is reserved for a future `layer-collection-builder` resource." This packet introduces that resource with two methods:

- `set-entity-order(items: list<tuple<u32, bool>>) -> result<_, string>` — the module declares a permutation of the host-assembled entities plus an optional per-entity reversal flag. The shape mirrors OrcaSlicer's `ShortestPath::chain_segments_closest_point` return type `vector<pair<size_t, bool>>`.
- `get-ordered-entities() -> list<ordered-entity-view>` — the module reads the host-staged `LayerCollectionIR.ordered_entities` as a flat snapshot (one record per entity carrying `original-index`, `region-key`, `role`, `start-point`, `end-point`, and `point-count`). This is required because the existing `regions: list<perimeter-region-view>` parameter on `run-path-optimization` only exposes perimeter wall-loops; infill and support entities live in `LayerCollectionIR.ordered_entities` and are otherwise invisible to the module. Without this read accessor, a module-side ordering algorithm cannot enumerate the entities it must permute (its `set-entity-order` proposal must carry exactly one entry per existing entity, including infill and support), and cannot reason about per-entity role for tiebreaks like the OrcaSlicer bridge-priority rule.

This packet introduces the WIT surface, the host validation/application logic, the read-projection helper, and the SDK plumbing; it does **not** migrate any path-optimization module to actually call `set_entity_order` or `get_ordered_entities`. The packet-18 host fallback (`order_entities_by_nearest_neighbor`) stays in place: when no module emits a proposal, the host applies its own NN ordering, preserving every packet-18 test. Packet 33 will migrate `path-optimization-default` to use the new builder and will then remove the fallback.

## Architecture Constraints

- Selected approach: introduce a dedicated `layer-collection-builder` resource as a new parameter on `run-path-optimization`. The host owns `LayerCollectionIR.ordered_entities`; the module's call to `set-entity-order` is a *proposal* the host validates and applies after the module returns. Validation runs **before any mutation** so a malformed proposal leaves the layer state unchanged. The packet-18 host fallback remains active when no proposal is emitted.
- The same resource exposes a synchronous read accessor `get-ordered-entities() -> list<ordered-entity-view>` so the module can enumerate the entities it must permute. This is required because `run-path-optimization`'s existing `regions: list<perimeter-region-view>` parameter only carries perimeter wall-loops; the host-staged `LayerCollectionIR.ordered_entities` is a mixed list of perimeters + infill + support entities, and a `set-entity-order` proposal must carry exactly one entry per existing entity. The read accessor is total: an empty list means "no `LayerCollectionIR` is staged on the arena" — there is no error path. (In live dispatch the dispatch arm always pre-stages a `LayerCollectionIR` before calling the module, so the empty case is observable only in direct host-side tests.)
- The application is unreleased, so `slicer:world-layer` and `slicer:ir-types` do **not** require version bumps even though the export signature change is technically breaking. Existing modules will be swept in this packet to match the new signature, and all WASM binaries will be rebuilt before packet close.
- The host stores a single optional ordering proposal per call (`HostExecutionContext.layer_collection_proposal: Option<Vec<(u32, bool)>>`). Multiple `set-entity-order` calls within one `run-path-optimization` are rejected as a contract violation (the second call returns `Err` from the WIT-level method).
- Reversal mutates `path.points` in place via `Vec::reverse()` after the entity has been moved to its post-permutation slot. Per-point payloads (`width`, `flow_factor`) reverse with the points — this is correct for a reversed extrusion.
- `topo_order` is reassigned post-permutation to the 0-based slot index. `region_key`, `role`, `speed_factor` are preserved.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities` (mutated in place after validation passes)
  - `PrintEntity.path.points` (reversed in place when the per-entity flag is `true`)
  - `PrintEntity.topo_order` (reassigned to the post-permutation 0-based index)
- WIT boundary additions:
  - `record ordered-entity-view` in `slicer:ir-types` package (re-uses existing `region-key`, `extrusion-role`, `point3-with-width`)
  - `resource layer-collection-builder` in `slicer:ir-types` package with both `set-entity-order` and `get-ordered-entities`
  - `collection: layer-collection-builder` parameter on `run-path-optimization` in `slicer:world-layer` package
- Determinism / scheduler constraints:
  - The host applies the proposal deterministically: validation order is (1) length match, (2) range check per index, (3) duplicate check via a bitmap of seen indices. The first failure short-circuits with the corresponding diagnostic.
  - `Vec::reverse()` is deterministic. Multiple reversal flags on different entities cannot interact.
  - The fallback path remains the packet-18 deterministic NN ordering.

## Locked Assumptions and Invariants

- The host owns `LayerCollectionIR.ordered_entities` as a writable surface; the module's contribution is a validated proposal.
- Validation never partially mutates: a malformed proposal causes a fatal at dispatch time and `ordered_entities` stays in its pre-call state.
- `set-entity-order` is callable at most once per `run-path-optimization` invocation; a second call from the guest returns `Err` from the WIT method, which the SDK guest type also enforces by checking its internal `Option`.
- `get-ordered-entities` is total and idempotent: calling it any number of times returns the same snapshot of the host-staged `LayerCollectionIR.ordered_entities` as it stood when the module entered `run-path-optimization`. The host does not mutate `ordered_entities` while the module is executing; mutation happens after the module returns and the proposal is validated.
- **Snapshot capture site is a contract, not an ergonomic choice.** Dispatch projects `LayerCollectionIR.ordered_entities` exactly once per `Layer::PathOptimization` invocation via `project_ordered_entities(arena)` and stashes the result on `LayerCollectionBuilderData.ordered_entities` at `push_layer_collection_builder` time. `HostLayerCollectionBuilder::get_ordered_entities` reads from this resource-local snapshot rather than re-projecting from the live arena, which structurally guarantees the "same snapshot across calls" invariant above without relying on the host abstaining from arena mutation.
- **Macro-call-once is a contract, not an ergonomic choice.** The macro-generated `__slicer_populate_layer_collection` adapter MUST call `wit_resource.get_ordered_entities()` **exactly once** per `run-path-optimization` invocation, before the trait method runs, and store the result via `sdk_builder.set_ordered_entities(snapshot)`. The trait method's repeated `collection.get_ordered_entities()` calls MUST hit the SDK-local cache and MUST NOT re-invoke the WIT host method. This is enforced by an acceptance test (`macro_drain_invokes_host_get_ordered_entities_exactly_once`) that counts host-side invocations via the cross-call `HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS: AtomicU32` static (reset by the test before exercising a layer; incremented in lockstep with the per-context `host_get_ordered_entities_call_count` field at the top of `HostLayerCollectionBuilder::get_ordered_entities`). The static counter is the test-observable handle because `execute_per_layer` consumes the wasmtime `Store` internally, making the per-context field unreachable from a layer-level integration test; the per-context field is retained for direct host-side tests that own the store.
- The SDK type `LayerCollectionBuilder` MUST NOT carry a reference to a `wasmtime::component::Resource` for the layer-collection-builder. Its `get_ordered_entities()` reads from a local `Vec<OrderedEntityView>` field. Modules that bypass the SDK type and obtain the WIT resource directly are out of contract for this packet.
- `OrderedEntityView` is a flat read snapshot. Modules MUST NOT cache view fields across calls — each `run-path-optimization` invocation produces a fresh snapshot.
- `point_count` always equals the number of points in `path.points` for the corresponding entity at the time the view was projected; reversal applied later by `apply_entity_order_proposal` does not retroactively change the snapshot value.
- Reversal preserves the path's payload; only the order of `points` within a single path changes.
- `topo_order` always equals the entity's index in `ordered_entities` after the permutation is applied.
- The packet-18 host fallback remains the default until packet 33 removes it.

## Risks and Tradeoffs

- Risk: macro-embedded WIT drifts from disk WIT. Mitigation: `wit_drift_detection_tdd` regression covers the new resource and the export signature. Add an explicit assertion for both.
- Risk: dispatch wiring forgets to reset `HostExecutionContext.layer_collection_proposal` between calls, leaking a proposal across layers. Mitigation: reset on every push of the new resource (mirroring how `gcode_output.commands` is implicitly cleared per call). Add a regression test that runs two layers consecutively, only one of which emits a proposal.
- Risk: reversing `path.points` interacts badly with downstream consumers that assume monotonic Z. Mitigation: paths are emitted at a single layer Z; reversal does not change Z values, only X/Y/width sequence.
- Risk: existing test guests under `test-guests/` are not all rebuilt as part of `build-core-modules.sh` (which only covers manifest-shipped core modules). Mitigation: implementation-plan Step 8 extends the dedicated `test-guests/build-test-guests.sh` script to enumerate and rebuild every test guest including the new `path-optimization-multi-read`; `modules/core-modules/build-core-modules.sh` continues to rebuild `path-optimization-default` and the other core modules.
- Risk: host validation order yields a confusing diagnostic when several errors apply (e.g., wrong length AND duplicate). Mitigation: validate length first and short-circuit; tests pin the exact expected substring.
- Risk: a `PrintEntity` with empty `path.points` would cause `project_ordered_entities` to panic on `first()/last()`. Mitigation: empty `path.points` is already a host invariant violation upstream of `LayerCollectionIR` assembly; `project_ordered_entities` documents this with a debug-assert and treats the unwrap as a host bug, not a module-facing error. No production fixture exercises empty paths.
- Risk: `get-ordered-entities` returning a Vec each call is allocation-heavy if a module calls it in a hot loop. Mitigation: the SDK builder caches the snapshot (populated once by the macro drain), and the trait method reads from that cache. The macro-call-once policy is a contract pinned by the `macro_drain_invokes_host_get_ordered_entities_exactly_once` acceptance test, not just a recommendation. Modules that obtain the raw WIT resource and invoke `get_ordered_entities` directly bypass the cache and pay the allocation; this is out-of-contract usage.
