---
status: draft
packet: layer-collection-builder-wit-surface
task_ids:
  - TASK-152g
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: layer-collection-builder-wit-surface

## Goal

Introduce a new `layer-collection-builder` WIT resource (the planned host-collection mutation surface from `docs/01_system_architecture.md`) and wire it through the host bindings, SDK guest type, `#[slicer_module]` macro, and `LayerModule::run_path_optimization` trait signature. Provide one method, `set-entity-order(items: list<tuple<u32, bool>>)`, that lets a path-optimization module declare a permutation of `LayerCollectionIR.ordered_entities` and an optional per-entity reversal flag (the OrcaSlicer `pair<size_t, bool>` shape). The host validates the proposal and applies it to the layer's pre-staged ordered entity list and per-entity `path.points`. The packet-18 host-side `order_entities_by_nearest_neighbor` is retained as a fallback when no module emits a proposal — it is removed in packet 33, not here.

## Scope Boundaries

- In scope:
  - new `resource layer-collection-builder` in `wit/deps/ir-types.wit` with one method `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>`
  - new `collection: layer-collection-builder` parameter on `run-path-optimization` in `wit/world-layer.wit`
  - host backing data type `LayerCollectionBuilderData` and host trait impl in `crates/slicer-host/src/wit_host.rs`
  - host resource constructor `push_layer_collection_builder()`
  - dispatch wiring in `crates/slicer-host/src/dispatch.rs` that pushes the new resource into the store, captures the proposal in `HostExecutionContext`, and applies validated ordering + reversal to `arena.layer_collection().ordered_entities` after the module returns
  - SDK guest builder `LayerCollectionBuilder` in `crates/slicer-sdk/src/postpass_builders.rs` (or a new `layer_collection_builder.rs` sibling)
  - `#[slicer_module]` macro: embedded WIT update, new `__slicer_drain_layer_collection` adapter, trait-method bridging
  - trait change: `LayerModule::run_path_optimization` gains `collection: &mut LayerCollectionBuilder` with a default empty body
  - sweep update of every in-tree `LayerModule::run_path_optimization` impl to accept the new parameter (existing modules pass it unused)
  - rebuild all core-modules WASM artifacts via `./modules/core-modules/build-core-modules.sh`
  - host validation rejecting malformed permutations atomically (no partial state)
  - `wit_drift_detection_tdd.rs` updated for the new resource and method
  - new `crates/slicer-host/tests/layer_collection_builder_tdd.rs` covering host validation and ordering+reversal application
- Out of scope:
  - moving the nearest-neighbor algorithm into `path-optimization-default` (packet 33)
  - removing `order_entities_by_nearest_neighbor` from `crates/slicer-host/src/layer_executor.rs` (packet 33)
  - marking packet 18 as `status: superseded` (packet 33)
  - any methods on `layer-collection-builder` other than `set-entity-order`
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

