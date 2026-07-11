# 18 — Arachne Parity Audit (2026-07-09, second round)

Read-only audit of the Arachne implementation on `parity/arachne`
(`34ce576e`) against the canonical OrcaSlicer reference at
`OrcaSlicerDocumented/src/libslic3r/`. This is the **second** audit round:
the first round's deliverable is `crates/slicer-runtime/tests/arachne_parity.rs`,
whose gaps were largely closed by packets 148/149 (its tests are now green
regression locks, except the still-red concentric-infill test).

**Deliverable of this round:** `crates/slicer-runtime/tests/arachne_parity_gaps.rs`
— 10 red tests, one per open gap, each failing with a
`PARITY GAP: <feature> | expected | got | ref` message. Fixtures:
`crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs` (programmatic,
no STL needed). Run with:

```bash
cargo test -p slicer-runtime --test arachne_parity_gaps
```

All 10 tests MUST fail until their gap is fixed. Do not `#[ignore]` or weaken
them; each test body already asserts the correct end state, so closing a gap
turns its test green with no rewrite.

Numbering note: this doc is `18_` because `15_` (config keys), `16_`
(slicer report) and `17_` (agent debugging) were already taken.

## What is at parity (do NOT re-audit)

The core Arachne engine reached canonical parity through packets 110–113c and
the N1–N13 chain (packets 141–147, ADR-0035, D-147-CHAIN-CLOSURE):

| Subsystem | PnP path | Closure |
|---|---|---|
| Voronoi / SkeletalTrapezoidation graph | `crates/slicer-core/src/voronoi.rs`, `crates/slicer-core/src/skeletal_trapezoidation/` | P110, P113b/c |
| BeadingStrategy stack (Distributed, Redistribute, Widening, OuterWallInset, Limited) | `crates/slicer-core/src/beading/` | P111, D-111 |
| Junction generation / connectJunctions / transitions | `crates/slicer-core/src/arachne/generate_toolpaths.rs` | D-141…D-146 |
| Post-process order (stitch → remove_small → separate inner contour → simplify → remove empty) | `crates/slicer-core/src/arachne/{stitch,remove_small,separate_inner_contour,simplify}.rs` | D-146-POSTPROCESS-ORDER |
| Per-vertex parity flags (overhang_quartile, is_bridge, is_thin_wall, boundary_type, seam candidates, precise_outer_wall) | `modules/core-modules/arachne-perimeters/src/lib.rs` | P148 (D-104 closed) |
| alternate_extra_wall, extra_perimeters_on_overhangs, bridge_flow (thin branch) | module + `crates/slicer-core/src/flow.rs` | P149 (D-104e closed) |
| Module selection via `wall_generator` | `crates/slicer-scheduler/src/execution_plan.rs:182-260` | D-112-WALL-GENERATOR-SELECT |

Deliberate divergences (documented, NOT locked by red tests): Visvalingam–
Whyatt simplify instead of Orca's triangle-height heuristic (ADR-0035,
D-112-SIMPLIFY-DP-addendum); `detect_thin_wall` as a config key vs Orca's
hardcoded `fill_outline_gaps = true` (`WallToolPaths.hpp:18`); PnP default
`wall_generator = "classic"` vs Orca default Arachne.

## Open gaps (each locked by a red test)

Categories: **CONFIG** (key exposure), **ALGO** (behavior), **INTEG**
(pipeline dispatch), **MODEL** (data/config model). All tests live in
`arachne_parity_gaps.rs` unless stated otherwise.

### G1 — `wall_direction` winding control (CONFIG + ALGO)
- OrcaSlicer: `PrintConfig.cpp:2188-2198` (enum CCW/CW, default CCW);
  applied via `make_counter_clockwise/make_clockwise` in
  `PerimeterGenerator.cpp:527-545`; holes wound opposite the contour.
- PnP: zero readers of `wall_direction` anywhere in `crates/` or `modules/`;
  key not in `arachne-perimeters.toml`.
- Test: `arachne_parity_pipeline_wall_direction_controls_winding`.

### G2 — `only_one_wall_first_layer` (CONFIG + ALGO)
- OrcaSlicer: `PrintConfig.cpp:1513-1517`; forces `loop_number = 0` on the
  first printed layer, `PerimeterGenerator.cpp:2137-2139`.
- PnP: key unregistered, wall count never reduced on layer 0.
- Test: `arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall`.

### G3 — `only_one_wall_top` behaviorally inert (ALGO)
- OrcaSlicer: topmost layer forces `loop_number = 0`
  (`PerimeterGenerator.cpp:2140-2144`); non-topmost top surfaces get a
  SECOND `Arachne::WallToolPaths` pass over non-top area with `inset_idx`
  renumbering and `min_width_top_surface`-based top-area derivation
  (`PerimeterGenerator.cpp:2160-2246`, second constructor at `:2242`).
- PnP: module reads the key and discards it
  (`arachne-perimeters/src/lib.rs:305-306`, deferred under
  D-104d-MIN-WIDTH-TOP-SURFACE-NONE).
- Test: `arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top`.

