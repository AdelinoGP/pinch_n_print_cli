# Design: config-schema-registry

## Controlling Code Paths

- `crates/slicer-ir/src/config_schema.rs` — new module-path home for `ConfigFieldEntry` and `ConfigSchema`; the former keeps its existing parsed/UI fields, drops `validate` in the retirement step, and gains serde-defaulted `selector`, `base_key`, and `denied_scopes`.
- `crates/slicer-ir/src/slice_ir.rs` — module-path home for `AggregatedRegionSplitEntry` and `RegionSplitValueType`; `crates/slicer-ir/src/lib.rs` owns the exact flat re-exports.
- `crates/slicer-ir/src/resolved_config.rs` — owns the const-safe `HostRuntimeKey`, `HOST_RUNTIME_KEYS`, and relocated `DEFAULT_WALL_GENERATOR`.
- `crates/slicer-ir/src/feedrate.rs` — existing module-level `pub const SPEED_KEYS: &[(&str, fn(&mut FeedrateConfig) -> &mut f32)]`; no edit is expected.
- `crates/slicer-scheduler/src/manifest.rs` — remains the manifest parser and config-schema JSON emitter; it re-exports relocated config types, parses the three new optional fields, migrates the host-runtime loop, and retires `validate`/bumps the wire version.
- `crates/slicer-config/src/lib.rs` — owns `ConfigSchemaRegistry`, reconciliation, row-1 structural validation, errors/warnings, channel assembly, and the public row-1 API.
- `modules/core-modules/*/<name>.toml`, the five guest `src/lib.rs` readers, and `crates/slicer-runtime/src/layer_executor.rs` — the exact declaration and percent-reader repair surface.
- `xtask/src/gen_config_docs.rs` — read-only generator authority; `render_table` emits backticks only around key/default/owner cells, not type or range cells. AC-9 proves that shape by anchoring on the first key cell and final owner cell, so the raw pipes in an enum range cannot create a false row count.

## Architecture Constraints

- **Layering:** `slicer-config` depends on `slicer-ir` only. `slicer-ir` gains no first-party dependency. `slicer-core` drops its `slicer-scheduler` dependency; scheduler continues to depend on IR/schema and preserves transitional aliases.
- **Config-schema aliases:** `ConfigSchema` is defined at `slicer_ir::config_schema::ConfigSchema`; `slicer-ir/src/lib.rs` must contain the exact flat lines `pub use config_schema::{ConfigFieldEntry, ConfigSchema};` and `pub use slice_ir::{AggregatedRegionSplitEntry, RegionSplitValueType};`. Scheduler's module aliases and flat exports preserve existing downstream paths, with `crates/slicer-scheduler/src/lib.rs` containing the exact transitional lines `pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};` and `pub use region_split::AggregatedRegionSplitEntry;`.
- **Const-safe host carrier:** `HostConfigKey.default` is `Option<String>`, so it is not const-friendly. The only accepted runtime carrier is:
  ```rust
  pub struct HostRuntimeKey {
      pub key: &'static str,
      pub field_type: &'static str,
      pub scope: &'static str,
      pub default: &'static str,
      pub meta: HostKeyMeta,
      pub selector: bool,
      pub denied_scopes: &'static [&'static str],
  }
  pub const HOST_RUNTIME_KEYS: &[HostRuntimeKey] = &[
    HostRuntimeKey { key: "use_relative_e_distances", field_type: "bool", scope: SCOPE_PRINTER, default: "true", meta: HostKeyMeta::NONE, selector: false, denied_scopes: &[] },
    HostRuntimeKey { key: "thumbnail_path", field_type: "string", scope: SCOPE_PRINTER, default: "", meta: HostKeyMeta { display: Some("Thumbnail path"), description: Some("File path the slicer writes its thumbnail plate into; empty disables thumbnails."), group: Some("Output"), ..HostKeyMeta::NONE }, selector: false, denied_scopes: &[] },
    HostRuntimeKey { key: "wall_generator", field_type: "string", scope: SCOPE_PRINT, default: "classic", meta: HostKeyMeta::NONE, selector: true, denied_scopes: &["object", "layer_range", "modifier", "paint_semantic", "tool"] },
  ];
  ```
   Host rows may be formatted across lines but may not change their fields or values. The empty rows use `denied_scopes: &[]`; the `wall_generator` row uses `denied_scopes: &["object", "layer_range", "modifier", "paint_semantic", "tool"]`, exactly the scopes below `global/print` in the plan's order `global/print < object < layer_range < modifier < paint_semantic < tool`. `HostChannels::from_live`/assembly converts `default` to `Some(String)` when forming a common `HostConfigKey` and copies the static denial slice into `RegistryEntry.denied_scopes`; the `HostRuntimeKey` itself never embeds `HostConfigKey`. `build_host_key_entries` continues emitting the existing host JSON fields and ignores `selector` and `denied_scopes`; no runtime-carrier wire behavior is claimed to change.
