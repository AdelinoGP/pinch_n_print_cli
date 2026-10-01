# Design: scope-resolution-module

## Controlling Code Paths

- Primary code path: `slicer_config::resolution::{query_z_grid, resolve_scope_stack}` consumes packet-03 `ScopedConfig`/`ScopeDelta`, resolves values through packet-01 registry metadata, invokes packet-04 expansion, and supplies LayerPlanning/RegionMapping.
- Neighboring tests/fixtures: new `crates/slicer-config/tests/scope_resolution_tdd.rs`; existing scheduler config-resolution tests; new runtime integration module; layer-planner tests; WIT contract/binding tests.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Scope values are deltas. Presence is `ScopeDelta.values.contains_key`, never inequality with `ResolvedConfig::default()`.
- One ordered engine owns both queries. `query_z_grid` uses global/object data today; `resolve_scope_stack` additionally applies modifier, lexical paint-semantic, and tool deltas. Row 9 inserts layer-range values between object and modifier without introducing a second resolver.
- The settled future overlap contract is not optional: later-starting `layer_height` wins while re-deriving the Z-grid; conflicting values of the same non-`layer_height` key are a load error. This packet documents and preserves the insertion seam but does not fabricate a current Rust layer-range implementation.
- Phase-B expansion runs after the complete applicable merge and before a `ResolvedConfig` is interned or a planning record crosses WIT.
- `prepass-layer-planning.run` parameter addition is one major package bump to `2.0.0`; update every hard-coded package/export assertion in the same coordinated step.
- The five-field host struct and WIT record must remain field-for-field adapters: `object_id/object-id`, `object_height/object-height`, `layer_height/layer-height`, `first_layer_height/first-layer-height`, and `support_raft_layers/support-raft-layers`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add `crates/slicer-config/src/resolution.rs` with a private ordered delta applicator and two public query functions. Keep typed ingestion and automatic expansion as dependencies rather than duplicating either concern. Convert to `ResolvedConfig` only at the resolver boundary.
- Net-new host symbols:
  - `slicer_config::resolution::ResolvedObjectLayerConfig`: public record `{ object_id: String, object_height: f64, layer_height: f64, first_layer_height: f64, support_raft_layers: u32 }`.
  - `slicer_config::resolution::ResolutionTarget`: public query descriptor identifying object id, ordered modifier ids, ordered/selected paint semantics, and optional tool index; no layer-range field until row 9.
  - `slicer_config::resolution::ResolutionError`: public error with `InvalidObjectHeight { object_id: String, value: Option<f64> }` and transparent expansion/application failures.
  - `slicer_config::resolution::resolve_scope_stack(registry: &ConfigSchemaRegistry, scoped: &ScopedConfig, target: &ResolutionTarget, expansion: &ExpansionContext) -> Result<ResolvedConfig, ResolutionError>`.
  - `slicer_config::resolution::query_z_grid(registry: &ConfigSchemaRegistry, scoped: &ScopedConfig, object_heights: &BTreeMap<String, f64>, expansion: &ExpansionContext) -> Result<Vec<ResolvedObjectLayerConfig>, ResolutionError>`; records are object-id sorted.
- Net-new WIT symbol: `object-layer-config` in `slicer:prepass-layer-planning@2.0.0`, shaped as the exact five fields above; `layer-planning.run` adds `object-configs: list<object-layer-config>`.
- Existing functions removed after migration: `resolve_global_config`, `resolve_per_object_configs`, `resolve_per_paint_semantic_configs`, `resolve_per_tool_configs`, and `apply_overlay` in `crates/slicer-scheduler/src/config_resolution.rs`; `overlay_resolved` in `crates/slicer-core/src/algos/region_mapping.rs`; `object_layer_height` and `object_height` in the default layer-planner guest.
- Existing call sites changed: `run_slice_with_collector`/`prepare_prepass_context` in `crates/slicer-runtime/src/run.rs`; configured prepass resolution in `crates/slicer-runtime/src/prepass.rs`; modifier/paint/tool application in `crates/slicer-core/src/algos/region_mapping.rs`; layer-planning dispatch in `crates/slicer-wasm-host/src/dispatch.rs`; `build_prepass_layer_planning_glue` in `crates/slicer-macros/src/lib.rs`; `PrepassModule::run_layer_planning` in `crates/slicer-sdk/src/traits.rs`.
- Rejected alternative: retain full `ResolvedConfig` overlays and add a presence bitmap. This duplicates packet-03 deltas and preserves two representations.
- Rejected alternative: introduce `ConfigScope::LayerRange` now. No layer-range source exists; row 9 owns ingestion and geometry/overlap behavior.
- Rejected alternative: keep object planning data in `ConfigView`. It preserves a host namespace across WIT and allows LayerPlanning to diverge from RegionMapping.

## Files in Scope (read + edit)

