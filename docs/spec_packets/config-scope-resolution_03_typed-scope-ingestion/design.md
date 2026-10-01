# Design: typed-scope-ingestion

## Controlling Code Paths

- Primary code path: `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) discovers manifests before registry assembly and `ConfigIngestor`; `run_slice_with_collector` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`) retain the returned `ScopedConfig` for binding and compatibility resolution.
- Ingestion adapters: `parse_cli_config_source` (`crates/slicer-scheduler/src/execution_plan.rs`) and `read_3mf_project_settings`/`parse_project_settings_json` (`crates/slicer-model-io/src/loader.rs`) parse source syntax but do not guess declared types.
- Claim path: `dedup_same_claim_modules_with_wall_generator` and `module_claims_match_active_region` (`crates/slicer-scheduler/src/execution_plan.rs`) plus `resolve_held_claims` (`crates/slicer-scheduler/src/validation.rs`).
- Neighboring tests/fixtures: `typed_scope_ingestion_tdd.rs`; packet 02's read-only `ingestion_fidelity_oracle_tdd.rs`; scheduler `scheduler_contract` and `scheduler_integration` binaries; `config_scope_ingestion_visual_debug_tdd.rs`; `resources/cube_4color.3mf`.
- OrcaSlicer comparison: none. ADR-0068 locks flat Orca-shaped wire compatibility, but this packet ports no OrcaSlicer behavior or source.

## Architecture Constraints

- `slicer-config` remains the deep owner of declaration-aware config semantics and depends only on `slicer-ir`; adapters may depend on it, never the reverse.
- Flat prefix spelling is an input compatibility detail. After `ConfigIngestor::ingest_flat`, no compatibility resolver or claim selector may call `starts_with("object_config:")`, `starts_with("paint_config:")`, or `starts_with("tool_config:")`.
- A scope delta contains authored entries only. No `ResolvedConfig::default()` value is inserted during ingestion, and an explicitly authored default-valued setting remains present.
- Exact registry declarations and `<prefix>:*` declarations both count as declared. `object_height:<id>` remains a global dynamic key until packet 05 because the approved plan assigns its typed per-object query replacement there.
- Registry scope denials are carried but not enforced here; queue row 7 owns loud rejection and legacy-table removal.
- Percent bounds semantics (measured during packet-03 implementation): the legacy `ConfigBoundsIndex` enforces a declared `[min, max]` against the raw magnitude of a `Percent` / `FloatOrPercent { is_percent: true }` value. A declared bound is expressed in the key's **absolute** unit (`overhang_reverse_threshold` declares `max = 10.0` mm) while a percent-authored value carries a **relative** magnitude whose absolute equivalent is only known at Phase-B expansion — packet 04's owner. Enforcing only the `min` side for percent-authored values is therefore required correct behaviour, not a relaxation: a negative percent is invalid in either domain, whereas an absolute max compared against a raw percentage rejects valid Orca data (`resources/cube_4color.3mf` authors `overhang_reverse_threshold = "50%"`, i.e. `50.0` against `max = 10.0` mm). `FloatOrPercent { is_percent: false }` and plain `Float`/`Int`/list values keep full two-sided bounds enforcement; the `check_percent_scalar` helper (`crates/slicer-scheduler/src/config_resolution.rs`) carries the asymmetric rule and its rationale. Baseline never surfaced this because an untyped `String` skips bounds in `check_value`; packet 03 typing the value correctly exposed the latent category error.
- Declaration repair (`tree_support_wall_count`, owner-authorized during implementation): both manifests declare `min = 0` rather than the original `min = 1`. Canonical OrcaSlicer declares `def->min = 0` for this key (`PrintConfig.cpp`, tooltip "range of [0,2]. 0 means auto") and clamps at the consumption site with `std::max(1, config.tree_support_wall_count.value)` (`TreeSupportCommon.hpp`, `TreeSupport.cpp::generate_toolpaths`) — and both PNP modules already replicate that clamp (`tree-support/src/lib.rs`, `tree-support-planner/src/lib.rs`). `resources/cube_4color.3mf` authors `0`, which AC-2 pins as `Int(0)`, so the original bound made the packet's own acceptance criterion unsatisfiable and rejected every one of the eight real fixtures. The change is one bound, is not a relaxation of enforcement (a negative wall count is still rejected), and leaves the module-visible behaviour identical because the clamp lives at the consumption site. Two further canonical divergences are deliberately **retained** and documented in both manifests so a later reader does not "fix" them blind: `max = 10` against canonical `2`, and `default = 1` against canonical `0`. Tightening the maximum would make a module-supported value unreachable (`tree-support/tests/tree_support_tdd.rs::tree_support_wall_count` pins `render(3) == 3`) with no correctness gain, and the default is behaviour-neutral because the module clamps. This is the only bounds violation across the whole 607-key fixture (census measured in the implementation session). `docs/15_config_keys_reference.md` is generated from manifests and was regenerated under the `gen-config-docs --check` gate.
- Selector values are derived from `RegistryEntry.selector`, global-only, and already typed before scheduler code reads them. Packet 01 makes `wall_generator` the sole current startup claim selector: `host_declaration` assigns false to ordinary host rows, manifest selector metadata defaults false (including both `spiral_vase` declarations, which omit it), and no module manifest declares `support_family`.
- No WIT or public IR version changes occur. Edited production paths are host-only and do not feed guest compilation, so the `wasm-staleness` snippet does not apply; executor and visual-debug tests still require a clean `cargo xtask build-guests --check` exit before diagnosis.
- Tests using structs with at least five public named fields must use FRU or an `// exhaustive: <reason>` waiver under `docs/21_data_defaults_and_fixtures.md`.