- **Four channels:** assembly reads `ResolvedConfig::host_config_keys()`, module-level `slicer_ir::feedrate::SPEED_KEYS` plus `SPEED_META`, `slicer_ir::resolved_config::HOST_RUNTIME_KEYS`, and `ModuleDeclaration` manifest schemas. It does not expand automatic values.
- **Reconciliation:** type and enum domains are fatal disagreements; numeric bounds intersect and produce a warning naming all declarers; defaults choose a host declaration before the alphabetically-first module; claim-exclusive divergent defaults/bounds warn without changing deterministic selection; `denied_scopes` is a union; UI metadata is advisory (host then alphabetical); every entry retains sorted provenance.
- **Validation boundary:** packet 1 carries and reconciles selector flags, and structurally rejects any host or module declaration marked `selector` when its represented `denied_scopes` policy leaves one of the per-region scopes `object`, `layer_range`, `modifier`, `paint_semantic`, or `tool` statable. An absent/empty denial means every scope is statable under ADR-0069. The `wall_generator` host row explicitly denies all five narrower scopes and therefore passes; `global/print` remains allowed. Base keys must exist, be percent-compatible, and be acyclic; denied scopes are limited to `global`, `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool`. Packet 7 (`TASK-568`) owns the later broad machine/emitter deny-list rollout, not this required selector policy.
- **Wire/version lock:** `CONFIG_SCHEMA_WIRE_VERSION` moves exactly from `"1.2.0"` to `"1.3.0"` only for the `validate` removal under owner decision 1. The version assertion and all validate-bearing literal/assertion fallout land in that step.
- **Schema field shape:** final `ConfigFieldEntry` is the current field set without `validate`, plus `selector: bool`, `base_key: Option<String>`, and `denied_scopes: Vec<String>` with serde defaults. `ConfigSchema` remains `{ pub entries: BTreeMap<String, ConfigFieldEntry> }`.
- **Guest freshness:**
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- AC-8's static reader check is syntax-tolerant about receiver names, whitespace, and line breaks, but not about semantics: for every required file/key pair it requires a `get_abs_value` method call whose first argument is the exact repaired key, whose second argument is a non-empty explicit base expression, and whose returned option is consumed. Its result-consumer proof rejects a standalone `get_abs_value("bridge_line_width", nozzle_diameter);` statement, rejects `_ = get_abs_value("bridge_line_width", nozzle_diameter);` discard, and accepts a real assignment, `return`, enclosing function/macro argument, or fallback/method chain.
- **No geometry/coordinate work:** the coordinate-system snippet is not applicable.

## Code Change Surface

