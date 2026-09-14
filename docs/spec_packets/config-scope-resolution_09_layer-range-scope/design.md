# Design: layer-range-scope

## Controlling Code Paths

- Primary source path: `read_3mf_layer_config_ranges` in `crates/slicer-model-io/src/layer_config_ranges.rs` reads the optional ZIP member once and returns raw ordinal/range/value records; the pnp-cli model-source adapters map ordinals to loaded object IDs and feed packet-03 `ConfigIngestor` after registry assembly.
- Primary resolution path: packet-05 `slicer_config::resolution::{query_z_grid, resolve_scope_stack}` consumes the same validated range collection; Z-grid composition handles only `layer_height`, while scope-stack resolution handles other admitted values at a target layer's top print Z.
- Runtime entry paths: `run_slice_with_collector` and `prepare_prepass_context` in `crates/slicer-runtime/src/run.rs`; pnp-cli ordinary slicing and `run_visual_debug` supply the same model-authored range input.
- Neighboring tests/fixtures: `layer_config_ranges_tdd`, `layer_range_scope_tdd`, runtime integration bucket registration, `layer_range_scope_visual_debug_tdd`, and `resources/layer_range_one_range.3mf`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- XML shape is exact: `objects/object@id/range@min_z,@max_z/option@opt_key`, with option value in text. `object@id` is a one-based object-list ordinal, not a 3MF object XML id.
- `RawLayerConfigRange` preserves strings and millimetre bounds. Registry-aware ingestion, not model IO, creates typed `ConfigValue`s and scope denials.
- `LayerConfigRange` is host-side resolution data. Do not add it to serialized IR or WIT.
- Range membership is finite world-space Z millimetres, half-open `[min_z, max_z)`, evaluated against layer top print Z; this also defines catch-up inheritance.
- `layer_height` composition sorts by `(min_z, max_z, source_index)`, inserts the fixed first-layer interval first, retains earlier coverage, trims each later low to the last retained high, drops emptied segments, and fills gaps from the resolved object base height. It never overlays later values onto earlier overlap.
- Non-`layer_height` overlap validation groups by object/key and rejects only non-empty overlaps with unequal typed values. Equal values may overlap; selector/denied statements reject before any range is committed.
- Both packet-05 resolver queries consume one normalized typed range set; no caller computes precedence locally.
- Additive `ConfigScope::LayerRange` requires an inventory of all exhaustive matches and serialized/display spellings, but no public IR/schema version bump because the type is host-only.
- No edited production path feeds guest WASM. Run guest freshness only as a diagnostic prerequisite for integration/visual-debug failures; the verbatim WASM snippet does not apply.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a narrow model-IO parser returning raw records, extend packet-03's typed ingestion with an interval-bearing range collection, normalize/validate once, then fill packet-05's existing resolver seam. Runtime paths carry the collection explicitly rather than hiding it in MeshIR or re-reading the archive.
- Net-new model-IO symbols:
  - `slicer_model_io::RawLayerConfigRange { object_ordinal: u32, source_index: u32, min_z: f64, max_z: f64, values: BTreeMap<String, String> }`.
  - `slicer_model_io::LayerRangeParseError`: named malformed XML, invalid/missing attribute, duplicate object ordinal, and invalid bound diagnostics.
  - `slicer_model_io::read_3mf_layer_config_ranges(path: &Path) -> Result<Vec<RawLayerConfigRange>, LayerRangeParseError>`; missing member returns `Ok(Vec::new())`.
- Net-new slicer-config symbols:
  - `ConfigScope::LayerRange { object_id: ObjectId, range_index: u32 }`.
  - `LayerConfigRange { scope: ConfigScope, min_z: f64, max_z: f64, delta: ScopeDelta }` with a constructor that validates the variant and finite ascending bounds.
  - `LayerRangeLoadError` with `InvalidObjectOrdinal`, `InvalidInterval`, `ConflictingOverlap { object_id, key, first_range, second_range }`, and transparent ingestion/`ResolutionError::ScopeDenied` paths.
  - `ConfigIngestor::ingest_layer_ranges` maps raw object ordinals after the caller supplies an ordinal-to-`ObjectId` table, assigns deterministic zero-based `range_index`, types values, validates admission/conflicts, and commits atomically.
  - `ScopedConfig::layer_ranges(&self, object_id: &ObjectId) -> &[LayerConfigRange]` returns normalized source ranges without exposing mutable storage.
