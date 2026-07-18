# Design: 149-pipeline-config-wiring

## Controlling Code Paths

- Primary code path: `modules/core-modules/arachne-perimeters/src/lib.rs::run_perimeters` (lines 236-352) gains D3 `max_bead_count` bump (on odd layers when gate conditions met) and D4 bridge flow reduction; `arachne_params_from_config` (lines 106-197) gains reads for `alternate_extra_wall`, `bridge_flow`, `thick_bridges`.
- `modules/core-modules/classic-perimeters/src/lib.rs::run_perimeters` gains D4 bridge flow reduction (for parity with arachne).
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` and `classic-perimeters.toml` gain new `[config.schema.*]` sections.
- `crates/slicer-core/src/flow.rs` gains a new helper `pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32` (canonical location: `slicer-core` is the host-side math crate).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load). Do not restate the delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (manifest, `slicer-core/src/flow.rs` if the helper is in scope, both perimeter modules' `src/lib.rs`), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The 11 new arachne manifest keys (8 audit keys + `spiral_vase` + `sparse_infill_density` for the D3 gate + `only_one_wall_top` for the AC-2 read) are all read via the existing `ConfigView::get_bool` / `get_float` pattern; no WIT boundary change.
- The D-104f (concentric infill Arachne) gap is explicitly OUT OF SCOPE for this packet. The `arachne_parity_pipeline_concentric_infill_uses_arachne` test stays red at packet close. The deviation row is registered as `Status: Open — deferred to follow-up workstream`.
- D1 (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`) and D2 (`min_width_top_surface`) are manifest-only registrations. The red tests assert key presence, not behavior. The behavior implementation (overhang reverse logic, min-width-top-surface threshold for `only_one_wall_top`) is deferred to a follow-up packet; this packet does NOT implement the behavior.
- D3 (`alternate_extra_wall`) and D4 (`bridging_flow`) ARE behavior changes — they require source code in the perimeter modules, not just manifest entries.

## Code Change Surface