- **Given** the on-disk file `wit/deps/ir-types.wit`, **when** the file is read, **then** it contains a `resource layer-collection-builder` block declaring exactly one method `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>`. | `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit && grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit`
- **Given** the on-disk file `wit/world-layer.wit`, **when** the file is read, **then** the `run-path-optimization` export signature includes a `collection: layer-collection-builder` parameter directly after the existing `output: gcode-output-builder` parameter. | `grep -E "run-path-optimization:\\s*func" wit/world-layer.wit && grep -E "collection:\\s*layer-collection-builder" wit/world-layer.wit`
- **Given** a `HostExecutionContext` set up for a 3-entity layer with raw start-point x values `[30.0, 0.0, 10.0]`, **when** a test calls `commit_layer_outputs_for_test("Layer::PathOptimization", ...)` with a captured ordering proposal `[(2,false),(0,false),(1,false)]`, **then** the resulting `LayerCollectionIR.ordered_entities` start-point x values are exactly `[10.0, 30.0, 0.0]` and `topo_order` values are `[0,1,2]`. | `cargo test -p slicer-host --test layer_collection_builder_tdd valid_permutation_is_applied_to_ordered_entities -- --exact --nocapture`
- **Given** a single-entity layer whose `path.points` has start `(0.0, 0.0, 0.2)` and end `(5.0, 0.0, 0.2)`, **when** a test commits a proposal `[(0, true)]`, **then** the resulting `ordered_entities[0].path.points.first()` x equals `5.0` and `ordered_entities[0].path.points.last()` x equals `0.0` (the points sequence is reversed in place). | `cargo test -p slicer-host --test layer_collection_builder_tdd reversal_flag_reverses_path_points_in_place -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(0,false),(0,false),(1,false)]` (duplicate index `0`), **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: duplicate index 0"` and `LayerCollectionIR.ordered_entities` is left in the pre-call state (no partial mutation). | `cargo test -p slicer-host --test layer_collection_builder_tdd duplicate_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(99,false),(0,false),(1,false)]`, **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: index 99 out of range [0, 3)"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd out_of_range_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** a 3-entity layer fixture, **when** a test commits a proposal `[(0,false),(1,false)]` (length 2 ≠ 3 entities), **then** the commit returns a `FatalModule` error whose message contains the literal substring `"set-entity-order: expected 3 indices, got 2"`. | `cargo test -p slicer-host --test layer_collection_builder_tdd wrong_length_proposal_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- **Given** the existing packet-18 test fixture and a `Layer::PathOptimization` runner that does **not** call `set_entity_order`, **when** `execute_per_layer` runs, **then** `LayerCollectionIR.ordered_entities` matches the host-fallback NN ordering (start-point x values `[0.0, 10.0, 30.0]`) — the fallback path remains active. | `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- **Given** the canonical on-disk WIT files, **when** the drift-detection test runs, **then** the `#[slicer_module]` macro's embedded WIT in `crates/slicer-macros/src/lib.rs` contains `resource layer-collection-builder` with `set-entity-order` and references it from the layer-module world's `run-path-optimization` signature. | `cargo test -p slicer-host --test wit_drift_detection_tdd macro_embeds_layer_collection_builder_resource -- --exact --nocapture`
- **Given** the workspace after this packet's edits, **when** `cargo build --workspace` runs, **then** the build succeeds with zero errors and every existing `LayerModule::run_path_optimization` impl has been updated to accept the new `collection: &mut LayerCollectionBuilder` parameter. | `cargo build --workspace 2>&1 | tee /tmp/build.log && ! grep -E "error\\[E" /tmp/build.log`
- **Given** the workspace after this packet's edits, **when** `./modules/core-modules/build-core-modules.sh` runs, **then** all `.wasm` artifacts under `modules/core-modules/*/` rebuild successfully (`*.wasm` files exist and have `mtime` newer than the start of the run). | `./modules/core-modules/build-core-modules.sh && ls modules/core-modules/path-optimization-default/path-optimization-default.wasm`

## Negative Test Cases

- **Given** a malformed proposal (duplicate, out-of-range, or wrong length), **when** the host applies it, **then** the layer's `LayerCollectionIR.ordered_entities` is unchanged (no partial reorder, no partial reversal). | `cargo test -p slicer-host --test layer_collection_builder_tdd malformed_proposal_leaves_ordered_entities_unchanged -- --exact --nocapture`

## Verification

- `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit`
- `grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit`
- `grep -E "collection:\\s*layer-collection-builder" wit/world-layer.wit`
- `cargo test -p slicer-host --test layer_collection_builder_tdd valid_permutation_is_applied_to_ordered_entities -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd reversal_flag_reverses_path_points_in_place -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd duplicate_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd out_of_range_index_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd wrong_length_proposal_is_rejected_with_fatal_diagnostic -- --exact --nocapture`
- `cargo test -p slicer-host --test layer_collection_builder_tdd malformed_proposal_leaves_ordered_entities_unchanged -- --exact --nocapture`
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
