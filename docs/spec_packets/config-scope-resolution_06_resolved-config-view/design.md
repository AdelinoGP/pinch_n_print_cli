# Design: resolved-config-view

## Controlling Code Paths

**Resolution**

`resolve_scope_stack` → `apply_delta` → `expand_automatic_values`. The first two live in `crates/slicer-config/src/resolution.rs` and the third in `crates/slicer-config/src/lib.rs`. They are called from `resolve_runtime_scopes` (`crates/slicer-runtime/src/run.rs`) for global, object, and region configs.

- `apply_delta` inserts every key that `apply_cli_key` does not claim into `extensions`. Unknown keys retained by ingestion land there too.
- `expand_automatic_values` also writes the documented `support_line_width` shadow into `extensions`.

**Host key table**

`declare_resolved_config!` / `__drc!` in `crates/slicer-ir/src/resolved_config.rs` generates:

- `ResolvedConfig` and `apply_cli_key` (`cli` / `cli_opt` rows only; `plain` rows fall through to `Ok(false)`);
- `host_config_keys()`, which excludes `plain` rows.

The same file holds `HostKeyMeta` (with `HostKeyMeta::NONE`), `HOST_RUNTIME_KEYS`, and the hand-written `to_config_map`. `HOST_RUNTIME_KEYS` currently holds `use_relative_e_distances`, `thumbnail_path`, and `wall_generator` (a `selector` row). Each runtime row carries a `HostKeyMeta`.

**Registry**

`assemble_registry(&[ModuleDeclaration], &HostChannels) -> Result<AssemblyOutcome, RegistryLoadError>` produces `ConfigSchemaRegistry { entries: BTreeMap<String, RegistryEntry> }` (`crates/slicer-config/src/lib.rs`).

- `RegistryEntry` derives `Default`.
- Module declarations come from `parse_config_field_entry` → `ConfigFieldEntry`. The parser is in `crates/slicer-scheduler/src/manifest.rs`; `ConfigFieldEntry` is defined in `crates/slicer-ir/src/config_schema.rs` and derives `Default` and `Serialize`.

**Load-time view**

`bind_module_config_view(module: &LoadedModule, source: &HashMap<ConfigKey, ConfigValue>) -> Arc<ConfigView>` (`crates/slicer-scheduler/src/execution_plan.rs`) is called only from `build_live_execution_plan(.., config_source: &HashMap<ConfigKey, ConfigValue>, ..)` (`crates/slicer-wasm-host/src/execution_plan_live.rs`).

- The production callers are `run_slice_with_collector` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`).
- Both pass `expanded_global_source`, built by `typed_module_config_source` → `overlay_expanded_global` → `overlay_object_layer_planning`.
- `load_live_modules_for_plan_manifest_first` in the same file reads `SUPPORT_GENERATOR_CONFIG_KEY` / `SUPPORT_FAMILY_CONFIG_KEY` from the ingested global `ScopeDelta`'s values.

**Per-region view**

The layer-dispatch region view in `crates/slicer-wasm-host/src/dispatch.rs` is built as `ConfigView::from_declared(&resolved_config_to_map(map.config_for(&key)), declared_keys)`. It needs no code change; it becomes complete through seeding.

**Emission**

- `run_slice_with_collector` → `run_pipeline_fork` (both in `crates/slicer-runtime/src/run.rs`) → crate-private `run_pipeline_with_raw_config_authority` / `run_pipeline_with_instrumentation_authority` → `run_pipeline_core` → `run_postpass_with_thumbnail` (all in `crates/slicer-runtime/src/pipeline.rs`).
- `run_postpass_with_thumbnail` works in four stages:
  - it reads `thumbnail_path` from `raw_config_source`;
  - it builds `effective_config = resolved_config_to_map(default_resolved_config)`;
  - it overlays `raw_config_source` and removes `thumbnail_path`;
  - it calls `ThumbnailAwareSerializer::new` (flavor taken from the map's `gcode_flavor`) → `serialize_config_block` (`crates/slicer-gcode/src/serialize.rs`).
- `serialize_config_block` synthesizes `filament_diameter`, `filament_colour`, `extruder_colour`, and `printer_model` only when the map lacks them. It writes `SUPPORT_CONFIG_DEFAULTS` and the map keys in sorted order, then pads from `ORCA_CONFIG_PADDING` while fewer than 96 keys have been emitted.
- The public `run_pipeline_with_raw_config` / `run_pipeline_with_instrumentation` share `run_pipeline_core`; their test callers are listed in the Step 1 inventory.
- `run_slice_with_collector` also reads `gcode_flavor` from its authored `config_source`.
- `raw_config_source` has consumers other than CONFIG_BLOCK: the `thumbnail_path` and `thumbnails` reads in `run_postpass_with_thumbnail`, and prepass `resolve_line_width_mm` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`), which reads `line_width` / `outer_wall_line_width`.

