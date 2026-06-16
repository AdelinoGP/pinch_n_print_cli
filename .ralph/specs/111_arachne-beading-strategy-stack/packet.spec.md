---
status: draft
packet: 111_arachne-beading-strategy-stack
task_ids:
  - T-210
  - T-211
  - T-212
  - T-213
  - T-214
  - T-215
  - T-215b
  - T-216
  - T-218
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 111_arachne-beading-strategy-stack

## Goal

Port the OrcaSlicer Arachne BeadingStrategy stack into `slicer_core::beading`: define the `BeadingStrategy` trait (T-210), port all 5 strategies — `Distributed` (Gaussian-weighted width distribution), `Redistribute` (preserve outer-wall width consistency), `Widening` (thin-feature single-wall regime), `OuterWallInset` (outer-wall toolpath offset decorator), `Limited` (max-bead-count cap with internal 0-width sentinel insertion) — implement the T-215b strip-pass that drops zero-width beads from output before `WallLoop` assembly (closing D-9), wire the `BeadingStrategyFactory` that composes the stack in the canonical order `Distributed → Redistribute → Widening → OuterWallInset → Limited`, and register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` and the `arachne-perimeters.toml` manifest.

## Scope Boundaries

Touches `crates/slicer-core/src/beading/` (NEW sub-module with `mod.rs` + 5 strategy files + `factory.rs`), `docs/15_config_keys_reference.md` (11 new key entries), and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (11 new `[config.schema.*]` entries). NO wire-up into `arachne-perimeters::run_perimeters` (that's P112's T-230). NO consumption of the SkeletalTrapezoidationGraph from P110 — this packet ships a pure-data BeadingStrategy stack that takes thickness inputs and returns `Beading` outputs; P112 connects per-edge bead-count assignments from the SKT graph to this stack.

## Prerequisites and Blockers

- Depends on:
  - **P110** — provides the slicer-core infrastructure (`mod.rs` registration pattern); the `arachne-perimeters` module skeleton from T-205 receives the 11 config keys.
  - **P109** — M1 verification closed so M1 regressions don't drown M2 noise.
- Unblocks:
  - **P112** — `BeadingStrategyFactory::create_stack` is the entry point P112's T-220+ (centrality + bead-count) will call to derive per-edge `Beading` outputs.
- Activation blockers: D-9 closure for T-215b (zero-width sentinel strip-pass) was decided pre-packet — sentinels stay INTERNAL to the LimitedBeadingStrategy decorator, stripped before external output. The packet implements the closed decision; no open D-9 question remains.

## Acceptance Criteria

- **AC-1. Given** the new `BeadingStrategy` trait in `crates/slicer-core/src/beading/mod.rs`, **when** the trait is inspected, **then** it carries (a) `fn compute(&self, thickness: f64, bead_count: usize) -> Beading`, (b) `fn optimal_bead_count(&self, thickness: f64) -> usize`, (c) `fn get_transition_thickness(&self, lower_bead_count: usize) -> f64`, (d) `fn optimal_thickness(&self, bead_count: usize) -> f64`, all returning slicer-unit-scaled values. The `Beading` struct carries `total_thickness: f64`, `bead_widths: Vec<f64>`, `toolpath_locations: Vec<f64>`, `left_over: f64`. | `rg -q 'fn compute.*thickness.*bead_count.*-> Beading' crates/slicer-core/src/beading/mod.rs && rg -q 'fn optimal_bead_count' crates/slicer-core/src/beading/mod.rs && rg -q 'fn get_transition_thickness' crates/slicer-core/src/beading/mod.rs`
- **AC-2. Given** `DistributedBeadingStrategy` in `beading/distributed.rs` (Gaussian-weighted width distribution), **when** called against 10 recorded thickness inputs from OrcaSlicer's reference table, **then** each output's `bead_widths` and `toolpath_locations` match within 0.0001 mm tolerance. | `cargo test -p slicer-core distributed_beading_strategy_orca_table -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** `RedistributeBeadingStrategy` (preserve outer-wall width consistency — decorator over `Distributed`), **when** called against the outer-consistent fixture, **then** the outermost `bead_widths[0]` and `bead_widths[-1]` match `optimal_width` exactly (within 0.0001 mm); inner beads absorb the residual thickness. | `cargo test -p slicer-core redistribute_outer_consistent -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `WideningBeadingStrategy` (thin-feature single-wall regime — decorator), **when** called against a thin-wedge fixture with thickness < `min_input_width`, **then** the output carries a single bead at `bead_widths = [min_bead_width]` (NOT empty) and `total_thickness == thickness` exactly. | `cargo test -p slicer-core widening_below_min_input_width -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `OuterWallInsetBeadingStrategy` (outer-wall toolpath offset decorator), **when** called with `outer_wall_offset > 0`, **then** ONLY the outer wall's `toolpath_locations[0]` and `toolpath_locations[-1]` are offset inward by `outer_wall_offset`; inner toolpath locations are unchanged. | `cargo test -p slicer-core outer_wall_inset_offset_outer_only -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `LimitedBeadingStrategy` (max-bead-count cap with 0-width sentinel insertion — decorator), **when** the inner strategy returns `bead_count > max_bead_count`, **then** the limited output carries `bead_widths.len() == max_bead_count + 2 * sentinel_count` with sentinel `bead_widths == 0.0` at the cap boundaries; bead-count math via `optimal_bead_count` returns capped value end-to-end. | `cargo test -p slicer-core limited_inserts_sentinels_at_cap -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the T-215b strip-pass at `LimitedBeadingStrategy::compute_and_strip` (per D-9), **when** the strategy is called via the `compute_and_strip` entry point that downstream code (P112) will use, **then** the returned `Beading` carries NO zero-width entries; `bead_widths.iter().all(|&w| w > 0.0)` is true; `toolpath_locations.len() == bead_widths.len()` invariant holds. The internal `compute` (without strip) still returns sentinels for invariant testing. | `cargo test -p slicer-core limited_compute_and_strip_no_zero_widths -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** `BeadingStrategyFactory::create_stack(params)` in `beading/factory.rs`, **when** inspected and called, **then** (a) the returned trait object's runtime type composition is verifiably `Limited<OuterWallInset<Widening<Redistribute<Distributed>>>>` in THAT order (asserted via a downcast or a recorded type-name string), (b) calling `compute(thickness, n)` on the stack against a recorded multi-stage fixture matches OrcaSlicer's `BeadingStrategyFactory::create_strategy` output within 0.0001 mm. | `cargo test -p slicer-core factory_stack_composition_order -- --nocapture 2>&1 | tee target/test-output.log && cargo test -p slicer-core factory_matches_orca_reference -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-9. Given** the 11 `m_params.*` config keys (T-218: `min_feature_size`, `min_bead_width`, `wall_transition_filter_deviation`, `wall_transition_length`, `wall_transition_angle`, `wall_distribution_count`, `min_length_factor`, `initial_layer_min_bead_width`, `outer_wall_offset`, `max_bead_count`, `optimal_width`), **when** `docs/15_config_keys_reference.md` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` are inspected, **then** each key (a) appears in the docs reference with a 1-line description + default value + units, (b) appears as a `[config.schema.<key>]` block in the manifest with `type`, `default`, and `description` fields. | `for k in min_feature_size min_bead_width wall_transition_filter_deviation wall_transition_length wall_transition_angle wall_distribution_count min_length_factor initial_layer_min_bead_width outer_wall_offset max_bead_count optimal_width; do rg -q "$k" docs/15_config_keys_reference.md && rg -q "config.schema.$k" modules/core-modules/arachne-perimeters/arachne-perimeters.toml || { echo "MISSING $k"; exit 1; }; done`

