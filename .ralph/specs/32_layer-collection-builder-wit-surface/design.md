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
- The application is unreleased, so `slicer:world-layer` and `slicer:ir-types` do **not** require version bumps even though the export signature change is technically breaking. Existing modules will be swept in this packet to match the new signature, and all WASM binaries will be rebuilt before packet close.
- The host stores a single optional ordering proposal per call (`HostExecutionContext.layer_collection_proposal: Option<Vec<(u32, bool)>>`). Multiple `set-entity-order` calls within one `run-path-optimization` are rejected as a contract violation (the second call returns `Err` from the WIT-level method).
- Reversal mutates `path.points` in place via `Vec::reverse()` after the entity has been moved to its post-permutation slot. Per-point payloads (`width`, `flow_factor`) reverse with the points — this is correct for a reversed extrusion.
- `topo_order` is reassigned post-permutation to the 0-based slot index. `region_key`, `role`, `speed_factor` are preserved.

## Code Change Surface

- Selected approach:
  - one new WIT resource declaration; one new export parameter; one new host-side resource type with one trait method; one new SDK guest type; one new macro drain helper; one trait-method signature change; one host post-call apply step in dispatch; one new integration test file
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `wit/deps/ir-types.wit` — add `resource layer-collection-builder { set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>; }`
  - `wit/world-layer.wit` — add import alias `layer-collection-builder` to the existing `ir-handles` import block; add `collection: layer-collection-builder` parameter to `export run-path-optimization`
  - `crates/slicer-host/src/wit_host.rs`:
    - new `pub struct LayerCollectionBuilderData;` (zero-sized resource backing, mirrors `GcodeOutputBuilderData`)
    - new field `pub layer_collection_proposal: Option<Vec<(u32, bool)>>` on `HostExecutionContext`
    - new field on `HostExecutionContext` reset on each call
    - new `impl ir::HostLayerCollectionBuilder for HostExecutionContext` with one method `set_entity_order` that validates "no second call" (returns `Err` on duplicate emission) and stores the proposal
    - new `pub fn push_layer_collection_builder(&mut self) -> wasmtime::Result<Resource<LayerCollectionBuilderData>>` constructor
  - `crates/slicer-host/src/dispatch.rs`:
    - in the `"Layer::PathOptimization"` arm of `dispatch_layer_call`: push the new resource into the store, pass it to `call_run_path_optimization` as the new `collection` argument, then **after** the call read back `HostExecutionContext.layer_collection_proposal` and either:
      - if `Some(proposal)`: validate against `arena.layer_collection().ordered_entities.len()`. On `Ok` apply the permutation + reversal to a fresh `Vec<PrintEntity>`, replace `arena.layer_collection_mut().ordered_entities`, and reassign `topo_order`. On `Err` return a `LayerStageError::FatalModule { message: validation_error.to_string() }`.
      - if `None`: leave `ordered_entities` as the host fallback already produced — packet-18 behavior is preserved.
    - introduce a small `pub fn apply_entity_order_proposal(arena: &mut LayerArena, proposal: &[(u32, bool)]) -> Result<(), String>` helper for direct test access (mirrors `commit_layer_outputs_for_test`)
  - `crates/slicer-sdk/src/postpass_builders.rs` (or new sibling file) — `pub struct LayerCollectionBuilder { proposal: Option<Vec<(u32, bool)>> }` with `pub fn set_entity_order(&mut self, items: Vec<(u32, bool)>) -> Result<(), String>` (only one set permitted; second call returns `Err`)
  - `crates/slicer-sdk/src/lib.rs` — re-export the new SDK type
  - `crates/slicer-sdk/src/traits.rs` — `LayerModule::run_path_optimization` gains `collection: &mut LayerCollectionBuilder` (default body unchanged → still returns `Ok(())`)
  - `crates/slicer-macros/src/lib.rs`:
    - update embedded layer-module WIT (line ~2810–2857) to include the new resource and updated export signature
    - add a `__slicer_drain_layer_collection(sdk_builder, wit_resource)` adapter mirroring `__slicer_drain_gcode`
    - update the `run_path_optimization` macro expansion (line ~2115–2125) to construct an SDK `LayerCollectionBuilder`, pass it into the trait method, and drain on return
  - existing impls of `LayerModule::run_path_optimization` (sweep-update with default ignored binding):
    - `modules/core-modules/path-optimization-default/src/lib.rs`
    - any other module under `modules/core-modules/*/src/lib.rs` that implements `LayerModule::run_path_optimization` (likely zero — only `path-optimization-default` ships an impl in tree, but the sweep is part of acceptance)
    - any test guest under `test-guests/` that implements `LayerModule::run_path_optimization`
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — add `macro_embeds_layer_collection_builder_resource` test asserting both the resource declaration and the export-signature parameter appear in the macro's embedded WIT
  - new `crates/slicer-host/tests/layer_collection_builder_tdd.rs` — host validation + ordering/reversal application tests using `apply_entity_order_proposal` (the test-side handle introduced in dispatch.rs above)