**Host-consumed keys no declaration covers**

These keys are read by `serialize.rs` / `pipeline.rs` or required by `docs/02_ir_schemas.md` §"CONFIG_BLOCK viewer-key contract":

- `gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`;
- `filament_cost`, `printable_area`;
- `thumbnails`;
- `machine_max_acceleration_retracting` (listed in the docs/02 viewer-key contract).

`support_type` and `support_family` are read from the global `ScopeDelta` by `load_live_modules_for_plan_manifest_first`. `extruder` is read from region `extensions` by `crates/slicer-wasm-host/src/dispatch.rs`. Model scopes that set it skip ingestion (`merge_model_scopes`), but a flat-config `extruder` would be dropped by the flip.

- None of these has a host row, a runtime row, or a manifest declaration.
- `support_type` is also a `plain` `ResolvedConfig` field.
- Today they survive only because ingestion is warn-and-keep and the raw overlay re-injects them.
- `HostRuntimeKey.default` is `&'static str` today, and `runtime_declaration` (`crates/slicer-config/src/lib.rs`) always emits `default: Some(row.default.to_owned())`. So a runtime row cannot currently be registered without a default.
- `build_host_key_entries` (`crates/slicer-scheduler/src/manifest.rs`) renders every `HOST_RUNTIME_KEYS` row into the `module config-schema` `host` array.
- `crates/slicer-scheduler/tests/contract/typed_selector_claim_selection_tdd.rs` asserts that an undeclared `support_family` is retained with an `UnrecognizedKey` warning.

**Guest reads**

`slicer_ir::ConfigView` (`crates/slicer-ir/src/slice_ir.rs`); every accessor returns `Option`. Guest errors are `slicer_sdk::error::ModuleError` (`crates/slicer-sdk/src/error.rs`).

**Neighboring tests**

- Twelve files call `bind_module_config_view` / `build_live_execution_plan`, listed in implementation-plan Steps 4c–4c‴, plus the `#[cfg(test)]` module in `execution_plan.rs`.
- CONFIG_BLOCK-asserting tests:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`;
  - `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` (`wedge_per_region_config_delivery_structural_canary` pins exactly 95 keys and the padding-only `wall_loops`);
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` (AC-Neg-3 `start_block_not_inside_other_blocks`, plus exactly-once pins);
  - `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs`.
- `crates/slicer-config/tests/registry_census_tdd.rs` pins the runtime channel at exactly 3 keys and provides `assembled_real_registry`.

**OrcaSlicer comparison**

- `GCode::append_full_config` (`GCode.cpp`) dumps the full print config as `; key = opt_serialize(key)`.
- `ConfigOptionString::serialize` (`Config.hpp`) calls `escape_string_cstyle` (`Config.cpp`), which escapes `\n`, `\r`, `\\`, and `"`.
- `ConfigBase::load_from_gcode_file` (`Config.cpp`) rejects a footer with fewer than 80 pairs. That is the reason for the 96-key padding cap.

## Architecture Constraints

- `slicer-config` remains dependent only on `slicer-ir`. Scheduler, runtime, and wasm-host orchestrate; they do not become config-semantic authorities.
- `ConfigView` means effective resolved values everywhere. No binding API accepts a raw or expanded source map.
- A declared key with a registry default or an authored value is always present with its effective value, and an undeclared key is always absent. `None` from a getter means "not declared, or declared without default and never authored".
- Registry defaults enter below every authored scope and before Phase-B expansion. An explicitly authored value equal to the default still overrides a lower non-default scope, because presence lives in `ScopeDelta`, not in value comparison.
- **Seed set.** A registry key is seeded into `extensions` only if all four hold:
  - its key is exact, not a `prefix:*` wildcard;
  - it has a registry default;
  - it is not `selector` (today only `wall_generator`, whose value travels as a selector value, not a delta);
  - it is not a `declare_resolved_config!` field (`cli`, `cli_opt`, or `plain` row).

  Do not test "no typed field" with `apply_cli_key`, because `plain` rows also return `Ok(false)`. Test membership in `ResolvedConfig::typed_field_keys()` instead. Typed fields already carry their defaults through `ResolvedConfig::default()`. Seeding them into `extensions` would shadow the typed value, because `to_config_map` merges `extensions` last.
