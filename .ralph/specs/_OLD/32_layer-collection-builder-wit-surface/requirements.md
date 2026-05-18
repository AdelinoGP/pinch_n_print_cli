# Requirements: layer-collection-builder-wit-surface

## Packet Metadata

- Grouped task IDs:
  - `TASK-152g` — add the `layer-collection-builder` WIT resource and wire it through the host bindings, SDK guest builder, `#[slicer_module]` macro, and `LayerModule::run_path_optimization` trait so a path-optimization module can (1) read the host-staged `LayerCollectionIR.ordered_entities` via `get-ordered-entities() -> list<ordered-entity-view>` and (2) declare an entity-order permutation with per-entity reversal via `set-entity-order(items: list<tuple<u32, bool>>)`. The host validates and applies the proposal to `LayerCollectionIR.ordered_entities`. Migration of `path-optimization-default` to actually use the new builder is deferred to packet 33.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Packet 18 (`18_path-optimization-entity-ordering`) implemented entity ordering as a host helper `order_entities_by_nearest_neighbor` in `crates/slicer-host/src/layer_executor.rs`. That was a deliberate stopgap because the `Layer::PathOptimization` WIT surface had no way for the module to express a reordering: the existing `gcode-output-builder` only accepts GCode-shaped commands plus `push-z-hop`, and the `run-path-optimization` export takes a read-only `list<perimeter-region-view>` and returns nothing.

`docs/01_system_architecture.md` notes that "Reordering / mutation of `ordered_entities` is reserved for a future `layer-collection-builder` resource." This packet introduces that resource with two methods:

- `set-entity-order(items: list<tuple<u32, bool>>) -> result<_, string>` — the module declares a permutation of the host-assembled entities plus an optional per-entity reversal flag. The shape mirrors OrcaSlicer's `ShortestPath::chain_segments_closest_point` return type `vector<pair<size_t, bool>>`.
- `get-ordered-entities() -> list<ordered-entity-view>` — the module reads the host-staged `LayerCollectionIR.ordered_entities` as a flat snapshot (one record per entity carrying `original-index`, `region-key`, `role`, `start-point`, `end-point`, and `point-count`). This is required because the existing `regions: list<perimeter-region-view>` parameter on `run-path-optimization` only exposes perimeter wall-loops; infill and support entities live in `LayerCollectionIR.ordered_entities` and are otherwise invisible to the module. Without this read accessor, a module-side ordering algorithm cannot enumerate the entities it must permute (its `set-entity-order` proposal must carry exactly one entry per existing entity, including infill and support), and cannot reason about per-entity role for tiebreaks like the OrcaSlicer bridge-priority rule.

This packet introduces the WIT surface, the host validation/application logic, the read-projection helper, and the SDK plumbing; it does **not** migrate any path-optimization module to actually call `set_entity_order` or `get_ordered_entities`. The packet-18 host fallback (`order_entities_by_nearest_neighbor`) stays in place: when no module emits a proposal, the host applies its own NN ordering, preserving every packet-18 test. Packet 33 will migrate `path-optimization-default` to use the new builder and will then remove the fallback.

## In Scope

- new `resource layer-collection-builder` in `wit/deps/ir-types.wit` with two methods:
  - `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>`
  - `get-ordered-entities: func() -> list<ordered-entity-view>`