### G4 — Wall gap uses Flow spacing, not raw width (ALGO, D-105) — closed (packet 150)
- **Closed (packet 150):** `slicer_core::flow::line_width_to_spacing` is now wired into
  bead placement in both `classic-perimeters` and `arachne-perimeters` (was raw
  width); see `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` D-105
  (now closed).
- OrcaSlicer: `bead_width_0 = ext_perimeter_spacing`
  (`PerimeterGenerator.cpp:2129`); `WallToolPaths` receives
  `perimeter_spacing = perimeter_flow.scaled_spacing()`
  (`PerimeterGenerator.cpp:578, 2172-2173`; `Flow.hpp:67`) — spacing is
  layer-height dependent (width − h·(1 − π/4)).
- PnP: raw `optimal_width` used; `layer_height` never read;
  `slicer_core::flow::line_width_to_spacing` exists but unwired
  (D-105-FLOW-NOT-WIRED). Observed centerline gap 0.4000 mm vs expected
  ≈0.3571 mm at 0.4 mm width / 0.2 mm layers.
- Test: `arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width`.

### G5 — `thick_bridges` bridging flow stubbed to 1.0 (ALGO, D-104g) — closed (packet 150)
- **Closed (packet 150):** `bridging_flow`'s `thick_bridges == true` branch now
  computes the round cross-section formula `π·dmr²/(4·w·h)`
  (`dmr = nozzle_diameter·sqrt(bridge_flow_ratio)`) instead of a hardcoded `1.0`;
  see `docs/DEVIATION_LOG.md` D-104g (now closed).
- OrcaSlicer: `overhang_flow = bridging_flow(frPerimeter, thick_bridges)`
  (`LayerRegion.cpp:135`, impl `:31-50`); with thick bridges the flow is a
  round cross-section of thread diameter (`Flow.hpp:106`), ≈1.57× a flat
  0.4×0.2 mm bead.
- PnP: `slicer_core::flow::bridging_flow`'s `thick_bridges == true` branch
  returns hardcoded `1.0` (`crates/slicer-core/src/flow.rs:85-92`). Observed
  bridge-vertex flow factors: all 1.0.
- Test: `arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one`.

### G6 — No percent / float-or-percent config type (MODEL, D-104h) — closed (packet 150)
- **Closed (packet 150):** new `percent`/`float_or_percent` config-schema types,
  resolved module-side via `ConfigView::get_abs_value(key, base)`; see
  `docs/03_wit_and_manifest.md` §Config Field Types Reference and
  `docs/DEVIATION_LOG.md` D-104h (now closed).
- OrcaSlicer: `min_width_top_surface` coFloatOrPercent 300%
  (`PrintConfig.cpp:1498-1511`); `min_feature_size` coPercent 25%
  (`:7217-7226`); `wall_transition_length` coPercent 100% (`:7169-7178`);
  all relative to nozzle diameter / wall width.
- PnP: keys declared `type = "float"` with pre-resolved absolute defaults
  (`arachne-perimeters.toml:38-42, 68-72, 257-261`); nozzle changes leave
  them stale.
- Test: `arachne_parity_pipeline_percent_config_type_for_arachne_keys`.

### G7 — `overhang_reverse` registration-only (ALGO, D-104c)
- OrcaSlicer: with `detect_overhang_wall` off + `overhang_reverse` on,
  contour/holes unconditionally marked steep and reversed on odd layers
  (`PerimeterGenerator.cpp:422-429`; `detect_steep_overhang` `:58-98`;
  applied in `traverse_extrusions` `:370-523`); tunable via
  `overhang_reverse_threshold` (coFloatOrPercent).
- PnP: `overhang_reverse` / `overhang_reverse_internal_only` /
  `detect_overhang_wall` registered but have zero readers in the module;
  `overhang_reverse_threshold` unregistered. Toggling changes nothing.
- Test: `arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls`.

### G8 — Spiral vase does not force the classic generator (INTEG)
- OrcaSlicer: Arachne dispatch gated on `wall_generator == Arachne &&
  !spiral_mode` (`LayerRegion.cpp:138-141`).
- PnP: `dedup_same_claim_modules_with_wall_generator`
  (`crates/slicer-scheduler/src/execution_plan.rs:256-261`, called from
  `crates/slicer-runtime/src/run.rs:342`) keys only off `wall_generator`;
  no spiral-vase input exists anywhere in the selection path.
- Test: `arachne_parity_pipeline_spiral_vase_forces_classic_generator`.

### G9 — `wall_maximum_resolution` / `wall_maximum_deviation` unregistered (CONFIG)
- OrcaSlicer: `PrintConfig.cpp:7242-7263` (defaults 0.5 mm / 0.025 mm),
  consumed by outline prep (`WallToolPaths.cpp:487-503`) and
  `simplifyToolPaths` (`:702-719`).
- PnP: internal equivalents exist
  (`ArachneParams.smallest_line_segment_squared` /
  `allowed_error_distance_squared`, `pipeline.rs:149-154`) but are
  compile-time defaults, never config-driven.
- Test: `arachne_parity_pipeline_wall_max_resolution_deviation_registered`.

