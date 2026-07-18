---
status: implemented
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

Port the OrcaSlicer Arachne BeadingStrategy stack into `slicer_core::beading`: define the `BeadingStrategy` trait (T-210), port all 5 strategies — `Distributed` (Gaussian-weighted width distribution), `Redistribute` (preserve outer-wall width consistency), `Widening` (thin-feature single-wall regime), `OuterWallInset` (outer-wall toolpath offset decorator), `Limited` (max-bead-count cap with internal 0-width sentinel insertion) — implement the T-215b strip-pass that drops zero-width beads from output before `WallLoop` assembly (per D-9 in the roadmap — the decision is already made; this packet implements it and registers the rationale as `D-111-ARACHNE-SENTINEL-STRIP` in `docs/DEVIATION_LOG.md`), wire the `BeadingStrategyFactory` that composes the stack in the canonical order `Distributed → Redistribute → Widening → OuterWallInset → Limited`, and register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` and the `arachne-perimeters.toml` manifest.

## Scope Boundaries

Touches `crates/slicer-core/src/beading/` (NEW sub-module with `mod.rs` + 5 strategy files + `factory.rs`), `docs/15_config_keys_reference.md` (11 new key entries), and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (11 new `[config.schema.*]` entries). NO wire-up into `arachne-perimeters::run_perimeters` (that's P112's T-230). NO consumption of the SkeletalTrapezoidationGraph from P110 — this packet ships a pure-data BeadingStrategy stack that takes thickness inputs and returns `Beading` outputs; P112 connects per-edge bead-count assignments from the SKT graph to this stack.

## Prerequisites and Blockers

- Depends on:
  - **P110** (`implemented` — sibling M2 packet) — FORWARD-DEP, now resolved. P110 provides the infrastructure pattern (Voronoi + SKT foundations) and, via T-205, CREATED the `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` manifest that AC-9's 11 config-schema blocks are written into. P110's beading-unrelated symbols (`voronoi.rs`, `skeletal_trapezoidation/`, `arachne/preprocess.rs`) are NOT consumed by this packet. **Split dependency:** the pure-data beading code (AC-1..AC-8) was written independently of P110, and **AC-9's forward-dep on P110's T-205 skeleton manifest** is satisfied — P110 is now `implemented`, so AC-9 was closeable against the real manifest.
  - **P105** (`implemented`) — the `slicer_core::flow` module now exists (`crates/slicer-core/src/flow.rs`, carrying `line_width_to_spacing`), but `to_slicer_units` specifically was never added; `flow_correction` remains in `lib.rs`. P111 uses inline `/100` division and calls NO `flow` symbol, so this is moot either way.
  - **P109** (`implemented`) — M1 verification closed so M1 regressions don't drown M2 noise.
- Unblocks:
  - **P112** — `BeadingStrategyFactory::create_stack` is the entry point P112's T-220+ (centrality + bead-count) will call to derive per-edge `Beading` outputs.
- Activation blockers: The D-9 decision (zero-width sentinel strip-pass) was made pre-packet — the decision lives in the roadmap (D-9 is a roadmap ID, not a `DEVIATION_LOG.md` entry). T-215b implements that decision and registers `D-111-ARACHNE-SENTINEL-STRIP` in the log. No open questions remain.

### Accepted Forward-DEPs

| Symbol | Producing packet | Names/shapes reconciled? |
| --- | --- | --- |
| `crates/slicer-core/src/voronoi.rs` (`voronoi_from_segments`, `HalfEdgeGraph`, `VoronoiError`, `Segment`) | implemented P110 | NOT consumed — reference pattern only ✓ |
| `crates/slicer-core/src/skeletal_trapezoidation/` (`SkeletalTrapezoidationGraph`) | implemented P110 | NOT consumed ✓ |
| `crates/slicer-core/src/arachne/preprocess.rs` (`preprocess_input_outline`) | implemented P110 | NOT consumed ✓ |
| `slicer_core::flow` (module exists post-P105; `to_slicer_units` never added) | implemented P105 | NOT consumed — inline `/100` used instead ✓ |

## Acceptance Criteria

- **AC-1. Given** the new `BeadingStrategy` trait in `crates/slicer-core/src/beading/mod.rs`, **when** the trait is inspected, **then** it carries (a) `fn compute(&self, thickness: f64, bead_count: usize) -> Beading`, (b) `fn optimal_bead_count(&self, thickness: f64) -> usize`, (c) `fn get_transition_thickness(&self, lower_bead_count: usize) -> f64`, (d) `fn optimal_thickness(&self, bead_count: usize) -> f64`, all returning slicer-unit-scaled values. The `Beading` struct carries `total_thickness: f64`, `bead_widths: Vec<f64>`, `toolpath_locations: Vec<f64>`, `left_over: f64`. | `rg -q 'fn compute.*thickness.*bead_count.*-> Beading' crates/slicer-core/src/beading/mod.rs && rg -q 'fn optimal_bead_count' crates/slicer-core/src/beading/mod.rs && rg -q 'fn get_transition_thickness' crates/slicer-core/src/beading/mod.rs`
- **AC-2. Given** `DistributedBeadingStrategy` in `beading/distributed.rs` (Gaussian-weighted width distribution), **when** called against 10 recorded thickness inputs from OrcaSlicer's reference table, **then** each output's `bead_widths` and `toolpath_locations` match within 0.0001 mm tolerance. | `cargo test -p slicer-core distributed_beading_strategy_orca_table -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** `RedistributeBeadingStrategy` (preserve outer-wall width consistency — decorator over `Distributed`), **when** called against the outer-consistent fixture, **then** the outermost `bead_widths[0]` and `bead_widths[-1]` match `optimal_width` exactly (within 0.0001 mm); inner beads absorb the residual thickness. | `cargo test -p slicer-core redistribute_outer_consistent -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4 (corrected post-closure — see closure-log.md §2a). Given** `WideningBeadingStrategy` (thin-feature single-wall regime — decorator, ported to exact OrcaSlicer parity: `WideningBeadingStrategy.cpp:48-91`), **when** `compute(thickness, bead_count)` is called against a wrapped parent with `optimal_width`, `min_input_width`, `min_output_width` configured, **then** the result is one of three regimes: (a) `thickness >= optimal_width` → the result equals the parent's own `compute(thickness, bead_count)` output exactly (full delegation); (b) `min_input_width <= thickness < optimal_width` → the output carries exactly ONE bead at `bead_widths = [thickness.max(min_output_width)]` with `left_over == 0.0` exactly (bead_count is ignored in this branch); (c) `thickness < min_input_width` → `bead_widths` and `toolpath_locations` are EMPTY and `left_over == thickness` exactly (the entire thickness is unprinted — this replaces the packet's original AC-4, which incorrectly required a non-empty forced bead here; upstream's actual behavior is to emit nothing below `min_input_width`). | `cargo test -p slicer-core widening_below_min_input_width -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5 (corrected post-closure — see closure-log.md §2a). Given** `OuterWallInsetBeadingStrategy` (outer-wall toolpath offset decorator, ported to exact OrcaSlicer parity: `OuterWallInsetBeadingStrategy.cpp:69-92`), **when** called with a nonzero `outer_wall_offset` against a `Beading` with 2 or more non-zero-width beads, **then** ONLY the outer wall's `toolpath_locations[0]` is offset inward by `outer_wall_offset`, clamped to `thickness / 2.0` (the region's centerline); `toolpath_locations[1..]` — including the opposite/innermost end `toolpath_locations[last]` — are left COMPLETELY UNCHANGED (this replaces the packet's original AC-5, which incorrectly required both ends to shift; upstream is single-sided). Bead widths, `left_over`, and `total_thickness` are untouched. | `cargo test -p slicer-core outer_wall_inset_offset_outer_only -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `LimitedBeadingStrategy` (max-bead-count cap with 0-width sentinel insertion — decorator), **when** the inner strategy returns `bead_count > max_bead_count`, **then** the limited output carries `bead_widths.len() == max_bead_count + 2 * sentinel_count` with sentinel `bead_widths == 0.0` at the cap boundaries; bead-count math via `optimal_bead_count` returns capped value end-to-end. | `cargo test -p slicer-core limited_inserts_sentinels_at_cap -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the T-215b strip-pass at `LimitedBeadingStrategy::compute_and_strip` (decision recorded as `D-111-ARACHNE-SENTINEL-STRIP` per the roadmap's D-9 direction), **when** the strategy is called via the `compute_and_strip` entry point that downstream code (P112) will use, **then** the returned `Beading` carries NO zero-width entries; `bead_widths.iter().all(|&w| w > 0.0)` is true; `toolpath_locations.len() == bead_widths.len()` invariant holds. The internal `compute` (without strip) still returns sentinels for invariant testing. | `cargo test -p slicer-core limited_compute_and_strip_no_zero_widths -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-8 (clarified post-closure — see closure-log.md §2a). Given** `BeadingStrategyFactory::create_stack(params)` in `beading/factory.rs`, **when** inspected and called, **then** (a) WHEN `params.outer_wall_offset != 0.0`, the returned trait object's runtime type composition is verifiably `Limited<OuterWallInset<Widening<Redistribute<Distributed>>>>` in THAT order (asserted via `type_chain()`); WHEN `params.outer_wall_offset == 0.0`, `OuterWallInsetBeadingStrategy` is correctly ABSENT from the chain (`Limited<Widening<Redistribute<Distributed>>>`), matching upstream `BeadingStrategyFactory.cpp:50-97`'s `if (outer_wall_offset != 0)` conditional wrap — both cases are covered by `factory_stack_composition_order` (nonzero offset) and `factory_stack_composition_order_skips_outer_wall_inset_when_offset_zero` (zero offset); (b) calling `compute(thickness, n)` on the stack against a recorded multi-stage fixture matches the hand-derived, upstream-faithful reference output within 0.0001 mm. | `cargo test -p slicer-core factory_stack_composition_order -- --nocapture 2>&1 | tee target/test-output.log && cargo test -p slicer-core factory_stack_composition_order_skips_outer_wall_inset_when_offset_zero -- --nocapture 2>&1 | tee target/test-output.log && cargo test -p slicer-core factory_matches_orca_reference -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-9 (extended post-closure — see closure-log.md §2a). Given** the original 11 `m_params.*` config keys (T-218: `min_feature_size`, `min_bead_width`, `wall_transition_filter_deviation`, `wall_transition_length`, `wall_transition_angle`, `wall_distribution_count`, `min_length_factor`, `initial_layer_min_bead_width`, `outer_wall_offset`, `max_bead_count`, `optimal_width`) PLUS 2 new keys registered while closing the packet's OrcaSlicer-parity scope gaps (`detect_thin_wall` — gates whether `WideningBeadingStrategy` is wrapped at all, matching upstream's `print_thin_walls`/`PrintConfig.cpp`'s `detect_thin_wall` boolean; `preferred_bead_width_outer` — the unconditional outer-wall target width `RedistributeBeadingStrategy` uses, and the conditionally-selected base width for `Distributed`/`Widening` when `max_bead_count <= 2`), **when** `docs/15_config_keys_reference.md` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` are inspected, **then** each of the 13 keys (a) appears in the docs reference with a 1-line description + default value + units, (b) appears as a `[config.schema.<key>]` block in the manifest with `type`, `default`, and `description` fields, AND (c) `BeadingStrategyFactory::create_stack` actually consumes `detect_thin_wall`/`preferred_bead_width_outer` in its composition logic (not registration-only, unlike some of the other 11) — verified by `factory_stack_composition_order_widening_only_when_thin_walls_true` and `factory_max_bead_count_le_2_selects_preferred_bead_width_outer`. | `for k in min_feature_size min_bead_width wall_transition_filter_deviation wall_transition_length wall_transition_angle wall_distribution_count min_length_factor initial_layer_min_bead_width outer_wall_offset max_bead_count optimal_width detect_thin_wall preferred_bead_width_outer; do rg -q "$k" docs/15_config_keys_reference.md && rg -q "config.schema.$k" modules/core-modules/arachne-perimeters/arachne-perimeters.toml || { echo "MISSING $k"; exit 1; }; done && cargo test -p slicer-core --test beading_factory -- --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a `Beading` output where `toolpath_locations.len() != bead_widths.len()`, **when** the invariant assertion runs in any strategy's `compute` in debug builds, **then** the function panics via `debug_assert_eq!` with a message naming both lengths (per design.md § Architecture Constraints). Release builds silently accept the malformed `Beading`; downstream validation is the caller's responsibility. | `cargo test -p slicer-core beading_invariant_locations_len_eq_widths_len -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the `Limited` strategy WITHOUT the strip-pass (raw `compute` not `compute_and_strip`), **when** the output is inspected, **then** zero-width sentinels ARE present (the underlying `compute` still emits them for invariant testing); only `compute_and_strip` drops them. This negative test guards against accidentally folding the strip into `compute` and losing the invariant test surface. | `cargo test -p slicer-core limited_raw_compute_retains_sentinels -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --test beading_distributed --test beading_redistribute --test beading_widening --test beading_outer_wall_inset --test beading_limited --test beading_factory 2>&1 | tee target/test-output.log` (T-210..T-216 unit suites + T-215b strip-pass)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 11 (T-210..T-218). Range-read those rows.
- `docs/15_config_keys_reference.md` — existing config-key entry format.
- `docs/03_wit_and_manifest.md` — `[config.schema.*]` block format for T-218 manifest entries.
- `docs/01_system_architecture.md` — register the new `beading` sub-module.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — 11 new key entries — `for k in min_feature_size min_bead_width wall_transition_filter_deviation wall_transition_length wall_transition_angle wall_distribution_count min_length_factor initial_layer_min_bead_width outer_wall_offset max_bead_count optimal_width; do rg -q "$k" docs/15_config_keys_reference.md || { echo "MISSING $k"; exit 1; }; done`
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — 11 new schema blocks — same loop as AC-9 against the manifest path.
- `docs/01_system_architecture.md` — register `beading` sub-module entry — `rg -q 'beading' docs/01_system_architecture.md`.
- `docs/DEVIATION_LOG.md` — add a NEW `D-111-ARACHNE-SENTINEL-STRIP` entry recording the strip-pass rationale (sentinels stay internal to `LimitedBeadingStrategy`; stripped before external output via `compute_and_strip`). Note: D-9 is a roadmap-level ID that lives in `docs/specs/perimeter-modules-orca-parity-roadmap.md`, NOT in `DEVIATION_LOG.md` — the AC greps the log for the new `D-111-ARACHNE-SENTINEL-STRIP` entry, not for `D-9`. Verification: `rg -q 'D-111-ARACHNE-SENTINEL-STRIP' docs/DEVIATION_LOG.md`.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — flip T-210/T-211/T-212/T-213/T-214/T-215/T-215b/T-216/T-218 rows to DONE — `rg -q 'T-210.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md`.

