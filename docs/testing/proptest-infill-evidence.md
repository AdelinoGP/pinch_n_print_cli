# Infill, linking, wave and ironing property evidence

This file records executed evidence for `proptest-openrscad_13_pbt-infill`. Entries
marked **pending** are implementation placeholders, not claims of execution.

## Source and environment

- `source_revision`: `cbd4998210c0b86ab16b285f5536677efb05e41f` at the start of the
  run (`git rev-parse HEAD`).
- Initial `git status --porcelain=v1`: empty (clean working tree).
- Platform: Microsoft Windows 11 Home, X64; `Microsoft Windows NT 10.0.26300.0`.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`; `cargo 1.96.0 (30a34c682 2026-05-25)`.
- `CARGO_TARGET_DIR`: unset; Cargo used the workspace `target/` directory.
- Proptest 1.11.0 (workspace pin, `default-features = false`, features
  `std`, `fork`, `timeout`). `generator_version = 1`.
- Canonical source inspection: `D:/slicerProject/Orca(pnp_gui)` v2.5.0 checkout,
  read-only, cited by file + function (never by line number).

## Prerequisite surface (Step 1)

- `proptest-openrscad_01_pbt-foundation`, `_02_pbt-domains` and `_09_pbt-geometry`
  all report `status: implemented` on disk.
- The live SDK surface provides `PropertyRun::{new,bound,check}`, `CaseGuard`,
  `domains::{DomainError, GeometryBounds, GeometryDomain, GeometryCase, TopologyClass,
  BoundaryRule, geometry_cases}`, `oracles::{validate_geometry, point_in_region,
  segment_in_region, signed_area2, SegmentRelation, PointLocation}` and
  `regression::format_regression_fixture` under
  `slicer_sdk::test_support::property_testing`.
- `PreparedPolylineClip::{new,clip_batched}` is live at
  `crates/slicer-core/src/polygon_ops.rs`; its documented contract is the exact
  1-unit pre-inflation of the clip universe, subject-major batched results and a
  ±2-unit boundary-coordinate envelope.
- `cargo metadata` confirms lib targets `rectilinear_infill`, `gyroid_infill`,
  `infill_linker`, `wave_overhangs`, `top_surface_ironing`, each with the new
  `property_testing` feature and no `required-features` gate on the lib target.
- `#11`'s ordered priority/name chain handoff is read as coordination only; this
  packet neither waits for it nor edits it.
- Guest freshness: `cargo xtask build-guests --check` exited **0** before any edit
  (all artifacts fresh). The post-scaffolding stale/rebuilt/re-checked sequence is
  recorded below and in the final gate.

### Prerequisite controls executed (all Windows, all exit 0)

| Control | Result | Log |
|---|---|---|
| `property_harness -- case_guard_cleans_success_failure_and_unwind` | 1 passed, 14 filtered | `step1/prereq_case_guard.log` |
| `property_domain_contracts -- domain_exact_predicates_counterexamples` | 1 passed, 10 filtered | `step1/prereq_domain_exact.log` |
| `property_geometry_controls -- geometry_controls_boolean_and_clip_mutations` | 1 passed, 4 filtered | `step1/prereq_geometry_controls.log` |

## Before-change baselines (Step 1)

Every command ran separately through `run_logged` (`target/test-output.log`, archived
before the next invocation). All exited 0.

| Baseline | Result | Log |
|---|---|---|
| `rectilinear_raw_emit_tdd -- polygon_with_hole_segments_split_around_hole` | 1 passed | `step1/rect_polygon_with_hole.log` |
| `rectilinear_raw_emit_tdd -- solid_spacing_adjusted_for_solid_role` | 1 passed | `step1/rect_solid_spacing_adjusted.log` |
| `rectilinear_infill_edge_cases_tdd -- very_small_polygon_emits_one_scan_row_without_panic` | 1 passed | `step1/rect_very_small_polygon.log` |
| `gyroid_infill_tdd -- align_to_grid_snaps_bbox_min` | 1 passed | `step1/gyroid_align_to_grid.log` |
| `gyroid_infill_tdd -- square_10mm_z_0p2_emits_raw_waves` | 1 passed | `step1/gyroid_square_raw_waves.log` |
| `connect_tdd -- endpoints_on_different_rings_are_never_joined` | 1 passed | `step1/linker_different_rings.log` |
| `anchor_length_tdd -- stub_is_exactly_anchor_length_via_a_lerped_partial_segment` | 1 passed | `step1/linker_anchor_stub.log` |
| `orchestrate_tdd -- cross_tool_paths_not_compatible_in_orchestrate` | 1 passed | `step1/linker_cross_tool.log` |
| `wave_overhangs_tdd -- waves_cover_domain_without_seam_gap` | 1 passed | `step1/wave_cover_no_seam_gap.log` |
| `wave_overhangs_tdd -- wave_bead_footprint_stays_inside_trim_boundary` | 1 passed | `step1/wave_bead_footprint.log` |
| `top_surface_ironing_emission_tdd -- u_shape_top_fill_produces_disjoint_segments_per_row` | 1 passed | `step1/ironing_u_shape.log` |

## Step 2 — opt-in owner-local homes

- Five module manifests gained an empty, default-off `property_testing` feature,
  a `proptest` workspace dev-dependency, and `property_testing` on the existing
  `slicer-sdk` dev-dependency edge.
- Each owner `src/lib.rs` declares the out-of-line module with the required
  stacked gates (`#[cfg(test)]` + `#[cfg(feature = "property_testing")]`).
- After the manifest edits `cargo xtask build-guests --check` exited **1** and
  named exactly `rectilinear-infill-guest`, `gyroid-infill-guest`,
  `infill-linker-guest`, `wave-overhangs-guest` and `top-surface-ironing-guest`
  stale (fingerprint mismatch). `cargo xtask build-guests` rebuilt those five and
  the immediate re-check exited **0**. The guest fingerprint also covers the
  workspace manifest and `Cargo.lock`, so later lockfile churn during the run is
  re-checked and re-recorded in the final gate below.
- `cargo check -p <owner> --all-targets` and `--all-targets --features property_testing`
  passed for all five owners; feature-enabled lib discovery listed the compile
  witness, default discovery did not.
- Proptest and the SDK `property_testing` feature are dev-graph only: no normal
  dependency or guest enables them.

