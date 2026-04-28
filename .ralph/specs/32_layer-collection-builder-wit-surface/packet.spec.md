---
status: active
packet: layer-collection-builder-wit-surface
task_ids:
  - TASK-152g
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: layer-collection-builder-wit-surface

## Goal

Introduce a new `layer-collection-builder` WIT resource (the planned host-collection mutation surface from `docs/01_system_architecture.md`) and wire it through the host bindings, SDK guest type, `#[slicer_module]` macro, and `LayerModule::run_path_optimization` trait signature. Provide two methods:

- `set-entity-order(items: list<tuple<u32, bool>>)` — the module declares a permutation of `LayerCollectionIR.ordered_entities` and an optional per-entity reversal flag (the OrcaSlicer `pair<size_t, bool>` shape). The host validates the proposal and applies it to the layer's pre-staged ordered entity list and per-entity `path.points`.
- `get-ordered-entities() -> list<ordered-entity-view>` — read accessor projecting the host-staged `LayerCollectionIR.ordered_entities` so the module can compute a permutation over the **full mixed perimeters + infill + support entity list** (which `run-path-optimization`'s existing `regions: list<perimeter-region-view>` parameter does not surface). Each view carries `original-index`, `region-key`, `role`, `start-point`, `end-point`, and `point-count`.

The packet-18 host-side `order_entities_by_nearest_neighbor` is retained as a fallback when no module emits a proposal — it is removed in packet 33, not here.

## Scope Boundaries

- In scope:
  - new `resource layer-collection-builder` in `wit/deps/ir-types.wit` with two methods:
    - `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>`
    - `get-ordered-entities: func() -> list<ordered-entity-view>`
  - new `record ordered-entity-view` in `wit/deps/ir-types.wit` with fields `original-index: u32`, `region-key: region-key`, `role: extrusion-role`, `start-point: point3-with-width`, `end-point: point3-with-width`, `point-count: u32`
  - new `collection: layer-collection-builder` parameter on `run-path-optimization` in `wit/world-layer.wit`
  - host backing data type `LayerCollectionBuilderData` and host trait impl in `crates/slicer-host/src/wit_host.rs` (both `set_entity_order` and `get_ordered_entities`)
  - host resource constructor `push_layer_collection_builder()`
  - host helper `apply_entity_order_proposal(arena, proposal)` for write validation/application (already landed) plus new helper `project_ordered_entities(arena) -> Vec<OrderedEntityView>` for read projection (returns empty `Vec` when no `LayerCollectionIR` is staged)
  - dispatch wiring in `crates/slicer-host/src/dispatch.rs` that pushes the new resource into the store, captures the proposal in `HostExecutionContext`, and applies validated ordering + reversal to `arena.layer_collection().ordered_entities` after the module returns; the read accessor is synchronous (no post-call wiring needed)
  - SDK guest builder `LayerCollectionBuilder` in `crates/slicer-sdk/src/layer_collection_builder.rs` exposing both `set_entity_order(items)` and `get_ordered_entities() -> &[OrderedEntityView]` (the macro drain populates the read snapshot before invoking the trait method)
  - SDK record `OrderedEntityView` mirroring the WIT shape in `crates/slicer-sdk/src/views.rs`
  - `#[slicer_module]` macro: embedded WIT update (record + both methods), new `__slicer_drain_layer_collection` adapter, trait-method bridging that calls `wit.get_ordered_entities()` to populate the SDK builder's snapshot before the trait method runs and drains `set_entity_order` afterward
  - trait change: `LayerModule::run_path_optimization` gains `collection: &mut LayerCollectionBuilder` with a default empty body
  - sweep update of every in-tree `LayerModule::run_path_optimization` impl to accept the new parameter (existing modules pass it unused)
  - rebuild all core-modules WASM artifacts via `./modules/core-modules/build-core-modules.sh`
  - host validation rejecting malformed permutations atomically (no partial state)
  - `wit_drift_detection_tdd.rs` updated for the new resource, record, and both methods
  - new `crates/slicer-host/tests/layer_collection_builder_tdd.rs` covering host validation, ordering+reversal application, read-projection correctness, and the macro-call-once contract via a counting test guest
  - new instrumentation: per-context `pub(crate) host_get_ordered_entities_call_count: u32` field on `HostExecutionContext` (reset to `0` by `push_layer_collection_builder()`, incremented inside `HostLayerCollectionBuilder::get_ordered_entities`, exposed via a `#[doc(hidden)] pub fn host_get_ordered_entities_call_count(&self) -> u32` getter for direct host-side tests that own the store) **plus** a cross-call `pub static HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS: AtomicU32` (incremented in lockstep at the top of `get_ordered_entities`, reset by tests via `.store(0, Ordering::SeqCst)`). The static counter is the observation handle for the macro-call-once acceptance test because `execute_per_layer` consumes the wasmtime `Store` internally and the per-context counter is unreachable from a layer-level integration test.
  - new test guest crate `test-guests/path-optimization-multi-read/` whose `LayerModule::run_path_optimization` calls `collection.get_ordered_entities()` exactly 5 times in succession, asserts each call returns identical content (via internal `assert_eq!` on the slice contents), and traps with a recognizable substring on mismatch — used by the macro-call-once acceptance test
- Out of scope:
  - moving the nearest-neighbor algorithm into `path-optimization-default` (packet 33)
  - removing `order_entities_by_nearest_neighbor` from `crates/slicer-host/src/layer_executor.rs` (packet 33)
  - marking packet 18 as `status: superseded` (packet 33)
  - any methods on `layer-collection-builder` other than `set-entity-order` and `get-ordered-entities`
  - exposing additional `PrintEntity` fields beyond those listed in `ordered-entity-view` (e.g., per-point `width`, `flow_factor`, `speed_factor`, full `path.points`) — the projection is intentionally a flat summary sufficient for ordering algorithms; richer access is a future packet if needed
  - any change to other WIT worlds (prepass, postpass, finalization)
  - tool-change ordering (TASK-152b), cooling policy (TASK-152c), finalization travel coordination (TASK-152f)

## Prerequisites and Blockers

- Depends on:
  - packet 18 (`18_path-optimization-entity-ordering`) — host fallback ordering must remain functional during this packet so existing acceptance tests stay green
- Unblocks:
  - packet 33 (`33_path-optimization-module-ordering`) — migrates `path-optimization-default` to use the new builder and removes the host fallback
- Activation blockers:
  - None. Packet remains `draft` by default.

## Acceptance Criteria

- **Given** the on-disk file `wit/deps/ir-types.wit`, **when** the file is read, **then** it contains a `resource layer-collection-builder` block declaring both methods `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>` and `get-ordered-entities: func() -> list<ordered-entity-view>`. | `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit && grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit && grep -E "get-ordered-entities:\\s*func\\(\\)\\s*->\\s*list<ordered-entity-view>" wit/deps/ir-types.wit`
- **Given** the on-disk file `wit/deps/ir-types.wit`, **when** the file is read, **then** it contains a `record ordered-entity-view` block declaring exactly the fields `original-index: u32`, `region-key: region-key`, `role: extrusion-role`, `start-point: point3-with-width`, `end-point: point3-with-width`, and `point-count: u32`. | `grep -E "record ordered-entity-view" wit/deps/ir-types.wit && grep -E "original-index:\\s*u32" wit/deps/ir-types.wit && grep -E "point-count:\\s*u32" wit/deps/ir-types.wit`
- **Given** the on-disk file `wit/world-layer.wit`, **when** the file is read, **then** the `run-path-optimization` export signature includes a `collection: layer-collection-builder` parameter directly after the existing `output: gcode-output-builder` parameter. | `grep -E "run-path-optimization:\\s*func" wit/world-layer.wit && grep -E "collection:\\s*layer-collection-builder" wit/world-layer.wit`
- **Given** a `LayerArena` staging a 3-entity `LayerCollectionIR` with raw start-point x values `[30.0, 0.0, 10.0]`, **when** a test calls `apply_entity_order_proposal(&mut arena, &proposal)` with `proposal = [(2,false),(0,false),(1,false)]`, **then** the resulting `LayerCollectionIR.ordered_entities` start-point x values are exactly `[10.0, 30.0, 0.0]` and `topo_order` values are `[0,1,2]`. | `cargo test -p slicer-host --test layer_collection_builder_tdd valid_permutation_is_applied_to_ordered_entities -- --exact --nocapture`
- **Given** a single-entity layer whose `path.points` has start `(0.0, 0.0, 0.2)` and end `(5.0, 0.0, 0.2)`, **when** a test commits a proposal `[(0, true)]`, **then** the resulting `ordered_entities[0].path.points.first()` x equals `5.0` and `ordered_entities[0].path.points.last()` x equals `0.0` (the points sequence is reversed in place). | `cargo test -p slicer-host --test layer_collection_builder_tdd reversal_flag_reverses_path_points_in_place -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(0,false),(0,false),(1,false)]` (duplicate index `0`), **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: duplicate index 0"` and `LayerCollectionIR.ordered_entities` is left in the pre-call state (no partial mutation). | `cargo test -p slicer-host --test layer_collection_builder_tdd duplicate_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(99,false),(0,false),(1,false)]`, **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: index 99 out of range [0, 3)"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd out_of_range_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(0,false),(1,false)]` (length 2 ≠ 3 entities), **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: expected 3 indices, got 2"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd wrong_length_proposal_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** a `LayerArena` staging a 3-entity `LayerCollectionIR` whose entities have `(start-x, role, region_key.object_id)` tuples `[(30.0, SparseInfill, "obj"), (0.0, BridgeInfill, "obj"), (10.0, SparseInfill, "obj")]` in `ordered_entities` order, **when** `project_ordered_entities(&arena)` is called, **then** the returned `Vec<OrderedEntityView>` has length 3 and the i-th entry's `original_index == i`, the start-x sequence is `[30.0, 0.0, 10.0]`, the role sequence is `[SparseInfill, BridgeInfill, SparseInfill]`, and every `region_key.object_id == "obj"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_projects_staged_entities_in_index_order -- --exact --nocapture`
- **Given** a `LayerArena` whose staged `LayerCollectionIR.ordered_entities[0].path.points` has start `(0.0, 0.0, 0.2)` and end `(5.0, 0.0, 0.2)`, **when** `project_ordered_entities(&arena)` is called, **then** the returned view's `start_point.point.x == 0.0`, `end_point.point.x == 5.0`, and `point_count == 2`. | `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_carries_endpoints_and_point_count -- --exact --nocapture`
- **Given** a `LayerArena` with no `LayerCollectionIR` staged, **when** `project_ordered_entities(&arena)` is called, **then** it returns an empty `Vec` (NOT an error — the read accessor is total; absence of staging is observable as an empty list). | `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_returns_empty_when_no_layer_collection_is_staged -- --exact --nocapture`
- **Given** an SDK `LayerCollectionBuilder` whose `set_ordered_entities` has been called once with a 3-entity snapshot, **when** `get_ordered_entities()` is called twice in succession, **then** both calls return slices of length 3 with identical content (`assert_eq!(first, second)`) — proving the SDK type stores the snapshot in a local field and reads from it without round-tripping to the WIT host. | `cargo test -p slicer-sdk --test layer_module_tdd layer_collection_builder_get_ordered_entities_reads_local_cache -- --exact --nocapture`
- **Given** a test guest `path-optimization-multi-read.component.wasm` whose `run_path_optimization` body calls `collection.get_ordered_entities()` exactly 5 times in a row, **when** the guest is dispatched through `WasmRuntimeDispatcher` against a 3-entity layer, **then** (a) every call returns the same 3-element slice content (the guest itself asserts internal stability and panics the trap with a recognizable substring on mismatch), and (b) the cross-call counter `slicer_host::HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS` reads exactly `1` after `execute_per_layer` returns (test resets it to `0` before dispatch via `.store(0, Ordering::SeqCst)`) — proving the macro-generated populate adapter calls `wit_resource.get_ordered_entities()` exactly once at call entry and the trait method's repeated calls hit the SDK cache. | `cargo test -p slicer-host --test layer_collection_builder_tdd macro_drain_invokes_host_get_ordered_entities_exactly_once -- --exact --nocapture`
- **Given** the existing packet-18 test fixture and a `Layer::PathOptimization` runner that does **not** call `set_entity_order`, **when** `execute_per_layer` runs, **then** `LayerCollectionIR.ordered_entities` matches the host-fallback NN ordering (start-point x values `[0.0, 10.0, 30.0]`) — the fallback path remains active. | `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- **Given** the canonical on-disk WIT files, **when** the drift-detection test runs, **then** the `#[slicer_module]` macro's embedded WIT in `crates/slicer-macros/src/lib.rs` contains `resource layer-collection-builder` with **both** `set-entity-order` and `get-ordered-entities` methods, the `record ordered-entity-view` declaration, and references the resource from the layer-module world's `run-path-optimization` signature. | `cargo test -p slicer-host --test wit_drift_detection_tdd macro_embeds_layer_collection_builder_resource -- --exact --nocapture`
- **Given** the workspace after this packet's edits, **when** `cargo build --workspace` runs, **then** the build succeeds with zero errors and every existing `LayerModule::run_path_optimization` impl has been updated to accept the new `collection: &mut LayerCollectionBuilder` parameter. | `cargo build --workspace 2>&1 | tee /tmp/build.log && ! grep -E "error\\[E" /tmp/build.log`
- **Given** the workspace after this packet's edits, **when** `./modules/core-modules/build-core-modules.sh` runs, **then** all `.wasm` artifacts under `modules/core-modules/*/` rebuild successfully (`*.wasm` files exist and have `mtime` newer than the start of the run). | `./modules/core-modules/build-core-modules.sh && ls modules/core-modules/path-optimization-default/path-optimization-default.wasm`

