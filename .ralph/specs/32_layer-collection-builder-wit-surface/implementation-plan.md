# Implementation Plan: layer-collection-builder-wit-surface

## Execution Rules

- One atomic step at a time, validated before moving on.
- Land WIT and host scaffolding before SDK / macro work; that prevents bindgen errors from blocking later steps.
- Land the host validation + apply helper before the dispatch wiring, so the test surface exists when dispatch is integrated.
- Do not alter `order_entities_by_nearest_neighbor` in this packet (packet 33).
- Do not change any module's behavior beyond signature compatibility (packet 33).

## Steps

### Step 1: Declare `layer-collection-builder` resource and `ordered-entity-view` record in WIT

- Task IDs:
  - `TASK-152g`
- Objective:
  Add the new resource (with both methods) and the new view record to the canonical disk WIT.
- Precondition:
  `wit/deps/ir-types.wit` declares `gcode-output-builder` (lines 106–118) and no `layer-collection-builder` resource.
- Postcondition:
  `wit/deps/ir-types.wit` contains:
  - `record ordered-entity-view { original-index: u32, region-key: region-key, role: extrusion-role, start-point: point3-with-width, end-point: point3-with-width, point-count: u32 }`
  - `resource layer-collection-builder { set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>; get-ordered-entities: func() -> list<ordered-entity-view>; }`
  Both placed alongside the existing builder resources, the record declared before the resource that references it. The existing `extrusion-role` import already covers the role field.
