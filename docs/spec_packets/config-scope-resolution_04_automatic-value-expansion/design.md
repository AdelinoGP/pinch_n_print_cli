# Design: automatic-value-expansion

## Controlling Code Paths

- Primary code path: net-new `ExpansionContext`, `ExpansionError`, and `expand_automatic_values` in `crates/slicer-config/src/lib.rs`; `run_slice_with_collector` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`) expand their already-merged global/object maps before `build_live_execution_plan` binds a module `ConfigView`, and the run-level tool map before emitter use. The production pipeline path (`crates/slicer-runtime/src/pipeline.rs`) carries the same registry/context authority into the configured prepass family. Its shared core, `execute_prepass_with_builtins_configured_instr_collecting` (`crates/slicer-runtime/src/prepass.rs`), currently rebuilds tool configs with `resolve_per_tool_configs` and paint-semantic configs through `build_paint_semantic_configs`; those maps must be expanded after their resolvers and before `commit_region_mapping_builtin` can cause any `RegionMapIR::intern_config` call.
- Region-map handoff: `commit_region_mapping_builtin` (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`) receives the expanded default/object/paint/tool map set. `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`) remains read-only in this packet; its three `RegionMapIR::intern_config` branches are the downstream boundary the runtime regression must cover.
- Width dispatch: `resolve_role_width` and `RoleWidthContext` (`crates/slicer-core/src/flow.rs`), with the support-only duplicate `resolve_support_line_width_mm` currently in `crates/slicer-ir/src/resolved_config.rs` and host consumers in `crates/slicer-runtime/src/run.rs` and `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`.
- Sentinel consumers: support planners' bottom-layer mirrors and support renderers' bottom-spacing mirrors in the four `modules/core-modules/{traditional-support-planner,tree-support-planner,traditional-support,tree-support}/src/lib.rs` files.
- Speed declarations/consumer: the four `overhang_*_speed` sections in `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`, their host `SPEED_META` declarations and `FeedrateConfig::from_raw_config` consumption (`crates/slicer-ir/src/feedrate.rs`), and the expanded global overlay supplied by `crates/slicer-runtime/src/run.rs`.
- Generated reference: `xtask/src/gen_config_docs.rs::KeyRow`, `module_rows`, and `render_table` must carry manifest `base_key` into `docs/15_config_keys_reference.md`; the current renderer has no base-key field or column.
- Neighboring tests/fixtures: `crates/slicer-config/tests/{automatic_value_expansion_tdd.rs,registry_assembly_tdd.rs}`; `crates/slicer-core/tests/flow_tdd.rs`; net-new `crates/slicer-runtime/tests/integration/automatic_value_expansion_tdd.rs` registered in the existing `crates/slicer-runtime/tests/integration/main.rs` aggregator and containing a painted material/tool chain plus a non-support modifier child, with placeholders at global, object, paint-semantic, and tool scopes; inline `gen_config_docs` rendering tests; direct support guest tests; net-new `crates/pnp-cli/tests/fixtures/automatic_value_expansion/{visual-debug.json,config.json}` plus existing `resources/regression_wedge.stl`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Expansion is evaluation, not registry assembly: packet 1's registry remains declaration/provenance metadata, while this packet runs only after applicable scope values merge (ADR-0068 amendment).
- At this packet's current runtime seam, “applicable scope values” means each fully resolved map that exists before packet 5: global/default, per-object, per-paint-semantic, and per-tool. Packet 5 must reuse `expand_automatic_values` after its layer-range/modifier scope-stack merges; this packet does not pre-expand a delta and pretend it is equivalent to expanding a merged config.
- `slicer-config` continues to depend on `slicer-ir` only. It must not import runtime, scheduler, module-loader, WIT, geometry, or emitter types.
- `expand_automatic_values` is atomic. It derives replacements into scratch state, validates every recursive base and sentinel dependency, and mutates the supplied `ResolvedConfig` only after the whole expansion succeeds.
- Base resolution is deterministic and cycle-free because packet 1 validates the registry graph. For the selected tool, `ExpansionContext.tool_bases[tool_index][base_key]` wins when present; otherwise the already-merged config value wins; `nozzle_diameter` may fall back to `ExpansionContext.nozzle_diameter_mm`. Missing/non-positive required bases are errors, never fallback guesses.
- Only values with `RegistryEntry.base_key: Some(_)` receive generic percent expansion. Unitless percentages without a typed absolute base remain ratio values; this packet does not reinterpret every `percent` declaration.
- The four overhang-speed percent forms are Phase B: each registry entry has typed base `outer_wall_speed`, and expansion emits absolute mm/s before `FeedrateConfig::from_raw_config` or a module `ConfigView` reads it. Canonical `PrintConfigDef::init_fff_params` supplies this `ratio_over`; the active extrusion role is not the percent base.
- The only Phase-B zero-width rules are `line_width -> 1.125 * nozzle_diameter` and `support_line_width -> nozzle_diameter`. Zero role-specific width overrides remain a Phase-C dispatch signal to `resolve_role_width`, whose final fallback is the now-expanded `line_width`. Numeric overhang-speed zero remains the existing no-override value and is not converted to `outer_wall_speed` by generic percent expansion.
- The only Phase-B negative mirror rules are `support_interface_bottom_layers -> support_interface_top_layers` and `support_bottom_interface_spacing -> support_interface_spacing`. Packet 10 owns all context-dependent negative sentinels.
- No mm↔internal-unit boundary is introduced or changed: all expansion inputs/outputs are config-space millimetres or mm/s. The coordinate-system snippet is therefore intentionally absent.
- Compatibility checklist: no serialized IR field/type/variant changes; no WIT edit/package bump; no CLI JSON field/semantic change; no manifest field-vocabulary change; no schema/version constant bump. Existing manifest `type` and `base_key` values are declaration content, not a manifest schema-shape change.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: expose one small expansion module from `slicer-config`, operating on the existing `ResolvedConfig` plus its `extensions` map. It obtains generic values through the existing `ResolvedConfig::to_config_map`/`apply_cli_key` surfaces, stages absolute replacements, and writes host fields through `apply_cli_key` while retaining undeclared/module keys in `extensions`. Packet 6 may replace the current projection with its registry-driven complete map without changing this packet's exported expansion API.
- Exact new exports:
  - `slicer_config::ExpansionContext` — `pub struct ExpansionContext { pub nozzle_diameter_mm: f64, pub tool_bases: BTreeMap<u32, BTreeMap<String, f64>> }` with `Default` producing no tool bases and no fabricated positive nozzle.
  - `slicer_config::ExpansionError` — public error enum with at least `UnknownBaseKey { key: String, base_key: String }`, `MissingAutoBase { key: String, base_key: String }`, and `NonPositiveBase { key: String, base_key: String, value: f64 }`; display text names both the dependent and base key.
  - `slicer_config::expand_automatic_values` — `pub fn expand_automatic_values(registry: &ConfigSchemaRegistry, config: &mut slicer_ir::ResolvedConfig, context: &ExpansionContext, tool_index: Option<u32>) -> Result<(), ExpansionError>`.