## Negative Test Cases

- **Given** a malformed proposal (duplicate, out-of-range, or wrong length), **when** the host applies it, **then** the layer's `LayerCollectionIR.ordered_entities` is unchanged (no partial reorder, no partial reversal). | `cargo test -p slicer-host --test layer_collection_builder_tdd malformed_proposal_leaves_ordered_entities_unchanged -- --exact --nocapture`
- **Given** an SDK `LayerCollectionBuilder` whose `set_entity_order` has already been called once, **when** a second call is made within the same `run_path_optimization` invocation, **then** the second call returns `Err` carrying the substring `"set-entity-order called twice"` and the first proposal is preserved. | `cargo test -p slicer-sdk --test layer_module_tdd set_entity_order_second_call_returns_err -- --exact --nocapture`
- **Given** a `LayerArena` with no `LayerCollectionIR` staged, **when** `apply_entity_order_proposal` is called with any proposal, **then** it returns `Err` whose message contains the substring `"set-entity-order: no LayerCollectionIR staged on arena"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd missing_layer_collection_is_rejected -- --exact --nocapture`

## Verification

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
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_projects_staged_entities_in_index_order -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_carries_endpoints_and_point_count -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd get_ordered_entities_returns_empty_when_no_layer_collection_is_staged -- --exact --nocapture`
- `cargo test -p slicer-sdk --test layer_module_tdd layer_collection_builder_get_ordered_entities_reads_local_cache -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd macro_drain_invokes_host_get_ordered_entities_exactly_once -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd malformed_proposal_leaves_ordered_entities_unchanged -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd missing_layer_collection_is_rejected -- --exact --nocapture`
- `cargo test -p slicer-sdk --test layer_module_tdd set_entity_order_second_call_returns_err -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- `cargo test -p slicer-host --test wit_drift_detection_tdd macro_embeds_layer_collection_builder_resource -- --exact --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Authoritative Docs

- `docs/01_system_architecture.md` — host vs module ownership of `LayerCollectionIR.ordered_entities`; planned `layer-collection-builder` resource
- `docs/02_ir_schemas.md` — `LayerCollectionIR.ordered_entities`, `PrintEntity.path`, `ExtrusionPath3D.points`, `PrintEntity.topo_order`
- `docs/03_wit_and_manifest.md` — WIT resource declaration rules; PathOptimization output contract
- `docs/04_host_scheduler.md` — PathOptimization stage scheduling and dispatch order
- `docs/05_module_sdk.md` — SDK builder pattern (`#[slicer_module]`, drain helpers)
- `docs/07_implementation_status.md` — TASK-152g

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` — `vector<pair<size_t, bool>>` shape carried over into `set-entity-order` (tuple of index + reverse flag)

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