- Files expected to change:
  - `wit/deps/ir-types.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- Verification:
  - `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit`
  - `grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit`
  - `grep -E "get-ordered-entities:\\s*func\\(\\)\\s*->\\s*list<ordered-entity-view>" wit/deps/ir-types.wit`
  - `grep -E "record ordered-entity-view" wit/deps/ir-types.wit`
- Exit condition:
  All four grep commands return a non-empty match.

### Step 2: Add the new parameter to `run-path-optimization` in `world-layer.wit`

- Task IDs:
  - `TASK-152g`
- Objective:
  Surface the new resource through the layer-module world.
- Precondition:
  Step 1 complete; `wit/world-layer.wit` already imports the other builder aliases and declares `run-path-optimization`.
- Postcondition:
  `wit/world-layer.wit` imports `layer-collection-builder` from `slicer:ir-types/ir-handles` and the `run-path-optimization` export signature carries `collection: layer-collection-builder` immediately after `output: gcode-output-builder`.
- Files expected to change:
  - `wit/world-layer.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/04_host_scheduler.md`
- Verification:
  - `grep -E "layer-collection-builder" wit/world-layer.wit`
  - `grep -E "collection:\\s*layer-collection-builder" wit/world-layer.wit`
- Exit condition:
  Both grep commands match. The file still parses as valid WIT (verified in Step 3 via cargo build).

### Step 3: Wire the host bindings (resource type + both trait methods)

- Task IDs:
  - `TASK-152g`
- Objective:
  Add the host-side resource backing and both host trait methods (`set_entity_order`, `get_ordered_entities`) without touching dispatch yet.
- Precondition:
  Steps 1–2 complete.
- Postcondition:
  `crates/slicer-host/src/wit_host.rs`:
  - declares `pub struct LayerCollectionBuilderData;`
  - adds `pub layer_collection_proposal: Option<Vec<(u32, bool)>>` to `HostExecutionContext` (initialized to `None` and reset by the constructor `push_layer_collection_builder`)
  - adds `pub(crate) host_get_ordered_entities_call_count: u32` to `HostExecutionContext` (initialized to `0` and reset by `push_layer_collection_builder`); exposes `#[doc(hidden)] pub fn host_get_ordered_entities_call_count(&self) -> u32` for tests
  - `impl ir::HostLayerCollectionBuilder for HostExecutionContext` provides:
    - `set_entity_order(&mut self, _self_, items) -> wasmtime::Result<Result<(), String>>` that returns `Ok(Err("set-entity-order called twice within one run-path-optimization".into()))` if `self.layer_collection_proposal.is_some()`, otherwise stores `Some(items.into_iter().map(|(i, r)| (i, r)).collect())` and returns `Ok(Ok(()))`
    - `get_ordered_entities(&mut self, _self_) -> wasmtime::Result<Vec<ir::OrderedEntityView>>` that **first** increments `self.host_get_ordered_entities_call_count`, then delegates to `dispatch::project_ordered_entities(self.current_arena())` (the helper introduced in Step 4) and maps the SDK-shaped `OrderedEntityView` to the wasmtime-bindgen `ir::OrderedEntityView` type. Returns an empty `Vec` when no `LayerCollectionIR` is staged (do not error).
  - new `pub fn push_layer_collection_builder(&mut self) -> wasmtime::Result<Resource<LayerCollectionBuilderData>>` constructor exists and resets both `layer_collection_proposal = None` and `host_get_ordered_entities_call_count = 0`.
  - if `HostExecutionContext` does not already carry an arena handle reachable from trait methods, add one (mirror the pattern used by other read views — e.g., the way `slice-region-view` accessors thread arena state).
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo check -p slicer-host`
- Exit condition:
  `cargo check -p slicer-host` succeeds; `wasmtime::component::bindgen!` accepts the new resource (no missing host-trait-impl errors).

### Step 4: Add the host helpers `apply_entity_order_proposal` (write) and `project_ordered_entities` (read)

- Task IDs:
  - `TASK-152g`
- Objective:
  Provide two single-purpose, test-callable functions: one that validates a proposal and applies it to `LayerCollectionIR.ordered_entities`, and one that projects the staged ordered entities into a flat `Vec<OrderedEntityView>`. Validation, application, and projection live in one place so dispatch and host-trait wiring are trivial.
- Precondition:
  Step 3 complete.
- Postcondition:
  `crates/slicer-host/src/dispatch.rs` exports:
  - `pub fn apply_entity_order_proposal(arena: &mut LayerArena, proposal: &[(u32, bool)]) -> Result<(), String>` validating in this order:
    1. `proposal.len() == ordered_entities.len()` — else `Err("set-entity-order: expected N indices, got M")`
    2. each index in `[0, N)` — else `Err("set-entity-order: index N out of range [0, M)")`
    3. no duplicate indices — else `Err("set-entity-order: duplicate index N")`
    Only on `Ok` does it (a) build a fresh `Vec<PrintEntity>` by cloning entities into the proposed order, (b) reverse `path.points` for entries whose flag is `true`, (c) reassign `topo_order` to each entity's new 0-based slot, (d) `mem::replace` the arena's `ordered_entities`.
  - `pub fn project_ordered_entities(arena: &LayerArena) -> Vec<OrderedEntityView>` that returns one `OrderedEntityView` per `LayerCollectionIR.ordered_entities` entry (in `original_index` order). Each view projects: `original_index = i as u32`, `region_key = entity.region_key.clone()`, `role = entity.path.role`, `start_point = entity.path.points.first().expect("PrintEntity invariant: path.points non-empty").clone()`, `end_point = entity.path.points.last().expect(...).clone()`, `point_count = entity.path.points.len() as u32`. When `arena.layer_collection()` is `None`, returns an empty `Vec`. The function does not allocate beyond the result `Vec` and the per-entity `region_key` clones.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/lib.rs` (re-export `apply_entity_order_proposal` and `project_ordered_entities`)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
- Verification:
  - `cargo check -p slicer-host`
- Exit condition:
  Both helpers exist, are `pub`, and behave as stated. `cargo check -p slicer-host` succeeds.

### Step 5: Add the host validation, read-projection, and SDK-cache tests

- Task IDs:
  - `TASK-152g`
- Objective:
  Write the host integration test file that pins every write-validation branch, the write success path, the read-projection contract, and the SDK-cache contract (the macro-call-once test is added in Step 8 alongside the test guest it requires).
- Precondition:
  Step 4 complete; both helpers exist and are reachable from `slicer-host` integration tests.