- **Selected approach:** Split into 5 atomic sub-tasks (one per deviation) plus a Doc Impact step. Each sub-task has its own red test, its own implementation surface, and its own deviation log row. The deviation rows land in the same commit as the code (per the grilling-session decision: rows land with each implementation packet).
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`:
    - Add `[config.schema.detect_overhang_wall]` (bool, default `true`).
    - Add `[config.schema.overhang_reverse]` (bool, default `false`).
    - Add `[config.schema.overhang_reverse_internal_only]` (bool, default `false`).
    - Add `[config.schema.min_width_top_surface]` (float, default verified via sub-agent dispatch against `docs/ORCA_CONFIG_REFERENCE.md:135` BEFORE commit; the spec's guess of 1.2mm is unverified).
    - Add `[config.schema.alternate_extra_wall]` (bool, default `false`).
    - Re-publish `[config.schema.extra_perimeters_on_overhangs]` (copy from `classic-perimeters.toml:45`; default `false`).
    - Add `[config.schema.bridge_flow]` (float, default `1.0`; the OrcaSlicer coFloat default is 1.0, NOT 0.85 — the previous spec's 0.85 constant was a fabrication).
    - Add `[config.schema.thick_bridges]` (bool, default `false`; `ORCA_CONFIG_REFERENCE.md:150`).
    - Add `[config.schema.spiral_vase]` (bool, default `false`) and `[config.schema.sparse_infill_density]` — the D3 gate reads both; neither is registered or read in the module today (verified: zero grep matches).
    - Add `[config.schema.only_one_wall_top]` (copy classic's section) — the AC-2 predicate requires the arachne SOURCE to read it; registration accompanies the read.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`:
    - Re-publish the 4 missing keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`) and add the 4 new keys (`alternate_extra_wall`, `bridge_flow`, `thick_bridges`).
  - `crates/slicer-core/src/flow.rs`:
    - Add `pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32 { if thick_bridges { 1.0 } else { bridge_flow_ratio } }` (mirrors OrcaSlicer's `LayerRegion.cpp:135` formula simplified for PnP's per-vertex model. The canonical OrcaSlicer formula is `base_flow.with_flow_ratio(bridge_flow_ratio)` for the non-thick branch; PnP's per-vertex `flow_factor` model is a divergence — D-104g documents this).
  - `modules/core-modules/arachne-perimeters/src/lib.rs`:
    - In `arachne_params_from_config` (or `run_perimeters` directly), read `alternate_extra_wall`, `bridge_flow`, `thick_bridges`.
    - In `run_perimeters`, after packet 148's `is_bridge` per-vertex flag is set, for each `path.points[i]` with `feature_flags[i].is_bridge == true`, set `pt.flow_factor = slicer_core::flow::bridging_flow(bridge_flow_ratio, thick_bridges)`.
    - **D3 mechanism rewrite**: apply the alternating-layer `max_bead_count` bump: when `alternate_extra_wall` is `true` AND `layer_index % 2 == 1` (odd) AND `!spiral_vase` AND `sparse_infill_density > 0`, set `params.max_bead_count = params.max_bead_count + 1` BEFORE calling `generate_arachne_walls(...)`. The bump flows into `WallToolPaths(..., coord_t(loop_number + 1), ...)` → `max_bead_count = 2 * inset_count` at `WallToolPaths.cpp:525` (mirrors OrcaSlicer's `loop_number++` at `PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne)). On even layers, the base `max_bead_count` is used. The unit test in `arachne-perimeters/tests/alternate_extra_wall_tdd.rs` (NEW) verifies the alternating behavior.
  - `modules/core-modules/classic-perimeters/src/lib.rs`:
    - Apply the same `bridging_flow(bridge_flow_ratio, thick_bridges)` flow_factor reduction on bridge segments (the classic path's `is_bridge` flag is set per-vertex at `lib.rs:677`; the same `pt.flow_factor` reduction applies).
    - Add a `min_width_top_surface` read-and-validate config read (AC-2's predicate greps `CLASSIC_MODULE_SRC` for the string; behavior deferred per D-104d).
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (additional, for AC-2): add an `only_one_wall_top` read-and-validate config read (AC-2's predicate greps `ARACHNE_MODULE_SRC` for the string; behavior deferred per D-104d).
  - `crates/slicer-runtime/tests/arachne_parity.rs` (AC-4 only): REWRITE `arachne_parity_pipeline_bridge_flow_factor_on_overhang` (:232-250) to drive `run_perimeters` natively with a `bridge_areas` fixture — its current predicate asserts on HOST-pipeline `junctions[].p.flow_factor` for a bridgeless square and cannot observe the guest-side fix. Preserve the test name; the other three pipeline red tests keep their existing predicates (AC-1/AC-3 are manifest greps that flip on Step 1; AC-2 is a source grep that flips on the reads above).
  - `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs` (NEW): unit test for AC-3's behavior assertion. Drives `run_perimeters` natively and asserts the wall count is 3 on odd layers and 2 on even layers when `alternate_extra_wall=true` and the gate conditions are met.
  - `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs` (NEW): unit test for AC-4's behavior assertion. Drives `run_perimeters` natively with `bridge_areas` non-empty and `bridge_flow = 0.7`, asserts `flow_factor == 0.7` for bridge vertices.
  - `docs/DEVIATION_LOG.md`:
    - Add 6 new rows (D-104b, D-104c, D-104d, D-104e, D-104f, D-104g) matching the existing-row format. D-104b/c/d/e's `Status` flips to `Closed — <date>: packet 149` at packet close; D-104f's `Status` is `Open — deferred to follow-up workstream`; D-104g's `Status` is `Open` (documents the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence — the `thick_bridges` branch in the helper is the canonical realization site).
  - `docs/14_deviation_audit_history.md`:
    - Append one row per new deviation (6 total).
  - `docs/15_config_keys_reference.md`:
    - Append the 11 new config keys (4 in §Overhangs, 2 in §Walls incl. `only_one_wall_top`, 3 in §Strength incl. the D3 gate keys, 2 in §Bridging) — creating the §Overhangs/§Strength/§Bridging subsections, which do not exist today (only `## Walls (packet 104)` does).
- **Rejected alternatives:**
  - Implementing the behavior for D1 (overhang_reverse logic) and D2 (min-width-top-surface threshold) in this packet: rejected — the red tests assert key presence, not behavior; implementing the behavior would require a much larger surface (overhang reverse is a path-optimization concern, not a perimeter concern; min-width-top-surface is an `only_one_wall_top` enhancement). Defer behavior to a follow-up packet; this packet only registers the keys.
  - Implementing D-104f (concentric infill Arachne) in this packet: rejected — the user explicitly deferred D5 to a follow-up workstream because it's the largest of the 15 gaps and warrants a multi-packet design effort. The corresponding red test stays red.
  - Adding `bridging_flow()` to `slicer-sdk` (alongside the existing `slicer-core` re-exports): rejected — `slicer-sdk` is the WIT-boundary surface; flow math is host-side and belongs in `slicer-core`. The classic path's existing `flow.rs` use confirms this.
  - One mega-step covering all 4 sub-tasks: rejected — each sub-task has its own red test, its own risk surface, and its own deviation log row. Splitting them makes the review easier and the failure recovery faster.
  - **Shipping a constant `0.85` for `bridging_flow()` (the previous spec)**: rejected — OrcaSlicer's real formula is `base_flow.with_flow_ratio(bridge_flow_ratio)` with `bridge_flow_ratio` defaulting to 1.0 (NOT 0.85). PnP implements the real formula: `flow_factor = bridge_flow_ratio` (or 1.0 when `thick_bridges`). The per-vertex `flow_factor` model diverges from OrcaSlicer's per-path `Flow` model — this is D-104g.
  - **Implementing D3 (alternate_extra_wall) as a post-hoc wall-count mutation (the previous spec)**: rejected — OrcaSlicer's mechanism is `loop_number++` → `WallToolPaths(..., coord_t(loop_number + 1), ...)` → `max_bead_count = 2 * inset_count`, which is a beading-stack-level bump, NOT a wall-count mutation. PnP mirrors this: bump `ArachneParams.max_bead_count` by 1 on odd layers when the gate conditions are met.

## Files in Scope (read + edit)

- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — role: arachne module manifest; expected change: 11 new `[config.schema.*]` sections.
- `crates/slicer-runtime/tests/arachne_parity.rs` — role: audit test suite; expected change: AC-4 red test rewritten to drive `run_perimeters` natively (host-junction predicate cannot observe the guest-side fix); other tests untouched.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: arachne module source; expected change: read new keys, apply D3 `max_bead_count` bump + D4 bridge flow.
- `modules/core-modules/arachne-perimeters/Cargo.toml` — role: arachne module deps; no change (slicer-core was added in packet 148).
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — role: classic module manifest; expected change: 7 new `[config.schema.*]` sections (4 re-publications + 3 new keys: `alternate_extra_wall`, `bridge_flow`, `thick_bridges`).
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: classic module source; expected change: apply D4 bridge flow (mirrors arachne).
- `crates/slicer-core/src/flow.rs` — role: host-side flow math; expected change: add `pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32`.
- `docs/DEVIATION_LOG.md` — role: deviation table; expected change: 6 new rows.
- `docs/14_deviation_audit_history.md` — role: audit log; expected change: 6 new rows.
- `docs/15_config_keys_reference.md` — role: config key reference; expected change: 11 new rows across new §Overhangs/§Strength/§Bridging subsections + §Walls.

## Read-Only Context

- `docs/ORCA_CONFIG_REFERENCE.md` lines 135, 161, 165-168, 178 — the canonical OrcaSlicer defaults for the 7 new keys.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml:45-50` — the existing `extra_perimeters_on_overhangs` section (the source of truth for the re-publication).
- `modules/core-modules/classic-perimeters/src/lib.rs:204-205, 302` — the existing `extra_perimeters_on_overhangs` consumer.
- `modules/core-modules/classic-perimeters/src/lib.rs:677` — the per-vertex `is_bridge` assignment (the site that D4 reads).
- `modules/core-modules/classic-perimeters/src/lib.rs:222, 268` — the existing `only_one_wall_top` reader (the precedent for D2's classic-path behavior, deferred).
- `crates/slicer-core/src/flow.rs:42-122` — the existing flow math; the `bridging_flow()` helper is added to the same file.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/src/run.rs` — out of scope; the perimeter modules are invoked from `run.rs` but `run.rs` does not need to change.
- `crates/slicer-core/src/arachne/pipeline.rs` — out of scope; the host service bridge is unchanged.
- `crates/slicer-core/src/beading/*` — out of scope; the beading-strategy stack is unchanged.
- The arachne per-vertex parity code (packet 148) — the D3 and D4 logic reads `feature_flags[i].is_bridge` and the wall count, but does not modify packet 148's fields.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output.log`; return FACT (pass count vs expected ≥ 14) and the failing-test detail block (≤ 20 lines) on any failure." — purpose: AC-6.
- "Read `docs/ORCA_CONFIG_REFERENCE.md` lines 135, 146, 150, 161, 165-168, 178; return SNIPPETS (verbatim, ≤ 30 lines) of the canonical OrcaSlicer defaults for the new keys (`min_width_top_surface` :135, `bridge_flow` :146, `thick_bridges` :150, `alternate_extra_wall` :178, overhang keys :165-168)." — purpose: confirm the default values are byte-for-byte.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:135`; return SUMMARY (≤ 200 words) of the `bridging_flow(frPerimeter, thick_bridges)` formula." — purpose: confirm the bridge multiplier value.
- "Run `cargo xtask build-guests --check 2>&1 | tee target/guest-check.log`; return FACT (Fresh/STALE)." — purpose: confirm the manifest change doesn't leave the arachne guest stale.
- "Run `grep -c 'config\.schema\.precise_outer_wall\]' modules/core-modules/arachne-perimeters/arachne-perimeters.toml && grep -c 'config\.schema\.seam_candidate_angle_threshold_deg\]' modules/core-modules/arachne-perimeters/arachne-perimeters.toml`; return FACT (each count ≤ 1 = pass for AC-N1; the keys are legitimately present from packet 148, duplication is the failure)." — purpose: manifest-drift guard.
- "Run `rg -A1 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md | head -5`; return SNIPPETS." — purpose: AC-N2 (Target Close does not name a fabricated packet).
- "Run `rg -q 'D-104b-OVERHANG-FLOW-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104c-OVERHANG-REVERSE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104d-MIN-WIDTH-TOP-SURFACE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104e-ALTERNATE-EXTRA-WALL-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md; echo $?`; return FACT (exit 0 = pass for AC-5)." — purpose: deviation row presence.

## Data and Contract Notes

- **IR or manifest contracts touched:**
  - `Point3WithWidth.flow_factor: f32` — already present, no shape change. The new behavior sets it to `bridge_flow_ratio` (config-driven, default 1.0) for bridge vertices when `thick_bridges=false`, and leaves 1.0 when `thick_bridges=true`.
  - `WallFeatureFlags.is_bridge: bool` — already set per-vertex (packet 148). The D4 logic reads it.
  - `arachne-perimeters.toml [config.schema]` — 11 new entries: 4 D1 keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `extra_perimeters_on_overhangs`), 1 D2 key (`min_width_top_surface`), 1 D3 key (`alternate_extra_wall`), 2 D4 keys (`bridge_flow`, `thick_bridges`), 2 D3-gate keys (`spiral_vase`, `sparse_infill_density`), 1 AC-2 key (`only_one_wall_top`). All bool/float; default values match `docs/ORCA_CONFIG_REFERENCE.md`.
  - `classic-perimeters.toml [config.schema]` — 7 new entries (4 missing keys + `alternate_extra_wall` + `bridge_flow` + `thick_bridges`; `extra_perimeters_on_overhangs` and `only_one_wall_top` are already there).
- **WIT boundary considerations:** none. The new config keys are read via the existing `ConfigView::get_bool`/`get_float` pattern. The `bridging_flow()` helper is host-side, not guest-side.
- **Determinism or scheduler constraints:** none beyond what packet 148 + classic already enforce. The bridge flow reduction is a pure function of config (`bridge_flow_ratio` or 1.0) per vertex, deterministic.

## Locked Assumptions and Invariants

- The 11 new arachne manifest keys (and 7 classic) MUST have **identical** defaults to the OrcaSlicer reference. The implementer should `diff` against `docs/ORCA_CONFIG_REFERENCE.md` before committing. `min_width_top_surface`'s default is verified via sub-agent dispatch BEFORE commit (the spec's guess of 1.2mm is unverified).
- The `bridging_flow()` helper MUST be defined in `slicer_core::flow` (not in `slicer-sdk`); both perimeter modules call it. Single source of truth.
- The bridge flow factor MUST be applied to BOTH the classic and arachne paths. Classic was not in the audit's red tests, but the parity is implied by the audit's framing ("the PnP pipeline as a whole").
- The D3 (`alternate_extra_wall`) mechanism MUST be a `max_bead_count` bump on odd layers, NOT a wall-count mutation. Gate: `layer_index % 2 == 1 && !spiral_vase && sparse_infill_density > 0`. Mirrors OrcaSlicer's `loop_number++` at `PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne).
- The D-104f row's `Status` field is `Open — deferred to follow-up workstream`; its `Target Close` field is `— (deferred; follow-up workstream TBD)` (no fabricated schedule). The corresponding red test `arachne_parity_pipeline_concentric_infill_uses_arachne` STAYS RED at packet close. This is the explicit success criterion for the deviation registration (NOT a defect).
- The D-104g row's `Status` field is `Open` (documents the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence — the `thick_bridges` branch in the helper is the canonical realization site; the `bridge_flow` ratio itself is correctly modelable per-vertex, so this is a limited divergence, not a gap).
- The D-104b/c/d/e rows' `Status` fields flip to `Closed — 2026-07-09: packet 149` at packet close.
- None — change is reversible via existing config defaults (the 4 D1 keys default to `false`; `min_width_top_surface` defaults to the OrcaSlicer-resolved value; `alternate_extra_wall` defaults to `false`; `bridge_flow` defaults to `1.0`; `thick_bridges` defaults to `false`). The bridge flow factor is gated on `is_bridge` being set, which only happens when `region.bridge_areas()` is non-empty. No behavior locks introduced beyond the test suite.

## Risks and Tradeoffs

- **Risk:** the bridge flow factor test fixture (AC-4) needs `region.bridge_areas()` to be non-empty. The audit's test reads the `arachne-perimeters` source via `include_str!`; it does not actually exercise a bridge-area fixture. The implementer must add a unit test that does (`arachne-perimeters/tests/bridge_flow_factor_tdd.rs`). **Mitigation:** the unit test scaffold is straightforward — use the existing `SliceRegionViewBuilder` test-support to set `bridge_areas` to a small polygon, run the perimeter module, and assert the wall loop's `flow_factor`.
- **Risk:** the D3 (alternating_extra_wall) implementation requires a wall count parameter. The classic and arachne paths have different wall-count sources (classic: explicit `wall_count` config; arachne: derived from the beading-strategy stack). The implementer must be careful to apply the alternating logic to the FINAL wall count, not the base config. **Mitigation:** for arachne, the wall count is determined by `run_arachne_pipeline`'s output (one wall per `inset_idx`); the alternating logic adds a +1 on odd layers, which means the `inset_idx` range grows by 1 on odd layers. The unit test verifies the resulting wall count.
- **Risk:** the 6 new deviation log rows change the table's row count. The `cargo xtask check-deviations` tool (per `docs/14_deviation_audit_history.md` "non-authoritative views") regenerates `docs/07_implementation_status.md`'s Open Deviation Map. **Mitigation:** the implementer runs `cargo xtask check-deviations` after the deviation log edit; the regenerated file is committed in the same packet.
- **Risk:** the new manifest keys may collide with packet 148's keys (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`). **Mitigation:** the names are distinct (`detect_overhang_wall` vs `precise_outer_wall`; `seam_candidate_angle_threshold_deg` is unique). The implementer should grep for each new key against the existing manifest to confirm no collision.

## Context Cost Estimate

- Aggregate (sum across all steps): M (5 steps × S/S/S/M/S).
- Largest single step: M (Step 4: D3 + D4 + `bridging_flow()` helper; touches both perimeter modules + `slicer-core/src/flow.rs` + 2 new test files).
- Highest-risk dispatch: the `min_width_top_surface` default verification — a poorly-shaped dispatch (asking for the full `ORCA_CONFIG_REFERENCE.md` file) blows budget. The dispatch contract must be: "Read `docs/ORCA_CONFIG_REFERENCE.md` lines 135, 146, 150, 161, 165-168, 178; return SNIPPETS (verbatim, ≤ 30 lines) of the canonical OrcaSlicer defaults for the new keys, and confirm whether `min_width_top_surface` is a percent or mm (verified 300%, coFloatOrPercent), and the resolved mm value for a 0.4mm nozzle."

## Open Questions

- None. All forward-flagged open questions are resolved by this refined design. The `bridging_flow()` formula's exact behavior (real OrcaSlicer ratio vs PnP's per-vertex `flow_factor` model) is documented as D-104g; the D3 mechanism (max_bead_count bump) is documented with the canonical OrcaSlicer precedent.