- **Selected approach:** perform ordered relocation and edge cleanup; add the crate shell; retire the dead wire field; implement the registry and its two direct integration-test binaries; repair manifests; migrate only the enumerated readers; then update docs/ADR (including the bounded `docs/04_host_scheduler.md` RegionMapping threading section) and run generated-doc, guest, compile, lint, literal, and test-quality gates.
- **Exact public API:**
  - `slicer_config::ConfigSchemaRegistry` owns a `BTreeMap<String, RegistryEntry>` and exposes `keys(&self) -> impl Iterator<Item = &str>`, `entry(&self, key: &str) -> Option<&RegistryEntry>`, `len(&self) -> usize`, and `is_empty(&self) -> bool`.
  - `slicer_ir::resolved_config::HostRuntimeKey` is `{ pub key: &'static str, pub field_type: &'static str, pub scope: &'static str, pub default: &'static str, pub meta: HostKeyMeta, pub selector: bool, pub denied_scopes: &'static [&'static str] }`; its `wall_generator` row carries exactly `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool`.
  - `slicer_config::RegistryEntry` is `{ pub key: String, pub field_type: String, pub default: Option<String>, pub min: Option<f64>, pub max: Option<f64>, pub values: Option<Vec<String>>, pub denied_scopes: Vec<String>, pub selector: bool, pub base_key: Option<String>, pub host_meta: Option<slicer_ir::resolved_config::HostKeyMeta>, pub module_meta: Option<ModuleKeyMeta>, pub provenance: Vec<String> }`; runtime denial slices are copied into this owned registry field during assembly.
  - `slicer_config::ModuleKeyMeta` is `{ pub display: Option<String>, pub description: Option<String>, pub group: Option<String>, pub unit: Option<String>, pub tags: Vec<String>, pub advanced: bool }`.
  - `slicer_config::ModuleDeclaration` is `{ pub module_id: slicer_ir::ModuleId, pub schema: slicer_ir::config_schema::ConfigSchema, pub claim_exclusive_group: Option<String> }`.
  - `slicer_config::HostChannels` is `{ pub host_keys: Vec<slicer_ir::resolved_config::HostConfigKey>, pub speed_keys: Vec<slicer_ir::resolved_config::HostConfigKey>, pub runtime_keys: Vec<slicer_ir::resolved_config::HostRuntimeKey> }` with `from_live()` and `from_parts(host_keys, speed_keys, runtime_keys)`; runtime rows stay `HostRuntimeKey` until assembly converts their defaults for common-field reconciliation and copies their static denials.
  - `slicer_config::AssemblyOutcome` is `{ pub registry: ConfigSchemaRegistry, pub warnings: Vec<RegistryWarning> }`.
  - `slicer_config::RegistryWarning` has `ClaimExclusiveDivergence { key, first_module, first_value, second_module, second_value }`, `ClaimExclusiveBoundsDivergence { key, first_module, first_min: Option<f64>, first_max: Option<f64>, second_module, second_min: Option<f64>, second_max: Option<f64> }`, `BoundsIntersection { key, effective_min, effective_max, declarers: Vec<String> }`, and `ModuleDefaultForHostKey { key, module_id }`.
  - `slicer_config::RegistryLoadError` has `TypeDisagreement { key, declarers: Vec<String> }`, `EnumDomainDisagreement { key, declarers: Vec<String> }`, `SelectorStatablePerRegion { key }` for the host/module selector structural check, `BaseKeyMissing { key, base }`, `BaseKeyNotPercentCompatible { key, base }`, `BaseKeyCycle { key, base }`, and `UnknownDeniedScope { key, scope }`.
  - `slicer_config::assemble_registry(modules: &[ModuleDeclaration], host: &HostChannels) -> Result<AssemblyOutcome, RegistryLoadError>` is deterministic and non-panicking for expected declaration conflicts.
- **Exact pre-existing edits:** `RegionSplitDeclaration.value_type` remains `RegionSplitValueType`; the scheduler JSON emitter keeps its host object shape; `ConfigSchema.entries` remains public; `DEFAULT_WALL_GENERATOR` remains `"classic"` through its scheduler re-export; all existing scheduler/runtime import paths remain resolvable through re-exports.
- **Rejected alternatives:** embedding `HostConfigKey` in a const runtime row (owned default is not const-friendly); retaining scheduler-owned schema types (violates the dependency direction); moving runtime keys into `slicer-config` (inverts host-schema ownership); a hand-listed census roster (violates docs/22); widening scalar IR fields to absorb percent values (expansion belongs to later rows); and changing the JSON host object to serialize `selector` (not approved by row 1).

## Files in Scope (read + edit)

Each implementation substep edits at most three paths; this is the complete aggregate surface:

- `crates/slicer-ir/src/config_schema.rs` (new) — relocated schema carriers and final typed fields.
- `crates/slicer-ir/src/slice_ir.rs` — relocated region-split carriers.
- `crates/slicer-ir/src/resolved_config.rs` — exact const-safe runtime rows and default constant.
- `crates/slicer-ir/src/lib.rs` — module declaration and exact flat re-exports; the config re-export is added in Step 1a and the region-split re-export in Step 1b-ii, after its IR definitions exist.
- `crates/slicer-scheduler/src/manifest.rs` — aliases, parser, runtime-row consumer, wire retirement/version.
- `crates/slicer-scheduler/src/region_split.rs`, `crates/slicer-scheduler/src/lib.rs`, and `crates/slicer-scheduler/src/execution_plan.rs` — transitional module/flat aliases and default-constant re-export; `crates/slicer-scheduler/src/lib.rs` owns the two exact flat compatibility lines pinned above.
- `crates/slicer-core/Cargo.toml`, `src/algos/region_mapping.rs`, and its existing region-mapping test import site — edge removal/import migration only.
- root `Cargo.toml` and new `crates/slicer-config/{Cargo.toml,src/lib.rs}` — workspace/crate shell and registry owner.
- new `crates/slicer-config/tests/registry_assembly_tdd.rs` and `registry_census_tdd.rs` — direct integration binaries; no aggregator registration is needed.
- `crates/slicer-scheduler/src/manifest.rs` inline tests, `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-runtime/tests/contract/{config_view_binding_tdd.rs,raft_bounds_tdd.rs}`, `crates/slicer-runtime/tests/integration/{region_mapping_tdd.rs,runtime_wiring_tdd.rs}`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, and `crates/slicer-scheduler/tests/unit/execution_plan_tdd.rs` — exact wire/field assertion fallout for the 19 construction literals.
- `modules/core-modules/{arachne-perimeters,classic-perimeters,gyroid-infill,lightning-infill,rectilinear-infill,wave-overhangs,traditional-support}/*.toml` — pinned declaration edits only.
- `crates/slicer-runtime/src/layer_executor.rs` and the five listed guest `src/lib.rs` files — pinned reader migrations only; classic guest is read-only control.
- `docs/03_wit_and_manifest.md` (`## Module Manifest Schema (TOML)` example, common per-field table, cross-field validation example, and `## Validation Expression Language`), `docs/04_host_scheduler.md` (the bounded `### RegionMapping (Builtin) — \`aggregated_region_split\` Threading` section), `docs/05_module_sdk.md` (`pnp_cli module diagnose` checks), `crates/slicer-scheduler/src/manifest.rs` (config-schema wire doc/serialization/parser), `docs/adr/0019-aggregated-region-split-entry-cross-crate-dependency.md` (`## Status`), and `crates/slicer-schema/src/lib.rs` (`VALID_SEVERITIES` constant/comment only) — doc/ADR retirement surfaces. The docs/04 edit replaces only the stale deferred dependency paragraph.
- `docs/15_config_keys_reference.md` — tool-generated output only; never hand-edit generated blocks.

## Read-Only Context

- `crates/slicer-ir/src/resolved_config.rs` — ranges around `HostConfigKey`/`HostKeyMeta`, macro-generated `host_config_keys`, and the host declaration tail; locate before reading.
- `crates/slicer-ir/src/feedrate.rs` — `SPEED_KEYS`, `SPEED_META`, and the `SPEED_KEY_COUNT` assertion only.
- `crates/slicer-ir/src/slice_ir.rs` — `ModuleId`, `ConfigView::get_abs_value`, and the region-split placement neighborhood only.
- `crates/slicer-scheduler/src/manifest.rs` — top-level wire constant, current type definitions, `read_config_schema`, runtime tuple/consumer, JSON emitter, and inline tests at located ranges.
- `crates/slicer-scheduler/src/{region_split.rs,lib.rs,execution_plan.rs}` — current definitions/re-export blocks and default constant neighborhood.
- `crates/slicer-core/Cargo.toml` and `src/algos/region_mapping.rs` — dependency and import ranges only.
- the seven manifest TOMLs and the seven reader/control source files — only the named key tables/read functions.
- `xtask/src/gen_config_docs.rs` — `KeyRow`, `module_rows`, and `render_table`; `docs/15` generated markers/rows only.
- `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, ADR-0019, `docs/11_operational_governance_and_acceptance_gate.md`, and docs/21/22 — the bounded sections named in `requirements.md`.
- `docs/04_host_scheduler.md` — only the `RegionMapping (Builtin) — aggregated_region_split Threading` section, bounded by its next `###` heading.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, `docs/07_implementation_status.md`, and every other packet directory — read-only/forbidden edits.
- `OrcaSlicerDocumented/**` — delegated only; never load directly.
- `target/`, `Cargo.lock`, generated code outside the generator output, and vendored dependencies — never load.
- `crates/slicer-schema/src/lib.rs` except `VALID_SEVERITIES` plus its stale cross-validation comment in Step 6a.
- `crates/slicer-scheduler/src/manifest.rs` except the config-schema wire documentation, parser, and serializer fragments owned by the validate retirement.
- all crates/modules not listed in Files in Scope, including WIT files, `slicer-gcode`, `slicer-sdk`, `slicer-macros`, wasm-host sources, and unrelated scheduler/runtime consumers.
- `docs/config/host-keys.toml` — read-only lock-test context.

## Expected Sub-Agent Dispatches

