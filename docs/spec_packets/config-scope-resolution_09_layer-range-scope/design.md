# Design: layer-range-scope

## Controlling Code Paths

- Primary source path: `read_3mf_layer_config_ranges` in `crates/slicer-model-io/src/layer_config_ranges.rs` reads the optional ZIP member once and returns raw ordinal/range/value records; `map_layer_config_ranges` maps one-based ordinals onto the loaded `MeshIR.objects` order into `LayerRangeInput` values.
- Registry-aware path: the model-source adapters pass `Vec<LayerRangeInput>` into runtime setup; `slicer-wasm-host::execution_plan_live::load_live_modules_for_plan_manifest_first` calls `ConfigIngestor::ingest_layer_ranges` after registry assembly and returns the typed ranges beside the existing ingestion outcome.
- Primary resolution path: `slicer-config::resolution` owns the per-object layer-height profile (`query_layer_height_profile`), the explicit top-Z schedule (`layer_top_zs`), and the per-target scope resolution (`resolve_scope_stack` with `ResolutionTarget.layer_top_z`); `query_z_grid` produces per-object scalars and `layer_zs` for the planning record.
- Schedule path: `run.rs::layer_planning_objects` carries `layer_zs` into the `PrePass::LayerPlanning` WIT record; guest `layer-planner-default` emits explicit top-Z schedules; the host harvests `LayerPlanIR.global_layers` (layer top Z).
- Per-layer config path: `crates/slicer-runtime/src/builtins/region_mapping_producer.rs::commit_region_mapping_builtin` → `slicer-core::algos::region_mapping::execute_region_mapping_inner`, which re-resolves a range-bearing `ResolutionTarget { object_id, layer_top_z: Some(layer.z), .. }` when any range covers that top Z.
- Visual-debug path: `pnp-cli/src/visual_debug.rs::run_model_source` passes the mapped ranges plus manifest additive field `scheduled_layer_zs` from `LayerPlanIR.global_layers`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Landing Notes (Step 1 reconciliation, 2026-09-26)

- Landed packet 03/05/07 names match the original design: `ConfigScope` (Global/Object/Modifier/PaintSemantic/Tool), `ScopeDelta`, `ScopedConfig { deltas }`, `ConfigIngestor::{new,tolerant,ingest_flat,ingest_delta,finish}`, `ConfigSchemaRegistry::admission_set`, `ResolutionError::ScopeDenied { key, scope }`, `query_z_grid`, `resolve_scope_stack`, `ResolutionTarget { object_id, modifier_ids, paint_semantics, tool_index }`. `ScopeDenied.key` is `String`.
- `scope_denial_label` (`crates/slicer-config/src/lib.rs`) is the only exhaustive `ConfigScope` match in production; it must gain a `LayerRange` arm returning `"layer_range"` (the label already exists in `ALLOWED_DENIED_SCOPES`/`PER_REGION_SCOPES`).
- `ResolutionTarget` has no Z field; the 4 exhaustive literals are in `crates/slicer-runtime/src/run.rs` and `crates/slicer-config/tests/scope_resolution_tdd.rs` (×3). Adding a 5th field also makes `ResolutionTarget` a watched type under `docs/21_data_defaults_and_fixtures.md`, so those test literals need FRU or an `// exhaustive:` waiver.
- `ScopedConfig { deltas }` exhaustive single-field literals: `ConfigIngestor::finish` plus helpers in `scope_resolution_tdd.rs`, `scope_eligibility_tdd.rs`, `resolved_config_view_tdd.rs`. Plan: add `pub layer_ranges: BTreeMap<ObjectId, Vec<LayerConfigRange>>` and convert call-site literals to FRU (or a `with_layer_ranges` constructor) in Step 3.
- No host profile/breakpoint API exists; `query_z_grid` returns only `ResolvedObjectLayerConfig` scalars. The packet originally assumed a profile seam in packet 05; packet 05 explicitly deferred it to row 9, so the profile API is net-new here.
- The Z schedule is produced by guest `layer-planner-default`; `PrePass::RegionMapping` is the host builtin `commit_region_mapping_builtin` (no per-layer `resolve_scope_stack` call today). `GlobalLayer.z` is the layer **top**; `catchup_z_bottom`/`effective_layer_height` live on `ActiveRegion`.
- Registry assembly + `ConfigIngestor` live in `crates/slicer-wasm-host/src/execution_plan_live.rs::load_live_modules_for_plan_manifest_first`; `pnp-cli` has no `slicer-config` dependency; `slicer-runtime` cannot name `slicer_model_io` in production (dev-only). Carrier therefore rides `SliceRunOptions.layer_ranges`, a new `prepare_prepass_context` parameter, and a new wasm-host loader parameter.
- WIT `object-layer-config` lives at `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit` (`package slicer:prepass-layer-planning@2.0.0`), mirrored by `StageSpec.wit_package` in `crates/slicer-schema/src/lib.rs`, `slicer-macros` binding-surface fixtures, the layer-planner binding test, `docs/03_wit_and_manifest.md`, and `crates/slicer-schema/wit/README.md`.
- MeshIR object order equals 3MF `<build><item>` order; `place_model_on_bed` is a rigid translation for 3MF (pass-through). The one-based range ordinal maps to `objects[ordinal - 1]`, deliberately independent of the model XML `objectid`.
- Canonical Orca evidence (delegated): XML shape `objects/object@id/range@min_z,max_z/option@opt_key` with value text and one-based ordinal; `layer_height_profile_from_ranges` iterates ascending and clips later lows to the last pushed high (earlier-starting wins), fixed first-layer interval pushed first, gaps filled with base `layer_height`.