- **Host-consumed registration.** Before the drop flip, every key the host reads from a `ScopeDelta`, from `extensions`, from the raw source, or from the CONFIG_BLOCK map is registered as a `HOST_RUNTIME_KEYS` row, as is every key the docs/02 viewer-key contract lists.
  - A key whose absence the serializer handles by synthesis (`filament_colour`, `extruder_colour`, `printer_model`, and `gcode_flavor` via the flavor fallback) is registered with **no** default. It is therefore not seeded, stays absent unless authored, and synthesis still fires.
  - `extruder` is also registered with no default, because a seeded `extruder` would change per-region tool assignment.
  - This requires `HostRuntimeKey.default: Option<&'static str>`. `runtime_declaration` then passes it through as-is, and `build_host_key_entries` renders `None` as JSON `null`.
- **CONFIG_BLOCK population.** A key of `to_config_map()` is emitted if either:
  - it is a registry entry without `omit_from_config_block`; or
  - it is a `typed_field_keys()` key with no registry entry (today `infill_type`).

  Retained unknown `extensions` keys are never emitted, even before the flip.
- **`config_block` metadata:**
  - It is polarity-safe. The Rust field is `omit_from_config_block: bool` (default `false`, meaning emitted) on `HostKeyMeta`, `ConfigFieldEntry`, and `RegistryEntry`, because all three default their `bool`s to `false` (`HostKeyMeta::NONE`, `#[derive(Default)]`). The manifest spelling stays `config_block = false`.
  - Reconciliation follows ADR-0067's deny-union shape for `denied_scopes`: a key is omitted if any host row or declaring module sets `config_block = false`. ADR-0067 does not constrain this field, so this is not an amendment.
- Canonical parity outranks byte stability: string values are escaped per `escape_string_cstyle`.
- No WIT or public IR schema version changes. The `CONFIG_BLOCK` textual change is intentional.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. Guests depend on `slicer-ir` and `slicer-sdk`, so every guest goes stale in Steps 2a and 2b (`crates/slicer-ir/src/{resolved_config.rs,config_schema.rs}`) and 4b (`crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/error.rs`), as well as in every Step 5 guest batch.

## Code Change Surface

**Selected approach:** seed registry defaults at resolution, register host-consumed keys, generate the host map, and project emission through reconciled metadata.

- Resolution produces complete configs, so load-time binding, region dispatch, and CONFIG_BLOCK all become complete through the existing `to_config_map` path, with no registry plumbing at dispatch sites.
- **Step order:**
  1. metadata, registration, and seeding;
  2. emission;
  3. binding;
  4. guest cleanup;
  5. no-drop, then the flip.

**Net-new public surface**

- `HostKeyMeta.omit_from_config_block: bool` (`false` in `HostKeyMeta::NONE`).
- `ConfigFieldEntry.omit_from_config_block: bool` — `#[serde(skip)]`, parsed from manifest `config_block`.
- `RegistryEntry.omit_from_config_block: bool` — reconciled any-true-wins.
- `ResolvedConfig::typed_field_keys() -> &'static [&'static str]` — macro-generated over every `cli` / `cli_opt` / `plain` row.
- `ConfigSchemaRegistry::config_block_map(&self, resolved: &ResolvedConfig) -> BTreeMap<String, ConfigValue>` — implements the CONFIG_BLOCK population rule above.
- `HostRuntimeKey.default: Option<&'static str>`, changed from `&'static str`. Existing rows become `Some(..)`.
- New `HOST_RUNTIME_KEYS` rows for the host-consumed keys (Step 1 fixes types and defaults).
- `ConfigView::require_bool / require_int / require_float / require_string (key) -> Result<_, ConfigReadError>` and `require_abs_value(key, base) -> Result<f64, ConfigReadError>`:
  - `ConfigReadError { key: String, expected: &'static str }` lives in `crates/slicer-ir/src/slice_ir.rs`;
  - `impl From<ConfigReadError> for ModuleError` (fatal) lives in `crates/slicer-sdk/src/error.rs`.
