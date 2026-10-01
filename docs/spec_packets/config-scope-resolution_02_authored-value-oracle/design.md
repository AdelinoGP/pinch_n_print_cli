# Design: authored-value-oracle

## Controlling Code Paths

- Primary path: new `ingestion_fidelity_oracle_tdd` tests (`crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs`) call `read_3mf_project_settings` for delivered input and `prepare_prepass_context` for live load, dedup, binding, and plan construction.
- Dedup seam: `dedup_same_claim_modules_with_wall_generator` (`crates/slicer-scheduler/src/execution_plan.rs`) selects `com.core.arachne-perimeters` or `com.core.classic-perimeters` before plan construction; support-family candidates remain retained.
- Observation seam: `CompiledModuleStatic::module_id` and `CompiledModuleStatic::config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) identify the exact owner and expose its immutable bound view.
- Independent expectation: raw zip/JSON decoding reads the source document without calling model-io coercion; packet-01 registry assembly supplies type and enum-domain declarations.
- Neighboring precedents: `build_live_execution_plan_filters_every_module_config_view_through_bind_helper` (`crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`) and `independent_support_layer_height_is_declared_and_bound_on_both_planners` (`crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs`).
- OrcaSlicer comparison: none; this packet tests an in-tree fixture against in-tree registry and scheduler contracts.

## Forward Dependency: Packet 01 Exact Exports

Packet 01 is now `implemented` (landed); this was a forward dependency until its exact registry API landed. These exact promised exports were reconciled against its landed code:

- `slicer_config::ConfigSchemaRegistry` owns a `BTreeMap<String, RegistryEntry>` and provides `keys`, `entry`, `len`, and `is_empty`.
- `slicer_config::RegistryEntry` is exactly `{ pub key: String, pub field_type: String, pub default: Option<String>, pub min: Option<f64>, pub max: Option<f64>, pub values: Option<Vec<String>>, pub denied_scopes: Vec<String>, pub selector: bool, pub base_key: Option<String>, pub host_meta: Option<slicer_ir::resolved_config::HostKeyMeta>, pub module_meta: Option<slicer_config::ModuleKeyMeta>, pub provenance: Vec<String> }`.
- `slicer_config::ModuleKeyMeta` is exactly `{ pub display: Option<String>, pub description: Option<String>, pub group: Option<String>, pub unit: Option<String>, pub tags: Vec<String>, pub advanced: bool }`.
- `slicer_config::ModuleDeclaration` is exactly `{ pub module_id: slicer_ir::ModuleId, pub schema: slicer_ir::config_schema::ConfigSchema, pub claim_exclusive_group: Option<String> }`.
- `slicer_config::HostChannels` has public `host_keys`, `speed_keys`, and `runtime_keys`, plus `from_live` and `from_parts`.
- `slicer_config::AssemblyOutcome` is exactly `{ pub registry: ConfigSchemaRegistry, pub warnings: Vec<RegistryWarning> }`.
- `assemble_registry(modules: &[ModuleDeclaration], host: &HostChannels) -> Result<AssemblyOutcome, RegistryLoadError>` is the deterministic constructor.

The oracle projects loaded manifests to `ModuleDeclaration` with `claim_exclusive_group: None`: it consumes registry entries, not packet-01's claim-exclusive warning behavior. Live claim selection is tested separately through the runtime plan.

## Architecture Constraints

- **Independent authorities:** expected values come only from raw fixture strings plus registry declarations. Delivered values come only from model-io plus the live runtime binding path. Neither side is used to derive the other.
- **Exact owner, not any carrier:** derive declaring module IDs from loaded schemas. A value carried by a different module does not satisfy an absent owner. Missing exact owners produce `MISSING_MODULE` diagnostics.
- **Selector-matrix observation:** clone one production-decoded source map. Preserve fixture-authored `wall_generator = "arachne"` in one run; replace only that selector with `ConfigValue::String("classic")` in the other. Assert the exact perimeter keep/drop sets before comparing values. Use the classic plan only for the classic perimeter owner and the arachne plan for all other owners.
- **Support behavior is not selector-simulated:** support family candidates survive startup dedup, so both support renderer/planner declarations are checked in the authored arachne plan. Do not add global support winner logic.
- **Default coincidence remains eligible:** the oracle observes typed values in `ConfigView`, not guest fallback output. `Float(0.0)` and `Bool(false)` remain unequal even if a guest's fallback magnitude is zero.
- **Automatic exclusion is value-sensitive:** exclude all `percent` declarations, percent-marked `float_or_percent` values, numeric zero with `base_key: Some(_)`, and integer `-1`. Do not exclude an entire field-type family when the authored value is already absolute and non-sentinel.
- **Enum derivation uses `RegistryEntry.values`:** an enum raw string outside the exact declared domain is malformed and excluded; omission of `values` from the consumed packet-01 shape is forbidden.
- **Determinism:** derive keys and module IDs into ordered maps/sets; sort diagnostics by `(key, module_id)`; do not assert filesystem enumeration order.
- **No vacuity or silent fixture skip:** assert raw values, eligible `(key, owner)` observations, arachne modules, and classic modules are non-empty. Missing fixture parts panic with their exact path/member.
- **Red means mismatch only:** the principal test accumulates value mismatches and then asserts that collection empty. Infrastructure, registry assembly, plan construction, missing owner, or selector-matrix errors fail earlier or with a distinct diagnostic and do not count as the accepted red state.
- **Dependency truth:** `zip`, `serde_json`, and `slicer-model-io` already exist under runtime dev-dependencies. Only `slicer-config` is added. No production dependency is authorized; the two host/speed retype changes in the Files-in-Scope exception use existing `slicer-ir` types only.
- **No contract/version changes:** IR schema, WIT package, CLI wire, and manifest schema are untouched. No edited path feeds guest WASM, and no geometry or coordinate conversion is involved.
- **Literal/test quality:** use FRU for new `ModuleDeclaration` literals if the watched-literal gate requires it; the comparator test carries a `// test-quality:` negative-control waiver naming the shared comparator.