## Architecture Constraints

- XML shape is exact: `objects/object@id/range@min_z,@max_z/option@opt_key`, with option value in text. `object@id` is a one-based object-list ordinal, not a 3MF object XML id.
- `RawLayerConfigRange` preserves strings and millimetre bounds; `map_layer_config_ranges` maps ordinals after model load. Registry-aware ingestion, not model IO, creates typed `ConfigValue`s and scope denials.
- `LayerConfigRange` is host-side resolution data. Do not add it to serialized IR or WIT. `LayerRangeInput` is the cross-crate transport value (object id + bounds + raw string values).
- Range membership is finite world-space Z millimetres, half-open `[min_z, max_z)`, evaluated against layer top print Z; this also defines catch-up inheritance.
- `layer_height` composition sorts by `(min_z, max_z, source_index)`, inserts the fixed first-layer interval first, retains earlier coverage, trims each later low to the last retained high, drops emptied segments, and fills gaps from the resolved object base height. It never overlays later values onto earlier overlap. The profile returns literal segments `(z_start, z_end, height)` and `layer_top_zs` evaluates the schedule `h0, h0+h1, …` until object height.
- Non-`layer_height` overlap validation groups by object/key and rejects only non-empty overlaps with unequal typed values. Equal values may overlap; selector/denied statements reject before any range is committed.
- `ResolutionTarget.layer_top_z: Option<f64>` is the only new resolver input; `None` means "no layer context" and range deltas never apply. A `Some(z)` matches ranges with `min_z <= z < max_z`.
- Guest `layer-zs` are object-local top Zs (raft offset applied by the guest); host `layer_top_zs` returns the same object-local sequence that `run.rs` sends. `layer-zs` empty preserves the uniform formula.
- Additive `ConfigScope::LayerRange` requires an inventory of all exhaustive matches and serialized/display spellings; `scope_denial_label` returns `"layer_range"`. No public IR/schema version bump because the type is host-only.
- WIT change is a breaking package bump to `@3.0.0`; guests must be rebuilt via `cargo xtask build-guests` before integration/visual-debug runs.
- The manifest addition is additive and version-gated (schema 1.3); older manifest schemas remain valid.

## Code Change Surface