- Exact value rules:
  - `ConfigValue::Percent(p)` and `ConfigValue::FloatOrPercent { value: p, is_percent: true }` with typed base `b` become absolute `p / 100.0 * b`; plain `Float`, `Int`, and non-percent `FloatOrPercent` remain absolute.
  - With `outer_wall_speed = 60.0`, overhang values `25%`, `50%`, `75%`, and `100%` become literal absolutes `15.0`, `30.0`, `45.0`, and `60.0`; numeric `0.0` stays `0.0` for role-context no-override handling.
  - The `line_width` zero rule computes `1.125 * nozzle_diameter`; at `0.4` the exact expected result is `0.45`.
  - The `support_line_width` zero rule computes `1.0 * nozzle_diameter`; at `0.4` the result is `0.4`.
  - The two bottom-side `-1` rules copy the already-expanded matching top value; explicit bottom zero is not an auto sentinel.
  - Generic base chains resolve base-before-dependent. A successful call leaves no covered `Percent`, percent-marked `FloatOrPercent`, zero width auto, or negative mirror sentinel in the target.
- Registry metadata changes: add `base_key = "nozzle_diameter"` to the existing `support_line_width` declaration in `modules/core-modules/tree-support-planner/tree-support-planner.toml`; add the same width-family `base_key = "nozzle_diameter"` to every other `float_or_percent` `*line_width` declaration that lacks one (`outer_wall_line_width` in `arachne-perimeters`/`classic-perimeters`, sparse/internal/top widths in `gyroid-infill`/`lightning-infill`/`rectilinear-infill`); widen `line_width` manifest minimums from `0.1` to `0` so the host-owned `0` auto placeholder validates before Phase-B expansion; retype the four overhang-speed manifest entries to `float_or_percent` with `base_key = "outer_wall_speed"`; and set the same four host speed rows' `HostKeyMeta.wire_type` to `float_or_percent` through `SPEED_META`. Registry assembly must reconcile both declaration channels and retain the module-supplied typed base.
- Runtime placement: assemble once per run with `assemble_registry(&module_declarations, &HostChannels::from_live())`; destructure `AssemblyOutcome { registry, warnings }`; preserve/forward packet-1 warnings through the existing loaded-module diagnostic channel. In `crates/slicer-runtime/src/run.rs`, expand the default and every object config only after their current resolvers merge scope, and expand the separately resolved run-level tool map before emitter/statistics consumers. Build the global module-binding/feedrate source by overlaying the expanded default map over the existing raw source, preserving unrelated runtime keys until packet 6 replaces raw binding completely; both `build_live_execution_plan` and `FeedrateConfig::from_raw_config` consume that expanded source, and no declared module `ConfigView` contains a covered placeholder.
- Prepass placement: do not assume the run-level tool map reaches RegionMapping. `execute_prepass_with_builtins_configured` and its collecting/instrumented forwarders in `crates/slicer-runtime/src/prepass.rs` currently receive only raw/default/object config plus bounds; the shared collecting core independently rebuilds tool and paint-semantic maps. Carry the registry/context authority through `crates/slicer-runtime/src/pipeline.rs` on the full-slice path and directly from `prepare_prepass_context`, expand each successful `resolve_per_tool_configs` result with `Some(tool_index)`, expand each successful `build_paint_semantic_configs` result with no selected tool, and pass only expanded default/object/tool/paint maps to `commit_region_mapping_builtin`. Expansion errors become `PrepassExecutionError` with dependent/base key text; they must not be swallowed by the existing paint/tool resolver fallbacks.
- Compatibility prepass wrappers may retain their old behavior only for empty raw input plus default/already-expanded maps. Any production path that can carry `object_config:*`, `tool_config:*`, or `paint_config:*` values must use the expansion-aware configured core; adding a parallel production path that silently leaves the existing configured path placeholder-capable is rejected.
- Generated-reference change: add `base_key: Option<String>` to `xtask/src/gen_config_docs.rs::KeyRow`; populate it from each module schema table and as `None` for host-only rows; render a `Base key` column using backticked key text or `—`; update the inline `KeyRow` literals and add `render_table_preserves_base_key`, whose populated-base assertion fails if parsing or rendering drops the field. Regenerate `docs/15_config_keys_reference.md` rather than hand-editing generated rows.
- Width cleanup: `resolve_role_width` keeps bridge > first layer > role > `line_width`, removes only the nozzle calculation, and consumes already-expanded values. `RoleWidthContext.nozzle_diameter` may be removed once the call-site census confirms it has no remaining semantic use; if removed, all struct literals are updated in the same step. `resolve_support_line_width_mm` is deleted after both host consumers read the expanded absolute support width.
- Sentinel cleanup: production support guests stop testing negative bottom values; direct module tests provide the expanded top-derived value because bypassing the host expansion boundary is an explicit test-fixture responsibility.
- Rejected alternatives and reasons:
  - Expanding in `assemble_registry`: rejected by ADR-0068; assembly lacks merged scope/tool values.
  - Keeping Phase-B typed-base resolution in `ConfigView::get_abs_value` or `feedrate.rs::read_speed` call sites: rejected because config-only callers can choose inconsistent bases and the latter currently drops percentages. The approved plan and canonical declaration assign all four overhang percentages to Phase B over `outer_wall_speed`.
  - Moving volumetric speed auto here: rejected because width/height and per-move volume exist only in the emitter; packet 10 owns it.
  - Adding fields to `ResolvedConfig` or WIT: rejected; `extensions` and existing scalar fields already carry the values, and a public contract bump is unnecessary.

