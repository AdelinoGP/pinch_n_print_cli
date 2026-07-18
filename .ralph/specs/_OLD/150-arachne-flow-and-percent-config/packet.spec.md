---
status: implemented
packet: 150-arachne-flow-and-percent-config
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 150-arachne-flow-and-percent-config

## Goal

Wire OrcaSlicer flow physics into the Arachne wall generator — feed Flow
**spacing** (not raw width) into bead placement, give `thick_bridges` a real
round-cross-section flow factor, and add a `percent` / `float_or_percent`
config type so nozzle-relative keys rescale — flipping gap tests G4, G5, G6.

## Scope Boundaries

Adds a percent-relative config type spanning the `config.wit` value variant,
the `slicer-macros` adapter, `ConfigValue`, and a read-time `get_abs_value`
resolver on `ConfigView`; wires `slicer_core::flow::line_width_to_spacing` into
the widths `arachne-perimeters` feeds the beading pipeline; and replaces the
`thick_bridges` 1.0 stub in `slicer_core::flow::bridging_flow` with the
OrcaSlicer round-section factor. Also registers `layer_height` and
`nozzle_diameter` on `arachne-perimeters`, and registers `nozzle_diameter` on
`classic-perimeters` (which reads the key today but never receives it). The
percent type is a WIT contract change on `config.wit` ⇒ all guests rebuild; the
`common.wit` boundary is untouched (that is packet 152). No winding, wall-count,
top-surface, or scheduler-dispatch behavior — those are packets 151/152.

## Prerequisites and Blockers

- Depends on: none (first of the three Arachne-parity-fix packets).
- Unblocks: packet 151 (`overhang_reverse_threshold` declared `float_or_percent`)
  and packet 152 (`min_width_top_surface` resolved via `get_abs_value`).
- Activation blockers: none.
  all frontmatter is `status: implemented`).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID.

- **AC-1. Given** the manifest keys `min_width_top_surface`, `min_feature_size`,
  and `wall_transition_length` in `arachne-perimeters.toml`, **when** each key's
  `type` is read, **then** every one is `percent` or `float_or_percent` (not
  `float`) with its canonical OrcaSlicer default (300% / 25% / 100%). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_percent_config_type_for_arachne_keys --exact`
- **AC-2. Given** a `percent`-typed key (e.g. `min_feature_size` = 25%) resolved
  against a base of 0.4 mm nozzle diameter, **when** the module calls the new
  `ConfigView::get_abs_value("min_feature_size", base)`-style accessor, **then**
  it returns `0.1 mm` (25% × 0.4); a `float_or_percent` literal (`1.2`) returns
  `1.2` unchanged regardless of base. |
  `cargo test -p slicer-ir --lib config -- percent --nocapture`
- **AC-3. Given** `wall_count=2`, `optimal_width=0.4 mm`, `layer_height=0.2 mm`
  on a 10 mm square, **when** `arachne-perimeters` emits walls, **then** the
  perimeter_index 0 and 1 centerlines sit one Flow *spacing* apart —
  `0.4 − 0.2·(1−π/4) ≈ 0.3571 mm`, within 0.02 mm — not one raw width (0.4 mm)
  apart. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact`