- Postcondition:
  New file `crates/slicer-host/tests/layer_collection_builder_tdd.rs` contains nine tests:
  - write-side: `valid_permutation_is_applied_to_ordered_entities`, `reversal_flag_reverses_path_points_in_place`, `duplicate_index_is_rejected_with_fatal_diagnostic`, `out_of_range_index_is_rejected_with_fatal_diagnostic`, `wrong_length_proposal_is_rejected_with_fatal_diagnostic`, `malformed_proposal_leaves_ordered_entities_unchanged`
  - read-side: `get_ordered_entities_projects_staged_entities_in_index_order`, `get_ordered_entities_carries_endpoints_and_point_count`, `get_ordered_entities_returns_empty_when_no_layer_collection_is_staged`
  Each write test sets up a 1- or 3-entity `LayerArena`, calls `apply_entity_order_proposal`, and asserts on the post-call state (start-x sequence, topo_order, point reversal) or on the exact error substring. Each read test sets up a `LayerArena` (or leaves it without `LayerCollectionIR`), calls `project_ordered_entities`, and asserts on the projected `Vec<OrderedEntityView>` (length, per-entry `original_index`/`region_key`/`role`/`start_point`/`end_point`/`point_count`).
  In addition, `crates/slicer-sdk/tests/layer_module_tdd.rs` gains `layer_collection_builder_get_ordered_entities_reads_local_cache`: it constructs a `LayerCollectionBuilder::new()`, calls `set_ordered_entities(snapshot)` with a 3-entity fixture, then calls `get_ordered_entities()` twice and asserts the two slices are equal in content. The SDK type is structurally cache-only (no WIT resource field), so the test pins the public observable contract without instrumenting internals.
- Files expected to change:
  - `crates/slicer-host/tests/layer_collection_builder_tdd.rs` (new)
  - `crates/slicer-sdk/tests/layer_module_tdd.rs` (add the cache test)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo test -p slicer-host --test layer_collection_builder_tdd 2>&1 | grep "test result:"`
  - `cargo test -p slicer-sdk --test layer_module_tdd layer_collection_builder_get_ordered_entities_reads_local_cache -- --exact --nocapture`
- Exit condition:
  All nine host tests and the SDK cache test pass. (Step 4 implemented both helpers; Step 7 will land the SDK type — order this step's SDK test addition after Step 7 if needed.) If any test fails, fix the helper before proceeding.

### Step 6: Wire dispatch to push the resource and apply the proposal

- Task IDs:
  - `TASK-152g`
- Objective:
  Make the live dispatch path actually pass the new resource into the call and consume the proposal afterward.
- Precondition:
  Steps 3–5 complete.
- Postcondition:
  In `crates/slicer-host/src/dispatch.rs` the `"Layer::PathOptimization"` arm of `dispatch_layer_call`:
  1. calls `push_layer_collection_builder()` and `push_gcode_output_builder()` and forwards both to `call_run_path_optimization(store, layer_index, &region_handles, own(output), own(collection), own(config))`
  2. after the call, takes `HostExecutionContext.layer_collection_proposal.take()` and, if `Some`, calls `apply_entity_order_proposal`. Validation failures map to `LayerStageError::FatalModule { stage_id, module_id, message }` with the validation-error string as the message.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- Verification:
  - `cargo build -p slicer-host`
  - `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- Exit condition:
  Build succeeds. The packet-18 acceptance test still passes — fallback is still active because no module emits a proposal yet.

### Step 7: Add the SDK guest type, view record, and macro plumbing

- Task IDs:
  - `TASK-152g`
- Objective:
  Make the new resource (both methods) reachable from guest modules through the SDK and `#[slicer_module]` macro.
- Precondition:
  Step 6 complete.