- Selected approach: add a narrow model-IO parser returning raw records plus an ordinal mapper, extend packet-03 ingestion with an interval-bearing range collection, derive the host profile/top-Z schedule once, carry the schedule across the WIT seam, and re-resolve per-layer region config only when a range covers the layer top Z.
- Net-new model-IO symbols:
  - `RawLayerConfigRange { object_ordinal: u32, source_index: u32, min_z: f64, max_z: f64, values: BTreeMap<String, String> }`.
  - `LayerRangeParseError`: named malformed XML, invalid/missing attribute, duplicate object ordinal, invalid bound, and unmapped ordinal diagnostics.
  - `read_3mf_layer_config_ranges(path: &Path) -> Result<Vec<RawLayerConfigRange>, LayerRangeParseError>`; missing member returns `Ok(Vec::new())`.
  - `MappedLayerConfigRange { object_id: ObjectId, source_index: u32, min_z: f64, max_z: f64, values: BTreeMap<String, String> }` and `map_layer_config_ranges(raw: &[RawLayerConfigRange], object_ids: &[ObjectId]) -> Result<Vec<MappedLayerConfigRange>, LayerRangeParseError>`; unmapped/zero ordinals return `LayerRangeParseError::InvalidObjectOrdinal { object_ordinal }`.
  - Note: `slicer-config` cannot depend on `slicer-model-io`, so the cross-crate transport is `slicer_config::LayerRangeInput { object_id: ObjectId, source_index: u32, min_z: f64, max_z: f64, values: BTreeMap<String, String> }`; pnp-cli converts `MappedLayerConfigRange` → `LayerRangeInput` field-for-field (no semantics).
- Net-new slicer-config symbols:
  - `ConfigScope::LayerRange { object_id: ObjectId, range_index: u32 }`.
  - `LayerConfigRange { scope: ConfigScope, min_z: f64, max_z: f64, delta: ScopeDelta }` with a constructor validating the variant and finite ascending bounds.
  - `LayerRangeLoadError { Denied(ResolutionError), ConflictingOverlap { object_id, key, first_range, second_range }, InvalidInterval { .. }, UnknownObject { object_id } }`.
  - `ConfigIngestor::ingest_layer_ranges(values: &[LayerRangeInput]) -> Result<(), LayerRangeLoadError>`; assigns deterministic zero-based `range_index` in `(min_z, max_z, source_index)` order, types values through the registry, validates admission/conflicts, and commits atomically.
  - `ScopedConfig { deltas, layer_ranges }` with `ranges_for(&self, object_id: &ObjectId) -> &[LayerConfigRange]` and `has_layer_ranges(&self) -> bool`; `ScopedConfig::default()` remains constructible. (`layer_ranges` is the field; the accessor is `ranges_for` to avoid a field/method name collision.) Derive `Eq` is dropped if the `f64` interval fields make it underivable; `PartialEq` is retained.
  - `HeightProfileSegment { z_start: f64, z_end: f64, height: f64 }`.
  - `query_layer_height_profile(registry, scoped, object_id, object_height, expansion) -> Result<Vec<HeightProfileSegment>, ResolutionError>`.
  - `layer_top_zs(segments: &[HeightProfileSegment], object_height: f64) -> Vec<f64>` (pure, independent of production helpers).
- Packet-05 extensions:
  - `ResolutionTarget.layer_top_z: Option<f64>`.
  - `resolve_scope_stack` inserts matching range deltas between object and modifier (the reserved seam comment is replaced by the real loop).
  - `ResolvedObjectLayerConfig` gains `layer_z_tops: Vec<f64>` so `run.rs` can populate the WIT record.
- WIT/SDK/macros:
  - `object-layer-config` gains `layer-zs: list<f64>`.
  - `LayerPlanningObject` gains `layer_zs: Vec<f64>`; dispatch adapter, macros glue, native module adapter, and host contract tests update together.
- Guest:
  - `ObjectPlan` gains `layer_zs`; a new explicit-schedule generator offsets by `raft_top`; `merge_different_heights` handles explicit lists with variable-step `effective_layer_height` and catch-up; `merge_same_height` keeps uniform behavior only when explicit lists are empty.
- Runtime carrier:
  - `SliceRunOptions.layer_ranges: Vec<LayerRangeInput>`; `prepare_prepass_context(..., layer_ranges: Vec<LayerRangeInput>, ...)`; `load_live_modules_for_plan_manifest_first(..., layer_ranges: &[LayerRangeInput], ...)`.
  - `PipelineConfig` (or the prepass authority call) gains range-bearing authority so `commit_region_mapping_builtin` can re-resolve; `execute_region_mapping_inner` gains the optional resolver authority and applies it when a range covers `layer.z`.