## Negative Test Cases

- **AC-N1. Given** a `Beading` output where `toolpath_locations.len() != bead_widths.len()`, **when** the invariant assertion runs in any strategy's `compute` in debug builds, **then** the function panics via `debug_assert_eq!` with a message naming both lengths (per design.md § Architecture Constraints). Release builds silently accept the malformed `Beading`; downstream validation is the caller's responsibility. | `cargo test -p slicer-core beading_invariant_locations_len_eq_widths_len -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the `Limited` strategy WITHOUT the strip-pass (raw `compute` not `compute_and_strip`), **when** the output is inspected, **then** zero-width sentinels ARE present (the underlying `compute` still emits them for invariant testing); only `compute_and_strip` drops them. This negative test guards against accidentally folding the strip into `compute` and losing the invariant test surface. | `cargo test -p slicer-core limited_raw_compute_retains_sentinels -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core beading 2>&1 | tee target/test-output.log` (T-210..T-216 unit suites + T-215b strip-pass)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 11 (T-210..T-218). Range-read those rows.
- `docs/15_config_keys_reference.md` — existing config-key entry format.
- `docs/03_wit_and_manifest.md` — `[config.schema.*]` block format for T-218 manifest entries.
- `docs/01_system_architecture.md` — register the new `beading` sub-module.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — 11 new key entries — `for k in min_feature_size min_bead_width wall_transition_filter_deviation wall_transition_length wall_transition_angle wall_distribution_count min_length_factor initial_layer_min_bead_width outer_wall_offset max_bead_count optimal_width; do rg -q "$k" docs/15_config_keys_reference.md || { echo "MISSING $k"; exit 1; }; done`
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — 11 new schema blocks — same loop as AC-9 against the manifest path.
- `docs/01_system_architecture.md` — register `beading` sub-module entry — `rg -q 'beading' docs/01_system_architecture.md`.
- `docs/DEVIATION_LOG.md` — record D-9 closure with rationale (sentinels stay internal; strip before external output via `compute_and_strip`) — `rg -qi 'D-9.*closed' docs/DEVIATION_LOG.md`.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — flip T-210/T-211/T-212/T-213/T-214/T-215/T-215b/T-216/T-218 rows to DONE — `rg -q 'T-210.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h` — base interface (T-210). ONE LOCATIONS dispatch (≤ 10 entries): virtual method signatures + `Beading` struct fields.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp` — Gaussian distribution math (T-211). ONE SUMMARY (≤ 150 words): the `compute` algorithm + Gaussian decay constant.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/RedistributeBeadingStrategy.cpp` — outer-wall preservation (T-212). ONE SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/WideningBeadingStrategy.cpp` — thin-feature single-wall (T-213). ONE SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp` — outer-wall offset (T-214). ONE SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp` — cap + sentinels (T-215). ONE SUMMARY (≤ 150 words): the sentinel-insertion rule + how `optimal_bead_count` is capped.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategyFactory.cpp` — stack composition (T-216). ONE LOCATIONS dispatch (≤ 10 entries): `create_strategy` body showing the wrapping order.
- 11 `m_params.*` defaults — search `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` via ONE LOCATIONS dispatch (≤ 20 entries) for each key's default + unit + description.

## Context Discipline Note

<!-- snippet: context-discipline -->
This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
