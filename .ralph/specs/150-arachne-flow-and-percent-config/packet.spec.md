---
status: active
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
- Activation blockers: none. No other packet is `status: active` (verified
  2026-07-09: the four `.ralph/specs` matches for "active" are body prose;
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
  `cargo test -p slicer-scheduler --test contract -- config_percent_type --nocapture`
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