## Step 3 — rectilinear scan intervals and canonical solid spacing

Files: `modules/core-modules/rectilinear-infill/src/property_tests.rs` (2,001 lines,
`compile_witness_property_support` removed), `.../src/lib.rs` (`adjust_solid_spacing`
only), `.../tests/rectilinear_raw_emit_tdd.rs` (one new regression, two fixture
re-baselines).

- The scan oracle enumerates rows arithmetically from the fixture bbox on the
  half-open grid and derives each row's interior X intervals by probing open
  cells of a **doubled** copy of the region with `oracles::point_in_region`, so
  no probe can land on an edge. It never calls `scan_expolygon`, `rotate_point` or
  the production edge collectors. Every emitted segment is additionally checked
  with the boundary-inclusive `oracles::segment_in_region`, and role, both
  endpoints, Z, width, flow, speed and `None` tool/lock are compared bit-exactly.
- Shifted output is compared against the correspondingly translated region, never
  the unshifted one.
- Generated classes: `Rectangle`, `SubSpacingRectangle`, `LNotch`, `UNotch`,
  `HoledRectangle`, each with solid/sparse, shifted/unshifted and negative-XY
  modes. The generator builds one parameter draw into a *suite* of cases so every
  class and mode is exercised per accepted call; per-class segment counters are
  asserted positive after `check`.
- Exact charter controls: outer `[0,10] x [0,10]` mm with hole `[2,3] x [4,6]` mm
  yields 10 rows / 12 segments with row `y = 5` split in two, and the mutated
  `(1,5) -> (9,5)` mm chord (endpoints and midpoint `(5,5)` all in material) is
  rejected by `segment_in_region`. A filled-hole chord, a removed split
  interval, a U-notch chord through `x in (4,5)` at `y = 8` and a positive
  sub-spacing rectangle (one row, one segment) are all exercised.
- Legitimate empty modes are separate: empty contour, one-point contour,
  zero-height, zero-width, `spacing = 0` and `spacing = -10000` all emit nothing;
  a tangential base vertex on a triangle emits no zero-length row.

### `M-RECTILINEAR-SPACING` — confirmed defect, focused fix

Canonical source re-read directly in the available OrcaSlicer checkout:
`Fill::_adjust_solid_spacing` (`src/libslic3r/Fill/FillBase.cpp`) computes
`number_of_intervals = coord_t((width - EPSILON) / distance)`, then
`distance_new = coord_t((width - EPSILON) / number_of_intervals)` (or `distance`
when that count is zero), then caps when `distance_new / distance > 1.2` by
returning `coord_t(floor(distance * 1.2 + 0.5))`, with `EPSILON = 1e-4`.

Measured before the fix: width 15,000 units (1.5 mm) / distance 10,000 units
(1.0 mm) returned **10,000** (a reset to the requested distance); canonical
returns **12,000** (the 1.2x cap). Reproduced in
`step3/pre_fix_rect_controls.log` before the fix was applied.

Fix: `adjust_solid_spacing` now uses the canonical numerator, integer truncation
for both divisions, and the capped return. Two derived facts are recorded in the
source:

- The source's coordinate-scaled `1e-4` is observationally identical to `width - 1` on the
  100 nm grid: any `0 < epsilon < 1` unit yields the same truncated interval
  count for every integer `(width, distance)` pair, and
  `floor((width-1)/n) == floor((width-1e-4)/n)` for every integer `n >= 1`. An
  exhaustive sweep over `distance in 1..400` and `width in 0..200000` found zero
  divergences between `epsilon = 1e-4` and `epsilon = 1`.
- The over-cap comparison mirrors the source quotient
  `factor = distance_new / distance` rather than `distance_new > distance * 1.2`,
  which can disagree at a rounding boundary because the product rounds once.

Exact canonical cases now asserted (independently computed from the source
formula, reproduced by both the SUT and the test's reference):
`(15000,10000) -> 12000`, `(12000,10000) -> 11999`, `(12001,10000) -> 12000`,
`(12002,10000) -> 12000`, `(10000,10000) -> 10000`, `(5000,10000) -> 10000`,
`(100000,4200) -> 4347`, `(100000,5000) -> 5263`, plus
`(9000,10000) -> 10000`, `(10001,10000) -> 10000`, `(20000,10000) -> 12000`,
`(50000,10000) -> 12000`, `(60000,10000) -> 11999`, `(70000,10000) -> 11666`,
`(7,5) -> 6`, `(8,5) -> 6`, `(0,10000) -> 10000`.

Named no-RNG regression: `solid_spacing_over_cap_uses_canonical_1_2_factor`
(`tests/rectilinear_raw_emit_tdd.rs`) drives real `run_infill` on a 10 mm x 1.5 mm
top-solid region at 0 deg with 1.0 mm solid spacing. Plan correction: that input
produced **two** rows both before and after the fix; what changes is the row
*grid*. Pre-fix the rows sat at `{0, 10000}` units, canonical puts them at
`{0, 12000}`. The regression pins the grid, and its pre-fix failure was
`left: {0, 10000} / right: {0, 12000}`.

Fixture re-baselines forced by the canonical fix (both canonical-correct, no
assertion loosened):

| Test | Before | After | Canonical reason |
|---|---|---|---|
| `solid_spacing_adjusted_for_solid_role` (10 mm span, 4200-unit spacing) | 23 rows, spacing 4348 | 24 rows, spacing 4347 | `floor((100000-1)/4200) = 23` intervals, `floor(99999/23) = 4347`; `23*4347 = 99981 < 100000` admits a 24th row |
| `solid_surfaces_use_full_density_and_role_width`, first-layer 0.7 mm | 14 rows | 15 rows | `floor(99999/14) = 7142`, `14*7142 = 99988 < 100000` |
| same test, internal 0.6 mm | 16 rows | 17 rows | `floor(99999/16) = 6249`, `16*6249 = 99984 < 100000` |

Pilot counters (24 accepted cases, 0 rejects, 0 local/global rejects):
`M-RECTILINEAR-SCAN` → rect 820, sub_spacing 96, notch 1840, hole 988,
shifted 1872, solid 1880, negative_xy 3744 segments.
`M-RECTILINEAR-SPACING` → width_below_distance 26, exact_multiple 24,
interval 43, over_cap 26 pairs.

Gates: `rect_controls` and `rect_shrink_bounds` pass; whole
`rectilinear_raw_emit_tdd` (9) and `rectilinear_infill_edge_cases_tdd` (2) pass;
whole crate 37/37; default and enabled `--all-targets` checks, enabled clippy
`-D warnings`, `check-literals`, `check-test-quality --report` and `check-fmt`
clean.

## Step 4 — gyroid phase/grid and raw-wave clipping

File: `modules/core-modules/gyroid-infill/src/property_tests.rs` (3,455 lines,
compile witness removed). No SUT retention defect was found; no production code
changed.

- **Phase.** Generated phases are bounded to one full period
  (`max_phase_milliradians = 6284`); `z_sin`/`z_cos` come from that same phase and
  the orientation is the caller's own rule (`abs(sin) <= abs(cos)`, asserted
  against `expected_vertical`). Branch ranges are derived, not observed: both
  `asin` terms lie in `[-pi/2, pi/2]`, so vertical output lies in `[0, 2pi]` and
  horizontal in `[-pi/2, 3pi/2]`; the asserted envelope is `1e-12`, over 500x the
  derived term-wise error.