- Manifest: `Manifest.scheduled_layer_zs: Option<Vec<f64>>`, populated in `run_model_source` for model sources; schema 1.3 additive.
- Exact XML fixture: `resources/layer_range_one_range.3mf` contains one model object and the AC-1 member. Tests assert the member shape so binary content is not a hidden oracle.
- Rejected alternative: put ranges in `ObjectMesh`/MeshIR. That creates an unnecessary public serialized IR field/version bump for host configuration source data.
- Rejected alternative: encode bounds in flat keys. It violates ADR-0068's decode-once typed boundary and cannot express overlap validation safely.
- Rejected alternative: use later-starting last-writer-wins for `layer_height`. Delegated canonical evidence shows ascending iteration plus trimming preserves earlier ranges.
- Rejected alternative: silently resolve conflicting non-height overlaps by source order. The approved policy requires a load error.
- Rejected alternative: change only the guest planner without host profile/top-Z derivation (would duplicate clipping logic in the guest and violate "no caller computes precedence locally").

## Files in Scope (read + edit)

- `crates/slicer-model-io/src/{layer_config_ranges.rs,lib.rs}` and `tests/layer_config_ranges_tdd.rs`.
- `crates/slicer-config/src/{ingestion.rs,resolution.rs,lib.rs}` and `tests/layer_range_scope_tdd.rs`.
- `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`; `crates/slicer-schema/src/lib.rs`; `crates/slicer-schema/wit/README.md`.
- `crates/slicer-sdk/src/{traits.rs,prepass_types.rs,native.rs,native_prepass_types.rs}`; `crates/slicer-macros/src/lib.rs` and `crates/slicer-macros/tests/binding_surface_tdd.rs`.
- `crates/slicer-wasm-host/src/{dispatch.rs,execution_plan_live.rs}`; `crates/slicer-wasm-host/tests/contract/*layer_planning*`.
- `modules/core-modules/layer-planner-default/` (`src/lib.rs`, `tests/{layer_planning_tdd.rs,slicer_module_binding_tdd.rs}`).
- `crates/slicer-core/src/algos/region_mapping.rs`; `crates/slicer-runtime/src/{run.rs,prepass.rs,pipeline.rs,builtins/region_mapping_producer.rs}`.
- `crates/pnp-cli/src/{main.rs,visual_debug.rs,support_preview.rs}` and `crates/pnp-cli/Cargo.toml`.
- `crates/slicer-runtime/tests/integration/{main.rs,layer_range_scope_tdd.rs}`; `crates/pnp-cli/tests/layer_range_scope_visual_debug_tdd.rs`.
- `resources/layer_range_one_range.3mf`.
- `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md`, `docs/19_visual_debug.md`.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md` — RC-7, Resolution, Layer range geometry semantics, cross-cutting requirements, and queue row 9 only.
- `crates/slicer-model-io/src/sidecar.rs` — `parse_3mf_sidecar` XML/ZIP error-style reference only.
- `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` and silhouette tests — request/manifest fixture helpers only.
- `docs/08_coordinate_system.md`, `docs/22_test_quality.md` — relevant bounded sections only.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–08, `docs/DEVIATION_LOG.md`, and unrelated docs — never edit.
- `OrcaSlicerDocumented/**` — delegate; never load directly.
- Existing `resources/*.3mf` fixtures other than the new owned fixture — do not modify or load directly.
- Other WIT packages and their guests, generated bindings, and schema/version constants (except `prepass-layer-planning` and its StageSpec literal).
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load or edit.

## Expected Sub-Agent Dispatches

- Step 1 reconciliation and blast-radius inventories — done; results in §Landing Notes.
- Question: implement/verify model-IO parser and ordinal mapper; return `FACT` / `SNIPPETS ≤20 lines`.
- Question: implement/verify typed scope, carrier, profile/top-Z, resolver seams; return task-worker TOML.
- Question: reconcile and update all `@2.0.0` version surfaces and WIT/guest glue; return `LOCATIONS` + `FACT`.
- Question: write and run the runtime convergence and visual-debug tests; return task-worker TOML.
- Question: run each named cargo/xtask command and report verdict; `FACT: ≤5 lines` or failure `SNIPPETS ≤20 lines`.

## Data and Contract Notes

- IR/manifest contracts: no MeshIR/SliceIR change. Manifest gains additive `scheduled_layer_zs` under schema 1.3; `layer_range` remains the existing packet-01/07 scope spelling; runtime config key strings stay snake_case.
- WIT boundary: `slicer:prepass-layer-planning@3.0.0`; `layer-zs` carries host-derived object-local top Zs only; raw XML/strings do not cross.
- Determinism/scheduler constraints: object ordinal mapping follows loaded object-list order; ranges normalize by `(min_z, max_z, source_index)`; range indices are assigned after normalization; option maps are ordered.
- Error boundary: XML syntax/shape errors originate in model IO; object mapping, registry typing, denied scope, invalid interval, and overlap conflicts become one model/config load failure before execution-plan/prepass mutation.
- Unit boundary: authored model Z values are millimetres (`f64`); guest schedules remain `f32`; host `layer_top_zs` is `f64` and compared with a 1e-9 tolerance in tests.

## Locked Assumptions and Invariants

- `ConfigScope::LayerRange { object_id, range_index }` is net-new in this packet; packet 03/05/07 have no layer-range variant, carrier, or accessor.
- Packet 05's reserved precedence slot is filled here; no second resolver is introduced.
- Packet 07's `admission_set` + `ResolutionError::ScopeDenied` are the only denial authority; range ingestion adds no hard-coded roster.
- Canonical XML uses one-based model-object ordinals and text-valued options.
- Canonical `layer_height` overlap is earlier-starting-wins through trimming, not later-starting replacement. This is an attribution correction, not a divergence.
- Gaps use resolved base `layer_height`; the fixed first-layer interval has precedence because it is retained first.
- Non-height conflicting overlap and selector/denied range statements are atomic load errors.
- World-Z interval membership is half-open on `LayerConfigRange::covers` — the layer top print Z (`GlobalLayer.z`, an `f32`) against each bound taken as the smaller of its authored `f64` value and its `f32` image, so the transported form of a top placed on a bound still resolves to that bound. This also defines catch-up inheritance.
- WIT `layer-zs` is object-local top Zs; empty means uniform fallback.

## Risks and Tradeoffs

- Adding an enum variant, a `ScopedConfig` field, and a `ResolutionTarget` field has a broad compile blast radius; inventories (Step 1) bound it and caller updates are split into ≤3-file substeps.
- WIT `@3.0.0` requires guest rebuilds; `cargo xtask build-guests --check` must pass before interpreting integration/visual-debug failures.
- Object ordinals can be confused with 3MF object XML ids. The fixture deliberately uses a model id that is not the ordinal and asserts mapping to close that regression.
- Floating endpoints can invite fuzzy/closed comparisons, and the layer top the host evaluates is an `f32` (`GlobalLayer.z`) while authored bounds are `f64`. `LayerConfigRange::covers` is therefore the single membership authority: it compares each bound as the smaller of the authored `f64` value and its `f32` image, which keeps the authored half-open rule in both representations instead of making `max_z` effectively inclusive. It is deliberately not a general epsilon or half-ULP shave — a power-of-two bound has unequal forward and backward `f32` gaps, so half the forward gap lands on a genuinely interior representable top and excludes it; the minimum-of-both rule has no such edge. A bound lying between two `f32` values is the documented limit: narrowing moves it by up to half a local `f32` ULP, so a top within that distance resolves to the bound rather than its authored side. Every ordinary print height is representable, so the band does not arise there. The resolver and the runtime kernel both call it, so they cannot disagree.
- A normalized profile test can become self-referential. Expected segments and resolved values are literal authored values, with a negative control that would expose later-wins replacement.
- Visual-debug currently has a separate config-source setup; AC-5/AC-6 prevent it from silently omitting model-authored ranges.
- `merge_different_heights`' first-layer credit uses a `last_z == 0.0` sentinel; explicit schedules keep the same convention because host top Zs are strictly positive.

## Context Cost Estimate

- Aggregate: `L` (owner-approved in-place expansion).
- Largest step: `M` (WIT/guest reconciliation, runtime carrier and per-layer application).
- Highest-risk dispatch and required return format: WIT version-surface reconciliation and runtime caller/literal inventory, bounded `LOCATIONS` batches.

## Open Questions

- None. Amendment 1 resolved the WIT/guest/profile scope and the owner approved it.
