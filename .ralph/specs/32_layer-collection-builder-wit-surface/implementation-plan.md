# Implementation Plan: layer-collection-builder-wit-surface

## Execution Rules

- One atomic step at a time, validated before moving on.
- Land WIT and host scaffolding before SDK / macro work; that prevents bindgen errors from blocking later steps.
- Land the host validation + apply helper before the dispatch wiring, so the test surface exists when dispatch is integrated.
- Do not alter `order_entities_by_nearest_neighbor` in this packet (packet 33).
- Do not change any module's behavior beyond signature compatibility (packet 33).

## Steps

### Step 1: Declare `layer-collection-builder` resource in WIT

- Task IDs:
  - `TASK-152g`
- Objective:
  Add the new resource to the canonical disk WIT.
- Precondition:
  `wit/deps/ir-types.wit` declares `gcode-output-builder` (lines 106–118) and no `layer-collection-builder` resource.
- Postcondition:
  `wit/deps/ir-types.wit` contains `resource layer-collection-builder { set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>; }` placed alongside the existing builder resources.
- Files expected to change:
  - `wit/deps/ir-types.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- Verification:
  - `grep -E "resource layer-collection-builder" wit/deps/ir-types.wit`
  - `grep -E "set-entity-order:\\s*func\\(items:\\s*list<tuple<u32,\\s*bool>>\\)" wit/deps/ir-types.wit`
- Exit condition:
  Both grep commands return a non-empty match.

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

### Step 3: Wire the host bindings (resource type + trait method)

- Task IDs:
  - `TASK-152g`
- Objective:
  Add the host-side resource backing and `set-entity-order` host trait method without touching dispatch yet.
- Precondition:
  Steps 1–2 complete.
- Postcondition:
  `crates/slicer-host/src/wit_host.rs` declares `pub struct LayerCollectionBuilderData;`, adds `pub layer_collection_proposal: Option<Vec<(u32, bool)>>` to `HostExecutionContext` (initialized to `None` and reset by the constructor `push_layer_collection_builder`), and `impl ir::HostLayerCollectionBuilder for HostExecutionContext` provides `set_entity_order(&mut self, _self_, items) -> wasmtime::Result<Result<(), String>>` that:
  - returns `Ok(Err("set-entity-order called twice within one run-path-optimization".into()))` if `self.layer_collection_proposal.is_some()`
  - otherwise stores `Some(items.into_iter().map(|(i, r)| (i, r)).collect())` and returns `Ok(Ok(()))`
  A new `pub fn push_layer_collection_builder(&mut self) -> wasmtime::Result<Resource<LayerCollectionBuilderData>>` constructor exists and resets `layer_collection_proposal = None`.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo check -p slicer-host`
- Exit condition:
  `cargo check -p slicer-host` succeeds; `wasmtime::component::bindgen!` accepts the new resource (no missing host-trait-impl errors).

### Step 4: Add the `apply_entity_order_proposal` host helper

- Task IDs:
  - `TASK-152g`
- Objective:
  Provide a single, test-callable function that validates a proposal and applies it to `LayerCollectionIR.ordered_entities`. Validation and application live in one place so dispatch wiring is trivial.
- Precondition:
  Step 3 complete.
- Postcondition:
  `crates/slicer-host/src/dispatch.rs` exports `pub fn apply_entity_order_proposal(arena: &mut LayerArena, proposal: &[(u32, bool)]) -> Result<(), String>`. It validates in this order:
  1. `proposal.len() == ordered_entities.len()` — else `Err("set-entity-order: expected N indices, got M")`
  2. each index in `[0, N)` — else `Err("set-entity-order: index N out of range [0, M)")`
  3. no duplicate indices — else `Err("set-entity-order: duplicate index N")`
  Only on `Ok` does it (a) build a fresh `Vec<PrintEntity>` by cloning entities into the proposed order, (b) reverse `path.points` for entries whose flag is `true`, (c) reassign `topo_order` to each entity's new 0-based slot, (d) `mem::replace` the arena's `ordered_entities`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/lib.rs` (re-export `apply_entity_order_proposal`)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
- Verification:
  - `cargo check -p slicer-host`
- Exit condition:
  `apply_entity_order_proposal` exists, is `pub`, and has the validation order stated above. `cargo check -p slicer-host` succeeds.

### Step 5: Add the failing host validation tests (TDD red)

- Task IDs:
  - `TASK-152g`
- Objective:
  Write the host integration test file that pins every validation branch and the success path.
- Precondition:
  Step 4 complete; the helper exists and is reachable from `slicer-host` integration tests.
- Postcondition:
  New file `crates/slicer-host/tests/layer_collection_builder_tdd.rs` contains six tests: `valid_permutation_is_applied_to_ordered_entities`, `reversal_flag_reverses_path_points_in_place`, `duplicate_index_is_rejected_with_fatal_diagnostic`, `out_of_range_index_is_rejected_with_fatal_diagnostic`, `wrong_length_proposal_is_rejected_with_fatal_diagnostic`, `malformed_proposal_leaves_ordered_entities_unchanged`. Each test sets up a 1- or 3-entity `LayerArena`, calls `apply_entity_order_proposal`, and asserts on the post-call state (start-x sequence, topo_order, point reversal) or on the exact error substring.
- Files expected to change:
  - `crates/slicer-host/tests/layer_collection_builder_tdd.rs` (new)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
- Verification:
  - `cargo test -p slicer-host --test layer_collection_builder_tdd 2>&1 | grep "test result:"`
- Exit condition:
  All six tests pass. (Step 4 already implemented validation, so they should be green on first run.) If any test fails, fix the helper before proceeding.

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

### Step 7: Add the SDK guest type and macro plumbing

- Task IDs:
  - `TASK-152g`
- Objective:
  Make the new resource reachable from guest modules through the SDK and `#[slicer_module]` macro.