- new `record ordered-entity-view` in `wit/deps/ir-types.wit` with fields `original-index: u32`, `region-key: region-key`, `role: extrusion-role`, `start-point: point3-with-width`, `end-point: point3-with-width`, `point-count: u32`
- new parameter `collection: layer-collection-builder` on `run-path-optimization` in `wit/world-layer.wit`
- host backing data and trait implementation in `crates/slicer-host/src/wit_host.rs` (both methods)
- host helper `project_ordered_entities(arena: &LayerArena) -> Vec<OrderedEntityView>` returning an empty `Vec` when no `LayerCollectionIR` is staged
- dispatch wiring in `crates/slicer-host/src/dispatch.rs` (read accessor is synchronous; only write needs post-call validation)
- SDK guest builder in `crates/slicer-sdk` exposing `set_entity_order` and `get_ordered_entities() -> &[OrderedEntityView]` (snapshot populated by the macro drain at call entry)
- SDK record `OrderedEntityView` mirroring the WIT shape
- `#[slicer_module]` macro updates in `crates/slicer-macros/src/lib.rs`: embedded WIT, drain helper, pre-call snapshot population
- trait change in `crates/slicer-sdk/src/traits.rs`
- sweep update of all in-tree `LayerModule::run_path_optimization` impls
- new host integration tests covering (a) host-side write validation and ordering+reversal application, (b) read-projection correctness (index ordering, endpoints, point count, empty when nothing staged), and (c) the macro-call-once contract — a real WASM guest that calls `collection.get_ordered_entities()` 5 times must trigger exactly one host-side `HostLayerCollectionBuilder::get_ordered_entities` invocation
- new instrumentation counter on `HostExecutionContext` to observe the host-side invocation count, reset on every `push_layer_collection_builder`
- new test guest crate `test-guests/path-optimization-multi-read/` whose body calls `get_ordered_entities` 5 times and asserts internal slice-content stability across calls
- new SDK unit test asserting `LayerCollectionBuilder::get_ordered_entities()` reads from a local field (no WIT round-trip)
- WIT drift-detection regression update covering the record and both methods
- WASM rebuild via `./modules/core-modules/build-core-modules.sh`

## Out of Scope