## Files in Scope (read + edit)

- `crates/slicer-config/src/lib.rs` and `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` — expansion API, deterministic rules/errors, and independent oracle tests.
- `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, `crates/slicer-runtime/tests/integration/automatic_value_expansion_tdd.rs`, and `crates/slicer-runtime/tests/integration/main.rs` — once-per-run registry assembly, post-merge global/object and emitter-tool expansion, expansion-authority threading, post-resolution expansion of prepass-rebuilt tool/paint maps, expanded binding source, registered integration coverage, and support-width consumption.
- `crates/slicer-core/src/flow.rs` and `crates/slicer-core/tests/flow_tdd.rs` — reduced role dispatch and preserved precedence tests.
- `crates/slicer-ir/src/resolved_config.rs` — remove the duplicate support resolver; no IR field or `ConfigValue` variant change.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` — add the typed support-width base to the declaration that exists today.
- `modules/core-modules/{arachne-perimeters,classic-perimeters,gyroid-infill,lightning-infill,rectilinear-infill,infill-linker,skirt-brim,support-surface-ironing,traditional-support,tree-support,wipe-tower}/*.toml` — width-family `base_key = "nozzle_diameter"` additions and `line_width` minimum widenings to admit the host-owned `0` auto placeholder.
- `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`, `crates/slicer-ir/src/feedrate.rs`, and `crates/slicer-config/tests/registry_assembly_tdd.rs` — retype all four overhang-speed declarations, supply matching host wire types, retain `outer_wall_speed` as registry `base_key`, and prove real assembly.
- `modules/core-modules/{traditional-support-planner,tree-support-planner,traditional-support,tree-support}/src/lib.rs` and their directly affected tests — delete duplicate negative mirrors and update bypass fixtures.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `crates/slicer-core/src/algos/lightning/mod.rs`, and `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — `RoleWidthContext` literal fallout only if the obsolete nozzle field is removed.
- `crates/pnp-cli/tests/fixtures/automatic_value_expansion/{visual-debug.json,config.json}` — deterministic visual-debug request/config.
- `xtask/src/gen_config_docs.rs`, `docs/02_ir_schemas.md`, and generated `docs/15_config_keys_reference.md` — `KeyRow`/`render_table` base-key support, placement/invariant documentation, and regenerated typed-base rows.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md` — approved plan and queue ownership.
- `docs/19_visual_debug.md` — lines around `Request Shape`, `Reading A Bundle`, and `Tap Classes And Execution Closure` only.
- `docs/02_ir_schemas.md` — `ResolvedConfig`, `RegionMapIR` interner, and `IR Versioning Contract` ranges only.
- `docs/11_operational_governance_and_acceptance_gate.md` — `Compatibility Policy` and `CLI output wire contracts` ranges only.
- `docs/adr/0067-unified-config-schema-registry.md` and `docs/adr/0068-config-scope-is-a-wire-encoding.md` — accepted decisions.
- `docs/spec_packets/config-scope-resolution_01_config-schema-registry/{packet.spec.md,requirements.md,design.md}` — packet-1 status and exact FORWARD-DEP exports; never edit.
- `crates/slicer-scheduler/src/config_resolution.rs` — named resolver signatures/return shapes only; packet 5 owns replacement, so this packet must not edit it.
- `crates/slicer-scheduler/src/execution_plan.rs` — `bind_module_config_view` signature/contract only; packet 6 owns replacement, so this packet must not edit it.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — `commit_region_mapping_builtin` inputs/handoff only; no edit is required when `prepass.rs` supplies the expanded maps.
- `crates/slicer-core/src/algos/region_mapping.rs` — `execute_region_mapping_inner` scope-fold and its `RegionMapIR::intern_config` branches only; read to make the runtime test cover every branch, but packet 5 owns resolver/precedence edits.
- `resources/regression_wedge.stl` — existing deterministic model fixture; never rewrite.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/emit.rs` and all emitter volumetric-auto code — packet 10; the four overhang-speed percentages are not deferred there.
- `crates/slicer-scheduler/src/config_resolution.rs` resolver replacement, `execute_region_mapping_inner` scope-precedence or interner edits in `crates/slicer-core/src/algos/region_mapping.rs`, and layer-planning WIT — packet 5.
- `crates/slicer-scheduler/src/execution_plan.rs` raw-binding retirement and complete registry-driven config flattening — packet 6.
- `crates/slicer-schema/wit/**` and every `CURRENT_*_SCHEMA_VERSION`/wire-version constant — no contract bump authorized.
- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- Other packet directories, `docs/07_implementation_status.md`, and the approved plan — read-only; backlog update is delegated only at completion.

## Expected Sub-Agent Dispatches

- Question: re-derive every `RegistryEntry.base_key` consumer and every percent-aware read whose base is config-only versus geometry-dependent, explicitly retaining all four overhang speeds over `outer_wall_speed`; scope: `crates/` and `modules/core-modules/*/{src/*.rs,*.toml}`; return: `LOCATIONS` ≤20; purpose: prevent approved Phase-B work from drifting to packet 10.
- Question: verify the exact packet-1 export names/shapes and status immediately before implementation; scope: `docs/spec_packets/config-scope-resolution_01_config-schema-registry/`; return: `FACT` ≤5 lines; purpose: clear FORWARD-DEP.
- Question: verify `Flow::new_from_config_width`/`auto_extrusion_width`, `PrintConfigDef::init_fff_params`'s four overhang float-or-percent declarations and `outer_wall_speed` ratio base, `GCode::_extrude`'s numeric-zero/volumetric behavior, and `number_of_support_interface_bottom_layers`; scope: the four Orca paths in `requirements.md`; return: `SUMMARY` ≤200 words; purpose: parity lock without loading upstream.
- Question: census all `RoleWidthContext` literals and support sentinel mirror branches before deletion; scope: `crates/` and the four support module trees; return: `LOCATIONS` ≤20; purpose: bounded blast radius.
- Question: re-derive the configured prepass call graph from `run_slice_with_collector`/`prepare_prepass_context` through `crates/slicer-runtime/src/pipeline.rs` into `execute_prepass_with_builtins_configured_instr_collecting`, including the internal tool/paint resolver calls and the `commit_region_mapping_builtin` handoff; scope: those named symbols only; return: `SNIPPETS` ≤4 snippets, 30 lines each; purpose: prevent a `run.rs`-only implementation that prepass silently bypasses.
- Question: run each cargo/check/clippy/guest/doc gate; scope: workspace commands listed in `requirements.md`; return: `FACT` pass/fail, and `SNIPPETS` ≤20 lines only on failure; purpose: keep command output out of controller context.

## Data and Contract Notes

- IR/manifest contracts: `ResolvedConfig`, `ConfigValue`, and `RegionMapIR` wire shapes are unchanged. Existing manifest fields `type` and `base_key` receive corrected values; the manifest schema vocabulary is unchanged.
- WIT boundary: no WIT edit. The behavioral invariant is that covered placeholders are gone before a `ConfigView` is constructed; typed instantiation shape is unchanged.
- Determinism/scheduler constraints: registry traversal plus object/tool/paint maps are ordered; expansion occurs after each applicable map's merge and before binding/interning, so equivalent expanded configs still deduplicate under `ResolvedConfig` bitwise equality/hash. Failure commits nothing and no partially expanded map reaches RegionMapping.
- Tool-base precedence: a matching selected-tool absolute base wins over the global merged base only for that tool's expansion. No selected tool means no tool-map lookup.
- Warnings: packet-1 registry warnings are preserved; expansion failures are fatal config-resolution errors with dependent/base names, not warnings followed by fallback.

## Locked Assumptions and Invariants

- `line_width = 0` resolves to `1.125 * nozzle_diameter`; `support_line_width = 0` resolves to exactly the nozzle diameter.
- Every percent-authored `overhang_1_4_speed` through `overhang_4_4_speed` resolves in Phase B against scope-resolved `outer_wall_speed`; numeric zero remains zero and is not moved to packet 10.
- Bridge override > first-layer override > role width > expanded base line width remains the role-width precedence.
- A bottom sentinel mirrors the corresponding top value only when the bottom value is exactly numeric `-1`; numeric zero is explicit.
- Generic percent expansion uses only `RegistryEntry.base_key`; no caller-selected ad hoc base remains for covered values.
- No Phase-B-covered placeholder reaches `bind_module_config_view`, `FeedrateConfig::from_raw_config`, `commit_region_mapping_builtin`, or any downstream `RegionMapIR::intern_config`; overhang-speed percentages are covered placeholders for this invariant.
- Both production entry points expand global/object values before plan binding and use the same expansion-aware configured prepass ordering. The prepass must not reconstruct raw tool/paint maps after the last expansion point.
- Packet 10 retains exclusive ownership of emitter volumetric auto and geometry-dependent sentinels.

## Risks and Tradeoffs

- `ResolvedConfig::to_config_map` is currently incomplete. This packet may use it only for the keys in the explicit Phase-B/base-key census and must fail on an unavailable base; packet 6 later replaces the projection registry-wide without changing the expansion API.
- The live host speed channel currently describes the four overhang rows as plain floats and `read_speed` drops percent variants. Retyping their `SPEED_META` wire declarations plus expanding before `FeedrateConfig::from_raw_config` is required; teaching `read_speed` to choose a base would recreate a second expansion owner.
- Direct guest tests bypass host expansion and previously relied on guest mirrors. Updating their fixtures to expanded values is required to avoid retaining a second production policy implementation.
- A `run.rs`-only implementation is a false fix: `execute_prepass_with_builtins_configured_instr_collecting` reconstructs tool and paint-semantic configs from `raw_config_source`, so covered scoped placeholders reappear immediately before RegionMapping unless `prepass.rs` expands those outputs.
- A non-positive nozzle previously yielded zero or a fallback. Fatal rejection is intentionally safer because successful zero-width geometry is invalid and NaN can poison interning.
- Guest mirror removal changes guest source and can expose stale artifacts; the mandatory freshness gate owns this risk.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (runtime/pipeline/prepass wiring across two production entry points)
- Highest-risk dispatch and required return format: config-only versus geometry-dependent percent/base census — `LOCATIONS` ≤20, one context line each.

## Open Questions

- [FWD] Active packet 1 remains an unsatisfied dependency until completion. Its current `RegistryEntry.base_key: Option<String>` shape is the expected export; this packet reads it and does not rename or duplicate it.
- [FWD] Active packet 1 currently defines `slicer_config::ConfigSchemaRegistry` with `entry(&self, key: &str) -> Option<&RegistryEntry>`; this packet consumes that accessor only after packet 1 completes.
- [FWD] Active packet 1 currently defines `slicer_config::ModuleDeclaration { pub module_id: slicer_ir::ModuleId, pub schema: slicer_ir::config_schema::ConfigSchema, pub claim_exclusive_group: Option<String> }`; runtime adapts loaded modules to this input only after packet 1 completes.
- [FWD] Active packet 1 currently defines concrete `slicer_config::HostChannels` with `from_live()` / `from_parts(...)` and `host_keys`, `speed_keys`, and `runtime_keys`; this packet uses `from_live()` and does not alter the current `speed_keys: Vec<slicer_ir::resolved_config::HostConfigKey>` shape.
- [FWD] Active packet 1 currently defines `slicer_config::AssemblyOutcome { registry: ConfigSchemaRegistry, warnings: Vec<RegistryWarning> }`; runtime preserves the warnings only after packet 1 completes.
- [FWD] Active packet 1 currently defines `slicer_config::assemble_registry(modules: &[ModuleDeclaration], host: &HostChannels) -> Result<AssemblyOutcome, RegistryLoadError>`; this packet assembles once per run only after packet 1 completes.