- **Reachability finding, recorded in the file.** With a consistent phase and the
  caller's orientation rule the unused component is at least `1/sqrt(2)`, so
  `r >= 0.707` and the `r < 1e-15` singular return is unreachable from generated
  cases. Near-singular finiteness is therefore an explicit deterministic control
  (`gyroid_f(0,1,0,vertical,false) = PI`,
  `gyroid_f(pi/2,0,1,false,false) = pi/2`, and an `r` clamp case), not a generated
  family. Periodic observations are additionally gated to `r >= 0.25` and
  `|u| <= 0.75`, which keeps them off the clamp plateaus.
- **Grid.** `align_to_grid` is compared bit-exactly to the exact integer model
  `val.div_euclid(grid) * grid`, including the charter's dyadic witnesses
  (grid 2: `-3 -> -4`, `-2 -> -2`, `+3 -> 2`; grid 4: `-9 -> -12`, `-8 -> -8`,
  `7 -> 4`), adjacent negatives, grid multiples and whole-period translation. A
  truncation-toward-zero mutant is rejected by the same checker. The derivation
  for bit-exactness (`grid <= 16384`, `|val/grid| <= 65`, `1/grid` far above the
  f64 division error) is recorded.
- **Implicit-surface residual**: deliberately not asserted. The header records
  that no meaningful residual subdomain is proved, because the emitter's
  swapped/offset/clamped coordinates, f32 stamp and grid expansion make the
  emitted-to-source mapping non-injective. No residual claim is made.
- **Clip.** Real `fill_expolygon` raw waves (not region-clipped) are converted to
  integer subjects and passed through the linker's own
  `PreparedPolylineClip::new(boundary).clip_batched(subjects)`; per-subject
  results are consumed positionally, never as a flat sequence. The independent
  model inflates each contour by exactly 1 unit and contracts each hole by 1 unit
  and classifies subject subintervals with exact `i128` rational arithmetic at
  every critical parameter, so both the retained and the missing subintervals are
  compared. Raw output metadata is checked without asserting hole exclusion: the
  stamped width must equal the module's own `line_width`, not the per-call spacing
  argument, and the control asserts that conflating them is rejected. Measured raw
  emission is genuinely unclipped: 139,846 of 158,070 raw points lie outside the
  outline in a representative 256-case run.
- **Tolerance.** The linear allowance is `CLIP_BAND_UNITS = 1.0` unit. The model
  already builds its expected intervals on the inflated edge (`outer ∓ 1`), and for
  these axis-aligned integer fixtures `inflate_paths_64(delta = 1.0)` lands exactly
  one unit out, so the only deviation of a reported coordinate from the *modelled*
  edge is the clipper's ±1-unit integer intersection rounding. There is no separate
  f32 term: the model and the clipper read the same integer subject. The
  returned-point projection onto the subject has a separate derived
  `SUBJECT_PROJECTION_UNITS = 2.0` (each coordinate moves <1 unit, Euclidean
  <√2, plus ≤0.5 unit of batch scanline rounding); it is used only for attribution,
  never as a boundary allowance.
  The parameter allowance is per expected-interval endpoint and derived from the
  local crossing geometry: `band / length` for a subject endpoint, and
  `band / |Δnormal|` for a crossing of a vertical or horizontal clip edge (`|Δx|`
  on a vertical edge, `|Δy|` on a horizontal one), which is the perpendicular worst
  case of any displacement within the band of the modelled edge. It equals
  `band / length` at a perpendicular crossing, grows as `1/sin θ` for tangential
  crossings, and the along-edge slide `(band/|Δnormal|)·cos θ` is smaller. A
  shallow-crossing control caught the earlier `(band/length)·max(1, cot θ)` form
  missing the perpendicular component by `cos θ`. Cut points are attributed to the
  **nearest** subject segment within a bounded window of adjacent segments, never to
  the first segment within the band. Coverage is
  required outside each interval eroded by its own endpoint terms, and containment
  permits a returned span only inside each interval dilated by its own terms.
- **Clip-oracle controls.** An all-empty clip and an omitted interior span are
  rejected by the coverage direction (`missing`), and a fabricated span joining two
  real points across the hole is rejected by the chord/hole checks. A shallow-
  crossing control accepts a cut exactly at the derived term and still rejects one
  5 units short, so the term is finite rather than a blanket pass. Legitimate empty modes are separate: empty contour, `density` 0/-0.5,
  `width` 0/-0.4 emit no wave, and a real wave set clipped against a far-away
  boundary yields `expected_intervals = 0` with an accepted empty result.
- **Counters.** A representative 256-case run: 3,113 raw paths, 154,957 raw
  segments, 158,070 raw points (139,846 outside), 2,475 expected intervals, 17,895
  returned segments, 1,366 retained subjects, 17,847 interior observations, 0
  saturated endpoints, 510 hole splits, 163 holed cases, 126 rotated and 144 dense
  cases. A representative phase run at 256 cases: 119 vertical, 137 horizontal,
  141 positive phase, 115 negative, 118 flip, 138 unflipped, 236 periodic, 128
  grid multiples, 68 negative non-multiples, 235 translated.