- Precondition:
  Step 6 complete.
- Postcondition:
  - `crates/slicer-sdk/src/postpass_builders.rs` (or new sibling) contains `pub struct LayerCollectionBuilder { proposal: Option<Vec<(u32, bool)>> }` with `pub fn new()`, `pub fn set_entity_order(&mut self, items: Vec<(u32, bool)>) -> Result<(), String>` (rejects a second call), and an internal `pub(crate) fn proposal(&self) -> Option<&[(u32, bool)]>` accessor used by the macro drain.
  - `crates/slicer-sdk/src/lib.rs` re-exports `LayerCollectionBuilder`.
  - `crates/slicer-sdk/src/traits.rs` updates `LayerModule::run_path_optimization` to accept `_collection: &mut LayerCollectionBuilder` (default body unchanged).
  - `crates/slicer-macros/src/lib.rs` updates the embedded layer-module WIT, adds `__slicer_drain_layer_collection(sdk_builder, wit_resource)` (calls `wit.set_entity_order(items)` only when the SDK builder has a `Some` proposal), and updates the macro expansion of `run_path_optimization` to construct an SDK `LayerCollectionBuilder::new()`, pass it as `&mut` into the trait method, and drain on return.
- Files expected to change:
  - `crates/slicer-sdk/src/postpass_builders.rs` (or new `layer_collection_builder.rs`)
  - `crates/slicer-sdk/src/lib.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo build -p slicer-sdk`
  - `cargo build -p slicer-macros`
- Exit condition:
  Both crates build. The macro's embedded WIT contains both the resource declaration and the updated export signature.

### Step 8: Sweep-update existing `LayerModule::run_path_optimization` impls and rebuild WASM

- Task IDs:
  - `TASK-152g`
- Objective:
  Keep the workspace compiling after the trait-method signature change and rebuild every core WASM artifact against the new WIT.
- Precondition:
  Step 7 complete.
- Postcondition:
  - Every in-tree `impl LayerModule for ... { fn run_path_optimization(...) }` accepts the new `_collection: &mut LayerCollectionBuilder` parameter (or `_collection: &mut slicer_sdk::LayerCollectionBuilder`) — the parameter is named with a leading underscore so it stays unused.
  - At minimum: `modules/core-modules/path-optimization-default/src/lib.rs` is updated. The grep below should show zero `run_path_optimization` impls without the new parameter.
  - Every existing test guest under `test-guests/` that implements `run_path_optimization` is updated similarly.
  - `./modules/core-modules/build-core-modules.sh` runs to completion and rebuilds all `.wasm` artifacts.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - any other `modules/core-modules/*/src/lib.rs` and `test-guests/*/src/lib.rs` that implement `run_path_optimization`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- Verification:
  - `cargo build --workspace`
  - `./modules/core-modules/build-core-modules.sh`
  - `! grep -RIn "fn run_path_optimization" modules/ test-guests/ | grep -v "LayerCollectionBuilder"`
- Exit condition:
  Workspace builds cleanly, build-core-modules.sh succeeds, and grep confirms no impl is missing the new parameter.

### Step 9: Update WIT drift-detection regression

- Task IDs:
  - `TASK-152g`
- Objective:
  Lock the macro-embedded WIT against the new on-disk surface.
- Precondition:
  Steps 1–8 complete.
- Postcondition:
  `crates/slicer-host/tests/wit_drift_detection_tdd.rs` adds `macro_embeds_layer_collection_builder_resource` asserting the macro source contains both `resource layer-collection-builder` and `collection: layer-collection-builder` for the `run-path-optimization` export.
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