## Code Change Surface

- Selected approach: one derived oracle file with shared helpers and four tests, mounted in the existing executor integration binary.
- `read_raw_authored_values` — opens the fixture with existing `zip`, finds exactly `Metadata/project_settings.config`, rejects a layer-range part, and returns scalar/list JSON without model-io coercion.
- `load_registry_and_owners` — calls `load_modules_from_roots`, projects schemas into packet-01 `ModuleDeclaration`s, assembles with `HostChannels::from_live`, and independently builds ordered declaring-module ownership from each schema.
- `derive_expected_value` — maps exact registry types to `ConfigValue`: `bool`, `int`, `float`, `string`, `enum`, `percent`, and `float_or_percent`; enum membership is checked through `values`; list declarations remain outside this scalar oracle.
- `requires_automatic_expansion` and `is_eligible` — implement the normative population rule in `requirements.md` without checking declaration defaults.
- `build_selector_plans` — loads one mesh, obtains the production-decoded sidecar, builds arachne/classic contexts through `prepare_prepass_context`, and returns plan observations keyed by exact module ID.
- `collect_observations` — for every eligible `(key, declaring module ID)`, chooses the proper plan, locates that ID, and records exact expected/delivered mismatch text or a distinct missing-module diagnostic.
- Four tests: `oracle_authored_values_reach_owning_module_config_views` (red), `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control` (green).
- Aggregator edit: add `mod ingestion_fidelity_oracle_tdd;` in lexical position in `crates/slicer-runtime/tests/executor/main.rs`.
- Manifest edit: add only `slicer-config = { path = "../slicer-config" }` under `[dev-dependencies]` in `crates/slicer-runtime/Cargo.toml`; preserve the existing `zip` line unchanged.
- Rejected alternatives:
  - A single fixture-authored plan: cannot observe classic after arachne dedup.
  - Matching an eligible key to whichever live module carries it: confuses ownership and cannot prove a removed declarer.
  - Calling `bind_module_config_view` directly on each manifest: bypasses the real dedup/plan behavior the packet must account for.
  - Using model-io output for expectations: self-referential to the defective coercion.
  - Excluding default-coincident values: contradicts the approved row-2 population and masks variant errors.
  - Adding `zip` again or as a normal dependency: duplicates an existing dev-dependency and widens production scope.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` (new) — oracle, matrix/population/comparator controls, and private helpers.
- `crates/slicer-runtime/tests/executor/main.rs` — one module registration.
- `crates/slicer-runtime/Cargo.toml` — one net-new `slicer-config` dev-dependency; existing `zip` unchanged.
- Doc-generator fallout owned by this packet's Out-of-Scope exception (see `requirements.md`): `xtask/src/gen_config_docs.rs` (optional per-key `type` honored before scalar inference), `docs/config/host-keys.toml` (per-key `type` for `internal_bridge_speed`), and `docs/15_config_keys_reference.md` (generated output — the host-speeds row for `internal_bridge_speed` is regenerated, not hand-edited).
- Retype fallout owned by this packet's Out-of-Scope exception (see `requirements.md`): `crates/slicer-ir/src/resolved_config.rs` (host `initial_layer_line_width` → `ResolvedFloatOrPercent`: declaration, config-map flatten, `PartialEq`/`Hash`, plus the `resolve_initial_layer_line_width_mm` helper), `crates/slicer-ir/src/feedrate.rs` (`internal_bridge_speed` → `float_or_percent`: `FeedrateField`, `SPEED_KEYS` wire table, `read_speed`, `resolve_internal_bridge_speed_mm`), its wire consumers `crates/slicer-config/src/lib.rs` (`HostChannels::from_live`) and `crates/slicer-scheduler/src/manifest.rs` (`build_host_key_entries`), the reader sites `crates/slicer-core/src/flow.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, and `crates/slicer-gcode/src/emit.rs`, typed fixtures in `crates/slicer-ir/tests/{resolved_config_defaults_tdd.rs,feedrate_default_tdd.rs,feedrate_from_raw_config_tdd.rs}` and `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (speed-typed `ConfigViewBuilder` fixture), `crates/slicer-config/tests/registry_census_tdd.rs` (speed-field one-liner), `crates/slicer-config/tests/registry_assembly_tdd.rs` (assembly fallout), `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, and `crates/slicer-sdk/src/test_support/fixtures.rs` (`ConfigViewBuilder`).

