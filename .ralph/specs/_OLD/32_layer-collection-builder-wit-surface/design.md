# Design: layer-collection-builder-wit-surface

## Controlling Code Paths

- WIT surface: `wit/deps/ir-types.wit` (resource declaration), `wit/world-layer.wit` (export-signature change)
- Host bindings: `crates/slicer-host/src/wit_host.rs` (resource backing data + host trait impl), `crates/slicer-host/src/dispatch.rs` (resource creation + post-call validation/application)
- SDK guest builder: `crates/slicer-sdk/src/postpass_builders.rs` (extend) or new `crates/slicer-sdk/src/layer_collection_builder.rs` (sibling module)
- Macro codegen: `crates/slicer-macros/src/lib.rs` — embedded WIT block (line ~2810–2857), `__slicer_drain_*` adapters (line ~2672–2727), `LayerModule::run_path_optimization` bridging (line ~2115–2125)
- Trait surface: `crates/slicer-sdk/src/traits.rs` — `LayerModule::run_path_optimization` signature (lines 327–346)
- Existing module sweep: every `LayerModule::run_path_optimization` impl in `modules/core-modules/*/src/lib.rs` and any test guest under `test-guests/`
- Neighboring tests: `crates/slicer-host/tests/wit_drift_detection_tdd.rs`, `crates/slicer-host/tests/path_ordering_tdd.rs`, new `crates/slicer-host/tests/layer_collection_builder_tdd.rs`
- Host fallback (kept for now): `crates/slicer-host/src/layer_executor.rs` — `order_entities_by_nearest_neighbor` and its call site in `execute_single_layer`
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` (`vector<pair<size_t, bool>>` shape)

## Architecture Constraints

- Selected approach: introduce a dedicated `layer-collection-builder` resource as a new parameter on `run-path-optimization`. The host owns `LayerCollectionIR.ordered_entities`; the module's call to `set-entity-order` is a *proposal* the host validates and applies after the module returns. Validation runs **before any mutation** so a malformed proposal leaves the layer state unchanged. The packet-18 host fallback remains active when no proposal is emitted.
- The same resource exposes a synchronous read accessor `get-ordered-entities() -> list<ordered-entity-view>` so the module can enumerate the entities it must permute. This is required because `run-path-optimization`'s existing `regions: list<perimeter-region-view>` parameter only carries perimeter wall-loops; the host-staged `LayerCollectionIR.ordered_entities` is a mixed list of perimeters + infill + support entities, and a `set-entity-order` proposal must carry exactly one entry per existing entity. The read accessor is total: an empty list means "no `LayerCollectionIR` is staged on the arena" — there is no error path. (In live dispatch the dispatch arm always pre-stages a `LayerCollectionIR` before calling the module, so the empty case is observable only in direct host-side tests.)
- The application is unreleased, so `slicer:world-layer` and `slicer:ir-types` do **not** require version bumps even though the export signature change is technically breaking. Existing modules will be swept in this packet to match the new signature, and all WASM binaries will be rebuilt before packet close.
- The host stores a single optional ordering proposal per call (`HostExecutionContext.layer_collection_proposal: Option<Vec<(u32, bool)>>`). Multiple `set-entity-order` calls within one `run-path-optimization` are rejected as a contract violation (the second call returns `Err` from the WIT-level method).
- Reversal mutates `path.points` in place via `Vec::reverse()` after the entity has been moved to its post-permutation slot. Per-point payloads (`width`, `flow_factor`) reverse with the points — this is correct for a reversed extrusion.
- `topo_order` is reassigned post-permutation to the 0-based slot index. `region_key`, `role`, `speed_factor` are preserved.

## Code Change Surface

- Selected approach:
  - one new WIT resource declaration with two methods; one new WIT record `ordered-entity-view`; one new export parameter; one new host-side resource type with two trait methods; one new SDK guest type carrying both a write proposal and a read snapshot; one new SDK record mirroring the WIT view; one new macro drain helper that populates the read snapshot at call entry and drains the write proposal at call exit; one trait-method signature change; one host post-call apply step in dispatch; one new integration test file
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `wit/deps/ir-types.wit`:
    - add `record ordered-entity-view { original-index: u32, region-key: region-key, role: extrusion-role, start-point: point3-with-width, end-point: point3-with-width, point-count: u32 }`
    - add `resource layer-collection-builder { set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>; get-ordered-entities: func() -> list<ordered-entity-view>; }`
  - `wit/world-layer.wit` — add import alias `layer-collection-builder` and `ordered-entity-view` to the existing `ir-handles` import block; add `collection: layer-collection-builder` parameter to `export run-path-optimization`
  - `crates/slicer-host/src/wit_host.rs`:
    - new `pub struct LayerCollectionBuilderData { pub ordered_entities: Vec<crate::dispatch::OrderedEntityView> }` carrying the per-call snapshot (eager-capture: dispatch projects `LayerCollectionIR.ordered_entities` once at `push_layer_collection_builder` time and stashes the result on the resource backing, so the trait method serves repeated reads from the resource without re-touching the live arena)
    - new field `pub layer_collection_proposal: Option<Vec<(u32, bool)>>` on `HostExecutionContext`
    - new field `pub(crate) host_get_ordered_entities_call_count: u32` on `HostExecutionContext`, reset to `0` by `push_layer_collection_builder()` and incremented at the top of `HostLayerCollectionBuilder::get_ordered_entities` (per-context observation point retained for direct host-side tests)
    - new `#[doc(hidden)] pub fn host_get_ordered_entities_call_count(&self) -> u32` getter on `HostExecutionContext` for tests
    - new `pub static HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS: AtomicU32` (cross-call observation point used by the macro-call-once acceptance test, since `execute_per_layer` consumes the store and the per-context counter is unreachable from a layer-level integration test). Both counters increment in lockstep at the top of `get_ordered_entities`; the static counter is reset by tests via `.store(0, Ordering::SeqCst)` before exercising a layer.
    - new `impl ir::HostLayerCollectionBuilder for HostExecutionContext` with two methods:
      - `set_entity_order(&mut self, _self_, items)` — validates "no second call" (returns `Err` on duplicate emission) and stores the proposal
      - `get_ordered_entities(&mut self, self_)` — increments both `host_get_ordered_entities_call_count` and `HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS`, then reads the snapshot stashed on `LayerCollectionBuilderData.ordered_entities` (captured at `push_layer_collection_builder` time) and maps each entry to the wasmtime-bindgen `ir::OrderedEntityView`
    - new `pub fn push_layer_collection_builder(&mut self, ordered_entities: Vec<crate::dispatch::OrderedEntityView>) -> wasmtime::Result<Resource<LayerCollectionBuilderData>>` constructor (takes the projected snapshot from dispatch, stashes it on the resource, and resets both `layer_collection_proposal` and `host_get_ordered_entities_call_count`)
  - `crates/slicer-host/src/dispatch.rs`:
    - in the `"Layer::PathOptimization"` arm of `dispatch_layer_call`: call `project_ordered_entities(arena)` first to capture the snapshot, push the new resource into the store via `push_layer_collection_builder(snapshot)`, pass it to `call_run_path_optimization` as the new `collection` argument, then **after** the call read back `HostExecutionContext.layer_collection_proposal` and either:
      - if `Some(proposal)`: validate against `arena.layer_collection().ordered_entities.len()`. On `Ok` apply the permutation + reversal to a fresh `Vec<PrintEntity>`, replace `arena.layer_collection_mut().ordered_entities`, and reassign `topo_order`. On `Err` return a `LayerStageError::FatalModule { message: validation_error.to_string() }`.
      - if `None`: leave `ordered_entities` as the host fallback already produced — packet-18 behavior is preserved.
    - introduce a small `pub fn apply_entity_order_proposal(arena: &mut LayerArena, proposal: &[(u32, bool)]) -> Result<(), String>` helper for direct test access (mirrors `commit_layer_outputs_for_test`)
    - introduce `pub fn project_ordered_entities(arena: &LayerArena) -> Vec<OrderedEntityView>` returning one view per `LayerCollectionIR.ordered_entities` entry (in `original_index` order). Empty `Vec` when `arena.layer_collection()` is `None`. Each view projects: `original_index = i`, `region_key = entity.region_key.clone()`, `role = entity.path.role`, `start_point = entity.path.points.first().clone()`, `end_point = entity.path.points.last().clone()`, `point_count = entity.path.points.len() as u32`. Asserts `points` is non-empty (an existing `PrintEntity` invariant); a fixture with empty points is a host-side bug elsewhere.
    - the dispatch arm calls `project_ordered_entities` exactly once per `Layer::PathOptimization` invocation; the trait method's `get_ordered_entities` reads from the stashed snapshot rather than re-projecting per call.
  - `crates/slicer-sdk/src/views.rs`:
    - new `pub struct OrderedEntityView { pub original_index: u32, pub region_key: RegionKey, pub role: ExtrusionRole, pub start_point: Point3WithWidth, pub end_point: Point3WithWidth, pub point_count: u32 }` with field-order matching the WIT record
  - `crates/slicer-sdk/src/layer_collection_builder.rs`:
    - extend `LayerCollectionBuilder` to also hold `ordered_entities: Vec<OrderedEntityView>` (snapshot populated by the macro before the trait method runs)
    - add `pub fn get_ordered_entities(&self) -> &[OrderedEntityView]` accessor
    - add a doc-hidden `pub fn set_ordered_entities(&mut self, snapshot: Vec<OrderedEntityView>)` constructor used by the macro drain
  - `crates/slicer-sdk/src/lib.rs` — re-export the new SDK types
  - `crates/slicer-sdk/src/traits.rs` — `LayerModule::run_path_optimization` gains `collection: &mut LayerCollectionBuilder` (default body unchanged → still returns `Ok(())`)
  - `crates/slicer-macros/src/lib.rs`:
    - update embedded layer-module WIT (line ~2810–2857) to include the new record, the new resource with both methods, and the updated export signature
    - add a `__slicer_drain_layer_collection(sdk_builder, wit_resource)` adapter that:
      - **before** invoking the trait method: calls `wit_resource.get_ordered_entities()` and stores the result via `sdk_builder.set_ordered_entities(snapshot)` so the module sees a populated read snapshot
      - **after** the trait method returns: if `sdk_builder.proposal()` is `Some(items)`, calls `wit_resource.set_entity_order(items)` once
    - update the `run_path_optimization` macro expansion (line ~2115–2125) accordingly
  - existing impls of `LayerModule::run_path_optimization` (sweep-update with default ignored binding):
    - `modules/core-modules/path-optimization-default/src/lib.rs`
    - any other module under `modules/core-modules/*/src/lib.rs` that implements `LayerModule::run_path_optimization` (likely zero — only `path-optimization-default` ships an impl in tree, but the sweep is part of acceptance)
    - any test guest under `test-guests/` that implements `LayerModule::run_path_optimization`
  - new test guest `test-guests/path-optimization-multi-read/`:
    - `src/lib.rs` declares a `#[slicer_module]` `LayerModule` impl whose `run_path_optimization` calls `collection.get_ordered_entities()` exactly 5 times, captures each returned slice in a local `Vec<Vec<OrderedEntityView>>`, and asserts every snapshot equals the first via `assert_eq!`. On mismatch the module traps with a panic message containing `"path-optimization-multi-read: snapshot drifted across calls"` so the host test can detect SDK-cache inconsistency. The module emits no `set_entity_order` proposal (so the host fallback runs after dispatch).
    - `Cargo.toml` mirrors the shape of the existing `test-guests/*` crates (test-guests do not ship a `manifest.toml` — the host integration test loads the produced `.component.wasm` directly via `WasmEngine::compile_component`)
    - the guest is rebuilt by `./test-guests/build-test-guests.sh` (Step 8 extends that script's `GUESTS` array; `modules/core-modules/build-core-modules.sh` covers only the manifest-shipped core modules under `modules/core-modules/`)
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — extend `macro_embeds_layer_collection_builder_resource` to assert the macro's embedded WIT contains the resource declaration with **both** methods, the `record ordered-entity-view` declaration, and the export-signature parameter
  - new `crates/slicer-host/tests/layer_collection_builder_tdd.rs` — host validation + ordering/reversal application tests using `apply_entity_order_proposal` (the test-side handle introduced in dispatch.rs above), plus three read-projection tests using `project_ordered_entities` (`get_ordered_entities_projects_staged_entities_in_index_order`, `get_ordered_entities_carries_endpoints_and_point_count`, `get_ordered_entities_returns_empty_when_no_layer_collection_is_staged`)
- Rejected alternatives:
  - adding `set-entity-order` as a method on `gcode-output-builder`: rejected because `gcode-output-builder` is semantically a GCode-emission surface; bundling host-collection mutation onto it would mix concerns and make the `apply_entity_order_proposal` boundary harder to reason about. The new resource is the planned long-term surface per `docs/01`.
  - adding the proposal as a return value of `run-path-optimization` (`-> result<option<entity-order>, module-error>`): rejected because it would force every existing module to return an explicit value and changes the export's result type — much larger blast radius than adding a parameter.
  - bumping `slicer:world-layer` to `@2.0.0`: skipped because the application is unreleased; an in-place breaking change with a one-shot module sweep is acceptable.
  - widening `run-path-optimization`'s `regions` parameter to also carry infill and support entities (e.g., a new `entities: list<print-entity-view>` argument): rejected because it conflates the perimeter-region read surface with the host-collection ordering surface. `regions` continues to describe the perimeter wall-loop view used by other path-optimization concerns; the read accessor lives on the same `layer-collection-builder` resource that owns the write surface, keeping ordering inputs and outputs co-located.
  - returning a resource handle from `get-ordered-entities` (one resource per view) with per-view accessors: rejected because every existing read view (e.g., `slice-region-view`) is heavyweight by necessity (large polygon lists). `ordered-entity-view` is a flat 6-field record; resource overhead is unjustified and makes algorithm code more verbose.
  - exposing `path.points: list<point3-with-width>` in full on `ordered-entity-view`: deferred. The flat `start-point` + `end-point` + `point-count` summary is sufficient for the NN algorithm and any reversal-aware tiebreak. Modules that need the full path can be addressed by a future packet that adds an explicit accessor.

## Files Out of Bounds / Read-Only Context

- **Read-only context (consult, do not edit):**
  - authoritative docs `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` (parity reference for `vector<pair<size_t, bool>>` shape; must not be modified — it is upstream documentation)
  - existing world WITs not in scope: `wit/world-prepass.wit`, `wit/world-postpass.wit`, `wit/world-finalization.wit` (only `wit/world-layer.wit` and `wit/deps/ir-types.wit` are mutated)
  - existing fallback algorithm `order_entities_by_nearest_neighbor` and its call site in `crates/slicer-host/src/layer_executor.rs` (kept verbatim in this packet; removed in packet 33 only)
  - `docs/07_implementation_status.md` (only the `TASK-152` group is consulted; no other rows are read or edited)
- **Out of bounds entirely (do not read or load):**
  - `target/` and any cargo build artifacts
  - any `Cargo.lock` (workspace-level or per-crate)
  - generated WASM artifacts under `modules/core-modules/*/*.wasm` and `test-guests/*.component.wasm` (rebuilt by their respective scripts; never hand-edited)
  - vendored dependencies under any `vendor/` or `third_party/` directories
  - all of `OrcaSlicerOriginal/` (use `OrcaSlicerDocumented/` for documented references)

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

## Open Questions

- None. The selected approach, the WIT method shape, the validation order, the fallback behavior, and the test surface are decided in this packet. Packet 33 will revisit the fallback removal.