- **Gates.** `gyroid_controls` and `gyroid_shrink_bounds` pass (8 phase probes,
  simplify 9-14 with matching complicate and 5-9 repeated complicate calls; 5 clip
  probes, simplify 19-22 with 13-16 repeated complicate calls); whole
  `gyroid_infill_tdd` 19/19; default and enabled `--all-targets` checks, enabled
  clippy `-D warnings`, `check-literals`, `check-test-quality --report`,
  `check-fmt` and `build-guests --check` (exit 0) clean.

### Extended-profile failure and oracle repair

The first registry runs failed this record at both tiers (routine seed
`13981270879620051270`, extended seeds `3088738101425906196` and
`18422929474420911803`). Both causes were in the oracle; the real clip never
retained material outside its documented band.

1. **Attribution.** The returned cut point sits on the clip boundary a couple of
   units past a subject vertex. The mapper scanned forward and took the *first*
   segment within `CLIP_BAND_UNITS`, which near a vertex is the incoming segment
   clamped at its endpoint, so the cut was mapped to the vertex (`t = 0`) instead
   of its real position on the outgoing segment. In the two retained cases the true
   projection was `0.005115` and `0.00310` while the fabricated value was `0.005245`
   / `0.003396`; the distances to the incoming endpoint were exactly 3.0 and 2.828
   units. The parameter error equals the whole vertex-to-cut distance, which is why
   the excess looked tiny but was structural rather than a band shortfall. Fixed by
   attributing to the nearest segment within a bounded window of adjacent segments
   (ties to the earlier parameter).
2. **Uniform allowance.** The band is perpendicular to the clip edge, but the mapper
   converted it along the subject. A cut point that stays on the edge can slide
   along it: an along-edge slide `L` moves the subject projection by
   `L·|Δparallel|/length²` while its perpendicular distance is
   `L·|Δnormal|/length`, so the parameter slide is
   `(band/length)·max(1, cot θ)`. The third retained case crossed a vertical edge at
   1.1 deg with a true slide of `0.00611` against the old uniform `0.001598`; under
   the final `band / |Δnormal|` term (band 1.0) gives `1/36 ≈ 0.0278` for that case, so
   the observation is about 22% of the bound. Fixed with per-endpoint terms tagged by
   boundary axis, per-interval erode/dilate instead of one uniform delta, and the
   shallow-crossing negative control above.

Replay of all three retained contexts now passes (`accepted_cases` 256 / 64 / 256,
zero rejects), the retained seeds pass through the registry, and three fresh seeds
per tier pass (routine 64 accepted each, extended 256 each) with non-vacuous
counters (extended seed 404: 3,066 raw paths, 150,570 raw segments, 2,392 expected
intervals, 17,140 returned segments, 1,318 retained subjects, 2,381 interior
observations, 501 hole splits, 168 holed cases, 125 rotated cases).

**Recorded observation, not claimed as a retention defect.** In the third retained
case, single-subject `PreparedPolylineClip::clip` returns the cut at
`(18433,-1428)` while `clip_batched` with one unrelated subject added returns
`(18433,-1440)`. Both points are exactly on the inflated boundary (1 unit outside
the true boundary, inside the documented ±2 band) and the retained sliver deviates
only 0.22 units perpendicular to the subject; only the along-edge cut position
differs, because the batch engine quantizes it to a scanline contributed by the
added subject. This retains no material outside the band and does not block this
family. It is flagged for the `crates/slicer-core/src/polygon_ops.rs` owner to
decide whether `clip_batched`'s "same spans as a single-subject clip" wording
should exclude along-edge coordinate equality.

## Step 5 — linker ring walks, connection compatibility and chain origins

File: `modules/core-modules/infill-linker/src/property_tests.rs` (about 4.3k lines,
compile witness removed). No SUT defect found; no production code changed.

- **Ring model.** Every fixture ring is an axis-aligned integer polygon held in a small
  `RingModel` that computes the perimeter, ring start positions, the shortest
  forward/backward arc (ties to `Forward`), the corners a walk passes and a stub's
  corners and end point in exact `i64` arithmetic. Hand-derived literal witnesses pin
  the model before it is used: a 320,000-unit rectangle with a strictly shorter arc in
  each direction and one tie, a notch walk through both reflex corners, a clockwise hole
  ring starting at 400,000, a second component starting at 120,000, and the empty graph.
  `project`, `vertices_between` and the SUT's own walks are never used to build an
  expected value.