- **AC-4. Given** `thick_bridges=true`, `bridge_flow=1.0`, a 0.4 mm bead at
  0.2 mm layer height over a bridge region, **when** `arachne-perimeters` emits
  bridge vertices, **then** at least one `is_bridge` vertex carries a
  `flow_factor` differing from 1.0 by > 0.05 (≈1.57 round-section factor,
  `π·dmr²/(4·w·h)`), not the removed 1.0 stub. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one --exact`
- **AC-5. Given** `classic-perimeters.toml` now registering `nozzle_diameter`,
  **when** a config supplies `nozzle_diameter` distinct from the inner-wall line
  width, **then** `classic-perimeters` reads the supplied value instead of
  silently falling back to `inner_wall_line_width` (dead-read fixed). |
  `cargo test -p classic-perimeters --lib -- nozzle_diameter --nocapture`
- **AC-6 (regression lock). Given** the 14 green locks in `arachne_parity.rs`,
  **when** the flow-spacing change lands, **then** all 14 still pass (the
  `precise_outer_wall` lock asserts a *relative* min-x delta and must survive;
  self-captured baselines re-verified). |
  `cargo test -p slicer-runtime --test arachne_parity`

## Negative Test Cases

- **AC-N1. Given** a manifest declaring a config key `type = "percent"` with a
  malformed default (e.g. `"abc%"` or a bare number with no base-resolution
  path), **when** the manifest is parsed by `parse_config_field_entry`, **then**
  parsing rejects it with a diagnostic naming the key and the expected
  `<number>%` or literal form — it does not silently coerce to 0. |
  `cargo test -p slicer-scheduler --test scheduler_contract -- config_percent_type --nocapture`
- **AC-N2. Given** a `percent`-typed value resolved against a non-positive base
  (base ≤ 0), **when** `get_abs_value` is called, **then** it returns `None` (or
  the documented zero-base fallback) rather than producing a NaN/negative
  absolute value. |
  `cargo test -p slicer-ir --lib config -- percent_zero_base --nocapture`

## Verification

Gate commands only — full matrix in `requirements.md` §Verification Commands.

- `cargo xtask build-guests --check` (must be clean — this packet edits
  `config.wit`, `slicer-macros`, `slicer-ir`, and module sources)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one arachne_parity_pipeline_percent_config_type_for_arachne_keys`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — module manifest TOML schema + config type
  set. Delegate a SUMMARY of the config-type/validation section only.
- `docs/15_config_keys_reference.md` — Arachne key provenance; delegate the
  `min_feature_size` / `min_width_top_surface` / `wall_transition_length`
  entries only (file is large).
- `docs/08_coordinate_system.md` — read the mm↔unit rule for the spacing
  conversion (short; load directly).
- `docs/DEVIATION_LOG.md` — D-105, D-104g, D-104h close in this packet; load
  the three entries only.

## Doc Impact Statement (Required)

This packet adds a manifest config type and changes config contracts, so `none`
is not eligible. Sections added/modified:

- `docs/03_wit_and_manifest.md` §"Config type set" — add `percent` /
  `float_or_percent` — `rg -q 'float_or_percent' docs/03_wit_and_manifest.md`
- `docs/15_config_keys_reference.md` §"Arachne beading strategy stack" — retype
  the three keys + add `layer_height`/`nozzle_diameter` —
  `rg -q 'nozzle_diameter' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — mark D-105, D-104g, D-104h closed —
  `rg -q 'D-105.*(CLOSED|closed)' docs/DEVIATION_LOG.md`
- `docs/18_arachne_parity_audit.md` §"Gap summary table" — mark G4/G5/G6 closed —
  `rg -q 'G4.*closed' docs/18_arachne_parity_audit.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Flow.hpp` — `Flow::spacing()` (`:67`) and
  `Flow::bridging_flow` round-section width==height==dmr (`:106`); borrow the
  spacing formula and the thick-bridge cross-section.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `bridging_flow` thread-diameter
  (`dmr`) derivation from nozzle diameter and bridge flow ratio; borrow the exact
  `dmr` formula (the arbiter of AC-4's ≈1.57 factor).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2129,2172-2173` —
  `bead_width_0 = ext_perimeter_spacing`; `WallToolPaths` receives
  `perimeter_spacing = perimeter_flow.scaled_spacing()` (confirms spacing, not
  width, feeds bead placement).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1498-1511,7169-7178,7217-7226`
  — the `coFloatOrPercent`/`coPercent` defaults for the three retyped keys.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:31-50,135` — `overhang_flow
  = bridging_flow(frPerimeter, thick_bridges)` call context (deliberately NOT
  porting the LayerRegion plumbing, only the flow factor).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Packet Close Record

**Final full workspace run:** 2602 passed / 9 failed. All 9 failures are
expected-red or pre-existing — no regressions attributable to packet 150:

- 7 tests in `crates/slicer-runtime/tests/arachne_parity_gaps.rs` scoped to
  packets 151/152 (not this packet): `arachne_parity_pipeline_wall_direction_controls_winding`,
  `arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall`,
  `arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top`,
  `arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls`,
  `arachne_parity_pipeline_spiral_vase_forces_classic_generator`,
  `arachne_parity_pipeline_wall_max_resolution_deviation_registered`,
  `arachne_parity_arachne_path_remove_small_lines_top_layer_exception`.
- 1 pre-existing: `arachne_parity_pipeline_concentric_infill_uses_arachne`
  (D-104f-CONCENTRIC-INFILL-NO-ARACHNE, `docs/DEVIATION_LOG.md`).
- 1 pre-existing: `multi_tool_triangle_perimeter_parity`
  (`crates/slicer-runtime/tests/integration/perimeter_parity.rs`,
  actual=4 expected=11 points) — confirmed pre-existing via `git stash` A/B
  (reproduces identically on the pre-packet-150 baseline tree), NOT
  introduced by this packet. Registered as D-150-MULTI-TOOL-TRIANGLE-PREEXISTING
  in `docs/DEVIATION_LOG.md`, flagged for a separate bisection.

**Green at close:** all 14 `arachne_parity.rs` regression locks; the G4/G5/G6
gap tests (`arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width`,
`arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one`,
`arachne_parity_pipeline_percent_config_type_for_arachne_keys`); AC-1..AC-6 and
AC-N1/AC-N2 all green; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `cargo xtask build-guests --check` clean (guests fresh).

**Behavior-change risk carried by AC-5:** any profile where `nozzle_diameter`
differs from `inner_wall_line_width` now behaves differently in
`classic-perimeters` than before this packet, because the previously dead
`nozzle_diameter` read now actually resolves the supplied value instead of
silently falling back to `inner_wall_line_width`. Profiles that keep the two
equal are unaffected.

**Regenerated self-captured baselines** (via the sanctioned `#[ignore]` record
tests, per D-109-SELF-CAPTURED-FIXTURES provenance):
- `cube_4color_arachne` — genuine packet-150 D-105 spacing shift
  (0.4 mm → 0.3571 mm, Flow-spacing parity).
- `narrow_strip_widening` — GapFill→ThinWall classification shift traced to
  packet 148's `classify_line` refinement; golden was already stale since
  packet 147 and this is NOT a packet-150 change — the regeneration comment
  attributes the true cause.

**Discovered/handled during implementation, beyond the packet's original
plan:**
- Host `ConfigValueStorage` (`crates/slicer-wasm-host/src/host.rs`) needed new
  `Percent`/`FloatOrPercent` variants for percent values to survive
  host→guest delivery — the design under-specified this hop. Locked by
  `percent_config_value_round_trips_through_storage_and_wit_get`. Recorded in
  `docs/DEVIATION_LOG.md` D-104h-NO-PERCENT-CONFIG-TYPE.
- `precise_outer_wall`'s Arachne-parity inset was a real wiring bug: it paired
  the outer wall's raw width with the INNER wall's spacing. Fixed to use the
  outer wall's own spacing per OrcaSlicer `PerimeterGenerator.cpp:2103-2104,2158`.
  Two stale baselines corrected from `-0.05` to the spacing-correct
  `-(layer_height·(1-π/4))/2 = -0.0214602`: the `arachne_parity.rs` lock and
  `arachne-perimeters/tests/precise_outer_wall_tdd.rs`. Recorded in
  `docs/DEVIATION_LOG.md` D-105-FLOW-NOT-WIRED.
- `line_width_to_spacing` returns `0` for degenerate `width <= layer_height`
  configs (Orca rejects `width < height`); `arachne_params_from_config` now
  falls back to the raw width in that case to avoid zero-spacing bead
  collapse. The `perimeter_parity` fixtures `tapered_wedge` /
  `narrow_strip_widening` / `max_bead_count_cap` / `complex_multi_feature` use
  an unrealistic `layer_height=1.0mm` (a packet-109 artifact) that trips this
  path — flagged as a fixture-hygiene follow-up, not fixed here. Recorded in
  `docs/DEVIATION_LOG.md` under D-105-FLOW-NOT-WIRED.
