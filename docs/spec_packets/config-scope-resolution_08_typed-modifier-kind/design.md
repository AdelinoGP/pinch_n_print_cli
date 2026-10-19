# Design: typed-modifier-kind

## Controlling Code Paths

- Primary load path: `parse_3mf_sidecar` produces host-local `PartSubtype`; `resolve_object` (`crates/slicer-model-io/src/loader.rs`) routes `NormalPart` as solid geometry and maps the other four variants to `slicer_ir::ModifierKind` on `ModifierVolume`.
- Config path: the shared runtime composition used by `run_slice_with_collector` and `prepare_prepass_context` ingests each modifier delta through `ConfigIngestor::ingest_delta(ConfigScope::Modifier { object_id, modifier_id }, ..)`; packet-05 `resolve_scope_stack` consumes that `ScopeDelta` after packet-07 admission.
- Geometry/classification paths: the exact ten sites inventoried under Code Change Surface.
- Neighboring tests/fixtures: `slicer-ir/tests/ir_tests.rs`, `slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`, slicer-core region/paint tests, runtime `e2e` bucket, and new pnp-cli visual-debug test.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- `PartSubtype` is host-local XML syntax; `ModifierKind` is the IR-domain classification. `NormalPart` has no `ModifierKind` variant because it never creates a `ModifierVolume`.
- `ModifierKind` has exactly four variants: `ParameterModifier`, `NegativePart`, `SupportEnforcer`, and `SupportBlocker`. Production classification uses exhaustive `match` without wildcard arms, string conversion, or helper predicates that hide new variants from the ten consumers.
- `config_delta` remains authored settings only. The XML routing attribute `subtype` is not a config key, cannot be supplied as an override, and is removed from newly emitted deltas.
- Modifier delta typing and admission happen before resolver mutation. Rejection produces no partial `ResolvedConfig` or interner entry.
- Existing priority values remain 0, 100, 200, and 300 for parameter, negative, enforcer, and blocker kinds respectively.
- Parameter modifiers remain wall-less fill sub-regions under ADR-0030; support kinds stay on paint/support paths; negative parts stay on subtract.
- The owner decision mandates a minor MeshIR bump despite removing `applies_to`. Legacy 1.1.0 deserialization must recover kind from the old subtype entry or implementation escalates.
- Every existing `ModifierVolume` literal is part of the field-change blast radius. Use constructors/helpers or edit each literal in the field-change step; do not wait for `cargo check` to discover one.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add the IR enum plus a temporary `ModifierVolume` compatibility constructor/`kind()` accessor first, migrate every literal and consumer through that seam in bounded steps, then centralize the final field/removal/version change in `slice_ir.rs`; map once at model loading, normalize legacy serde at the IR boundary, ingest authored modifier settings into packet-03 scope deltas in common runtime setup, and replace every downstream classifier with an exhaustive enum match. The temporary subtype-backed bridge is never a completed packet state and is removed when `kind` becomes a field.
- IR symbols:
  - `slicer_ir::ModifierKind::{ParameterModifier, NegativePart, SupportEnforcer, SupportBlocker}` derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, and `Deserialize`.
  - `ModifierVolume` becomes exactly `{ id, mesh, config_delta, priority, kind }`; `applies_to` and `ModifierScope` are removed.
  - `CURRENT_MESH_IR_SCHEMA_VERSION` advances by exactly one minor from the live activation-time value, retaining its major and setting patch to zero; the implementation test records the pre-edit value rather than copying a speculative future version from this packet.
  - A private/custom serde compatibility representation may accept legacy `applies_to` plus `config_delta.fields["subtype"]`; serialization emits only the new-minor shape, and deserialization removes the legacy routing key after deriving kind.
- Exact ten production match sites:
  1. `stamp_modifier_sub_region_configs` (`crates/slicer-core/src/algos/region_mapping.rs`) skips only the two support kinds and stamps the remaining volumes' settings, so `ParameterModifier` and `NegativePart` volumes both proceed.
  2. `modifier_footprint_groups` (same file) admits every non-support kind as a footprint group, including `NegativePart`.
  3. `execute_region_mapping_inner` (same file) constructs modifier sub-region mappings for every non-support kind, including `NegativePart`.
  4. `slice_modifier_volumes` (`crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs`) maps the two support kinds to paint semantics and ignores parameter/negative kinds.
  5. `mesh_has_any_paint` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) recognizes only the two support kinds as modifier paint sources.
  6. `split_modifier_sub_regions_for_prepass` (`crates/slicer-runtime/src/region_partition.rs`) splits every non-support kind; only `SupportEnforcer`/`SupportBlocker` are excluded.
  7. Nested `build_paint_semantic_configs` inside `execute_prepass_with_builtins_configured_instr_collecting` (`crates/slicer-runtime/src/prepass.rs`) discovers support semantics from the two support kinds.
  8. `apply_negative_part_subtract` (`crates/slicer-runtime/src/negative_part_subtract.rs`) admits only `NegativePart`.
  9. `stage_modifier_footprints` (`crates/slicer-runtime/src/layer_executor.rs`) stages every non-support kind, including `NegativePart`.
  10. `collect_support_territory` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) skips only the two support kinds; `ParameterModifier` and `NegativePart` volumes proceed.