## Read-Only Context

- Packet-01 `design.md` §Code Change Surface — final exported registry shapes.
- `prepare_prepass_context` and `PrepassContext` (`crates/slicer-runtime/src/run.rs`) — supplied-map and returned-plan contract; symbol-targeted range only.
- `dedup_same_claim_modules_with_wall_generator` and `CompiledModuleStatic` (`crates/slicer-scheduler/src/execution_plan.rs`) — selector and observation contracts; symbol-targeted ranges only; `bind_module_config_view` moves to Files in Scope only for retype fallout.
- `LoadedModule` accessors and `load_modules_from_roots` (`crates/slicer-scheduler/src/manifest.rs`) — schema/owner projection; symbol-targeted ranges only.
- `read_3mf_project_settings` and `coerce_string_to_config_value` (`crates/slicer-model-io/src/loader.rs`) — delivered-input defect path; symbol-targeted ranges only.
- `ConfigValue` and `ConfigView` (`crates/slicer-ir/src/slice_ir.rs`) — exact variants/accessor behavior; symbol-targeted ranges only.
- `resources/cube_4color.3mf` — programmatic bounded fixture inspection only; never load binary bytes into context.

## Out-of-Bounds Files

- Packet 01, 03, and 04 artifacts; `docs/specs/config-scope-resolution-plan.md`; `docs/07_implementation_status.md` — read-only as bounded authority, never edit.
- `crates/slicer-scheduler/src/**` except the `build_host_key_entries` retype fallout in `manifest.rs`, `crates/slicer-runtime/src/**`, and `modules/core-modules/**` except the one retyped typed fixture `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` — production/read-only; no edits beyond the Files-in-Scope retype fallout list.
- `resources/cube_4color.3mf`, `target/`, generated code, vendored dependencies, and all unrelated crates — never edit or load broadly (`Cargo.lock` and guest lockfiles change only through the authorized refresh/retype dependency resolution). Exception: the three doc-generator paths named in Files in Scope (`xtask/src/gen_config_docs.rs`, `docs/config/host-keys.toml`, `docs/15_config_keys_reference.md`) are in scope — no other `docs/` or `xtask/src/` file is.
- `OrcaSlicerDocumented/**` — no parity question exists for this packet.