### G10 — `removeSmallLines` top-layer exception conflated with layer 0 (ALGO)
- OrcaSlicer: lenient `min_width/2` threshold applies on top **or** bottom
  layers (`WallToolPaths.cpp:684-700`; `is_top_or_bottom_layer =
  is_bottom_layer || is_topmost_layer`, `PerimeterGenerator.cpp:2153-2154`).
- PnP: `remove_small_lines` keys the lenient threshold on
  `is_initial_layer` only (`crates/slicer-core/src/arachne/remove_small.rs:44-80`);
  neither it nor `run_arachne_pipeline` can express "topmost layer", so
  top-surface thin walls are dropped by the strict threshold.
- Test: `arachne_parity_arachne_path_remove_small_lines_top_layer_exception`.

### G11 — Concentric infill not routed through Arachne (INTEG, D-104f)
- OrcaSlicer: `FillConcentric.cpp:80-118`, `FillConcentricInternal.cpp:29-55`.
- PnP: no infill module references the Arachne pipeline.
- Test: `arachne_parity_pipeline_concentric_infill_uses_arachne` —
  **pre-existing red test in `arachne_parity.rs`** (first-round audit); not
  duplicated here.

## Known residuals tracked elsewhere (no new test)

- **AC-1 e2e outer-wall closure** — `cube_4color_arachne_outer_walls_close_end_to_end`
  (`crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`) is kept
  `#[ignore]`d by explicit user decision 2026-07-08 (D-147-CHAIN-CLOSURE
  residual, 49.33% closure rate). The ignored test already encodes the gap.
- **Self-captured fixtures** — all Arachne parity baselines are self-captured,
  not OrcaSlicer-oracle outputs (D-109 / D-112-SELFCAPTURED-BASELINES, open,
  accepted).
- **`wall_sequence` ownership** — `InnerOuterInner` reordering exists in
  `perimeter_utils` / classic, but ownership is split with
  `path-optimization-default` contra ADR-0011 (DEV-070, open). Behavior for
  the Arachne path should be re-verified once DEV-070 is remediated.

## Gap summary table

| # | OrcaSlicer feature | Ref | PnP status | Red test |
|---|---|---|---|---|
| G1 | `wall_direction` CCW/CW winding | `PerimeterGenerator.cpp:527-545` | missing | `..._wall_direction_controls_winding` |
| G2 | `only_one_wall_first_layer` | `PerimeterGenerator.cpp:2137-2139` | missing | `..._only_one_wall_first_layer_forces_single_wall` |
| G3 | `only_one_wall_top` (incl. second Arachne pass) | `PerimeterGenerator.cpp:2140-2246` | key read, inert | `..._only_one_wall_top_forces_single_wall_on_top` |
| G4 | Flow spacing feeds bead widths | `PerimeterGenerator.cpp:2129,2172` | closed (packet 150) — `line_width_to_spacing` now feeds bead placement | `..._wall_gap_uses_flow_spacing_not_width` |
| G5 | Thick-bridge round-section flow | `LayerRegion.cpp:135`; `Flow.hpp:106` | closed (packet 150) — `π·dmr²/(4·w·h)` formula implemented | `..._thick_bridges_flow_factor_not_stubbed_to_one` |
| G6 | Percent-typed Arachne keys | `PrintConfig.cpp:1498-1511,7169-7226` | closed (packet 150) — `percent`/`float_or_percent` types added | `..._percent_config_type_for_arachne_keys` |
| G7 | `overhang_reverse` odd-layer reversal | `PerimeterGenerator.cpp:58-98,422-429` | registration-only | `..._overhang_reverse_flips_odd_layer_walls` |
| G8 | Spiral vase forces classic | `LayerRegion.cpp:138-141` | missing | `..._spiral_vase_forces_classic_generator` |
| G9 | `wall_maximum_resolution/deviation` | `PrintConfig.cpp:7242-7263` | internal-only | `..._wall_max_resolution_deviation_registered` |
| G10 | `removeSmallLines` top-layer exception | `WallToolPaths.cpp:684-700` | conflated with layer 0 | `..._remove_small_lines_top_layer_exception` |
| G11 | Concentric infill via Arachne | `FillConcentric.cpp:80-118` | missing (D-104f) | `arachne_parity.rs::..._concentric_infill_uses_arachne` |

## Porting reminders for the fixing agent

- 1 PnP unit = 100 nm; divide OrcaSlicer scaled constants by 100
  (`docs/08_coordinate_system.md`). Use `Point2::from_mm` / `mm_to_units`.
- Config keys snake_case; new keys go in `arachne-perimeters.toml`
  `[config.schema.*]` AND must be read via `ConfigView` in the module.
- G4/G5/G6 need `layer_height` / `nozzle_diameter` plumbed into the module's
  resolved config before the formulas can be computed.
- G8 belongs in the scheduler/runtime selection path, not the module.
- G10's fix requires extending `run_arachne_pipeline` (and the
  `generate-arachne-walls` host-service WIT contract — see WIT/type-change
  checklist in `CLAUDE.md`) with a top/bottom-layer flag distinct from
  `is_initial_layer`.
