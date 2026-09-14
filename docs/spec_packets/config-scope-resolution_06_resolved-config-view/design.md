# Design: resolved-config-view

## Controlling Code Paths

- Primary code path: packet 05's `resolve_scope_stack` produces the effective `ResolvedConfig`; `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) projects it for each module; `build_live_execution_plan` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) supplies the production views.
- Emission path: `ResolvedConfig::to_config_map` (`crates/slicer-ir/src/resolved_config.rs`) and `serialize_config_block` (`crates/slicer-gcode/src/serialize.rs`).
- Neighboring tests/fixtures: new `resolved_config_view_tdd` contract tests, `gcode_header_thumbnail_config_blocks_tdd`, new e2e module, and `resources/cube_4color.3mf`.
- OrcaSlicer comparison: none; this packet implements approved in-tree registry/delivery policy.

## Architecture Constraints

- `slicer-config` remains dependent only on `slicer-ir`; scheduler/runtime orchestrate module filtering and do not become config-semantic authorities.
- `ConfigView` means effective resolved values everywhere. Raw `config_source` must not be accepted by the final binding API.
- Absent declared values materialize from registry defaults; absent undeclared values remain absent.
- The 89-site census is fixed by classification: production config-read chain plus literal `unwrap_or`; exclude `unwrap_or_else`, sort comparators, non-config options, test code, and non-literal fallbacks.
- `config_block` defaults true and is reconciled deterministically across declarations; any false declaration excludes the key so shared keys cannot leak through another declarer.
- No WIT or public IR schema version changes. The `CONFIG_BLOCK` textual byte change is intentional.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: add registry projection APIs that materialize effective values/defaults, change module binding and G-code emission to consume those projections, then remove guest literals and finally switch unknown retention off behind the no-drop proof.
- Net-new public surface: `RegistryEntry.config_block: bool`; `ConfigSchemaRegistry::resolved_map(&ResolvedConfig) -> Result<BTreeMap<ConfigKey, ConfigValue>, ResolutionError>`; `ConfigSchemaRegistry::config_block_map(&ResolvedConfig) -> Result<BTreeMap<ConfigKey, ConfigValue>, ResolutionError>`.
- Binding API: retain the name `bind_module_config_view`, but change its inputs to the loaded module, effective `ResolvedConfig`, and `ConfigSchemaRegistry`; callers cannot pass a raw source map.
- Reconciliation: `config_block` is true only when every declaration permits emission; default true for all declaration channels.
- Rejected alternatives: filling defaults in guests preserves split semantics; retaining a hand roster repeats RC-4; deleting every textual `unwrap_or` would corrupt unrelated algorithms and tests; dropping unknowns before the oracle violates the approved staging gate.

## Files in Scope (read + edit)

- `crates/slicer-config/src/lib.rs`, `crates/slicer-ir/src/{config_schema.rs,resolved_config.rs}` — metadata and projection.
- `crates/slicer-scheduler/src/{execution_plan.rs,manifest.rs}` and `crates/slicer-wasm-host/src/execution_plan_live.rs` — resolved binding and manifest parse.
- `crates/slicer-gcode/src/serialize.rs` and runtime contract/integration/e2e test modules with their `main.rs` aggregators, including the `mod resolved_config_view_tdd;` registration in `crates/slicer-runtime/tests/contract/main.rs` — registry-driven emission and proofs.
- The 13 production guest source trees named by AC-3 — delete only classified config-literal fallbacks.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` — contract updates.

## Read-Only Context

- `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{packet.spec.md,requirements.md,design.md}` — final export reconciliation only.
- `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,requirements.md,design.md}` — ingestion warning/retention contract only.
- `resources/cube_4color.3mf` — delegate member/value inspection; never load directly.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–05 and 07+, and `docs/07_implementation_status.md` except delegated task-row update at completion.
- `crates/slicer-schema/wit/**`, schema-version constants, layer-range/modifier/eligibility code, and aliases.
- `OrcaSlicerDocumented/**`, `target/`, `Cargo.lock`, generated bindings, guest WASM binaries, vendored dependencies, and large fixture contents.

## Expected Sub-Agent Dispatches

- Question: reconcile packet 05's landed resolver signatures and all call sites feeding module binding; scope: packet 05 exports and named scheduler/runtime paths; return: `LOCATIONS` ≤20.
- Question: classify and return the 89 baseline config-literal fallback sites with the locked per-guest counts; scope: 13 named guest production trees; return: `LOCATIONS` in per-guest batches ≤20 each.
- Question: inventory struct literals for `ConfigFieldEntry`, `HostConfigKey`, and `RegistryEntry` before adding `config_block`; scope: crates/tests only; return: `LOCATIONS` ≤20 per type.
- Question: run each cargo command; scope: exact command; return: `FACT` ≤5 lines or failure `SNIPPETS` ≤20 lines.

## Data and Contract Notes

- IR/manifest contracts: `extensions` storage is unchanged; optional snake_case `config_block` is new manifest metadata with default true.
- WIT boundary: unchanged, consuming packet 05's already-bumped layer-planning package.
- Determinism/scheduler constraints: projections use registry ordering, module ownership/provenance filtering, and reconciled metadata; hash-map iteration never defines output order.

## Locked Assumptions and Invariants

- FORWARD-DEP packet 05 exports `resolve_scope_stack`, `query_z_grid`, `ResolvedObjectLayerConfig { object_id: String, object_height: f64, layer_height: f64, first_layer_height: f64, support_raft_layers: u32 }`, `ResolutionError::InvalidObjectHeight`, and `prepass-layer-planning@2.0.0`.
- The fallback census is exactly 89 across the 13 named guests under the approved classification; changing that population requires owner review, not fixture shrinkage.
- The three `mmu_segmented_region_*` keys remain the only intentional `CONFIG_BLOCK` omissions.

## Risks and Tradeoffs

- Adding declaration metadata has a broad struct-literal blast radius; inventory and edit it atomically.
- Materializing defaults can expose bad or conflicting manifest defaults formerly hidden by guest literals; registry assembly/type failures must stay loud.
- The accepted `CONFIG_BLOCK` byte change can update output fixtures; assert semantic key/value content rather than recapturing opaque whole-file baselines.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (guest cleanup batches and no-drop e2e)
- Highest-risk dispatch and required return format: fallback classification, `LOCATIONS` in bounded per-guest batches.

## Open Questions

- `[FWD]` Reconcile packet 05 and packet 03 final exported names/shapes before activation; preserve this packet's behavior if spelling changes.
- `[BLOCK]` None beyond forward dependencies.