- `SliceOutcome.ingestion_warnings: Vec<IngestionWarning>` (`crates/slicer-runtime/src/run.rs`).

**Changed signatures and semantics**

- `bind_module_config_view(module: &LoadedModule, resolved: &ResolvedConfig) -> Arc<ConfigView>`. Support-family injection reads the registered `support_type` / `support_family` from `resolved.to_config_map()`.
- `build_live_execution_plan` replaces `config_source` with `resolved: &ResolvedConfig`.
- CONFIG_BLOCK projection is an additional input:
  - The crate-private path (`run_pipeline_fork` → the `_authority` fns → `run_pipeline_core` → `run_postpass_with_thumbnail`) gains `config_block: Option<&BTreeMap<String, ConfigValue>>`.
  - `run_slice_with_collector` passes `Some(&registry.config_block_map(&global_resolved))`. `run_postpass_with_thumbnail` converts that `BTreeMap` into the `HashMap<String, ConfigValue>` that `ThumbnailAwareSerializer::new` takes; the serializer sorts keys itself. Otherwise the map passes through unchanged: no `to_config_map` base, no overlay, no `thumbnail_path` removal.
  - `raw_config_source` keeps its non-CONFIG_BLOCK roles unchanged: the `thumbnail_path` and `thumbnails` reads, and prepass `resolve_line_width_mm`.
  - The public `run_pipeline_with_raw_config` / `run_pipeline_with_instrumentation` keep their signatures and pass `None`. They are test and diagnostic entry points, and they keep today's composition: generated `to_config_map` plus raw overlay, minus `thumbnail_path`. Their test callers therefore need no migration.
  - AC-11 proves the production path through `run_slice`, so the public path's divergence cannot hide a production regression.

**Changed behavior**

- `to_config_map` is generated from every `cli` / `cli_opt` / `plain` row, and `extensions` are merged last. The rendering contract:
  - It matches the old hand map exactly for every key that map emitted: `support_type` via `as_canonical_str()` (its Debug form once broke family selection), `infill_type` in its current rendering, `filament_density` omitted when empty, and `cli_opt` rows only when `Some`.
  - Newly emitted keys render by field type: `f32`/`f64` → `Float`; integer types → `Int`; `bool` → `Bool`; `String` → `String`; enums → their canonical wire string; `Vec` → the list form the row's extractor parses.
  - AC-13 pins parity against the retained hand map.
- `apply_delta` and seeding validate `extensions` against registry type and bounds.
- `serialize_config_block`:
  - escapes `ConfigValue::String` values;
  - renders `filament_diameter` once per filament from the map's effective value, replacing the hard-coded `1.75`;
  - keeps the other synthesized keys "only when absent".

**Rejected alternatives**

