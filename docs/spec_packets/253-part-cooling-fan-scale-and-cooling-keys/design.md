# Design: 253-part-cooling-fan-scale-and-cooling-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/part-cooling/src/lib.rs` — `PartCooling::from_config` (reads 4 keys today via `config.get`), `layer_fan_speed`, `cooling_decision_for_event`, `run_finalization` (emits via `FinalizationOutputBuilder::push_fan_speed` → `GCodeCommand::FanSpeed`, and `push_annotation` → `LayerAnnotationKind::Raw`).
- Neighboring tests/fixtures: `modules/core-modules/part-cooling/tests/part_cooling_tdd.rs` (5 behavioural tests), `tests/cooling_config_schema_tdd.rs` (3 schema tests), `tests/slicer_module_binding_tdd.rs`; `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (annotation→G-code emission), `tests/contract/integrated_parity_part_cooling_tdd.rs`; `crates/slicer-sdk/src/traits.rs` `push_fan_speed` (value 0 → `Raw("M107")`, else `Raw("M106 S{v}")`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **Config keys are snake_case** (repo rule); new keys follow Orca's spelling exactly — this packet never invents a key name.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Schemas:** manifest `[config.schema]` edits flow through `read_config_schema` (`crates/slicer-scheduler/src/manifest.rs`) into `ConfigBoundsIndex::from_modules`; bounds/enum enforcement then happens host-side in `ConfigBoundsIndex::check` (`crates/slicer-scheduler/src/config_resolution.rs`) with zero host code change — the keys ride the `extensions` bucket of `ResolvedConfig` (untyped), merged by `to_config_map` (`crates/slicer-ir/src/resolved_config.rs`), pre-filtered per module by `ConfigView::from_declared` (`crates/slicer-ir/src/slice_ir.rs`). No typed `ResolvedConfig` field is added.
- **Co-declaration is an existing pattern:** `ConfigBoundsIndex::from_modules` walks *every* module's schema; multiple modules declaring one key merges their bounds (several keys are already co-declared across modules today). The five header/footer keys are therefore declared in both `part-cooling` and `machine-gcode-emit` without host changes.
- **Emission surface is generic:** `machine-gcode-emit`'s `substitute_placeholders` builds its lookup from `config.keys()` — every key in the module's `ConfigView` is substitutable as `[key]` with no code change; the reachability test (AC-8) pins this so a future lookup-source change cannot silently break it.
- **The percent→S conversion is shared:** implement exactly one helper, `percent_to_fan_s(p: i64) -> u8` = `floor(255.5 × p / 100)` — canonical `GCodeWriter::set_fan`'s `(unsigned int)(255.5 * speed / 100.0)`. Both the primary channel and the `P2` auxiliary channel use it. `overhang_fan_speed`'s existing percent scale is unaffected (it already stores percent and multiplies by the fan max), but its product with `fan_max_speed` must switch to percent×percent before conversion.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: deepen `PartCooling` in place (no new module, no WIT/IR change).
  1. **Manifest:** re-key the 2 scale keys to percent (`type = "float"`, default 100/20, min 0, max 100), add 17 new keys (the 19 minus the 2 re-keyed) with Orca defaults/bounds, add the threshold enum (`values = ["0%", "10%", "25%", "50%", "75%", "95%"]`, default `"95%"`), widen `internal_bridge_fan_speed`/`ironing_fan_speed`/`support_material_interface_fan_speed` to `min = -1, max = 100`. Co-declare the 4 header/footer keys in `machine-gcode-emit.toml` with identical types/defaults.
  2. **Percent normalization:** every site that turns a fan percent into an S-value calls the shared `percent_to_fan_s`; defaults 100/20 produce S255/S51 — the module's default-path bytes (`M106 S255`, `M107`) are unchanged, keeping existing fixtures honest.
  3. **Fan curve:** `PartCooling` grows the config fields and per-layer decision implementing the canonical branch chain (base = `R ? min : 0`; `T < S` → max; `S ≤ T < F` → `t = (T − S)/(F − S)`, speed = `floor(t·min + (1−t)·max + 0.5)` in percent; `T ≥ F` → base; ramp `factor = (idx+1−C)/(L−C)` multiplied and clamped only when `idx+1 < L`). Layer time `T` is derived per-layer in `run_finalization` from the view's entities (path length ÷ feedrate; `speed_factor` profiles honored), not guessed from scheduling data.
  4. **Role-fan selection:** per-layer role scan replaces today's `BridgeInfill`-only bump: `BridgeInfill`/`InternalBridgeInfill`/`SupportInterface`/`Ironing` map to their keys with canonical precedence and `-1` fallbacks (`internal_bridge` → `overhang_fan_speed`; the other two disabled at `-1`).
  5. **Threshold:** `overhang_fan_threshold` classifies whether an entity's `overhang_quartile` bands (map: 10%→quartile ≥1, 25%→≥2, 50%→≥3, 75%→4) or bridge/external-perimeter roles qualify it for the overhang fan.
  6. **Time-domain:** kickstart/speedup re-time rising `M106`s to an earlier entity boundary using the same per-entity time model; gated by `fan_speedup_overhangs`; emitted as `Raw` annotations ahead of the demanding entity.
  7. **Auxiliary channel:** with `auxiliary_fan`, each qualifying layer gains a `Raw("M106 P2 S{n}")` and the final layer gains `Raw("M106 P2 S0")`.
- Exact functions, traits, manifests, tests, and fixtures: `PartCooling::from_config` + new private helpers (`percent_to_fan_s`, layer-time estimator, role-fan selector, threshold classifier, re-timer); `FinalizationModule::run_finalization` orchestration unchanged in shape; new test files `cooling_curve_parity_tdd.rs` (module) and `cooling_placeholder_reachability_tdd.rs` (machine-gcode-emit); scheduler test `config_bounds_enforcement_tdd.rs` extended; fixture updates limited to restating raw-byte expectations in percent terms.
- Rejected alternatives and reasons:
  - New dedicated `exhaust-fan` / `air-filtration` modules: rejected — canonically these are header/footer emission features; a module boundary would invent a stage this tree's DAG does not have, and the co-declaration pattern reaches the same consumer with a manifest-only change.
  - Typed `ResolvedConfig` fields for the new keys: rejected — the extensions bucket already delivers values into the module `ConfigView`; typed fields are for host-side *decisions*, which (except the curve) none of these keys feed.
  - Converting at config-resolution time (storing S-values in config): rejected — Orca's percent must survive to templates/placeholders raw (AC-8 pins raw percents in templates); conversion belongs at emission.
  - Porting `FanMover` verbatim as a gcode-line post-filter over serialized G-code: rejected — this tree's guest modules decide before emission from IR views; re-implementing the same re-timing at the annotation level is strictly smaller and testable without a textual G-code rewriter.

### Honest dispositions recorded by this design (from authoring-time grounding)

- `dont_slow_down_outer_wall`: the canonical gate (`CoolingBuffer`'s external-perimeter adjustability) has **no counterpart stage** in this tree; the key is declared + emitted, and its absent consumer is recorded here rather than invented. Treated as plumbing with a gap, not silently "done".
- The 4 header/footer keys have no slicer-internal consumer: canonical semantics belong to machine custom G-code (`M191`/`M141`/`set_exhaust_fan`), which is user-authored here. Their deliverable is schema + placeholder reachability + config-block presence. `additional_cooling_fan_speed`/`auxiliary_fan` are also consumed per-layer by module logic (the `M106 P2` channel), so only `dont_slow_down_outer_wall` plus the 4 template keys live on the emission surface, and only the 4 template keys are co-declared into `machine-gcode-emit`.
- `slow_down_for_layer_cooling` / `slow_down_min_speed` / `slow_down_layer_time` already exist in the manifest: they stay (this packet does not firewall them), but `slow_down_layer_time` and `slow_down_min_speed` become *read* as curve inputs; the boolean's inertness is noted, untouched.

## Files in Scope (read + edit)

Target at most 3 primary files; extras below are test/co-declaration files justified per file.

- `modules/core-modules/part-cooling/part-cooling.toml` - role: owner manifest; expected change: re-key 2 scale keys to percent, add 11 keys + enum + widened ranges.
- `modules/core-modules/part-cooling/src/lib.rs` - role: the fan decision; expected change: percent normalize, curve port, role-fan selection, threshold, re-timing, P2 channel.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: co-declaration surface for the 5 header/footer keys; expected change: 5 `[config.schema]` tables, values byte-identical to part-cooling's.

Test files in scope (justified extras): `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs` (new), `tests/part_cooling_tdd.rs` + `tests/cooling_config_schema_tdd.rs` (fixture rekeying + new default rows), `modules/core-modules/machine-gcode-emit/tests/cooling_placeholder_reachability_tdd.rs` (new), `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` (new negative case), `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (AC-N2 leakage test appended; its existing byte assertions are a regression pin and must not change).

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — lines `2216-2255` (`ExtrusionRole`), `2340-2420` (`Point3WithWidth`/`ExtrusionPath3D`, incl. `overhang_quartile`), `2748-2830` (`PrintEntity`, `LayerAnnotation`), `2828-2875` (`LayerCollectionIR` + `speed_profiles`) only — the types the module reads.
- `crates/slicer-sdk/src/traits.rs` — lines `100-160`, `780-830`, `1280-1300` — `LayerCollectionView` API + `push_fan_speed` semantics.
- `crates/slicer-gcode/src/serialize.rs` — lines `480-575` (`ORCA_CONFIG_PADDING`), `820-840` (`GCodeCommand` writer arms) only — no edits; the padding table is hand-maintained and NOT regenerated by `gen-config-docs`, so newly declared keys reach the block through the raw-config passthrough, not this table.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` — lines `120-260`, `770-830` only — `INJECTION_POINTS` and the placeholder lookup source (`config.keys()`): read-only proof for AC-8's mechanism.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` (sibling path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) - delegate; never load. All behavioural quotes already captured in `requirements.md` §Per-Key Canonical Evidence; re-dispatch only to resolve disputes.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates (slicer-core host-algos, arachne, support planners) - delegate symbol lookups; do not browse
- `crates/slicer-gcode/src/serialize.rs` - read-only for the padding-table fact above; editing it is a behavior change this packet does not authorize (`ORCA_CONFIG_PADDING` is a viewer-compat gate, not a decision point).
- Any other `docs/spec_packets/<n>/**` directory - other packets' files are never edited.