## Expected Sub-Agent Dispatches

- Question: confirm packet 01 landed the exact `ConfigSchemaRegistry`, complete `RegistryEntry` including `values`, `ModuleKeyMeta`, `ModuleDeclaration`, `HostChannels`, `AssemblyOutcome`, and `assemble_registry` shapes; scope: named exports in `crates/slicer-config/src/lib.rs`; return: `FACT` ≤5 lines; purpose: Step 1 forward-dependency gate.
- Question: derive fixture scalar facts, module declarers, and both selector-plan module-ID sets without editing; scope: named fixture member and loaded core manifests; return: `FACT` ≤5 lines; purpose: Step 1 driveability gate.
- Question: run each AC command and return only pass/fail plus bounded mismatch snippets; scope: runtime executor test; return: `FACT` ≤5 lines or `SNIPPETS` ≤20 lines on failure; purpose: Steps 2-3.
- Question: run check-literals and test-quality report for touched tests; scope: touched files; return: `FACT` ≤5 lines; purpose: Step 3.

## Data and Contract Notes

- IR/manifest contracts: consumed read-only; no schema/version edit, except the two registry-assembly retype wires named in the Doc Impact Statement (`ResolvedConfig.initial_layer_line_width` → `ResolvedFloatOrPercent`, `FeedrateConfig.internal_bridge_speed` → `ResolvedFloatOrPercent`/`float_or_percent`) — no IR/WIT version constant changes either way.
- Resolution behavior change: percent-authored `initial_layer_line_width` and `internal_bridge_speed` now carry their percent bit to their resolution helpers (`resolve_initial_layer_line_width_mm` over nozzle diameter, `resolve_internal_bridge_speed_mm` over `bridge_speed`); resolution expectations for scope/expansion stay packets 4/5.
- Census coverage: `registry_census_tdd`'s speed field-type derivation follows the `SPEED_KEYS` wire table (the `internal_bridge_speed` one-liner), and `registry_assembly_tdd` stays green with the retyped channels.
- WIT boundary: untouched.
- Scheduler determinism: both plans use the same module roots and decoded source except the controlled whole-print selector; exact module IDs are the join key between manifest ownership and compiled bindings.
- Report format: `MISMATCH key=<key> module=<module-id> authored=<quoted-raw> expected=<ConfigValue> delivered=<ConfigValue|None>`; missing owners use the separate `MISSING_MODULE` form.

## Locked Assumptions and Invariants

- TASK-563 and `draft` status remain unchanged.
- Packet 02 closes red; packet 03 owns the green transition.
- The five plan-catalogued key names are absent as Rust string literals in the oracle source.
- Every expected value is raw-document + registry derived, every owner is manifest derived, and every delivered value is read from that exact compiled owner's production-bound view.
- Default-coincident keys are included; automatic placeholders are excluded by authored value semantics.
- Classic-perimeter observations come only from the classic selector plan; no assertion pretends classic survives the authored arachne plan.

## Risks and Tradeoffs

- Packet-01 API drift blocks compilation. Reconcile to packet 01; do not invent compatibility aliases in this packet.
- Two prepass contexts do more work than direct binding, but they are required to prove the observation survives real claim selection; runtime impact is unmeasured.
- The conservative integer `-1` exclusion can omit a future literal negative value that is not automatic. If registry metadata later distinguishes automatic sentinels, replace the conservative predicate in the owning expansion packet rather than adding a key roster here.
- A new claim-exclusive module family could produce another absent owner. `MISSING_MODULE` intentionally blocks packet closure until the selector matrix is extended from an authoritative selector declaration, not silently skipped.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, one integration-test module with two live plans and independent derivation)
- Highest-risk dispatch and required return: exact packet-01 API reconciliation, `FACT` ≤5 lines.

## Open Questions

- `[FWD]` Packet 01 must land the exact exports listed above before Step 2.
- `[BLOCK]` None beyond the declared forward dependency and independent preflight.