- **Connect.** Each output path is scanned for every eligible source and matched
  point-for-point bitwise, forward or reversed; every source must appear exactly once
  and no output may consist only of added points. Points between two matched sources
  are a connector and must pass four independent checks (same ring in the model, the
  independently computed compatibility predicate, a strictly shorter joining arc than
  the anchor, and the model's corner walk). Leading/trailing added points are stubs,
  checked on-ring and no longer than the anchor; their exact length is deliberately not
  predicted because of `DEV-110`.
- **Width boundary.** Compared exactly in `f64` (exact in this width domain): exactly
  `1e-6` and 33 ulps merge, `next_up(2c)` is rejected. Role, one-ulp speed, tool
  `Some(1)` vs `None`, and first/last width mismatches are each rejected individually.
- **Chain.** Chain-only output must be a permutation of the inputs, each possibly
  reversed, with nothing added or lost. For `orchestrate_infill` the test builds its own
  compatibility/ownership table and checks four things: each output's full identity
  exists and outputs come out in prior-region order; unmatched regions pass through
  bit-identical; locked paths appear exactly once and are emitted first; and each linked
  path contains only rows from its own group with an owner equal to the group member
  with the most independently measured row length (ties to the lower region id). Every
  row must be covered exactly once.
- **Derived tolerances.** Grid `2·(max_abs+1)·2⁻²⁴` and stub interpolation
  `16·(max_abs+1)·2⁻²⁴`, from the `i64 -> f32` round trip and the stub's interpolation
  sequence. No observed-output epsilon.

Measured populations (256 cases, 0 rejects): graph 509 queries (109 forward-shorter,
140 backward-shorter, 260 ties, 452 corner walks, 47 reflex, 63 hole, 50
second-component, 509 stubs, 181 clamped, 50 empty graphs); connect 547 sources, 52
merges, 174 separated pairs, 64 junctions, 40 cross-ring pairs, 97 mismatch pairs, 20
strict-anchor cases, 16 `dont_connect` identity cases, 18 short-path discards, 14
multi-path cases; chain 92 chain-only cases (128 reversals, 73 reorderings), 80
chain-connect cases with 17 junctions, 84 orchestrate cases, 15 union pairs, 69
separated pairs, 6 cross-region paths, 5 unequal-majority and 1 tied case, 72
pass-through regions, 41 locked bypasses, 103 legal-chain matches and 450 rows.

`linker_controls` rejects, through the same checkers: a notch walk replaced by a chord,
a same-length shortcut that skips the reflex corners, a hole connector judged against
the outer ring, outer-ring corners offered as a hole connector, a stub missing its
corner, a nonempty model against the empty graph, a dropped source, an unmerged
compatible pair, an interior connector chord, fabricated role/speed/tool/first-width/
last-width merges, a fabricated outer-to-hole merge, a `dont_connect` output with its
1-point path removed, a connector point added in chain-only mode, two merged chain-only
outputs, an origin with the chain reversed or emptied, a merged path re-attributed to
the minority owner, and an altered reversed-chain pass-through region.

### Anchor-boundary oracle repair (M-LINKER-CONNECT)

An extended-tier seed failed with the model expecting the stub branch while
production joined across a 3110-unit shortest arc against a 3110-unit anchor.
Measured from the retained case: the model's exact arc was 3110 and production's
`directed_distance` was `3109.999999999998`, a 2e-12 unit difference produced by
`project_on_ring`'s f64 quotient and product. That is twelve orders of magnitude
below the documented `ARC_EPSILON_UNITS = 1` snap in
`modules/core-modules/infill-linker/src/graph.rs`, so it is an oracle defect, not
a SUT defect. The oracle now derives a one-unit band around the anchor: arcs
strictly closer than `anchor - 1` must join, arcs strictly farther than
`anchor + 1` must separate, and both branches are accepted inside the band with
every per-branch ring-walk, stub and metadata check unchanged. The retained case
is a deterministic control, and replaying its seed now passes at 256 accepted
cases. No tolerance was widened beyond the documented snap.

Recorded gaps: exact ring-walk claims cover only endpoints at least 3 units from a
vertex (production merges arc positions within 1 unit); orchestrate connectors are
checked as axis-aligned and inside the group area rather than as exact walks, because
the offset boundary ring is not modelled independently; and the generated orchestrate
tie population was 0 at 24 cases and 1 at 256, which the deterministic tie witness
covers on every run.

Gates: `linker_controls` and `linker_shrink_bounds` pass; `connect_tdd` 8/8,
`anchor_length_tdd` 8/8, `orchestrate_tdd` 15/15 and `graph_tdd` 3/3; default and
enabled `--all-targets` checks, enabled clippy `-D warnings`, `check-literals`,
`check-test-quality --report` and `check-fmt` clean. One review nit (a header comment
calling `graph.rs` "`crates`-external") was corrected by the planner.

## Step 6 — wave trim, deposited capsule and sampled coverage

File: `modules/core-modules/wave-overhangs/src/property_tests.rs` (1,691 lines,
compile witness removed). No SUT defect found; no production code changed.

### Fixture deviation from the plan's literal direct-kernel translation

The plan said to adapt the existing `seam_fixture` into a direct `generate` call
with the same edge-flush support bands. Measured, that input produces **no
wave**: `generate` clips `lower` to the fill bbox before deriving anchors, so the
edge-touching bands leave `anchors` at the degenerate 1-unit clip strip and the
run returns `paths = []`, `fallbacks = [MissingAnchors, EmptyOutput]`
(`step6/probe-fixture.log`, variant "A literal"). This is a fixture-authoring
defect in the plan, not a production defect: `run_infill` always passes
`external_bridge U anchor_band`, so the generator never receives edge-flush
support in production.

Resolution: the fixture keeps the same `P`, `r`, `T` and rectangle, and extends
each support band 0.5 mm into the bridge (unsupported span still 5 mm). That
gives a positive-area anchor set while `inset_anchors` stays empty, so the cover
construction reduces analytically to the rectangle: `min_length = 0` ->
`filtered = [0,6]^2`; `anchor - offset(overhang, 1.84 mm) = empty` ->
`inset_overhang_area = filtered`; `perimeter_overlap = 0` -> cover = piece;
`minimum_width = 0` -> no narrow split. Therefore `P = [0,6] x [0,6]` mm,
`r = (0.4 * 1.875) / 2 = 0.375` mm, `T = [0.375, 5.625]^2` mm, pinned in
`wave_controls`.

### Derivation and allowance

- The sampled domain keeps `P` fixed and varies line spacing, flow width, flow
  ratio, pattern and iteration bound. `perimeter_overlap`, `minimum_width`,
  `min_length` and `base_spacing` never vary inside the property because they
  would change `P`.
- Centerline boundary: `CLIP_BAND_UNITS = 2` (documented `PreparedPolylineClip`
  ±2-unit band) + `TRIM_QUANTIZATION_UNITS = 2` (integer snap of the integer-unit
  trim offset) = 4 units = 0.0004 mm, plus a `1e-9` mm rational comparison
  epsilon. The capsule uses the same 4-unit outward band. `T` is rectangular and
  therefore convex, so endpoint containment proves the whole centerline lies in
  `T`, and the radius-`r` capsule then lies in `P`; the capsule is never required
  to fit inside `T`.
- The derived 0.0004 mm is 625x smaller than the 0.25 mm gap the nominal-radius
  negative control relies on, so it cannot mask that control.

### Checks

- Determinism pin (explicit `docs/22_test_quality.md` waiver, not a correctness
  oracle): identical inputs are passed to `generate` twice and every path point,
  filled contour/hole and fallback is compared in order. One-unit path-point,
  one-unit `filled`-contour and added-fallback mutations each break the pin.
- Whole-capsule containment uses one checker for the property and for the exact
  charter controls: `P = [0,10]^2`, `r = 1`, `T = [1,9]^2`, centerline
  `(1,2) -> (1,8)` is **accepted** (its capsule touches `P` at `x = 0`), and the
  same input is **rejected** by a `capsule subset T` checker, which is exactly
  the double-inset false rejection the control guards. The nominal-width input
  `(0.75,2) -> (0.75,8)` with nominal width 1 mm and `flow_ratio = 2` (deposited
  `r = 1`) reaches `x = -0.25` mm and is rejected at the deposited radius while
  being accepted at the mistaken nominal radius `0.5`.
- Coverage is a declared 96x96-centre lattice over `P` (9,216 cells, 0.0625 mm
  pitch). A cell is empty when farther than `r + 0.1` mm from every centerline,
  where 0.1 mm is half the existing seam fixture's 0.2 mm defect width. Empty
  cells 4-connected to the lattice border are boundary slivers; any other empty
  cell is an interior gap. Guaranteed detection: axis-aligned voids at least
  0.2625 mm wide and one pitch long. This is labelled sampled coverage, not a
  continuous-area proof, and it is never applied to `filled` (whose
  regularization can grow it).
- Removed-wave mutation: deleting the seam-closing pair at 0.35 mm spacing leaves
  692 interior gap cells, and deleting a middle segment of one front at 0.7 mm
  spacing leaves 39, both with containment still valid. The file records the
  subtlety that removing a *single* 0.35 mm front is not a coverage defect
  (`2 * 0.35 = 0.7 < 2r = 0.75`), which is why the middle-segment control uses the
  wider-spaced fixture.
- Fallback fixtures: empty `infill_area` -> `[NoOverhang]` with no paths or
  `filled`; 1 mm square with `min_length = 5` -> `[MinLengthFiltered]` with no
  output; no lower support -> `MissingAnchors` (+ `EmptyOutput`) with no paths;
  one-iteration cap -> `[IterationResidual]` with nonempty paths and `filled`, so
  paths, filled and fallbacks coexist. A false fallback on the unbounded class is
  rejected.

### Measured profile

| cases | pilot wall (cargo + libtest) | accepted | nonempty path cases | full/capped | coverage cases | segments | interior samples | interior empty |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 16 (pilot) | 14.03 s | 16 | 16 | 7 / 9 | 7 | 1,076 | 116,168 | 0 |
| 32 (routine) | 21.73 s | 32 | 32 | 15 / 17 | 15 | 2,079 | 238,857 | 0 |
| 64 | 44.61 s | 64 | 64 | 35 / 29 | 35 | 4,125 | 487,169 | 0 |
| 128 (extended) | 86.80 s | 128 | 128 | 65 / 63 | 65 | 7,945 | 955,248 | 0 |

The record therefore uses `cases = 32` routine and `128` extended: this property
is the expensive one (lattice coverage over every emitted segment), and the two
repeated pilots at both counts produced byte-identical counters. The registry lanes
on the final tree measured 18.36 s of libtest at 32 routine cases and 64.74 s at 128
extended cases (see the Step 8 table); the pilot rows above include cargo startup.

Gates: `wave_controls` and `wave_shrink_bounds` pass (six mode x pattern probes,
simplify 26-32 with matching complicate, `exhausted = true`); whole
`wave_overhangs_tdd` 13/13; default and enabled `--all-targets` checks, enabled
clippy `-D warnings`, `check-literals`, `check-test-quality --report` and
`check-fmt` clean.

## Step 7 — `M-IRONING-ZIGZAG`: ironing intervals and every extruded connection

File: `modules/core-modules/top-surface-ironing/src/property_tests.rs` and the focused
kernel fix in `.../src/lib.rs`; regression in
`.../tests/top_surface_ironing_emission_tdd.rs`.

- **Exact fixture grid.** Every generated coordinate and spacing is a multiple of
  0.25 mm, so `units_to_mm` produces exactly representable `f32` millimetres and
  `y_min + i * spacing` stays exact; the independent model is derived in `f64` and
  compared with `64·f32::EPSILON·(max_abs_mm + span_mm + 1.0)`, twice the largest
  allowed row count. Containment is exact integer arithmetic.
- **Oracle.** The expected row grid is `floor(span/spacing)+1` half-open rows from the
  bbox, and the expected X intervals come from fixture-edge arithmetic (rect one; notch
  two above the notch floor; hole two between the hole's bottom and top edges), reversed
  on odd rows. The complete expected point sequence is compared element-wise, and every
  consecutive segment of every returned path is checked with the boundary-inclusive
  `oracles::segment_in_region` — the check that `chunks(2)` would miss.
- **Required witnesses.** The 8 mm square with hole `[3,5]×[2,6]` mm at 1 mm spacing
  produces nonempty rows on both sides of the hole with no segment through its interior;
  a U-notch retains both arms. A generated hole/notch case always splits at least one
  row because the cut's bottom edge is placed exactly on an interior scan row.
- **Mutations rejected by the same checker:** a gap-connector merge of a same-row
  left/right path pair, a dropped row, a false-empty output, a mutated flow factor, and
  a synthetic single-row shortcut whose endpoints and midpoint are all on material.
- **Empty modes** are separate: empty contour (no bbox), zero-height, zero-width,
  nonpositive spacing, and a positive sub-spacing span which legitimately yields exactly
  one row.

### Confirmed defect and focused fix

The pre-fix kernel concatenated every row's intervals into ONE `ExtrusionPath3D` per
polygon. On the deterministic holed fixture the implicit join between the two intervals
of the row at `y = 3` mm was an extruded segment `(5,3) → (3,3)` mm: both endpoints lie
on material, but the open segment crosses the hole interior. Reproduced by stashing
`src/lib.rs` and re-running the new regression, which then failed with exactly
`ironing segment (5, 3) -> (3, 3) mm leaves the top-fill region`
(`step7/prefix-regression.log`).

The kernel now starts a new path whenever the joining move from the last emitted point to the
next interval's start would leave the region (`segment_stays_inside`: split the move at every
edge meeting and require each open sub-interval's midpoint to be inside, boundary counted as
inside). Straight and convex boundaries therefore keep a single snake, while a hole, a notch, or
a concave/slanted boundary that cuts the move starts a new path. Two slanted controls pin the
rule: a right triangle stays one path (its joins run along the hypotenuse) and a notch whose kink
sits inside one row spacing splits, both with every segment in-region. The no-RNG regression
`hole_crossing_regression_no_ironing_segment_enters_the_hole` asserts both the exact
`Point2`-unit `segment_in_region` containment and the sharper "no midpoint strictly
inside the hole" property, and fails on the old kernel.

Measured: 64 cases = 1.09 s with 1,066 points and 884 whole-path segments over 23
rectangle / 14 notch / 27 hole cases and 118 split rows; 256 cases = 3.71 s with 4,100
points and 3,412 segments. Controls and the shrink probe (154 simplify/complicate steps
per shape, false-return and repeated-complicate coverage, plus a real reduction from a
nonminimal start) pass. Whole `top_surface_ironing_emission_tdd` is 14/14 including the
new regression, and `slicer_module_binding_tdd` is 1/1.

D-209 entry: `D-209-IRONING-FILL-BEHAVIOUR-CHANGE` now also records the path split.

## Step 8 — measured profiles, registry execution and closure

All nine records are `target_kind = "lib"`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`, `isolation = "in_process"`,
`rng_algorithm = "chacha"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, and zero
local/global/flat-map rejection budgets (every generator below is total, so a
rejection is a generator change and aborts as `exhausted` rather than being
absorbed).

### Registered bounds and measured wall times (Windows)

Wall times are libtest wall times from the selected registry lanes under the real
profiles, separate from compilation.

| Record | routine cases | routine wall | extended cases | extended wall | registered bounds |
|---|---:|---:|---:|---:|---|
| `M-RECTILINEAR-SCAN` | 64 | 14.05 s | 256 | 53.85 s | `max_abs_units=200000, max_vertices=8, max_holes=1, max_components=1, max_shrink_candidates=16, max_spacing_units=20000, max_rows=32` |
| `M-RECTILINEAR-SPACING` | 64 | 0.04 s | 256 | 0.13 s | `max_width_units=200000, max_spacing_units=20000` |
| `M-GYROID-PHASE` | 64 | 0.04 s | 256 | 0.11 s | `max_phase_milliradians=6284, max_grid_units=512` |
| `M-GYROID-CLIP` | 64 | 0.46 s | 256 | 0.98 s | `max_abs_units=524288, max_vertices=32, max_holes=4, max_components=2, max_shrink_candidates=32, max_width_units=6000, min_density_ppm=100000, max_phase_milliradians=6284, max_wave_points=65536` |
| `M-LINKER-GRAPH` | 64 | 0.08 s | 256 | 0.24 s | `max_abs_units=200000, max_vertices=16, max_holes=2, max_components=2, max_shrink_candidates=4096` |
| `M-LINKER-CONNECT` | 64 | 0.09 s | 256 | 0.31 s | `max_abs_units=200000, max_paths=4, max_points=5, max_anchor_units=100000` |
| `M-LINKER-CHAIN` | 64 | 0.16 s | 256 | 0.37 s | `max_abs_units=400000, max_paths=4, max_points=4, max_regions=4, max_anchor_units=100000` |
| `M-WAVE-TRIM` | 32 | 21.12 s | 128 | 95.28 s | `max_abs_units=120000, max_spacing_units=6000, max_width_units=12500, max_iterations=3, max_cells=9216` |
| `M-IRONING-ZIGZAG` | 64 | 0.53 s | 256 | 1.98 s | `max_abs_units=80000, max_vertices=16, max_holes=2, max_components=1, max_shrink_candidates=16, max_spacing_units=15000, max_rows=64` |

`M-WAVE-TRIM` ships lower case counts than the others because it is the only
record whose cost is dominated by lattice coverage over every emitted segment; the
16/32/64/128-case pilots produced byte-identical counters at repeated runs, so the
reduction buys CI time without changing the observed case mix.

### Execution

Every lane below is a real registry run (`cargo xtask test --properties`), and
every record's accepted-case count was read from its controller `end` event:

| Lane | Result | Nine-record accepted counts |
|---|---|---|
| selected routine (nine `--property` flags) | `VERDICT: PASS`, 9/9 end events, 0 failed | 64 each except `M-WAVE-TRIM` 32 |
| selected extended | `VERDICT: PASS`, 9/9, 0 failed | 256 each except `M-WAVE-TRIM` 128 |
| full registry routine (no filter) | `VERDICT: PASS`, 0 failed records | same counts as selected routine |
| full registry extended (no filter) | `VERDICT: PASS`, 0 failed records | same counts as selected extended |

The full-registry lanes are exactly what `.github/workflows/ci.yml`'s
`properties-windows` job runs (`cargo xtask test --summary --properties --tier
routine`), and the nine IDs appear in that lane's log, so the job discovers and runs
them dynamically. After independent review, that job's existing hard-coded
assertion list was extended from 13 to 22 IDs so a silently dropped record turns
the lane red; it is still one roster, not a second mechanism, and the lane still
executes every other selected record. Failure artifacts are already uploaded by the
job's existing `properties-windows-failure` step (`target/test-output.log` and
`target/property-testing/`).