- Packet-05 extensions:
  - `query_z_grid` composes the normalized layer-height profile per object before generating the Z grid.
  - `ResolutionTarget` gains the layer top print Z needed by `resolve_scope_stack`; its struct-literal blast radius is inventoried and updated in one bounded step.
  - `resolve_scope_stack` inserts every matching range delta between object and modifier. Conflicting non-height values cannot reach this function because validation is atomic.
- Runtime carrier: add one explicit typed range input to `SliceRunOptions` and `prepare_prepass_context`, or use packet-05's landed equivalent composition input after reconciliation. Update all struct literals/callers found by the required inventory in the same bounded substeps; do not add global state or parse paths in runtime.
- Exact XML fixture: `resources/layer_range_one_range.3mf` contains one model object and the AC-1 member. Create it with a deterministic test-fixture authoring helper or a reviewed ZIP-member replacement; tests assert the member shape so binary content is not a hidden oracle.
- Rejected alternative: put ranges in `ObjectMesh`/MeshIR. That creates an unnecessary public serialized IR field/version bump for host configuration source data.
- Rejected alternative: encode bounds in flat keys. It violates ADR-0068's decode-once typed boundary and cannot express overlap validation safely.
- Rejected alternative: use later-starting last-writer-wins for `layer_height`. Delegated canonical evidence shows ascending iteration plus trimming preserves earlier ranges.
- Rejected alternative: silently resolve conflicting non-height overlaps by source order. The approved policy requires a load error.

## Files in Scope (read + edit)

- `crates/slicer-model-io/src/{layer_config_ranges.rs,lib.rs}` and `tests/layer_config_ranges_tdd.rs` — canonical ZIP/XML adapter, exports, parser failures, and fixture contract.
- `crates/slicer-config/src/{ingestion.rs,resolution.rs,lib.rs}` and `tests/layer_range_scope_tdd.rs` — typed scope/range carrier, validation, profile composition, and both resolver queries; use landed packet-03/05 paths if names differ.
- `crates/pnp-cli/src/{main.rs,visual_debug.rs}` — read/map the model-authored range part once for ordinary and visual-debug setup.
- `crates/slicer-runtime/src/run.rs` and all inventoried focused call sites/tests — explicitly carry one typed range set through `SliceRunOptions`, `run_slice_with_collector`, and `prepare_prepass_context` without local semantics.
- `crates/slicer-runtime/tests/integration/{main.rs,layer_range_scope_tdd.rs}` — production-path convergence proof.
- `crates/pnp-cli/tests/layer_range_scope_visual_debug_tdd.rs` — real model-source visual-debug bundle proof.
- `resources/layer_range_one_range.3mf` — purpose-built one-object/one-range fixture.
- `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` — final host-side scope and scheduler contracts.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md` — RC-7, Resolution, Layer range geometry semantics, cross-cutting requirements, and queue row 9 only.
- `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,design.md}`, `config-scope-resolution_05_scope-resolution-module/{packet.spec.md,design.md}`, and `config-scope-resolution_07_scope-eligibility/{packet.spec.md,design.md}` — exports and forward questions only.
- `crates/slicer-model-io/src/sidecar.rs` — `parse_3mf_sidecar` XML/ZIP error-style reference only.
- `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` and silhouette tests — request/manifest fixture helpers only.
- `docs/08_coordinate_system.md`, `docs/19_visual_debug.md`, and `docs/22_test_quality.md` — relevant bounded sections only.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–08, `docs/DEVIATION_LOG.md`, and unrelated docs — never edit.
- `OrcaSlicerDocumented/**` — delegate; never load directly.
- Existing `resources/*.3mf` fixtures other than the new owned fixture — do not modify or load directly.
- `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, guest/module source, manifests, generated bindings, and schema/version constants.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load or edit.

## Expected Sub-Agent Dispatches

- Question: reconcile landed packet-03/05/07 exports, including every exhaustive `ConfigScope`/`ResolutionError` match and every `ResolutionTarget` literal; scope: exact symbols under `crates/slicer-config/**` plus predecessor contracts; return: `LOCATIONS: <at most 20 file:line entries, one context line each>` per symbol family; purpose: Steps 1/3 blast radius.
- Question: verify canonical XML and one-based object ordinal behavior; scope: `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` named importer/exporter functions; return: `SNIPPETS: <at most 3 verbatim snippets, 30 lines each, with file:line>`; purpose: Step 2.
- Question: verify earlier-starting range retention, fixed-first-layer handling, trimming, and gap fill; scope: `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp::layer_height_profile_from_ranges`; return: `SUMMARY: <at most 200 words, no code unless requested>`; purpose: Step 4.
- Question: inventory every `SliceRunOptions` literal and `prepare_prepass_context` call plus both model-source adapters; scope: `crates/**/*.rs`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>` batches; purpose: Step 5 blast radius.
- Question: inspect only the new fixture's ZIP member names and bounded range XML; scope: `resources/layer_range_one_range.3mf`; return: `FACT: <5 lines or fewer>`; purpose: fixture review.
- Question: run each named cargo/xtask command and report verdict; scope: workspace command; return: `FACT: <5 lines or fewer>` or failure `SNIPPETS` ≤20 lines; purpose: every validation.