- Registry seam: add one shared runtime helper, at the final packet-03/05 composition location after reconciliation, that iterates `MeshIR.objects[].modifier_volumes`, reserves/removes `subtype`, calls `ConfigIngestor::ingest_delta` with exact object/modifier IDs, and returns errors rather than falling back to untyped extension copying.
- Test targets: existing `ir_tests`, existing `threemf_sidecar_classification_tdd`, `slicer-core` host-algos targets, runtime `--test e2e` with a registered `typed_modifier_kind_tdd` module, and standalone `pnp-cli --test typed_modifier_kind_visual_debug_tdd`.
- Rejected alternative: keep a string accessor around `config_delta["subtype"]`; this retains silent handling of future kinds and falsely treats routing metadata as user config.
- Rejected alternative: wire `ModifierScope`; ADR-0070 explicitly removes it because key eligibility and stated keys express the useful restriction.
- Rejected alternative: default every legacy modifier to parameter kind; that corrupts negative/support fixture behavior and violates the owner decision's compatibility condition.

## Files in Scope (read + edit)

Primary files:

- `crates/slicer-ir/src/slice_ir.rs` — enum, exact `ModifierVolume` shape, legacy serde adapter, and activation-derived MeshIR minor bump.
- `crates/slicer-model-io/src/loader.rs` — one typed `PartSubtype`→`ModifierKind` map and reserved-key handling.
- Final packet-03/05 common runtime composition path, expected `crates/slicer-runtime/src/run.rs` — ingest modifier deltas once for both entry points after forward-dependency reconciliation.

Necessary exhaustive-consumer and test files:

- `crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/tests/ir_tests.rs` — export and IR/serde/version contract.
- `crates/slicer-core/src/algos/region_mapping.rs`, `crates/slicer-core/src/algos/paint_segmentation/{mod.rs,modifier_volumes.rs,seam_annotations.rs}` — five production classifications plus in-source literals.
- `crates/slicer-runtime/src/{region_partition.rs,prepass.rs,negative_part_subtract.rs,layer_executor.rs}`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — five runtime classifications plus in-source literals.
- `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` — four-kind loader contract.
- `crates/slicer-core/tests/{algo_region_mapping_tdd.rs,paint_segmentation_base_fallback_tdd.rs,paint_segmentation_multi_object_isolation_tdd.rs}` — parameter/support behavior and literals.
- `crates/slicer-runtime/tests/executor/{cube_fuzzy_painted_tdd.rs,modifier_region_split_tdd.rs}`, `crates/slicer-runtime/tests/contract/modifier_split_subregion_density_tdd.rs` — executor/contract literals.
- `crates/slicer-runtime/tests/e2e/{main.rs,typed_modifier_kind_tdd.rs,acceptance_gate_gaps_tdd.rs,cube_4color_modifier_part_e2e_tdd.rs,mixed_density_internal_bridge_rejection_e2e_tdd.rs,modifier_support_type_family_e2e_tdd.rs,threemf_fixture_e2e_tdd.rs,threemf_subtypes_synthetic_e2e_tdd.rs}` — bucket registration, behavior proof, false-scope retirement, and literals.
- `crates/pnp-cli/tests/typed_modifier_kind_visual_debug_tdd.rs` — geometry acceptance bundle.
- `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — contract updates.

## Read-Only Context

- `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,requirements.md,design.md}` — reconcile `ConfigIngestor` and modifier scope only.
- `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{packet.spec.md,requirements.md,design.md}` — reconcile resolver target/delta ordering only.
- `docs/spec_packets/config-scope-resolution_07_scope-eligibility/{packet.spec.md,requirements.md,design.md}` — reconcile admission/error shape only.
- `docs/specs/config-scope-resolution-plan.md` — named row-8 and owner-decision sections only.
- `docs/19_visual_debug.md` — Request Shape and RegionMapping silhouette semantics only.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–07 and 09+, and `docs/07_implementation_status.md` except completion dispatch — never edit directly in implementation.
- `OrcaSlicerDocumented/**` — delegate; never load.
- `crates/slicer-schema/wit/**`, guest sources, module manifests, layer-range code, and unrelated IR version constants — no edit.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and binary fixture contents — never load.

## Expected Sub-Agent Dispatches

- Question: reconcile landed packet-03/05/07 names, signatures, error conversion, and the one composition helper shared by both runtime entry points; scope: their packet contracts plus `crates/slicer-config/**` and `crates/slicer-runtime/src/run.rs`; return: `FACT` ≤5 lines; purpose: activation/Step 1.
- Question: re-inventory all production reads/comparisons of modifier subtype routing and confirm exactly the ten named enclosing symbols; scope: `crates/slicer-core/src/**/*.rs` and `crates/slicer-runtime/src/**/*.rs`; return: `LOCATIONS` ≤20; purpose: exhaustive-match gate.
- Question: re-inventory every `ModifierVolume {` literal and every `ModifierScope`/`applies_to` use, grouped into batches of at most three edit files; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20 per batch; purpose: struct-literal blast-radius plan.
- Question: verify canonical support-kind exclusion and parameter-modifier wall-less behavior by function name; scope: `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp`; return: `SUMMARY` ≤200 words; purpose: parity lock.
- Question: run each named cargo command and report pass/fail only, with bounded failing context; scope: exact command; return: `FACT` ≤5 lines or `SNIPPETS` ≤20 lines; purpose: every validation.

## Data and Contract Notes

- IR/manifest contracts: the next live MeshIR minor adds `ModifierVolume.kind` and removes `applies_to`; no manifest change. `ConfigDelta` continues storing authored config values, never routing metadata.
- WIT boundary: unchanged; MeshIR is host-owned and guest modules see resolved per-region config rather than `ModifierVolume`.
- Determinism/scheduler constraints: priorities and stable tie ownership do not change; registry normalization/admission precedes resolution; both runtime entry points share one modifier-ingestion implementation.
- Compatibility: legacy read is one-way. Pre-change subtype/applies-to input may be accepted, but all post-bump serialization uses typed kind and omits both legacy surfaces.

## Locked Assumptions and Invariants

- FORWARD-DEP packet 03 exports `ConfigScope::Modifier { object_id, modifier_id }`, `ConfigIngestor::ingest_delta`, `ScopeDelta`, `ScopedConfig`, and `IngestionOutcome` with the shapes recorded in its design.
- FORWARD-DEP packet 05 exports `resolve_scope_stack`, `ResolutionTarget`, and `ResolutionError`; modifier precedence remains above object/future layer range and below paint/tool.
- FORWARD-DEP packet 07 exports `ConfigSchemaRegistry::admission_set(scope)` and `ResolutionError::ScopeDenied { key, scope }` and removes the inert manifest sections.
- Current MeshIR constant is re-grounded as 1.1.0 in `CURRENT_MESH_IR_SCHEMA_VERSION`; activation must re-ground it and compute one minor increment rather than trust this mutable ledger fact.
- The production subtype inventory is exactly five `slicer-core` reads and five `slicer-runtime` reads, in the ten symbols listed above.
- `ModifierScope` is read by no production code; `acceptance_gate_gaps_tdd` contains synthetic tests that claim behavior production never implements and must be retired.
- `PartSubtype::NormalPart` never becomes a `ModifierVolume`; every `ModifierVolume.kind` is one of the four non-normal variants.

## Risks and Tradeoffs

- A wildcard enum arm would preserve compilation but lose ADR-0070's compile-time exhaustiveness; static review and behavior tests require explicit arms.
- Legacy serde can misclassify old support/negative fixtures if it defaults kind rather than reading subtype; AC-4 locks representative legacy values.
- Routing modifier values through the registry can expose formerly tolerated invalid metadata. This is intended, but errors must name key/scope and remain atomic.
- The struct-literal blast radius is broad. Mechanical constructor/literal migration must be split into small steps without omitting embedded test modules in production files.
- Geometry behavior is intentionally unchanged but config delivery changes; the visual-debug bundle catches silent shape changes not visible in unit values.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (runtime registry wiring and exhaustive production migration)
- Highest-risk dispatch and required return format: all literal/obsolete-surface locations in bounded `LOCATIONS` batches of at most 20.

## Open Questions

- `[FWD]` Reconcile the final landed packet-03/05/07 API and error-conversion shapes before activation; preserve one shared runtime modifier-ingestion path without broadening this packet.
- `[FWD]` Choose the smallest private serde adapter that preserves all four legacy subtype classifications; if a committed legacy fixture cannot deserialize correctly, stop and escalate per the owner decision.
- `[BLOCK]` None beyond the forward dependencies.