## Post-Closure Amendment: OrcaSlicer Parity Fix

After first closure (status flipped to `implemented`), a follow-up audit
found that `RedistributeBeadingStrategy`, `WideningBeadingStrategy`,
`OuterWallInsetBeadingStrategy`, and `BeadingStrategyFactory`'s conditional
wrapping had all shipped as *documented, design.md-locked simplifications*
rather than faithful upstream ports. Per explicit user direction ("OrcaSlicer
parity is the current main goal, deferring features is not acceptable, fix
all deviations now"), all four were rewritten against verbatim OrcaSlicer
C++ source to match upstream exactly. AC-4, AC-5, and AC-8 above reflect the
corrected, parity-faithful behavior.

A first pass left two residual scope gaps (Widening's unconditional
wrapping; the merged outer/inner bead-width config key) undone, since
closing them required registering NEW config keys beyond this packet's
original T-218 11-key scope. Per explicit follow-up user direction ("close
both gaps now, do not write them off"), **both are now closed**: two new
config keys were registered — `detect_thin_wall` (bool, default `false`,
matches OrcaSlicer's real `PrintConfig.cpp` option of the same name exactly)
gates whether `WideningBeadingStrategy` is wrapped at all, and
`preferred_bead_width_outer` (float, default `4000`) supplies the
unconditional outer-wall target width `RedistributeBeadingStrategy` uses and
the conditionally-selected base width (`max_bead_count <= 2`) `Distributed`/
`Widening` use instead of `optimal_width`. AC-9 above now covers all 13
keys. See `closure-log.md` §2/§2a/§2b for the full before/after record,
exact C++ citations, and the two bugs (unrelated to parity) also fixed in
this pass: `BeadingFactoryParams::default()`'s stale values (diverged from
this packet's own registered config defaults) and `BeadingFactoryParams`'s
missing `serde` derive (now a real dependency, not a dev-only workaround).
No scope gaps remain.

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