- Rejected alternatives:
  - adding `set-entity-order` as a method on `gcode-output-builder`: rejected because `gcode-output-builder` is semantically a GCode-emission surface; bundling host-collection mutation onto it would mix concerns and make the `apply_entity_order_proposal` boundary harder to reason about. The new resource is the planned long-term surface per `docs/01`.
  - adding the proposal as a return value of `run-path-optimization` (`-> result<option<entity-order>, module-error>`): rejected because it would force every existing module to return an explicit value and changes the export's result type — much larger blast radius than adding a parameter.
  - bumping `slicer:world-layer` to `@2.0.0`: skipped because the application is unreleased; an in-place breaking change with a one-shot module sweep is acceptable.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities` (mutated in place after validation passes)
  - `PrintEntity.path.points` (reversed in place when the per-entity flag is `true`)
  - `PrintEntity.topo_order` (reassigned to the post-permutation 0-based index)
- WIT boundary additions:
  - `resource layer-collection-builder` in `slicer:ir-types` package
  - `collection: layer-collection-builder` parameter on `run-path-optimization` in `slicer:world-layer` package
- Determinism / scheduler constraints:
  - The host applies the proposal deterministically: validation order is (1) length match, (2) range check per index, (3) duplicate check via a bitmap of seen indices. The first failure short-circuits with the corresponding diagnostic.
  - `Vec::reverse()` is deterministic. Multiple reversal flags on different entities cannot interact.
  - The fallback path remains the packet-18 deterministic NN ordering.

## Locked Assumptions and Invariants

- The host owns `LayerCollectionIR.ordered_entities` as a writable surface; the module's contribution is a validated proposal.
- Validation never partially mutates: a malformed proposal causes a fatal at dispatch time and `ordered_entities` stays in its pre-call state.
- `set-entity-order` is callable at most once per `run-path-optimization` invocation; a second call from the guest returns `Err` from the WIT method, which the SDK guest type also enforces by checking its internal `Option`.
- Reversal preserves the path's payload; only the order of `points` within a single path changes.
- `topo_order` always equals the entity's index in `ordered_entities` after the permutation is applied.
- The packet-18 host fallback remains the default until packet 33 removes it.

## Risks and Tradeoffs

- Risk: macro-embedded WIT drifts from disk WIT. Mitigation: `wit_drift_detection_tdd` regression covers the new resource and the export signature. Add an explicit assertion for both.
- Risk: dispatch wiring forgets to reset `HostExecutionContext.layer_collection_proposal` between calls, leaking a proposal across layers. Mitigation: reset on every push of the new resource (mirroring how `gcode_output.commands` is implicitly cleared per call). Add a regression test that runs two layers consecutively, only one of which emits a proposal.
- Risk: reversing `path.points` interacts badly with downstream consumers that assume monotonic Z. Mitigation: paths are emitted at a single layer Z; reversal does not change Z values, only X/Y/width sequence.
- Risk: existing test guests under `test-guests/` are not all rebuilt as part of `build-core-modules.sh`. Mitigation: implementation-plan Step 8 enumerates and rebuilds every test guest alongside `path-optimization-default`.
- Risk: host validation order yields a confusing diagnostic when several errors apply (e.g., wrong length AND duplicate). Mitigation: validate length first and short-circuit; tests pin the exact expected substring.

## Open Questions

- None. The selected approach, the WIT method shape, the validation order, the fallback behavior, and the test surface are decided in this packet. Packet 33 will revisit the fallback removal.