- The packet-149 bridge stub test (`bridge_flow_factor_tdd.rs`) asserted the
  D-104g stub (`flow_factor == 1.0`) and was updated to assert the
  round-section factor per-vertex; AC-4's own assertion (`> 0.05` delta from
  1.0) was noted as weak and strengthened with physical-value locks (1.0996 /
  1.5708).
- Two self-captured perimeter_parity goldens were regenerated (see above):
  `cube_4color_arachne` (genuine D-105 spacing shift) and
  `narrow_strip_widening` (packet-148 `classify_line` cause, not packet 150).

## Deviations

- [design.md §Data/Contract] — Specified: percent variant crosses the config.wit host→guest boundary | Implemented: ALSO required extending host ConfigValueStorage (host.rs) with Percent/FloatOrPercent variants + config_value_to_storage/HostConfigView::get wiring + a round-trip test | Reason: design under-specified this hop; without it the guest received a downgraded String and get_abs_value never saw a Percent.
- [AC-6 / design Locked Assumptions] — Specified: precise_outer_wall lock asserts a relative delta and survives unchanged | Implemented: found a REAL inset wiring bug (used the inner wall's spacing); fixed to the outer wall's own spacing (Orca PerimeterGenerator.cpp:2103-2104); corrected the baseline -0.05 -> -(h*(1-PI/4))/2 = -0.0214602 in arachne_parity.rs AND precise_outer_wall_tdd.rs | Reason: the assumption was false; the lock encoded a raw-width-era value.
- [design.md Step 5] — Specified: bridging_flow signature change scoped to arachne-perimeters + flow.rs | Implemented: also updated classic-perimeters (import + emit_walls layer_height threading + new layer_height read + toml registration of nozzle_diameter/layer_height) | Reason: classic-perimeters also calls bridging_flow; the signature change forced it.
- [Step 4 / line_width_to_spacing] — Specified: feed line_width_to_spacing output as bead width | Implemented: added a module-side fallback to raw width when spacing<=0 (degenerate width<=layer_height configs Orca rejects) | Reason: line_width_to_spacing returns 0 for width<=layer_height; feeding 0 collapsed beading on fixtures using an unrealistic layer_height=1.0mm.
- [AC-4] — Specified: assert >=1 bridge vertex flow_factor differs from 1.0 by >0.05 | Implemented: kept the gap test AND strengthened bridge_flow_factor_tdd.rs to lock the exact round-section factor per-vertex + physical values 1.0996/1.5708 | Reason: the ">0.05 from 1.0" assertion was too weak (a wide-bead value of 0.06 satisfies it).
- [Step 3 / config_resolution.rs] — Specified: is_numeric_field_type updated; percent handled | Implemented: is_numeric_field_type includes percent/float_or_percent; the initial impl left check_value a no-op (silent bounds-skip on min_width_top_surface min=0.0) with an inaccurate comment — FOUND AND FIXED in close-out audit: check_value now enforces declared bounds against the raw percent/literal value via check_scalar; comment corrected | Reason: initial percent bounds enforcement was inconsistent; resolved so is_numeric_field_type and check_value agree.
- [AC-N1 command] — Specified: cargo test -p slicer-scheduler --test contract | Implemented: corrected to --test scheduler_contract (real binary) in packet.spec.md, requirements.md, and implementation-plan.md | Reason: the specified test binary did not exist.
- [perimeter_parity fixtures] — Specified: (unlisted) | Implemented: regenerated 2 self-captured goldens via sanctioned record tests — cube_4color_arachne (genuine D-105 spacing 0.4->0.3571) and narrow_strip_widening (shift traced to packet 148, NOT 150) | Reason: full-suite exposed the shifts; regenerated after verifying correctness.
- [residual, not a 150 change] — multi_tool_triangle_perimeter_parity is a PRE-EXISTING classic-perimeters failure (confirmed via stash A/B); registered as D-150-MULTI-TOOL-TRIANGLE-PREEXISTING, flagged for separate bisection.