## Code Change Surface

- Selected approach: add one builder-style ingestion boundary in `slicer-config`, preserve source values until that boundary, return immutable typed deltas plus a typed selector map and warnings, then adapt existing consumers without implementing packet 05's final resolver.
- New `slicer_config::ingestion` exports:
  - `ConfigScope`: `Global`, `Object(ObjectId)`, `Modifier { object_id: ObjectId, modifier_id: ModifierId }`, `PaintSemantic(String)`, and `Tool(u32)`; derives `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, and `Hash`.
  - `ScopeDelta { values: BTreeMap<ConfigKey, ConfigValue> }`; only authored values.
  - `ScopedConfig { deltas: BTreeMap<ConfigScope, ScopeDelta> }` with `delta`, `global`, and deterministic iteration accessors.
  - `ConfigIngestor::new(&ConfigSchemaRegistry)`, `ingest_flat(&HashMap<ConfigKey, ConfigValue>)`, `ingest_delta(ConfigScope, &HashMap<ConfigKey, ConfigValue>)`, and `finish()`.
  - `IngestionOutcome { scoped: ScopedConfig, selector_values: BTreeMap<ConfigKey, ConfigValue>, warnings: Vec<IngestionWarning> }`.
  - `IngestionWarning::UnrecognizedKey { wire_key, key, suggestion }`.
  - `ConfigIngestionError::MalformedScopeKey { wire_key, expected }` and `TypeMismatch { key, expected, authored }`.
- Type policy: preserve an already matching `ConfigValue`; parse sidecar strings according to `RegistryEntry.field_type`; normalise `Int` to `Float` for `float`; parse `percent` and `float_or_percent` into their dedicated variants; recursively type homogeneous list declarations; reject all lossy/incompatible conversions.
- Ingestion strictness mode (owner decision, 2026-09-15): `ConfigIngestor::new` is **strict** and remains the default API — `ConfigIngestionError::TypeMismatch` on a declared-but-untypeable value, exactly as `AC-N2` pins, with its tests unchanged. An explicit opt-in `ConfigIngestor::tolerant` is used by the production load path only. Tolerant mode softens **declared-type incompatibility only**: it types what it can and, for a value whose authored shape the declaration cannot represent, emits one deterministic `IngestionWarning::UntypedValue { key, declared_type, authored }` and retains the authored value (loud, never silent, never a fallback to raw prefixed keys). Malformed scope encodings (`AC-N1`) and non-finite declared floats remain fatal in both modes.
  - Rationale and measured evidence: `resources/cube_4color.3mf` authors 16 registry-declared keys in shapes their declarations reject — 14 OrcaSlicer per-filament/per-extruder **arrays** (`nozzle_diameter`, `fan_max_speed`, `close_fan_the_first_x_layers`, `filament_start_gcode`, `slow_down_layer_time`, `wipe_tower_x`, …) declared as scalar `float`/`int`/`string`, and 2 Orca **tri-state strings** (`dont_filter_internal_bridges = "disabled"`, `enable_extra_bridge_layer = "disabled"`) declared `bool`. Hard rejection makes `AC-5`/`AC-6` (real fixture loads and slices) unreachable, and per-filament array semantics plus the bool tri-state domain are unmodelled by every packet in this queue. Repairing the declarations at source requires module-manifest edits that are out of bounds here and would churn packet-01's census and type-conflict gates. Retaining the authored value preserves today's runtime behaviour exactly (modules fall back to their own defaults for these keys) while making the mismatch observable. Shape normalisation for per-filament arrays and the tri-state domain are deferred to the packet that owns drop-unknown and declaration repair.
- Suggestion policy: compare the stripped canonical key to registry canonical keys, choose the lexicographically first minimum Levenshtein-distance candidate only when distance is at most two, and otherwise produce no suggestion.
- Wire policy: only `object_config:<id>:<key>`, `paint_config:<semantic>:<key>`, and `tool_config:<u32>:<key>` create typed scopes in packet 03. Missing identifiers/sub-keys and non-`u32` tool indices are `MalformedScopeKey`. Other colon-bearing keys, including matched dynamic wildcards, remain global keys.
- Declaration lookup: packet-01 `ConfigSchemaRegistry` is exact-string only — it exposes `keys()`, `entry()`, `len()`, `is_empty()` and no pattern or glob API. Wildcard declarations survive as literal registry keys (`"object_height:*"` and `"layer_height:*"` come from the layer-planner manifest). `ConfigIngestor` therefore performs the `<base>:*` fallback match itself: try `entry(key)` first, then `entry(format!("{base}:*"))` where `base` is the key with its final `:<suffix>` segment removed. A `<base>:*` hit supplies the declared field type for the dynamic key.
- Adapter policy: `coerce_string_to_config_value` ceases to decide declared types; 3MF source strings remain `ConfigValue::String` until registry ingestion. Native JSON bool/number/list shapes remain authored variants and are validated/normalised at ingestion.
- Claim policy: live loading reads only registry-marked `wall_generator` through `IngestionOutcome.selector_values`. It reads declared non-selector controls `spiral_vase` and `support_type` from the typed global `ScopeDelta`. Undeclared `support_family` is retained as a string with `IngestionWarning::UnrecognizedKey` and never promoted into `selector_values`; existing per-region support matching continues through resolved config in `module_claims_match_active_region`, and support candidates continue to survive startup dedup.
- Rejected alternatives and reasons:
  - Extending `classify_declared_key`: it sees only host declarations and repeats ADR-0067's defect for module keys.
  - Keeping prefix parsing in compatibility resolvers: it violates ADR-0068's decode-once boundary and makes packet 05 harder to prove.
  - Dropping unknowns now: packet 06 owns the warn-to-drop flip after census/no-drop evidence.
  - Adding aliases: explicitly assigned to the separate Orca feature-gap rename workstream.

## Files in Scope (read + edit)

Primary files:

- `crates/slicer-config/src/ingestion.rs` — role: deep ingestion module; expected change: add all typed scope, typing, warning, and error behavior.
- `crates/slicer-model-io/src/loader.rs` — role: 3MF adapter; expected change: preserve authored strings and stop schema-free heuristic typing.
- `crates/slicer-runtime/src/run.rs` — role: production composition root; expected change: retain registry-first ingestion output in both ordinary and visual-debug prepass paths.

Necessary seam/test files beyond the three primaries:

- `crates/slicer-config/src/lib.rs`, `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs` — public exports and focused contract tests.
- `crates/slicer-model-io/tests/threemf_project_settings_extraction_tdd.rs` — adapter regression.
- `crates/slicer-model-io/tests/mod_cilindrical_modifier_infill_density_tdd.rs`, `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` — adapter regressions widened after measurement: both assert heuristic coercion of string-authored sidecar values and are updated to exact string preservation once the loader is syntax-only.
- `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-scheduler/src/validation.rs` — parser compatibility, typed delta consumption, and selector claim seams.
- `crates/slicer-scheduler/src/lib.rs` — public re-export surface; expected change: export `dedup_same_claim_modules_with_typed_wall_generator` for the contract test while retaining the raw compatibility helper.
- `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — compatibility regressions updated for the typed boundary: percent profile values assert `FloatOrPercent`, malformed scope keys assert ingestion rejection per AC-N1, and non-finite declared floats assert ingestion-boundary `TypeMismatch`.
- `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-wasm-host/src/lib.rs` — manifest-first live-load orchestration and typed-ingestion return path.
- `crates/slicer-runtime/src/lib.rs` — public re-export surface; expected change: forward `load_live_modules_for_plan_manifest_first` and `ManifestFirstLiveLoadOutput` from `slicer-wasm-host` so the manifest-first typed-ingestion seam sits beside the existing live-load exports.
- `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — production-path regression.
- `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` — compatibility regression; expected change: stage each base key's declaring manifest as an external module so packet-01 registry assembly accepts the single-module registries that test constructs.
- `crates/slicer-scheduler/tests/contract/typed_selector_claim_selection_tdd.rs`, `crates/slicer-scheduler/tests/contract/main.rs` — typed selector test and bucket registration.
- `crates/pnp-cli/tests/config_scope_ingestion_visual_debug_tdd.rs` — real visual-debug bundle regression.
- `crates/slicer-scheduler/Cargo.toml` — add normal `slicer-config = { path = "../slicer-config" }` because scheduler APIs directly consume `ScopedConfig`/`IngestionOutcome` values.
- `crates/slicer-wasm-host/Cargo.toml` — add normal `slicer-config = { path = "../slicer-config" }` because live loading directly assembles the registry and invokes ingestion.
- `crates/slicer-runtime/Cargo.toml` — add normal `slicer-config = { path = "../slicer-config" }` because the composition root directly retains and passes packet-03 ingestion types.
- `docs/02_ir_schemas.md` — canonical wire-to-typed ingestion contract.
- `docs/04_host_scheduler.md` — only section “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)”; replace its current raw-source `wall_generator` requirement with the typed global-selector handoff and preserve classic/default plus spiral-vase fallback semantics.
- `docs/15_config_keys_reference.md` — generated host/module config-key catalog; expected change: regenerated by `xtask gen-config-docs` so the `tree_support_wall_count` bound matches the owner-authorized manifest `min = 0`.

## Read-Only Context

- `docs/spec_packets/config-scope-resolution_01_config-schema-registry/{packet.spec.md,requirements.md,design.md}` — promised packet-01 exports only; do not read its implementation plan.
- `docs/spec_packets/config-scope-resolution_02_authored-value-oracle/{packet.spec.md,requirements.md,design.md}` — packet-02 oracle contract only; do not read its implementation plan.
- `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` and `crates/slicer-runtime/tests/executor/main.rs` — packet-02 evidence; never edit to obtain green.
- `crates/slicer-ir/src/resolved_config.rs` — only `classify_declared_key`, `is_declared_float_or_percent_key`, `ResolvedConfig::apply_cli_key`, and declaration macro output; comparison context, not an edit target.
- `crates/slicer-ir/src/slice_ir.rs` — only `ObjectId`, `ModifierId`, `ConfigKey`, and `ConfigValue` definitions.
- `docs/19_visual_debug.md` — only `Request Shape` and `Reading A Bundle`; file is long.
- `resources/cube_4color.3mf` — never load directly; delegate ZIP member/value inspection and return bounded facts.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet 01, packet 02, and packet 04 — do not edit.
- Packet 01/02 `implementation-plan.md` files — predecessor exports may be reconstructed only from their `packet.spec.md`, `requirements.md`, and `design.md`.
- `OrcaSlicerDocumented/**` — no parity read is required; never load.
- `crates/slicer-schema/wit/**`, guest source, and IR schema/version constants — no boundary change belongs here. Module manifests are out of bounds as a *general* rule; the single owner-authorized exception is recorded under Locked Assumptions (`tree_support_wall_count` `min = 1 → 0`, canonical-proven and required by AC-2). Any further manifest edit is out of bounds without a fresh authorization.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and large JSON/binary fixture contents — never load.
- Alias tables, automatic expansion, final unified resolution, scope-eligibility enforcement, modifier-kind migration, and layer-range ingestion code — later packets.

## Expected Sub-Agent Dispatches

- Question: confirm packet 01's final registry/assembly exports consumed here after it lands, including `AssemblyOutcome.warnings: Vec<RegistryWarning>` as an opaque collection with no variant/field matching; scope: packet 01 `packet.spec.md`, `requirements.md`, `design.md`; return: `SUMMARY` ≤200 words; purpose: FORWARD-DEP reconciliation before Step 1.
- Question: confirm packet 02's four exact tests—one red principal, `oracle_authored_values_reach_owning_module_config_views`, and three green controls, `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control`—plus registration and fixture member contract after independent preflight; scope: packet 02 authority files plus test paths only; return: `FACT` ≤5 lines; purpose: FORWARD-DEP reconciliation before Step 1.
- Question: inventory every call site of the three prefix parsers, raw `wall_generator` selector reads, and raw non-selector claim-control reads; scope: named scheduler/runtime/wasm-host files; return: `LOCATIONS` ≤20; purpose: Steps 4-6 completeness.
- Question: inspect `cube_4color.3mf` only for the five named authored values and required ZIP members; scope: that archive; return: `FACT` ≤5 lines; purpose: Steps 1 and 7 without loading the fixture.
- Question: run each cargo command and report only pass/fail plus bounded failure context; scope: command named by its step; return: `FACT` or `SNIPPETS` ≤20 lines; purpose: every verification step.

## Data and Contract Notes

- IR/manifest contracts: `ConfigValue` remains the value representation; packet 01's registry remains the declaration authority. No IR struct or manifest schema changes are introduced.
- WIT boundary: unchanged. `ConfigView` still receives `ConfigValue`; this packet corrects the values and scopes before binding.
- Determinism/scheduler constraints: `BTreeMap` ordering, lexicographic warning order, lexicographic tie-breaking for suggestions, manifest-first registry assembly, and typed selector extraction make startup results independent of hash-map or module discovery order.
- FORWARD-DEP packet 01 supplies the registry/assembly surfaces consumed here: `ConfigSchemaRegistry` exposes `keys(&self) -> impl Iterator<Item = &str>`, `entry(&self, key: &str) -> Option<&RegistryEntry>`, `len(&self) -> usize`, and `is_empty(&self) -> bool`; `RegistryEntry` supplies the declaration data used for field typing, wildcard matching, and `selector` extraction; `AssemblyOutcome` supplies `registry: ConfigSchemaRegistry` and `warnings: Vec<RegistryWarning>`, and `assemble_registry` returns `Result<AssemblyOutcome, RegistryLoadError>`. Packet 03 treats the warning collection as opaque and consumes no `RegistryWarning` variant or field. The warning enum is non-exhaustive from packet 03's perspective; all variant/field details are packet-01 diagnostics and out of scope.
- Packet 01's exact selector fact consumed here is narrower than its metadata capability: `wall_generator` is the only current startup claim selector with `selector = true`; `host_declaration` assigns false to ordinary `HostConfigKey` rows; optional manifest selector metadata defaults false; neither `spiral_vase` declaration supplies it; and `support_family` is absent from module manifests. Packet 03 must not synthesize selector metadata for non-selector or undeclared controls.
- FORWARD-DEP packet 02 exports no production API. Its exact edit surface is the new `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs`, the `mod ingestion_fidelity_oracle_tdd;` registration in `crates/slicer-runtime/tests/executor/main.rs`, and one net-new `slicer-config = { path = "../slicer-config" }` entry under `crates/slicer-runtime/Cargo.toml` `[dev-dependencies]`; the existing `zip = { version = "2", default-features = false, features = ["deflate"] }` dev-dependency is reused unchanged, not added by packet 02. The file supplies exactly four tests: the one red principal `oracle_authored_values_reach_owning_module_config_views` and the three green controls `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control`; it reads raw `Metadata/project_settings.config` from `resources/cube_4color.3mf`, derives expectations from packet 01's registry, and is read-only in packet 03.

## Locked Assumptions and Invariants

- Packet 01 is implemented (commit `063e04eb`); the surfaces consumed here are reconciled against the landed crate. `RegistryWarning` is publicly matchable at HEAD, so packet 03's non-exhaustive treatment of the warning collection is a deliberate discipline, not an API guarantee; all variant/field details remain out of scope here.
- Packet 02's oracle file, aggregator registration, and `slicer-config` runtime dev-dependency have landed (commit `5e4362fe`); measured baseline is one red principal (`oracle_authored_values_reach_owning_module_config_views`, 49 mismatches) plus three green controls. The oracle is read-only evidence throughout packet 03.
- The five packet-02 divergences are locked: `skirt_loops` → `Int(1)`, `brim_width` → `Float(0.0)`, `filter_out_gap_fill` → `Float(0.0)`, `tree_support_wall_count` → `Int(0)`, and `support_interface_bottom_layers` → `Int(0)`.
- `Cargo.lock` is out of bounds for edits, but the three added direct `slicer-config` dependencies (`slicer-scheduler`, `slicer-wasm-host`, `slicer-runtime`, each `[dependencies]`) mechanically require the lockfile's two `"slicer-config"` dependency-list additions (`slicer-scheduler`, `slicer-wasm-host`; the runtime entry already had it from packet 02's dev-dependency); the lockfile was refreshed by cargo as a generated side effect, is listed as a modified path, and no other lockfile change belongs to this packet.
- Unknown keys are retained byte/variant-equivalently in packet 03. Only packet 06 may change that behavior. This applies to the **ingestion** boundary only: `crates/slicer-model-io`'s object-metadata allowlist (`object_metadata_to_config_data`) already drops unknown per-object XML keys before they can become `object_config:<id>:<key>`, and that pre-existing loader behavior is preserved by this packet, not widened. An allowlisted key whose authored value is invalid (for example `support_filament="abc"`) is retained verbatim as a string at the loader boundary; rejection is registry typing's job downstream.
- `object_height:<id>` and `layer_height:<id>` replacement remains packet 05; packet 03 must recognise matching wildcard declarations without pretending those keys are a config-scope encoding.
- Support-family candidates are not collapsed at startup; typed selector ingestion must not reintroduce the pre-packet-221 support dedup defect.

## Risks and Tradeoffs

- Reordering live loading can accidentally create a dependency cycle; the one-way dependency rule and a dedicated orchestration function avoid making `slicer-config` depend on scheduler/wasm-host.
- Transitional compatibility consumers could reintroduce prefix parsing or promote ordinary values into the selector map. A bounded call-site inventory plus AC-1/AC-4 closes those silent paths until packet 05 deletes the compatibility layer.
- Near-miss suggestions can become nondeterministic when distances tie; sorting registry keys before selecting the first minimum is mandatory.
- Preserving 3MF strings shifts failures from heuristic coercion to explicit registry errors; AC-N2 requires key/type/value diagnostics rather than fallback guessing.
- The visual-debug test loads guest artifacts; stale artifacts must be corrected before attributing failure to this packet.

## Context Cost Estimate

- Aggregate: `M`
- Largest steps: `M` (Steps 4, 5a, and 5b)
- Highest-risk dispatch and required return format: complete raw-prefix/raw-claim-control call-site inventory, `LOCATIONS` ≤20 entries.

## Open Questions

- `[FWD]` Reconcile packet 01's packet-03-consumed registry/assembly exports after its active implementation lands; preserve the opaque warning-collection boundary and adapt any shape drift without broadening TASK-564.
- `[FWD]` Reconcile packet 02's exact oracle names/fixture contract after its independent preflight; do not edit the oracle to accommodate packet 03.
- `[BLOCK]` None beyond the two forward dependencies above.