## Expected Sub-Agent Dispatches

- Question: quote `CoolingBuffer.cpp` `change_extruder_set_fan`'s branch chain + `GCodeWriter::set_fan` conversion verbatim at implementation time; scope: sibling `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` + `GCodeWriter.cpp`; return: `SNIPPETS` (≤3, 30 lines); purpose: Step 4 re-verification.
- Question: confirm `ConfigBoundsIndex::check` enum-array handling for a NEW enum key (does a manifest `values` list produce enum enforcement for string ConfigValues?); scope: `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 6 negative-case plumbing.
- Question: enumerate test files asserting `M106`/`M107` byte sequences for part-cooling; scope: `crates/slicer-runtime/tests/**`, `modules/core-modules/part-cooling/tests/**`; return: `LOCATIONS`; purpose: Step 3 fixture blast radius.

## Data and Contract Notes

- IR/manifest contracts: no IR schema change; manifest `[config.schema]` entries are the single source of truth for keys, bounds, enums, defaults (parsed by `read_config_schema`, `crates/slicer-scheduler/src/manifest.rs`); `ConfigView::from_declared` hides undeclared keys (negative AC-N2) — no host change needed or allowed.
- WIT boundary: unchanged — the module's world and its `[config.schema]` are decoupled; config reaches the guest through the existing ConfigView type.
- Determinism/scheduler constraints: the fan decision is a pure function of (layer views, config); re-timing only ever moves a `Raw` annotation to an earlier `after_entity_index` within the same layer set — never across the `close_fan_the_first_x_layers` boundary (fan-off first layers stay fan-off).

## Locked Assumptions and Invariants

- Defaults 100/20 percent convert to S255/S51 — byte-identical to today's raw defaults; the pre-packet emission fixture bytes (`M106 S255`, `M107`, `M106 S100` for the overhang bump under `overhang_fan_speed=100` × max percent 100 → S255) remain the expected output under default config.
- `fan_min_speed` becomes read by the curve (today: declared, never read) — its first live read is Step 4's base branch, wired exactly per canonical (`R` idle).
- The `close_fan_the_first_x_layers` gate dominates every branch (canonical: early layers force fan 0 regardless of `R`).
- Ramp inert when `full_fan_speed_layer ≤ close_fan_the_first_x_layers` (canonical divisor would be ≤ 0).
- No textual G-code parsing is introduced — every decision reads IR views.

## Risks and Tradeoffs

- **Per-point vs per-layer granularity:** canonical `check_overhang_fan` compares per-extrusion-point overlap; PnP classifies per entity using quartile bands — a documented approximation (quartile bands are themselves an overhang-degree quantization). Behaviour converges as thresholds map to bands; exact per-point overlap equality is NOT claimed.
- **Layer-time estimation** from path length ÷ feedrate ignores accelerations/retractions; canonical's estimate has the same class of simplification at this layer. Documented as estimate semantics.
- The `-1` sentinel keys are declared `int` with `min = -1` — module code must read `i32`/`i64` (not `u8`) or the sentinel wraps; the curve tests pin the fallback explicitly (AC-4).
- Fixture churn: raw-byte assertions in three existing files must be restated in percent terms without weakening (Step Completion Expectations in `requirements.md`); any mid-scale expectation must be re-derived through `floor(255.5 × p / 100)` (50% → `floor(127.75)` = **S127**, never eyeballed), and the overhang bump at defaults stays S255 (100% × 100%).
- Emission-surface keys add no user-visible behaviour until a template references them — reviewers may mistake "no output change" for "not implemented"; AC-8 exists to make the reachability itself the assertion.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 fan-curve port + its invariant suite)
- Highest-risk dispatch and required return format: the CoolingBuffer re-verification — `SNIPPETS` capped at 3/30 lines; anything larger is rejected and re-dispatched narrower.

## Open Questions

- `[FWD]` Whether the scheduler's `ConfigView` enum check supports string enums declared via manifest `values` for **int-typed** canonical schemas — if the host stores the threshold as its string form (`"95%"`), module-side matching compares strings; if it stores the ordinal, matching compares integers. Resolvable at Step 6 by reading `ConfigBoundsIndex`/`ConfigValue` enum handling; no activation blocker.
- `[FWD]` Whether `check_overhang_fan`'s `0%` → external-perimeter role arm should also cover `ThinWall`/`OuterWall` sub-roles PnP may split later — pinned to `OuterWall` for now; PnP has no separate external/internal perimeter role pair today (`OuterWall` is the external wall role). Resolvable at Step 6; no activation blocker.