| Alternative | Why rejected |
| --- | --- |
| Projection at each consumer (the earlier draft) | Needs registry plumbing into `dispatch.rs` and materializes percent defaults after Phase-B expansion. |
| Keeping the raw overlay for CONFIG_BLOCK | Re-leaks unknown keys and bypasses the registry. |
| Replacing `raw_config_source` with the CONFIG_BLOCK map on every path | Breaks the `thumbnail_path` / `thumbnails` / prepass `resolve_line_width_mm` consumers, would leak `thumbnail_path` into the block through public-path test inputs, and forces the public entry points' test callers to migrate. |
| Leaving host-consumed keys unregistered | The drop flip would silently delete `gcode_flavor` and support selection. |
| Filling defaults in guests | Preserves the split semantics. |
| A `config_block: bool` field defaulting to `true` | Conflicts with `Default` derives. |
| A hand key roster | Repeats RC-4. |
| Deleting every textual `unwrap_or` | Corrupts unrelated algorithms and tests. |
| Dropping unknowns before the oracle | Violates the approved staging. |
| Excluding multi-line strings instead of escaping them | Diverges from canonical. |

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-ir/src/config_schema.rs`, `crates/slicer-ir/src/slice_ir.rs` — metadata, runtime-key registration, generated map, `typed_field_keys`, required accessors.
- `crates/slicer-config/src/lib.rs`, `crates/slicer-config/src/resolution.rs`, `crates/slicer-config/src/ingestion.rs` — reconciliation, `config_block_map`, seeding/validation, warn-to-drop.
- `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-scheduler/src/execution_plan.rs` — manifest parse; binding and its in-file tests.
- `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs` — resolved binding, CONFIG_BLOCK plumbing, `SliceOutcome.ingestion_warnings`.
- `crates/slicer-gcode/src/serialize.rs` — string escaping and `filament_diameter` rendering only.
- `crates/slicer-sdk/src/error.rs` — `From<ConfigReadError>`.
- New test files, each with a one-line `mod` in its aggregator:
  - `crates/slicer-config/tests/resolved_config_view_tdd.rs`;
  - `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs`, registered in `crates/slicer-runtime/tests/contract/main.rs`;
  - `crates/slicer-runtime/tests/e2e/resolved_config_view_no_drop_tdd.rs`, registered in `crates/slicer-runtime/tests/e2e/main.rs`.
- Existing tests named in the implementation-plan steps:
  - the 12 binding callers;
  - the four CONFIG_BLOCK tests;
  - `crates/slicer-config/tests/registry_census_tdd.rs`, `crates/slicer-config/tests/registry_assembly_tdd.rs`, and `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs`;
  - `crates/slicer-scheduler/tests/contract/typed_selector_claim_selection_tdd.rs`;
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` and `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, which assert unknown keys are retained in `extensions`;
  - guest test files inventoried by Step 1 item 11, which build `ConfigView::new()` or partial views reaching classified sites. Ten `modules/core-modules/*/tests/*.rs` files use `ConfigView::new()` today;
  - Step-1-inventoried `ResolvedConfig` whole-value assertions.
- The 13 guest production source trees named by AC-3, plus six manifests:
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/infill-linker/infill-linker.toml`
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/wave-overhangs/wave-overhangs.toml`
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`.

## Read-Only Context

- `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{packet.spec.md,requirements.md,design.md}` — final export reconciliation only.
- `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,design.md}` — warning/retention contract only.
- `crates/slicer-wasm-host/src/dispatch.rs` — confirm the region-view path consumes seeded configs; do not edit.
- `resources/cube_4color.3mf` and `OrcaSlicerDocumented/**` — delegate inspection; never load directly.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–05 and 07+, and `docs/07_implementation_status.md` except the delegated task-row update at completion.
- `crates/slicer-schema/wit/**`, schema-version constants, `CONFIG_SCHEMA_WIRE_VERSION`, `build_config_schema_json`, `ORCA_CONFIG_PADDING` contents and cap, layer-range/modifier/eligibility code, and aliases.
- `target/`, `Cargo.lock`, generated bindings, guest WASM binaries, vendored dependencies, and large fixture contents.

## Expected Sub-Agent Dispatches

Unless noted, each returns `LOCATIONS` ≤20.

- Reconcile packet 05's landed resolver signatures, `ScopedConfig` layering, and every `resolve_scope_stack` / `bind_module_config_view` / `build_live_execution_plan` / `run_pipeline_with_*` / `SliceOutcome {` site.
- Find tests asserting whole-value `ResolvedConfig` equality or empty `extensions` that seeding would change.
- For the 89 sites, give key, getter, registry `field_type`, and whether the reading guest declares the key. Also list every other undeclared string-literal getter or wrapper read, in per-guest batches.
- Find `HostKeyMeta`, `ConfigFieldEntry`, and `RegistryEntry` struct literals without a `..` rest (≤20 per type).
- Inventory host production reads of keys that have no registry entry, from any `ScopeDelta` values, `extensions`, raw or expanded source map, or CONFIG_BLOCK map. For each, record current absent-value behavior (synthesized, defaulted, or ignored) to fix its runtime-row default.
- Run each cargo command. Return `FACT` ≤5 lines, or failure `SNIPPETS` ≤20 lines.

## Data and Contract Notes

- **IR/manifest contracts:**
  - `extensions` keeps its storage and hash semantics, and now carries seeded defaults for the seed set.
  - Memory and interning impact is unmeasured; record the measured distinct-config count in the acceptance ceremony.
  - Manifest `config_block` is optional and defaults to true.
- **WIT boundary:** unchanged. The required accessors are guest-side Rust only.
- **Determinism/scheduler constraints:** seeding iterates `ConfigSchemaRegistry` (`BTreeMap`) order, `config_block_map` returns a `BTreeMap`, and hash-map iteration never defines output order.

## Locked Assumptions and Invariants

- FORWARD-DEP on packet 05 (draft, in progress). The names and shapes in `packet.spec.md` §Prerequisites match both packet 05's `design.md` and the working tree as of this preflight. Step 1 re-verifies them against the landed tree.
- Packet 05's layer-planning seam removes guest `object_height:<id>` / `layer_height:<id>` reads. If `overlay_object_layer_planning` or a guest wildcard-instance read survives packet 05, binding over the resolved config would drop those instance keys, and Step 1 must stop.
- The fallback census is exactly 89 across the 13 named guests under AC-3's chain-aware classification. It was verified against HEAD `a85421af` during preflight. A strict "getter immediately followed by `.unwrap_or`" count is lower; do not use it.
- Eleven guest:key reads were verified undeclared at preflight. Today they always return `None`, so the guest's literal or helper default runs:
  - overhang-classifier-default: `outer_wall_line_width`, `line_width`;
  - tree-support and traditional-support: `nozzle_diameter`, `layer_height`, `support_line_width` (via `get_abs_value`);
  - infill-linker: `infill_density`;
  - rectilinear-infill: `infill_shift_step`, which has no registry entry anywhere, so declaring it creates the entry with the guest's current effective fallback as its default;
  - wave-overhangs: `thick_bridges` (via `cfg_bool`, typed as the existing declarers declare it).

  Guest tests that pass an empty or partial `ConfigView` into a classified code path fail once fallbacks become `require_*` reads. They are migrated to `ConfigView::from_map` views holding the keys their path reads, at the guest's manifest-default values. No new test-support surface is added.

  Declaring them moves each read to its effective or registry-default value. The acceptance ceremony records any value delta against the old literal or helper default.
- Keys typed `float_or_percent` but read with `get_float` must be read with `require_abs_value`, using the base the registry `base_key` names, unless Phase-B expansion guarantees a `Float`. This covers arachne-perimeters `outer_wall_line_width`, `inner_wall_line_width`, `overhang_reverse_threshold`, and overhang-classifier-default `overhang_1_4_speed` … `overhang_4_4_speed`.
- Exactly four keys carry `config_block = false`: the three `mmu_segmented_region_*` keys and `thumbnail_path`. The plan says "three", meaning only the `to_config_map` omissions. `thumbnail_path` preserves today's removal of a fork-only runtime key, which would otherwise leak a local path.
- `wall_generator` is a registry runtime `selector` row. `infill_type` is the only typed field without a registry entry after this packet, and it keeps emitting through `typed_field_keys()`.

## Risks and Tradeoffs

- Seeding changes `resolve_scope_stack` output and may break whole-value equality assertions in packet 04/05 tests. Step 1 inventories them, and they are updated to independently derived expectations.
- A full-registry `CONFIG_BLOCK` exceeds 96 keys, so the existing cap suppresses nearly all `ORCA_CONFIG_PADDING`, including padding-only keys like `wall_loops`. The canary's `95` and `wall_loops` expectations are re-derived from the registry, and the ≥80-key Orca loader gate still holds.
- Registering host-consumed keys grows the runtime channel and the `module config-schema` `host` array. Several tests change with it:
  - `registry_census_tdd`'s runtime-row pins;
  - `registry_assembly_tdd`'s `runtime_key` helper;
  - the retained-`support_family` assertion in `typed_selector_claim_selection_tdd`.

  All three are updated in the same substep. The wire version stays the same, because rows are data and no field is added.
- Public pipeline entry points keep the legacy CONFIG_BLOCK composition, so two compositions coexist. Tests that call them exercise the serializer (escaping, flavor, synthesis) but not registry projection. Registry projection is proven only by AC-4 (unit) and AC-11 (production `run_slice`).
- Materializing defaults can expose bad or conflicting manifest defaults formerly hidden by guest literals. Registry assembly, typing, and bounds failures must stay loud.
- Metadata additions have a struct-literal blast radius; Step 1 inventories it and Steps 2a–2c′ edit it atomically.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (seeding/registration; emission; binding migration; guest batches; no-drop e2e)
- Highest-risk dispatches: fallback classification with key/type/declaration columns, and the host-consumed key inventory. Both return `LOCATIONS` in bounded batches.

## Open Questions

- `[FWD]` Packet 05 landing: re-reconcile exports and `ScopedConfig` layering, and check whether `overlay_object_layer_planning` survives.
- `[BLOCK]` None beyond forward dependencies. CONFIG_BLOCK population is owner-decided as the full registry (2026-09-18).