- `crates/slicer-config/src/resolution.rs`, `crates/slicer-config/src/lib.rs`, `crates/slicer-config/tests/scope_resolution_tdd.rs` — unified API and independent table oracle.
- `crates/slicer-scheduler/src/config_resolution.rs` and its config-resolution test modules — retire/migrate the old API while preserving bounds/type rejection tests.
- `crates/slicer-core/src/algos/region_mapping.rs` — replace `overlay_resolved` and modifier/paint/tool merging.
- `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/prepass.rs`, and `crates/slicer-runtime/tests/integration/{main.rs,scope_resolution_module_tdd.rs}` — route both production entry points and region mapping through the unified API.
- `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit` and `crates/slicer-schema/src/lib.rs` — WIT v2 record/signature and stage metadata.
- `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/tests/binding_surface_tdd.rs` — guest glue and package identity assertion.
- `crates/slicer-sdk/src/traits.rs` and affected SDK tests — typed object-record parameter on the native trait surface.
- `crates/slicer-wasm-host/src/{host.rs,dispatch.rs}`, `crates/slicer-wasm-host/test-guests/prepass-layer-planning-guest/src/lib.rs`, and contract registration/test files — generated host types, dispatch, and v2 boundary proof.
- `modules/core-modules/layer-planner-default/src/lib.rs` and its two affected test files — consume typed records and remove formatted prefixes/package v1 assertions.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` — architecture and WIT contract updates.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md` — Resolution through Guests and delivery, Testing, queue rows 5/9 only.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed-delta and Phase-B decisions.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — future loud-rejection boundary.
- `docs/11_operational_governance_and_acceptance_gate.md` — WIT compatibility policy section only.
- `docs/22_test_quality.md` — oracle and independent-expectation sections only.
- Packet 03/04 files — bounded symbol reconciliation only; never edit.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md` and packet directories 01–04 — read-only.
- `crates/slicer-model-io/**` and 3MF fixtures — layer-range ingestion belongs to row 9.
- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- Unrelated crates and guest modules — do not edit.

## Expected Sub-Agent Dispatches

- Question: reconcile the actually landed packet-03 and packet-04 exports with this packet's consumed names/shapes; scope: `crates/slicer-config/**` plus packet 03/04 contracts; return: `FACT: <5 lines or fewer>`; purpose: activation gate.
- Question: enumerate all calls/imports of the six removed resolver functions; scope: `crates/**/*.rs`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>`; purpose: migration completeness.
- Question: enumerate all `prepass-layer-planning@1.0.0` and `run_layer_planning` signature/package assertions; scope: `crates/**` and `modules/core-modules/layer-planner-default/**`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>`; purpose: WIT blast radius.
- Question: verify canonical later-starting overlap behavior by function name; scope: `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp`; return: `SUMMARY: <at most 200 words, no code unless requested>`; purpose: preserve row-9 seam without implementing it.
- Question: run each cargo/check command and report only verdict/failure excerpt; scope: workspace; return: `FACT: <5 lines or fewer>`; purpose: every verification step.

## Data and Contract Notes

- IR/manifest contracts: no IR schema or manifest vocabulary change. `ResolvedConfig` remains the final typed host record and extension carrier.
- WIT boundary: package identity changes from `slicer:prepass-layer-planning@1.0.0` to `@2.0.0`; the new parameter is intentionally incompatible and no v1 shim is retained.
- Determinism/scheduler constraints: object results are object-id sorted; modifiers are priority ascending with existing stable tie behavior; paint semantics are lexical; tool applies last.
- Layer range: precedence position and conflict policy are locked now; runtime interval and overlap logic remains absent until row 9.

## Locked Assumptions and Invariants

- Packet-03 and packet-04 exports are FORWARD-DEPs until landed; no implementation may guess around a mismatch.
- Absence remains absence. Explicitly stating a registry default at a narrower scope overrides a broader non-default.
- Both production entry points call the same two public resolver queries.
- `ConfigView` consumers receive no packet-04 placeholder from these paths.
- `layer_height_profile_from_ranges` and `LayerRanges::assign` are canonical-plan references, not pre-existing Rust symbols.
- Future layer-range overlap semantics are exactly: later-starting wins for `layer_height`; different values of the same non-`layer_height` key overlap are a load error.

## Risks and Tradeoffs

- The WIT bump has a broad compile blast radius; splitting declaration, adapters, dispatch, and guests into explicit steps limits context while they still land together.
- Removing scheduler APIs can break tests that were testing validation through those wrappers; preserve behavior by moving those tests to the unified resolver rather than deleting coverage.
- Converting `support_raft_layers` from a registry value must reject negative/out-of-range data rather than silently cast.
- A tempting premature layer-range enum would create an untested geometry contract; the public target deliberately leaves that field for row 9.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: WIT package/signature blast-radius inventory, `LOCATIONS` with at most 20 entries.

## Open Questions

- [FWD] At implementation start, reconcile packet-03 `ScopedConfig` accessors and packet-04 expansion function signatures against their landed code; adapt this packet internally without changing the approved scope or precedence.
- [FWD] If the landed `ScopeDelta` stores modifier priority/paint semantics outside `ConfigScope`, use its canonical metadata carrier in `ResolutionTarget`; do not add a duplicate ordering field.