Retained-seed replay: `--property M-GYROID-PHASE --tier routine --seed 42` was run
twice. Both runs selected seed `42` and 64 cases, both emitted begin/end controller
events with `outcome = passed` and `accepted_cases = 64`, and their
`observations.jsonl` files are identical once the process id is stripped (128 records each:
64 `started` plus 64 `passed`, same inputs in the same order).

### Manual FRU / test-quality review, docs and gates

The manual watched-type FRU and test-quality review is recorded above and in each
owner file header; the two gates do not scan `modules/**/src`, so a green run is
not evidence for those files.

Docs checks (all commands exit 0 and the required greps match):
`docs/testing/properties.md` contains all nine charters with the full field set
(`Stable property ID`, `Owner`, `Contract`, `Domain`, `Strategy`, `Oracle`,
`Tolerance`, `Shrinking`, `Execution`, `Failure record`, `Acceptance`);
`docs/05_module_sdk.md` documents the owner-module placement and the scanner blind
spot; `docs/21_data_defaults_and_fixtures.md` §7.1 already carried the blind spot
and manual-review requirement and was retained.

### Final gates (single run on the closed tree)

| Gate | Result |
|---|---|
| `cargo build --workspace` | exit 0 |
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo xtask check-literals` | 0 violations in 0 files (watchlist 196 types) |
| `cargo xtask check-test-quality --report` | 0 findings in 0 files |
| `cargo xtask check-fmt` | 62 packages clean |
| `cargo xtask check-deviations` | open map already current (72 open) |
| `cargo xtask build-guests --check` | exit 0 (fresh; the test-only edit to `gyroid-infill/src/property_tests.rs` marked its guest stale once, `cargo xtask build-guests` rebuilt exactly that guest, and the re-check exited 0) |
| Docs impact greps | all three match (see below) |

The ten owner controls, the eleven pre-existing witnesses plus the two new
regressions, and the three prerequisite controls were all re-run on this tree and
exited 0 (41 narrow commands archived under `target/proptest-infill-evidence/final/`).
`cargo xtask check-literals` and `check-test-quality` report zero *scanned* files
because module-source tests are outside their scope; the manual review above is the
evidence for those files, not these two lines.

Docs impact, verbatim from the packet:
`rg -n 'M-RECTILINEAR-SCAN|M-GYROID-CLIP|M-LINKER-CONNECT|M-WAVE-TRIM|M-IRONING-ZIGZAG' docs/testing/properties.md`
matches all five IDs; `rg -n 'Windows|source_revision|M-IRONING-ZIGZAG|FRU|shrink|tolerance' docs/testing/proptest-infill-evidence.md`
matches every token; `rg -n 'property_testing|manual|src/property_tests' docs/05_module_sdk.md`
and `rg -n 'manual FRU|property_tests' docs/21_data_defaults_and_fixtures.md` both
match in their retained sections.

### Post-review documentation pass

The independent full review returned **APPROVED WITH NOTES** with six low-severity
notes, all documentation or justification rather than behaviour: two stale comments in
the gyroid clip file (the superseded `cot θ` formula and the attribution window name),
one figure in this file (`a few percent` -> the measured 22% of the bound), the
reconciliation of the 1-unit boundary band with the 2-unit attribution window, and the
reworded anchor-band and `cot θ` justifications in deviations 7 and 10 (the anchor band
is an equality tolerance for off-by-one arc rounding, recorded as such, not derived
from `ARC_EPSILON_UNITS`). Every change in that pass is a comment or Markdown; no code
path, test assertion, tolerance value or registry entry changed. After it, the gyroid
controls, the gyroid shrink probe, both crates' clippy, formatting, literal,
test-quality, deviation and guest-freshness gates were re-run and all exited 0.

### Defect disposition

**Two confirmed SUT defects**, each with a focused canonical-correct fix and a
named no-RNG regression:

1. `adjust_solid_spacing` (`rectilinear-infill`) — over-cap branch reset to
   `distance`, bare `width`, `.round()`. Fix + regression
   `solid_spacing_over_cap_uses_canonical_1_2_factor`;
   `D-209-ADJUST-SOLID-SPACING-DIVERGENCE` closed.
2. `generate_zigzag_strokes_for_polygon` (`top-surface-ironing`) — one concatenated
   path per polygon crossed holes and notches. Fix + regression
   `hole_crossing_regression_no_ironing_segment_enters_the_hole`;
   `D-209-IRONING-FILL-BEHAVIOUR-CHANGE` extended.

**Two oracle defects repaired** (neither a SUT defect): `M-GYROID-CLIP`'s
first-segment attribution and uniform `band/length` allowance (Step 4), and
`M-LINKER-CONNECT`'s strict anchor comparison at the documented 1-unit arc snap
(Step 5). No production code changed for either. The clip band constant was
*lowered* (3.0 to 2.0 to 1.0) as the derivation was corrected, never raised, and
the linker band was derived from `ARC_EPSILON_UNITS`.

No other defect was confirmed by any property, control, shrink probe or pilot.

## Manual watched-type FRU / test-quality review

`cargo xtask check-literals` and `cargo xtask check-test-quality` do **not** scan
`modules/core-modules/*/src/**`, and neither follows an out-of-line
`mod property_tests;` (`docs/21_data_defaults_and_fixtures.md` §7.1). Both gates were
green during this run, which is **not** evidence for the five new files. The review
below was done by hand against the re-derived watchlist rule (`pub` struct, at least
five named fields, defined under `crates/*/src/**`); it is the mandatory manual pass,
not a gate result.

| File | Watched-type FRU sites | `// exhaustive:` waivers | `// test-quality:` waivers |
|---|---|---|---|
| `rectilinear-infill/src/property_tests.rs` | `ExtrusionPath3D` (FRU over a cloned real path) | `GeometryBounds` ×2 and `GeometryDomain` ×1, each spelling every measured field | none needed — every control feeds a deliberately wrong output through the same checker |
| `gyroid-infill/src/property_tests.rs` | none: `Point3WithWidth` and `ExtrusionPath3D` are produced by the SUT only; `Point2`/`Polygon`/`ExPolygon` are below the threshold and `GyroidInfill` is under `modules/` | none | mutation control and shrink probe (both record what they guard) |
| `infill-linker/src/property_tests.rs` | `Point3WithWidth` (FRU over `Default::default()` and over a copied point), `ExtrusionPath3D` (FRU over `extrusion_path3d_base`), `InfillRegion` (FRU over `Default::default()`) | none | none |
| `wave-overhangs/src/property_tests.rs` | `WaveParams` (FRU over `fixture_params()` for one override), `WaveOutput` (FRU over `Default::default()`) | `WaveParams` ×1 spells every field | determinism pin (the `docs/22_test_quality.md` exception, explicitly not a correctness oracle) |
| `top-surface-ironing/src/property_tests.rs` | `Point3WithWidth` (FRU over `Default::default()`), `ExtrusionPath3D` (FRU over `extrusion_path3d_base`) | `GeometryBounds` ×2, every field explicit | none needed |

Private module-local case/limit/report structs are not watched types. No production
struct literal was added by any of the five files, and no watched-type literal was
rewritten.

Test-quality review, by hand, against `docs/22_test_quality.md`:

- No property relies on a self-referential oracle: the five owners either compute an
  independent model (arithmetic interval sets, `RingModel`, exact rational clip
  intervals, `div_euclid`) or observe the SUT with an explicitly labelled determinism
  pin (`M-WAVE-TRIM`'s repeat comparison) that is never presented as correctness.
- Every control feeds a deliberately wrong result through the **same** checker the
  property uses and asserts rejection; none is a decorative or success-only assertion.
- Every required positive population is counted per class and asserted after `check`; a
  vacuous loop over empty output cannot satisfy the counters.
- No test greps source text, and no fixture is skipped silently.