- Postcondition:
  - `crates/slicer-sdk/src/views.rs` declares `pub struct OrderedEntityView { pub original_index: u32, pub region_key: RegionKey, pub role: ExtrusionRole, pub start_point: Point3WithWidth, pub end_point: Point3WithWidth, pub point_count: u32 }`.
  - `crates/slicer-sdk/src/layer_collection_builder.rs` contains `pub struct LayerCollectionBuilder { proposal: Option<Vec<(u32, bool)>>, ordered_entities: Vec<OrderedEntityView> }` with:
    - `pub fn new() -> Self` (proposal `None`, ordered_entities empty)
    - `pub fn set_entity_order(&mut self, items: Vec<(u32, bool)>) -> Result<(), String>` (rejects a second call)
    - `pub fn get_ordered_entities(&self) -> &[OrderedEntityView]` reading from the cached snapshot
    - doc-hidden `pub fn set_ordered_entities(&mut self, snapshot: Vec<OrderedEntityView>)` constructor used by the macro drain
    - doc-hidden `pub fn proposal(&self) -> Option<&[(u32, bool)]>` accessor used by the macro drain
  - `crates/slicer-sdk/src/lib.rs` re-exports `LayerCollectionBuilder` and `OrderedEntityView`.
  - `crates/slicer-sdk/src/traits.rs` updates `LayerModule::run_path_optimization` to accept `_collection: &mut LayerCollectionBuilder` (default body unchanged).
  - `crates/slicer-macros/src/lib.rs` updates the embedded layer-module WIT (record + resource with both methods + export signature) and adds `__slicer_drain_layer_collection(sdk_builder, wit_resource)` that:
    1. before invoking the trait method, calls `wit_resource.get_ordered_entities()` and stores the result via `sdk_builder.set_ordered_entities(snapshot)` (mapping the wasmtime-bindgen `OrderedEntityView` to the SDK `OrderedEntityView`)
    2. after the trait method returns, if `sdk_builder.proposal()` is `Some(items)`, calls `wit_resource.set_entity_order(items.to_vec())` exactly once
  - The macro expansion of `run_path_optimization` constructs an SDK `LayerCollectionBuilder::new()`, runs the pre-call drain step, passes `&mut` into the trait method, then runs the post-call drain step.