- migrating `path-optimization-default` to call `set_entity_order` or `get_ordered_entities` (packet 33)
- removing `order_entities_by_nearest_neighbor` from `crates/slicer-host/src/layer_executor.rs` (packet 33)
- marking packet 18 superseded (packet 33)
- adding any method other than `set-entity-order` and `get-ordered-entities` to the new resource
- exposing additional `PrintEntity` fields beyond those in `ordered-entity-view` (full `path.points`, per-point `width`, `flow_factor`, `speed_factor`, etc.) — the projection is intentionally a flat summary sufficient for ordering algorithms
- mutation of any other `LayerCollectionIR` field (`tool_changes`, `z_hops`, `annotations`, `retracts`, `travel_moves`)
- changes to `wit/world-prepass.wit`, `wit/world-postpass.wit`, `wit/world-finalization.wit`
- WIT version bumps (the application is unreleased — breaking changes are acceptable in place)

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` — `chain_segments_closest_point` returns `vector<pair<size_t, bool>>` (segment index, should-reverse). The new WIT method's `list<tuple<u32, bool>>` argument matches this shape.

## Acceptance Summary

### Positive Cases

- `wit/deps/ir-types.wit` declares `resource layer-collection-builder` with both `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>` and `get-ordered-entities: func() -> list<ordered-entity-view>`, and `record ordered-entity-view` with the documented fields.
- `wit/world-layer.wit` adds `collection: layer-collection-builder` as the parameter immediately after `output: gcode-output-builder` on `run-path-optimization`.
- A valid permutation `[(2,false),(0,false),(1,false)]` for a 3-entity layer with raw start-x `[30.0, 0.0, 10.0]` produces `ordered_entities` start-x sequence `[10.0, 30.0, 0.0]` with `topo_order` `[0,1,2]`.
- A `(0, true)` reversal flag on a single entity reverses `path.points` in place (first becomes last and vice versa).
- `project_ordered_entities` returns one `OrderedEntityView` per staged entity in `original_index` order, carrying the entity's `region_key`, the path's first/last point, the path's role, and `point_count`.
- `project_ordered_entities` returns an empty `Vec` when no `LayerCollectionIR` is staged on the arena.
- A test guest that calls `collection.get_ordered_entities()` 5 times triggers exactly **one** host-side `HostLayerCollectionBuilder::get_ordered_entities` invocation (the macro drain caches the snapshot at call entry; the trait method's repeated calls hit the SDK cache).
- The SDK type `LayerCollectionBuilder` has no `wasmtime::component::Resource` field for the layer-collection-builder; its `get_ordered_entities()` is implemented by reading a local `Vec<OrderedEntityView>` populated once via the doc-hidden `set_ordered_entities` constructor used by the macro drain.
- The packet-18 acceptance test `reordered_sequence_is_consumed_by_path_optimization_stage` still passes because the host fallback stays in place.
- The drift-detection test asserts the macro's embedded WIT mentions the new resource, the new record, and both methods.
- `cargo build --workspace` and `./modules/core-modules/build-core-modules.sh` succeed.

### Negative Cases

- Duplicate index in proposal → fatal `FatalModule` with message `"set-entity-order: duplicate index N"`.
- Out-of-range index → fatal with `"set-entity-order: index N out of range [0, M)"`.
- Wrong-length proposal → fatal with `"set-entity-order: expected M indices, got K"`.
- Any malformed proposal leaves `LayerCollectionIR.ordered_entities` unchanged (no partial reorder, no partial point reversal).

### Measurable Outcomes

- Host validation runs **before** any mutation of `ordered_entities`. The validation function returns `Result<Vec<(u32, bool)>, ValidationError>`; only on `Ok` does the host apply the reorder + reversal.
- Reversal mutates `path.points: Vec<Point3WithWidth>` via `Vec::reverse()` in place after the entity is moved to its final slot in `ordered_entities`.
- `topo_order` on each entity is reassigned to its post-permutation 0-based index.
- `region_key`, `role`, `speed_factor`, `width`, and `flow_factor` per-point fields remain unchanged (reversal preserves per-point payload — it only changes order of the points within a single path).
- The host fallback `order_entities_by_nearest_neighbor` continues to run when `HostExecutionContext.layer_collection_proposal` is `None` after the module returns.

### Cross-Packet Impact

- Packet 33 builds on this packet to migrate `path-optimization-default` to call `set_entity_order` and remove the host fallback.
- Packet 18 stays `implemented`; it will be marked `superseded` only in packet 33, after the fallback is removed.
- No prepass / postpass / finalization packet is affected (other worlds remain at `@1.0.0` semantics — only `world-layer` gains a parameter).

## Verification Commands

- `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit`
- `grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit`
- `grep -E "get-ordered-entities:\\s*func\\(\\)\\s*->\\s*list<ordered-entity-view>" wit/deps/ir-types.wit`
- `grep -E "record ordered-entity-view" wit/deps/ir-types.wit`
- `grep -E "collection:\\s*layer-collection-builder" wit/world-layer.wit`
- `cargo test -p slicer-host --test layer_collection_builder_tdd valid_permutation_is_applied_to_ordered_entities -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd reversal_flag_reverses_path_points_in_place -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd duplicate_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd out_of_range_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd wrong_length_proposal_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd malformed_proposal_leaves_ordered_entities_unchanged -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_projects_staged_entities_in_index_order -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_carries_endpoints_and_point_count -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_returns_empty_when_no_layer_collection_is_staged -- --exact --nocapture`
- `cargo test -p slicer-sdk --test layer_module_tdd layer_collection_builder_get_ordered_entities_reads_local_cache -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd macro_drain_invokes_host_get_ordered_entities_exactly_once -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- `cargo test -p slicer-host --test wit_drift_detection_tdd macro_embeds_layer_collection_builder_resource -- --exact --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the WIT surface change is bounded to one file at a time (ir-types → world-layer → host bindings → dispatch → SDK → macro → trait/sweep → tests)
- Postcondition: the new resource is observable through that layer (grep for WIT, host trait method exists, SDK builder compiles, macro drains, tests assert)
- Falsifying check: a focused targeted assertion fails if the step's contribution is missing
