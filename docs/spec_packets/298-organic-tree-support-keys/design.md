# Design: 298-organic-tree-support-keys

## Controlling Code Paths

- Primary code path: `SupportPlanner::from_config` (`modules/core-modules/tree-support-planner/src/lib.rs:1618`, `Result<Self, ModuleError>`) — reads the classic branch keys today (`tree_support_branch_angle` → `branch_angle_deg`, `tree_support_branch_diameter`, `tree_support_branch_distance`), resolves the style via `TreeSupportStyle::from_config` (`:225`) and the explicit-organic gate via `organic_substitution_requested` (`:252`), and builds `Ok(Self {` (`:1759`) feeding the classic `drop_nodes`-family passes. The organic set resolves into the same effective fields behind the gate — no new geometric pass, no engine.
- Renderer code path: `TreeSupport::from_config` (`modules/core-modules/tree-support/src/lib.rs:242`, `Result<Self, ModuleError>`) reading via `ConfigView::get` / `get_float` / `get_int` / `get_abs_value` (`crates/slicer-ir/src/slice_ir.rs:880,895,915,961`) and building `Ok(Self {` (`:300`); it consumes `SupportPlanEntry` via `PaintRegionLayerView::support_plan()` and `paint_policy_for`, and emits `SupportIR` with no brim stage today.
- Neighboring tests/fixtures: planner `tests/tree_style_styles_tdd.rs` (style routing via `ConfigView::from_map`, direct helper calls), `tests/orca_parity_tdd.rs` (`slicer_sdk::module_test` + `prepass_builders::SupportGeometryOutput` harness); renderer `tests/tree_support_tdd.rs` (`tree_support::TreeSupport`, `TreeSupport::from_config(&config).unwrap()`, `builders::SupportOutputBuilder`, `test_prelude`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Style gate, not engine fork: the organic set is selected exactly when `organic_substitution_requested(config)` is true (explicit `support_style = organic` on a tree family). `TreeSupportStyle::from_config` keeps mapping that input to Strong; DEV-156 stands; the code-1005 Warn condition is untouched. Default/`grid`/`snug`-on-tree runs keep classic params (recorded scoping divergence, DEV-189).
- Value semantics mirror the organic settings constructor (delegated reads): degree→radian conversion for the two angles, constructor clamps (angle, slow-vs-tree cap, tip≤diameter, top-rate percent scaling). Canonical minima are GUI hints — saturate, never reject (ticket-113 rule). The `Print.cpp` cross-validations are not ported (no validator seam; ticket 124).
- Renderer brim is a first-layer stage over build-plate-contact tree bases only; `0.0` (or empty derivation) emits no loops; bodies are otherwise untouched. The renderer must declare `support_style` to see the gate (declared-view whitelist, ticket-34 lesson) — supporting row, not a queue key.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Percent-string hazard (ticket-34 lesson): `tree_support_top_rate` arrives as a percent string; resolve it through the `get_abs_value`-class accessor path the module already uses for percent keys, never a bare-float read. Module bounds check the raw spelling; the percent string skips bounds by design.
- `check-literals` watchlist: both edited structs are watched types. Struct-literal blast radius is fully inventoried (LOCATIONS dispatch at authoring): `SupportPlanner` has 3 sites, all in `tree-support-planner/src/lib.rs` (`from_config` `Ok(Self {`, `default_planner`, test helper) — zero test-dir literals; `TreeSupport` has 1 site (`from_config` `Ok(Self {` in `tree-support/src/lib.rs`; `from_config_defaults` calls `from_config`, no literal). Steps 2–3 own every site.

## Code Change Surface

- Selected approach: resolve-into-existing-fields in the planner (organic values flow into the effective angle/diameter/distance/tip/top-rate/slow fields `SupportPlanner::from_config` already threads to the passes; field names keep their classic spelling with a comment at the selection site), plus a compact brim parameter block on `TreeSupport` (auto flag + width + gated style) with a first-layer loop-emission stage in the renderer. No new module, no claim change, no IR/WIT/schema change.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` — 6 new `[config.schema.*]` rows (float rows mirror the classic `tree_support_branch_angle` stanza shape: `type`/`default`/`min`/`max`/`display`/`group`; percent row mirrors `support_line_width`-class percent spelling with string default `"30%"`).
  - `modules/core-modules/tree-support-planner/src/lib.rs` — organic selection arm in `SupportPlanner::from_config`; effective-field saturation (tip≤diameter, slow cap, structural floors); update all 3 struct literals; unit coverage via the new integration file (no new `#[cfg(test)]` module needed).
  - `modules/core-modules/tree-support/tree-support.toml` — 2 brim rows + 1 supporting `support_style` enum row (values copied from the planner's row).
  - `modules/core-modules/tree-support/src/lib.rs` — brim params on `TreeSupport`, read in `TreeSupport::from_config`, first-layer brim emission over plate-contact bases, `Ok(Self {` updated.
  - `modules/core-modules/tree-support-planner/tests/organic_params_tdd.rs` (new, own `--test organic_params_tdd` binary, autodiscovered) — AC-2–AC-5, AC-N1–AC-N2 via `ConfigView::from_map` + `SupportPlanner::from_config` + `SupportGeometryOutput` harness + existing pub style helpers.
  - `modules/core-modules/tree-support/tests/tree_brim_tdd.rs` (new, own `--test tree_brim_tdd` binary, autodiscovered) — AC-6–AC-7 via `TreeSupport::from_config` + `SupportOutputBuilder` harness.
  - `docs/DEVIATION_LOG.md` — append DEV-189 row (Step 2).
  - `docs/15_config_keys_reference.md` — regenerated by tool (Step 4), not hand-edited.
- Rejected alternatives and reasons:
  - Wiring organic keys into the classic path for all styles: changes default tree output for every user to Strong-with-organic-params, which matches neither canonical (organic engine) nor current behaviour; rejected for default-path stability.
  - Returning the six params as unimplemented behind the missing engine (ticket-28/39 shape): rejected — unlike those tickets, the substituted engine already consumes the same-shaped parameters, so every key can drive a real, tested decision point today and migrate to the real engine later.
  - Porting the volumetric organic engine in this packet: an order of magnitude beyond an 8-key Tier B slice; stays queued at remediation-plan row 7.

## Files in Scope (read + edit)

- `modules/core-modules/tree-support-planner/tree-support-planner.toml` - role: declare 6 organic keys; expected change: 6 schema stanzas.
- `modules/core-modules/tree-support-planner/src/lib.rs` - role: style-gated selection + saturation; expected change: selection arm, field saturation, 3 literal updates (ranges: `from_config` ~1618–1800, literals at 6544/6771 — re-derive exact windows at implementation).
- `modules/core-modules/tree-support/tree-support.toml` - role: declare 2 brim keys + style gate row; expected change: 3 schema stanzas.
- `modules/core-modules/tree-support/src/lib.rs` - role: brim params + first-layer emission; expected change: struct fields, `from_config` reads, emission stage (ranges: `from_config` ~242–310 — re-derive at implementation).
- Justification for 4 files (over the 3-file target): two modules × (manifest + code) is the minimal surface for a two-seam packet; test files and the DEV-LOG row ride Steps 2–3 within the per-step edit cap. No split needed — aggregate stays M.

## Read-Only Context

- `modules/core-modules/tree-support-planner/src/lib.rs` - lines 202–258 only - purpose: style enum, substitution gate, family check (do not re-read whole file).
- `crates/slicer-ir/src/slice_ir.rs` - lines 875–975 only - purpose: `ConfigView` accessor semantics for float/int/percent reads.
- `docs/DEVIATION_LOG.md` - DEV-156 row only - purpose: substitution contract this packet preserves.
- `docs/specs/support-generation-remediation-plan.md` - row 7 only - purpose: engine scope boundary.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `docs/spec_packets/238b-tree-planner-canonical-fidelity/` and all other packet dirs - never modify; 238b is `implemented` context only
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - never touch (rule 2)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: exact organic settings-constructor formulas (radian conversion points, clamp order, tip≤diameter enforcement, top-rate application, slow-cap application, auto-brim derived-width formula, draw_circles first-layer/base conditions); scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` + `TreeSupport.cpp`; return: `SNIPPETS` (≤3, ≤30 lines) + `SUMMARY` (≤200 words); purpose: Steps 2–3 wiring.
- Question: canonical default/min/max re-verification for all eight keys; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT`; purpose: Step 1 manifest values.
- Question: `run_support_geometry` code-1005 Warn site shape (confirm condition untouched); scope: `modules/core-modules/tree-support-planner/src/lib.rs`; return: `LOCATIONS`; purpose: Step 2 AC-N2.

## Data and Contract Notes

- IR/manifest contracts: manifest-only additions; no IR field, no WIT change, no schema-version bump. Renderer declares `support_style` solely to pass the declared-view whitelist — the value is read-only there, never re-resolved into routing.
- WIT boundary: untouched. Guest rebuild is mechanical freshness (`build-guests --check`), not a contract change.
- Determinism/scheduler constraints: selection is a pure function of declared config + existing style resolution; no ordering, RNG, or cross-layer state added. Brim emission is deterministic over plate-contact bases.

## Locked Assumptions and Invariants

- Explicit-organic gate (`organic_substitution_requested`) is the sole selector of the organic set; classic styles are unreachable by organic keys (AC-2 pins it).
- Canonical scalarity held: all eight keys scalar-global + existing per-object overlay (explicitly not ticket 125's tool axis).
- No range rejection: sub-canonical inputs saturate (AC-N1 pins it).
- DEV-156 substitution + Warn survive (AC-N2 pins it).

## Risks and Tradeoffs

- Organic-params-on-classic-engine is a recorded stepping-stone divergence (DEV-189), not end-state parity: Strong-with-organic-params matches neither canonical engine. Accepted because it is strictly closer on the parameter axis for users who explicitly opted into the unimplemented engine, is fully tested, migrates cleanly to the real engine, and leaves everyone else byte-identical.
- Auto-brim derived-width formula must be borrowed exactly (delegated read); a guessed formula would bake in a silent geometry divergence. The dispatch above is mandatory, not optional.
- Percent-string reads must follow the existing percent accessor path; a bare-float read would reintroduce the ticket-34 units mismatch (strings skip bounds by design).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2: planner wiring + tests + DEV row)
- Highest-risk dispatch and required return format: organic-constructor formulas — `SNIPPETS` (≤3 × 30 lines) plus `SUMMARY` (≤200 words), Step 2/3 prerequisite.

## Open Questions

None. No `[FWD]`, no `[BLOCK]` — every load-bearing symbol verified against the tree at authoring; canonical formulas are delegated reads with known locations, not open design questions.