- Question: re-derive the `ConfigFieldEntry` construction census after relocation and before field retirement; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20 with the 19 literal sites grouped by file; purpose: blast-radius ownership.
- Question: parse only `modules/core-modules/*/<name>.toml` and report entries, distinct keys, multi-declared keys, wildcard owners, and the three repair declarer sets; scope: those manifests; return: `FACT` ≤5 lines; purpose: independent census and type-repair guard.
- Question: classify every `bridge_line_width`/`initial_layer_line_width` read under the listed host/guest source paths by access method; scope: those paths; return: `LOCATIONS` ≤20; purpose: reader migration list.
- Question: inspect canonical `PrintConfig.cpp` add-call declarations for the three repairs; scope: one Orca path; return: `LOCATIONS` ≤20; purpose: parity evidence.
- Question: run each targeted cargo/generator/guest command named by a step; scope: exact command only; return: `FACT` pass/fail and at most 20 failure lines; purpose: step gate evidence.

## Data and Contract Notes

- `ConfigFieldEntry` retains owned module-parsed strings and `ConfigSchema.entries`; new optional manifest fields default to `false`/`None`/empty so existing TOML remains loadable. `validate` is absent only after the retirement step.
- Host metadata remains `HostKeyMeta` with static fields; module metadata is `ModuleKeyMeta` with owned `String`s. No lifetime extension or module-string coercion into `HostKeyMeta` is allowed.
   - `HostRuntimeKey`'s `default` is converted to `HostConfigKey.default = Some(row.default.to_owned())` at channel assembly, while `row.denied_scopes` is copied into the registry entry. The current scheduler host JSON continues to use key/type/default/scope/metadata and does not expose selector or denials. `wall_generator` is the one host selector whose complete five-scope denial policy is authored and validated in packet 1; packet 7 owns the broad machine/emitter rollout.
- `slicer_ir::feedrate::SPEED_KEYS` is iterated together with `SPEED_META`; its function pointers are used to derive defaults from `FeedrateConfig::default()` without inventing a second table.
- Registry ordering is deterministic: BTreeMap keys, alphabetical module tie-break, sorted warning declarers/provenance, and no filesystem-order dependence in expected output.
- The census uses `toml::from_str` over the same manifest path/name rule used by `xtask` and `read_config_schema`; wildcard entries remain keys in the registry.

## Locked Assumptions and Invariants

   - The exact seven-field `HostRuntimeKey` shape and three rows are fixed; no `config: HostConfigKey` field is permitted, and `wall_generator`'s denial slice is exactly `object`, `layer_range`, `modifier`, `paint_semantic`, `tool`.
- `ConfigSchema`'s new API field uses `slicer_ir::config_schema::ConfigSchema`; flat aliases are compatibility surfaces, not the definition home.
- `slicer-config` has only `slicer-ir` as a normal first-party dependency; `toml = "0.8"` is dev-only.
- `SPEED_KEYS` remains module-level, 26 entries, and is referenced through `slicer_ir::feedrate::SPEED_KEYS`.
- `initial_layer_line_width` has exactly five declarers and each receives `base_key = "nozzle_diameter"`; `wave-overhangs` has no such declaration. `bridge_line_width` has six declarers, including wave.
   - `RegistryEntry` preserves enum values, denied-scope union, UI carriers, selector/base key, and sorted provenance; no warning is silently discarded. Selector eligibility is validated from the represented denial policy for both host and module rows; an absent/empty policy remains invalid for a selector.
- `CONFIG_SCHEMA_WIRE_VERSION` is exactly `"1.3.0"` after `validate` removal; no IR/WIT version changes.
- Generated docs are regenerated, not hand-edited; guest artifacts are fresh before closure.

## Risks and Tradeoffs

- Moving a public struct affects 19 literals across 7 files; the plan owns the census and FRU/waiver fallout rather than relying on a late compiler discovery.
- The scheduler manifest is large and combines parser, JSON, host rows, and inline tests; every read is ranged and each semantic edit has a separate step exit.
- Strict type/enum validation can expose an unlisted real declaration; the implementation must stop and re-census rather than widen the packet.
- Guest source changes require an artifact rebuild and an exit-code check; an infrastructure exit is not evidence of freshness.
- The host carrier relocation intentionally leaves selector out of the existing host JSON, so registry tests—not wire snapshots—prove selector propagation.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (registry assembly plus independent census, split across direct test binaries and a parser substep).
- Highest-risk dispatch: manifest/reader census; required return format is `FACT` or `LOCATIONS` ≤20, never a full file or build log.

## Open Questions

None. The percent-compatible base-type rule is fixed by the negative tests (non-compatible types reject); no implementer choice remains that changes row-1 scope.