- Files expected to change:
  - `crates/slicer-sdk/src/layer_collection_builder.rs`
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-sdk/src/lib.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo build -p slicer-sdk`
  - `cargo build -p slicer-macros`
- Exit condition:
  Both crates build. The macro's embedded WIT contains the new record, both resource methods, and the updated export signature.

### Step 8: Sweep-update existing impls, add the multi-read test guest, rebuild WASM, and add the macro-call-once host test

- Task IDs:
  - `TASK-152g`
- Objective:
  Keep the workspace compiling after the trait-method signature change, rebuild every core WASM artifact against the new WIT, add the counting test guest, and pin the macro-call-once contract with a host-side test.
- Precondition:
  Step 7 complete.
- Postcondition:
  - Every in-tree `impl LayerModule for ... { fn run_path_optimization(...) }` accepts the new `_collection: &mut LayerCollectionBuilder` parameter (or `_collection: &mut slicer_sdk::LayerCollectionBuilder`) — the parameter is named with a leading underscore so it stays unused.
  - At minimum: `modules/core-modules/path-optimization-default/src/lib.rs` is updated. The grep below should show zero `run_path_optimization` impls without the new parameter.
  - Every existing test guest under `test-guests/` that implements `run_path_optimization` is updated similarly.
  - New crate `test-guests/path-optimization-multi-read/` is added:
    - `Cargo.toml` mirrors the layout of an existing `test-guests/*` crate (test-guests do not ship a `manifest.toml` — the host integration test loads the produced `.component.wasm` directly via `WasmEngine::compile_component`)
    - `src/lib.rs` declares a `#[slicer_module]` `LayerModule` impl whose `run_path_optimization` body calls `collection.get_ordered_entities()` exactly 5 times, captures each returned slice into a `Vec<Vec<OrderedEntityView>>` (cloning the views), and asserts every snapshot equals the first via `assert_eq!`. On mismatch the module panics with `"path-optimization-multi-read: snapshot drifted across calls"`. The module emits no `set_entity_order` proposal and returns `Ok(())`.
    - the test-guest build script `./test-guests/build-test-guests.sh` is extended (its `GUESTS` array gains a `path-optimization-multi-read:path_optimization_multi_read_guest` entry) so the guest is compiled to `test-guests/path-optimization-multi-read.component.wasm`. `modules/core-modules/build-core-modules.sh` is unchanged — it covers manifest-shipped core modules only.
  - `crates/slicer-host/tests/layer_collection_builder_tdd.rs` gains `macro_drain_invokes_host_get_ordered_entities_exactly_once` that:
    1. loads `path-optimization-multi-read.wasm` and compiles it via `WasmEngine::compile_component`
    2. builds a `Blackboard` + minimal mesh + `ExecutionPlan` whose per-layer stages include `Layer::Infill` (mock seeds a 3-entity infill IR) followed by `Layer::PathOptimization` driving the multi-read guest through `WasmRuntimeDispatcher`
    3. runs `execute_per_layer`
    4. reads `store.data().host_get_ordered_entities_call_count()` and asserts the count equals exactly `1`
    5. ensures the run completed without trapping (no `"snapshot drifted"` panic)
  - `./modules/core-modules/build-core-modules.sh` runs to completion and rebuilds all `.wasm` artifacts including the new test guest.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - any other `modules/core-modules/*/src/lib.rs` and `test-guests/*/src/lib.rs` that implement `run_path_optimization`
  - `test-guests/path-optimization-multi-read/Cargo.toml` (new)
  - `test-guests/path-optimization-multi-read/src/lib.rs` (new)
  - `test-guests/build-test-guests.sh` (add the new guest to the `GUESTS` array)
  - `crates/slicer-host/tests/layer_collection_builder_tdd.rs` (add `macro_drain_invokes_host_get_ordered_entities_exactly_once`)
- Authoritative docs:
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo build --workspace` — authoritative sweep gate. The trait signature change makes `LayerCollectionBuilder` mandatory at the type level, so any in-tree `LayerModule::run_path_optimization` impl missing the new parameter fails compilation. A clean workspace build proves every impl was swept.
  - `./modules/core-modules/build-core-modules.sh`
  - `./test-guests/build-test-guests.sh --check`
  - `cargo test -p slicer-host --test layer_collection_builder_tdd macro_drain_invokes_host_get_ordered_entities_exactly_once -- --exact --nocapture`
- Exit condition:
  Workspace builds cleanly (which is what proves the sweep is complete), both build scripts succeed, and the macro-call-once test passes — `HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS` reads exactly `1` after a 5-call trait-method body.

### Step 9: Update WIT drift-detection regression

- Task IDs:
  - `TASK-152g`
- Objective:
  Lock the macro-embedded WIT against the new on-disk surface (record + resource + both methods + export signature).
- Precondition:
  Steps 1–8 complete.
- Postcondition:
  `crates/slicer-host/tests/wit_drift_detection_tdd.rs` adds `macro_embeds_layer_collection_builder_resource` asserting the macro source contains all of: `resource layer-collection-builder`, `set-entity-order: func(items: list<tuple<u32, bool>>)`, `get-ordered-entities: func() -> list<ordered-entity-view>`, `record ordered-entity-view`, and `collection: layer-collection-builder` for the `run-path-optimization` export.
- Files expected to change:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
- Verification:
  - `cargo test -p slicer-host --test wit_drift_detection_tdd macro_embeds_layer_collection_builder_resource -- --exact --nocapture`
- Exit condition:
  Test green.

### Step 10: Run the packet's full acceptance ceremony

- Task IDs:
  - `TASK-152g`
- Objective:
  Re-run every acceptance command from `packet.spec.md` and prove the host fallback still answers when no proposal is emitted.
- Precondition:
  Steps 1–9 complete.
- Postcondition:
  All commands in `packet.spec.md`'s Verification block pass; `cargo clippy --workspace -- -D warnings` is clean.
- Files expected to change:
  - none
- Verification:
  - run every command in `packet.spec.md` § Verification
- Exit condition:
  Every command succeeds. Document any flake or environmental issue in the deviation log before declaring the packet green.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands in `packet.spec.md` pass.
- `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` are clean.
- `./modules/core-modules/build-core-modules.sh` succeeds.
- `docs/07_implementation_status.md` has a row for `TASK-152g` reflecting the surface introduction (status `[~]` since packet 33 finishes the migration).
- `order_entities_by_nearest_neighbor` is still present in `crates/slicer-host/src/layer_executor.rs` (intentional — packet 33 removes it).

## Acceptance Ceremony

- Re-run every acceptance command from `packet.spec.md`.
- Confirm the packet-18 acceptance test (`reordered_sequence_is_consumed_by_path_optimization_stage`) still passes — proves the host fallback path is intact.
- Confirm all six host validation tests in `layer_collection_builder_tdd.rs` pass.
- Confirm the drift-detection regression covers the new resource.
- Record any remaining packet-local risk (e.g., test guests not exercised) before status changes.