## Data and Contract Notes

- IR/manifest contracts: no public IR or manifest change. `layer_range` is the existing packet-01/07 scope spelling; all runtime config key strings remain snake_case.
- WIT boundary: unchanged. Packet-05's host-resolved planning values continue across its existing WIT record; raw ranges do not cross.
- Determinism/scheduler constraints: object ordinal mapping follows loaded object-list order; ranges normalize by `(min_z, max_z, source_index)`; range indices are assigned after normalization; option maps are ordered; matching deltas preserve source-index tie order only where values agree.
- Error boundary: XML syntax/shape errors originate in model IO; object mapping, registry typing, denied scope, invalid interval, and overlap conflicts become one model/config load failure before execution-plan/prepass mutation.

## Locked Assumptions and Invariants

- Packet 03's current forward contract has no layer-range variant; this packet owns the net-new `ConfigScope::LayerRange` surface and its exhaustive-match blast radius.
- Packet 05 reserves the layer-range slot between object and modifier and exports two resolver queries; packet 09 fills that slot rather than replacing the resolver.
- Packet 07 exports registry-derived `admission_set` and `ResolutionError::ScopeDenied`; range ingestion calls that authority and adds no hard-coded machine/emitter roster.
- Canonical XML uses one-based model-object ordinal IDs and text-valued options.
- Canonical `layer_height` overlap is earlier-starting-wins through trimming, not later-starting replacement. This is an attribution correction, not a divergence.
- Gaps use resolved base `layer_height`; the fixed first-layer interval has precedence because it is retained first.
- Non-height conflicting overlap and selector/denied range statements are atomic load errors.
- World-Z interval membership is half-open and tests the layer top print Z, including catch-up layers.

## Risks and Tradeoffs

- Adding an enum variant and runtime carrier can have a broad compile blast radius; inventory before edits and split caller updates into ≤3-file substeps.
- Object ordinals can be confused with 3MF object IDs. The fixture deliberately uses a model id that is not the ordinal and asserts mapping to close that regression.
- Floating endpoints can invite fuzzy/closed comparisons. Reject non-finite bounds and use direct half-open comparisons in authored millimetres; only convert at an existing geometric boundary.
- A normalized profile test can become self-referential. Expected segments and resolved values must be literal authored values, with a negative control that would expose later-wins replacement.
- Visual-debug currently has a separate config-source setup; AC-5/AC-6 prevent it from silently omitting model-authored ranges.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (resolver/profile composition and runtime carrier blast radius)
- Highest-risk dispatch and required return format: runtime caller/struct-literal inventory, bounded `LOCATIONS` batches.

## Open Questions

- `[FWD]` Reconcile packet 03's landed ingestion file/type names and every exhaustive `ConfigScope` match before adding `LayerRange`; preserve the exact net-new semantic surface above.
- `[FWD]` Reconcile packet 05's landed resolver target and Z-profile representation. If it already has a range slot type, extend that type rather than introducing a parallel carrier.
- `[FWD]` Reconcile packet 07's landed denial error conversion so selector and machine/emitter denials surface as its exact `ResolutionError::ScopeDenied` shape during load.
- `[BLOCK]` None beyond forward-dependency reconciliation.
