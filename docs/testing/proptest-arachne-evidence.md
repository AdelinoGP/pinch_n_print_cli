# Property-testing evidence: Arachne host and perimeter kernels

Packet: `proptest-openrscad_10_pbt-arachne`. This document is the execution evidence surface for the
three host records (`H-ARACHNE-BEADING`, `H-ARACHNE-GRAPH`, `H-ARACHNE-PIPELINE`) and the three
module-owned records (`M-ARACHNE-CLASSIFY`, `M-ARACHNE-BUILD`, `M-ARACHNE-SEQUENCE`), and the
G-FAMILY disposition for the Arachne family (G-ARACHNE below).

Every number below comes from an archived log under `target/proptest-arachne-evidence/`,
`target/proptest-arachne-baseline/` or `target/proptest-arachne-acceptance/`, from a per-record
`context.json`, or from the run report of this packet's Step 7 where a figure was measured but not
archived. Each table says which. No hosted CI run is claimed.

## Provenance

- **Baseline revision.** `a35a8fb38158395efaa698ac3fedd79f225ceee1`, the packet's starting point.
- **Dirty-content note.** This packet's work was measured on a dirty tree. The per-record
  `context.json` under `target/property-testing/<run-token>/<ID>/` is authoritative for each
  measurement's exact `source_revision`, `dirty_paths`, `source_state_digest` and
  `cargo_target_dir`. This document does not freeze a workspace-wide dirty digest, because it
  changes with every edit. Example: the retained-seed replay's context
  (`target/property-testing/29d43e1e20aa98fce6670d40b9413df3/H-ARACHNE-BEADING/context.json`)
  records `source_revision = da263b14ad78755f66a2f9cc758145ea00c9045b`, `tier = routine`,
  `seed = 14268583537219271429` and `dirty_paths = [".github/workflows/ci.yml",
  "docs/spec_packets/proptest-openrscad_10_pbt-arachne/packet.spec.md", "test-properties.toml"]`.
- **Toolchain and host.** Windows, `rustc 1.96.0 (ac68faa20 2026-05-25)`,
  `cargo 1.96.0 (30a34c682 2026-05-25)`, `dev` profile, default workspace `target/`.
- **Features.** Host records: `["slicer-core/host-algos", "slicer-core/property_testing"]`.
  Module records: `["arachne-perimeters/property_testing"]`. Both owner features are empty
  selection features. Proptest and the SDK `property_testing` edge are dev-dependencies only.
- **Baseline outcomes.** These are the affected existing targets, captured before the production
  edits and archived as `target/proptest-arachne-baseline/<name>.log` by the retained launcher
  `target/proptest-arachne-baseline/run.sh`:

  | Command (`cargo test -p <pkg> [--features host-algos] --test <target>`) | Log | Result |
  |---|---|---|
  | `slicer-core --features host-algos --test arachne_pipeline` | `pipeline.log` | 9 passed, 0 failed |
  | `slicer-core --features host-algos --test arachne_invariants` | `invariants.log` | 6 passed, 0 failed |
  | `slicer-core --features host-algos --test arachne_beading_propagation_delta_bound` | `propagation_delta.log` | 1 passed, 0 failed |
  | `slicer-core --features host-algos --test arachne_construction_epilogue` | `construction_epilogue.log` | 2 passed, 0 failed |
  | `slicer-core --features host-algos --test arachne_local_maxima_single_beads` | `local_maxima.log` | 1 passed, 0 failed |
  | `slicer-core --features host-algos --test beading_distributed` | `beading_distributed.log` | 3 passed, 0 failed |
  | `slicer-core --features host-algos --test beading_limited` | `beading_limited.log` | 4 passed, 0 failed |
  | `slicer-core --features host-algos --test beading_widening` | `beading_widening.log` | 1 passed, 0 failed |
  | `arachne-perimeters --test wall_sequence_commit_tdd` | `wall_sequence_commit.log` | 3 passed, 0 failed |
  | `arachne-perimeters --test wall_width_emission_tdd` | `wall_width_emission.log` | 2 passed, 0 failed |
  | `arachne-perimeters --test arachne_parity_is_bridge_flag_tdd` | `bridge_flag.log` | 1 passed, 0 failed |

  The same eight `slicer-core` baselines were re-captured after the production fixes
  (`target/proptest-arachne-acceptance/C-base-*.log`) with identical pass counts. The
  `arachne_local_maxima_single_beads` count stays at 1, but its fixture expectation changed. See
  Defects item 2.
- **Guest freshness.** The first gated command after the production edits rebuilt the stale guests.
  See Measurements. A following `cargo xtask build-guests --check` returned exit `0`
  (`EXIT_FRESH`), judged by exit code, and later runs did not rebuild. The final closure gate log
  `target/proptest-arachne-evidence/closure-final-guests.log` records `build-guests --check`
  `GUESTS_EXIT=0`; the gate logs for build, check, clippy, formatting, literals, test quality and
  the owner-feature checks all record exit `0`.

## Profiles

All six records use the standard shape. Routine and extended values are given as routine / extended.

| Key | Value |
|---|---|
| `cases` | 64 / 1024 |
| `max_local_rejects`, `max_global_rejects`, `max_flat_map_regens` | 0 / 0 |
| `max_default_size_range` | 32 / 256 |
| `max_shrink_iters` | 64 / 256 |
| `max_shrink_time_ms` | 0 / 0 |
| `rng_algorithm` | `chacha` |
| `isolation` | `in_process` |
| `timeout_ms` | 0 |

Family bounds (committed in `test-properties.toml`):

| Record | bounds (routine) | bounds (extended) |
|---|---|---|
| `H-ARACHNE-BEADING` | `max_beads = 8`, `max_width_units = 20000`, `max_excess_units = 8000` | `16`, `40000`, `16000` |
| `H-ARACHNE-GRAPH` | `max_abs_units = 400000`, `max_vertices = 24`, `max_holes = 2`, `max_components = 3`, `max_shrink_candidates = 64` | `1000000`, `32`, `3`, `4`, `256` |
| `H-ARACHNE-PIPELINE` | the five graph keys at routine values plus `max_beads = 10` | the five graph keys at extended values plus `max_beads = 16` |
| `M-ARACHNE-CLASSIFY` | `max_inset_idx = 8` | `32` |
| `M-ARACHNE-BUILD` | `max_junctions = 32`, `max_abs_mm_units = 20000`, `max_spacing_units = 2000`, `max_height_units = 500` | `64`, `40000`, `8000`, `2000` |
| `M-ARACHNE-SEQUENCE` | `max_walls = 6` | `12` |

The zero rejection budgets are honest because every generator is construction-first and total. A
rejection would show that a generator changed, and the run would abort as `exhausted`. Each family
validates its bounds before it builds a strategy and rejects zero, non-representable, overflowing
and impossible-witness values. Those checks are pinned by `arachne_property_domain_rejections` and
`property_tests::arachne_module_domain_rejections`.

**`M-ARACHNE-BUILD` bounds.** The five-line roster needs **23** junctions before sampling, so the
committed `max_junctions` values are `32` routine and `64` extended: above the generator's minimum
and at or below its `23..=64` accepted range. The table above, `test-properties.toml` and the
charter in `docs/testing/properties.md` agree. Both the junction budget and the coordinate extent
drive generation rather than sitting unused; see
[M-ARACHNE-BUILD generated domain](#m-arachne-build-generated-domain).

## M-ARACHNE-BUILD generated domain

Both coordinate and junction bounds now drive generation; neither is a fixed fixture constant.

- **Half-extent.** `half_units` is sampled from `10_000..=max_abs_mm_units` (1 mm up to the declared
  bound). Every roster shape is an exact integer-unit fraction of it: the closed outer rectangle is
  `±half_units`, the closed inner rectangle `±half_units / 2`, the mixed bridge band spans `x` from
  `-3 * half_units / 8` to `+5 * half_units / 8` and `y` within `±5 * half_units / 8`, both open
  chains sit within `±half_units / 4` (inside the band), and the thin-wall rectangle is
  `±half_units / 10` (inside the band). The band is deliberately asymmetric: it contains the thin
  rectangle and both open chains whole, but of the closed inner rectangle it covers the two
  positive-x corners and neither negative-x corner, so a single closed line carries both
  `is_bridge` values.
  `MIN_ABS_MM_UNITS` is `10_000` units (1 mm); the smallest shape coordinate is therefore 1 000
  units (0.1 mm), the thin-wall rectangle's `half_units / 10`. The comparator asserts at every case
  that the observed coordinate extent is exactly the sampled `half_units`.
- **Junction total.** The five-line roster needs 23 junctions (three closed lines at four corners
  plus an explicit repeat = 15, and two open four-junction chains = 8). `max_junctions` is
  therefore accepted only in **23..=64**. Each case samples a total from `23..=max_junctions`; the
  shortfall between that total and the mode's roster count becomes extra collinear junctions on
  the outer rectangle's bottom and top edges, and the comparator asserts the input total equals the
  sampled total, equals the emitted point total, and stays within the bound.
- **Emitted counts.** For an accepted profile with bound `N`, every case emits a total in
  `23..=N`; the minimum is always 23, the maximum is the profile's own `max_junctions`. The
  deterministic control `property_tests::build_walls_bounds_drive_geometry_and_junctions` measures
  the two extremes it constructs directly: **23** junctions at `max_junctions = 23` and
  `half_units = 10_000`, and **32** junctions at `max_junctions = 32` and
  `half_units = 60_000`. The same control sweeps `max_junctions` 23, 32 and 64 at half-extents
  10 000, 60 000 and 1 000 000 units and both ends of each junction range.
- **Measured runs (synthetic context, not a registry lane).** With a hand-fed
  `PNP_PROPERTY_CONTEXT`, the build property passed 64/64 accepted cases at
  `max_junctions = 26`, `max_abs_mm_units = 40_000` (`target/logs/bounds/property-build-manual.log`)
  and 256/256 at the ceilings `max_junctions = 64`, `max_abs_mm_units = 1_000_000` with zero
  rejects (`target/logs/bounds/property-build-extended-manual.log`). The same harness rejects
  below-roster `max_junctions` values before sampling (`target/logs/bounds/property-below-roster.log`,
  `target/logs/bounds/property-extended-below-roster.log`). Registry timings on the final tree are
  0.33 s routine selected, 0.33 s combined routine, 5.90 s extended selected and 5.18 s combined
  extended (Measurements).
- **Flag-desync witness.** With `feature_flags.reverse()` removed from `build_walls`, the generated
  `M-ARACHNE-BUILD` property fails on the asymmetric band:
  `inset_idx=1 is_odd=false is_closed=true point 1 (source junction 3): is_bridge = false, but this
  point's own coordinates say true` (`target/logs/bounds/postfix-band-mutant-full.log`). The same
  tree passes after the reversal is restored
  (`target/logs/bounds/postfix-band-registry-restored.log`). This detection comes from the
  generated property, not from the deterministic regression.
- **Roster minimum fixture.** `valid_build_limits()` now derives from the real validator at
  `max_junctions = 23`, `max_abs_mm_units = 20_000`, `max_spacing_units = 10_000`,
  `max_height_units = 3_000`, so the deterministic witnesses cannot drift out of the accepted
  domain. The reverse-trigger regression keeps its own right-half bridge band, which fixes the
  closed inner rectangle's population at `(2 outside, 3 inside)`; the generated band is asymmetric
  in the same two-corner way, but its extent follows the sampled half-extent.

### `M-ARACHNE-CLASSIFY` shrink behaviour

The zero/positive inset choice is a tuple, not a union: `(bool, 1u32..=max_inset_idx)` maps the
boolean `true` to inset 0 and `false` to the positive draw. Proptest simplifies `bool` only
`true -> false`, and `false` is the positive row, so a positive-inset case's selector is already at
its shrink floor and its inset can only shrink inside `1..=max_inset_idx`. A `prop_oneof!` union
would instead be free to simplify an accepted positive case onto its `Just(0)` alternative. The
deterministic control `classify_shrink_retains_positivity` (inside
`property_tests::arachne_module_oracle_controls`) walks real strategy trees and asserts that no
positive inset ever crosses to zero and that a zero start may become positive. Expected
classifications are re-derived from the case that is actually checked, so no stale expectation can
survive either shrink direction.

## Canonical audit

- **Sources.** The local `OrcaSlicerDocumented` snapshot at revision `3e1daccd7c`, and a second
  checkout at v2.5.0, revision `40ecc66f78`. The seven seam bodies below are identical in both
  checkouts apart from includes.
- **Seams checked (file + function).**
  - `WideningBeadingStrategy::compute` and `WideningBeadingStrategy::getTransitionThickness`
    (`WideningBeadingStrategy.cpp`)
  - `SkeletalTrapezoidation::generateSegments` and `SkeletalTrapezoidation::interpolate`
    (`SkeletalTrapezoidation.cpp`)
  - `LimitedBeadingStrategy::compute` (`LimitedBeadingStrategy.cpp`)
  - `SkeletalTrapezoidation::generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp`) and
    `ExtrusionLine::ExtrusionLine` (`ExtrusionLine.cpp`)
  - `WallToolPaths::stitchToolPaths` (`WallToolPaths.cpp`)
  - `BeadingStrategyFactory::makeStrategy` (`BeadingStrategyFactory.cpp`)
- **Outcomes.**
  - Match canonical: `populate_beading_propagation`
    (`crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`) and `interpolate_beading`;
    `stitch_extrusions` against `stitchToolPaths`; the factory stack order and granularity.
  - Differed, now fixed: the widening gate, local-maximum closure, and the clamp in
    `LimitedBeadingStrategy::get_transition_thickness`. See Defects and regressions.
  - Diagnostic-only difference, not changed: `populate_beading_propagation` does not reproduce
    canonical's `total_thickness == 2 * distance_to_boundary` diagnostic and checks array-length
    parallelism instead. Behaviour is the same. The difference is recorded here, not fixed.
- **Not adjudicated.** `WallToolPaths::getRegionOrder` differs between the two checkouts. No
  property here depends on it: `M-ARACHNE-SEQUENCE` covers only the controlled single-outer batch.

### Wall winding: `traverse_extrusions` and `ExtrusionLine::is_contour`

Audited because AC-5 asserts open-path direction. Sources: `traverse_extrusions`
(`PerimeterGenerator.cpp`) and `ExtrusionLine::is_contour` (`Arachne/utils/ExtrusionLine.cpp`),
read in the local `OrcaSlicerDocumented` snapshot.

- **Canonical predicates.**
  - Only a closed line is reoriented. In the `extrusion->is_closed` branch,
    `(wall_direction == CounterClockwise) == (pg_extrusion.is_contour || pg_extrusions.size() == 2)`
    selects `make_counter_clockwise()`, otherwise `make_clockwise()`. The open-line branch builds
    `ExtrusionMultiPath`s in input order and never reorients.
  - `is_contour` is `ExtrusionLine::is_contour`: `false` for an open line, otherwise
    `Polygon(junctions).is_clockwise()` ("Arachne produces contour with clockwise orientation and
    holes with counterclockwise orientation"). It is computed in the
    `ordered_extrusions` construction inside `process_arachne` (`PerimeterGenerator.cpp`) from the
    line's own input junctions, and stored on each `PerimeterGeneratorArachneExtrusion`.
  - `pg_extrusions` is the ordered per-surface list. `process_arachne` skips empty
    `VariableWidthLines` and `WallToolPaths` already removed empty lines with
    `removeEmptyToolPaths`.
- **Measured PnP input winding.** A temporary native probe (removed again) ran
  `generate_arachne_walls` on a 20 mm square with a 6 mm square hole. The outer-contour family
  came out clockwise (signed areas −324.0, −353.4, −384.2 mm²) and the hole family
  counter-clockwise (+63.1, +51.5, +40.9 mm²). PnP's Arachne output therefore has the winding
  that canonical `is_contour` assumes.
- **PnP divergence found.** `ArachnePerimeters::build_walls`
  (`modules/core-modules/arachne-perimeters/src/lib.rs`) reoriented every line with three or more
  points, open lines included, toward a target taken from `boundary_type`. In practice inset 0
  was treated as the contour and every inset above 0 as a hole. As a result, open lines were
  reoriented, inner insets of the outer contour were wound as holes, a hole's inset-0 wall was
  wound as a contour, and the two-line term was missing.
- **Fix.** `build_walls` takes the input winding from the source junction XY before any
  mutation, sets `is_contour_line = line.is_closed && input_clockwise` and
  `region_line_count` = the batch's nonempty lines, and computes
  `wants_ccw = (contour_should_be_ccw == (is_contour_line || region_line_count == 2)) != g7_reverse`.
  It reverses `path.points`, `widths` and `feature_flags` together, and only when `line.is_closed`
  and the line has three or more points. `boundary_type` stays PnP's paint classification and no
  longer affects winding.
- **Witnesses.** These run in the `M-ARACHNE-BUILD` deterministic prelude, each under its own
  `CaseGuard`, and also as plain tests. All three failed before the fix and pass after it:
  - `build_walls_winding_witness_open_path_keeps_order`: an open CCW chain in a three-line region
    under `wall_direction = counter_clockwise` keeps its input order. Before the fix it was
    reversed.
  - `build_walls_winding_witness_clockwise_inner_is_contour`: a closed clockwise inset-1 line in a
    three-line region under `wall_direction = clockwise` is a contour and stays clockwise. Before
    the fix it was treated as a hole and reversed.
  - `build_walls_winding_witness_two_line_region_inner_is_contour_like`: a closed CCW inset-1 line
    in a two-line region under `wall_direction = counter_clockwise` stays counter-clockwise
    because of `pg_extrusions.size() == 2`. Before the fix it was reversed.
- **Remaining approximation, not fixed.** `region_line_count` counts one `build_walls` batch,
  which may differ from canonical's `pg_extrusions`. A PnP region can hold several ExPolygons,
  while canonical orders and traverses each `Surface` separately. The `only_one_wall_top` second
  pass calls `build_walls` once for the top lines and once for the remainder, while canonical
  merges `perimeters` with the inner-pass lines before ordering.

## Independent oracles and their limits

| Record | Reference | Independent of | Explicit limit |
|---|---|---|---|
| `H-ARACHNE-BEADING` | closed-form bead arithmetic on dyadic inputs; Higham `gamma(m)` absolute+relative envelope from the operation budget (≤ 2.0e-9 width units, ≤ 8.0e-9 location units at extended bounds) | every strategy's `compute` | equal-spacing `t = n*w` only for `n >= 3`; widening conservation deliberately not asserted |
| `H-ARACHNE-GRAPH` | indexed relation checker `check_relations` | the production compactor | relations checked at construction stage only; NaN radii are allowed sentinels |
| `H-ARACHNE-PIPELINE` | `check_output` per declared outcome mode | the pipeline's own closure and index logic | closure equality only in the explicit-repeat domain; no wall count, containment or golden output |
| `M-ARACHNE-CLASSIFY` | `expected_classification` truth table | `classify_line` | PnP ThinWall labelling, not an upstream role identity |
| `M-ARACHNE-BUILD` | real-number `spacing + height*(1-π/4)` with derived f32 envelope `width_envelope`; bridge membership from `point_in_region` / analytic inequalities; winding from canonical `traverse_extrusions` / `ExtrusionLine::is_contour` (`expected_want_ccw`); observed extent and junction total against the sampled draw | `PerimeterSpatialContext::is_bridge`, `build_wall_flags`, the production winding expression | rectangle-and-chain roster at a sampled half-extent; no arbitrary polygon shapes |
| `M-ARACHNE-SEQUENCE` | `expected_sequence_order` identity mapping | `commit_wall_sequence` | exact mode order only for a unique endpoint outer; otherwise a lossless permutation only |

## Measurements

Execution is libtest wall time per record. Compilation is not tabulated per record. The three host
records share one compile unit (`property_arachne`) and the three module records share another (the
`arachne_perimeters` lib test binary). With incremental builds, a per-record compile figure would
measure cache state, not the target. Proptest exposes no generation-versus-shrink split, and no
shrink campaign ran, so the table has no shrink time.

**Separate compilation measurement.** Compilation is measured per compile unit, in the workspace
`dev` profile, by cleaning only the package under test and timing `cargo test --no-run`:

| Compile unit | Command | Cold build | Log |
|---|---|---:|---|
| Host `property_arachne` | `cargo clean -p slicer-core`; `cargo test -p slicer-core --features host-algos,property_testing --test property_arachne --no-run` | 27.7 s | `compile-host-property.log` |
| Module `arachne_perimeters` lib | `cargo clean -p arachne-perimeters`; `cargo test -p arachne-perimeters --features arachne-perimeters/property_testing --lib --no-run` | 11.5 s | `compile-module-lib.log` |

Each figure includes the cleaned package's own rebuild and does not include a `pnp_cli` relink or a
guest rebuild. The host figure rebuilt `slicer-core` and `slicer-sdk` because `cargo clean -p`
removed them from that feature graph; the module figure was taken while those dependencies were
warm, so it measures the module crate and its test binary only.

| Record | Selected routine | Selected extended | Combined routine | Combined extended |
|---|---:|---:|---:|---:|
| `H-ARACHNE-BEADING` | 0.02 s | 0.28 s | 0.02 s | 0.31 s |
| `H-ARACHNE-GRAPH` | 0.22 s | 5.32 s | 0.25 s | 5.61 s |
| `H-ARACHNE-PIPELINE` | 0.23 s | 5.39 s | 0.24 s | 5.64 s |
| `M-ARACHNE-CLASSIFY` | 0.03 s | 0.24 s | 0.02 s | 0.31 s |
| `M-ARACHNE-BUILD` | 0.33 s | 5.90 s | 0.33 s | 5.18 s |
| `M-ARACHNE-SEQUENCE` | 0.03 s | 0.75 s | 0.04 s | 0.84 s |
| Log | `routine-six.log` | `extended-six-summary.log` | `combined-routine-summary.log` | `combined-extended-summary.log` |

All four columns were re-measured on the final tree after the AC-5 winding correction, after the
`M-ARACHNE-BUILD` bounds were raised, and after its bridge band became asymmetric, so the row
shares one source state; the superseded figures are not carried forward. `M-ARACHNE-BUILD` is the
slowest of the three module records in every lane (0.33 / 5.90 / 0.33 / 5.18 s) because each case
builds a five-line region with 23 to 64 junctions and a sampled 1–2 mm (routine) or 1–4 mm
(extended) half-extent, then checks the emitted extent and junction total. Totals: 19.342 s routine
and 35.708 s extended for the six-record selections; 2m34.964 s routine and 10m10.932 s extended
for the 32-record combined lanes. The combined lanes ran with all 32 record binaries already
linked, so their totals are warm-cache execution times and exclude first-run linking. The
retained-seed replay below covers `H-ARACHNE-BEADING` only and was taken before the
`M-ARACHNE-BUILD` changes, which cannot affect that record.

All logs are under `target/proptest-arachne-evidence/`. Per lane:

- **Six-record routine selection**
  (`cargo xtask test --summary --properties --property H-ARACHNE-BEADING … --property
  M-ARACHNE-SEQUENCE --tier routine`):
  - Total `real 0m19.342s`, recorded in the archived digest `routine-six.log`.
  - `VERDICT: PASS`.
  - 64/64 accepted per record, 0 rejects, `call_counts_complete = true`,
    `completion_eligible = true`, `shrink_completed = null`.
  - Run token `ed88fc5037adaae5a7f44366edcc8c88`; `routine-six-full.log` is the matching full
    output with the per-record begin/end events. The summary digest keeps the verdict and the six
    `test result:` lines. `VERDICT: PASS` is emitted only after the runner's evidence check
    requires a controller begin/end pair per record with `accepted_cases >= profile.cases` and
    `completion_eligible == true`, so the 64/64 claim is enforced by the runner, not asserted from
    this table.
- **Six-record extended selection** (same command, `--tier extended`, run token
  `87f1ec238c6ce82930c53c669a59acad`):
  - Six matching controller `begin`/`end` events.
  - Every `end` event: `accepted_cases = 1024`, `rejected_test_calls = 0`,
    `call_counts_complete = true`, `completion_eligible = true`, `shrink_completed = null`,
    `outcome = passed`.
  - Total `real 0m35.708s`, archived in `extended-six-summary.log`; `extended-six-full.log` is the
    matching full output.
- **Retained-seed replay** (`--property H-ARACHNE-BEADING --tier routine --seed
  14268583537219271429`, run token `29d43e1e20aa98fce6670d40b9413df3`):
  - `outcome = passed`, 64/64 accepted, 0 rejects, libtest 0.03 s.
  - Total `real 0m15.462s`, archived in `replay-beeding-summary.log`.
  - The archived log is `replay-beeding-routine.log`. That filename is the real one on disk,
    misspelling included.
- **Combined incremental routine** (`cargo xtask test --summary --properties --tier routine`, run
  token `883cc94987301d99f3e443273c5b51ea`):
  - 32 records. The four `requires_scad` records are skipped without `--with-scad`.
  - 32 `begin` and 32 `end` events. The six Arachne `end` events report 64 accepted and 0
    rejected.
  - `VERDICT: PASS` and a total of `real 2m34.964s`, archived in
    `combined-routine-summary.log`; `combined-routine-full.log` holds the per-record lines and
    events. This lane is warm-cache: every record binary was already linked.
- **Combined incremental extended** (`--tier extended`, run token
  `e45ac9033a51744597c0b66d67e092ad`):
  - 32 records, 32 `begin` and 32 `end` events. The six Arachne `end` events report 1024 accepted
    and 0 rejected.
  - `VERDICT: PASS` and a total of `real 10m10.932s`, archived in
    `combined-extended-summary.log`; `combined-extended-full.log` holds the per-record lines and
    events. This lane is warm-cache too.
- **First gated command after the production edits.**
  - 6 m 48.9 s total.
  - That includes 49 stale guest rebuilds, measured at 272.1 s as the artifact-write span, and an
    87 s `pnp_cli` relink.
  - The following `cargo xtask build-guests --check` returned exit `0`, and later runs did not
    rebuild.

**Measured outcome populations (host).**

- `H-ARACHNE-GRAPH`, class acceptance over 400 cases:

  | Class | Cases | Outcome |
  |---|---:|---|
  | `Empty` | 74 | `Err(EmptyInput)` |
  | `Convex` | 68 | `Ok`, nonempty graph |
  | `Concave` | 70 | `Ok`, nonempty graph |
  | `Holed` | 49 | `Ok`, nonempty graph |
  | `Thin` | 70 | `Ok`, nonempty graph |
  | `Disconnected` | 69 | `Ok`, nonempty graph |

  `NonStarShaped` is not sampled. `NearDegenerate` is excluded by derivation, not by observation:
  its one-catalog-unit clearance witness does not survive the exact ×2500 unit map (`SCALE`).
- `H-ARACHNE-PIPELINE`, 200-case probe:
  - 608 printable lines: 608/608 index-uniform, 608/608 satisfy the endpoint rule.
  - 470 closed lines, every one with an explicit repeated endpoint.
  - 29 `EmptyAfterPreprocess` outcomes.
  - 29/29 guaranteed-class cases nonempty.
- `H-ARACHNE-PIPELINE` deterministic witnesses (`deterministic_witnesses`, `pipeline.rs`):

  | Witness | Outcome |
  |---|---|
  | 10 mm square | `Ok`; 5 closed printable lines and 1 inner-contour marker; every line index-uniform; every closed line repeats its endpoint |
  | tapered wedge | `Ok`; 3 closed printable lines with differing widths |
  | 8 × 0.3 mm strip, `print_thin_walls` | `Ok`; one open, odd, two-junction 0.34 mm line |
  | 5 µm square | `Err(EmptyAfterPreprocess)` |

## Controls and rejected mutations

These run under plain `cargo test` with no registry context. Every arm runs the real SUT on a named
fixture, and that fixture must pass the same checker. The arm then shows the checker rejects one
specific wrong result.

- **`arachne_beading_oracle_controls`.** Rejects a lost sentinel pair (both arrays stay parallel, so
  the marker contract fires, not the length contract) and a non-parallel width/location pair. It
  also proves conservation must fail when applied to a real widening output.
- **`arachne_graph_oracle_controls_and_input_errors`.**
  - Empty input → `SktError::EmptyInput`.
  - Zero-point ring and two-point ring → `SktError::DegeneratePolygon`.
  - `check_relations` rejects broken twin reciprocity, an out-of-range `next`, an out-of-range
    `start_vertex`, a truncated `beading_propagation` and a truncated
    `beading_dist_to_bottom_source`.
- **`arachne_pipeline_oracle_controls`.** Rejects:
  - a falsely closed line with unequal endpoints in the explicit-repeat domain;
  - a corrupted `perimeter_index` (uniform rule);
  - an inset index carried by neither endpoint (pipeline rule);
  - a nonzero-first-width inner-contour marker;
  - an empty guaranteed output.
- **`property_tests::arachne_module_oracle_controls`.** Rejects:
  - the deeper-odd-as-GapFill classification;
  - a points-only reversal;
  - a missed spacing → width conversion;
  - fabricated closure on an open path;
  - a naive full reversal of an outer-first `InnerOuter` batch;
  - a lost feature-flag vector.
- **`property_tests::build_walls_bounds_drive_geometry_and_junctions`.** Builds the same mode and
  spacing base under two accepted bound pairs and asserts the observed coordinate extent
  (`10_000` vs `60_000` units) and junction total (`23` vs `32`) both differ and match the emitted
  walls, sweeps `max_junctions` 23/32/64 at half-extents 10 000/60 000/1 000 000 units with both
  ends of each junction range, and asserts a bound below the roster minimum (`22` junctions) and a
  half-extent below 1 mm are rejected before sampling.
- **Domain rejections and shrink witnesses.** `arachne_property_domain_rejections` and
  `property_tests::arachne_module_domain_rejections` reject unusable bounds before sampling. The
  latter also runs `shrink_retains_geometry_and_mode_witness` and
  `classify_shrink_retains_positivity`.

Archived results:

- `target/proptest-arachne-acceptance/C-contracts-step4.log`: `property_arachne_contracts`, 9
  passed, 0 failed.
- The three `M-*.log` files there: 1 passed, 0 failed each.

## Defects and regressions

Four production defects were confirmed against canonical, fixed, and given RNG-free regressions
that need no `PNP_PROPERTY_CONTEXT`:

1. **Widening used a thickness-only gate.** Canonical gates the single-bead branch on
   `bead_count <= 1 && thickness < getTransitionThickness(1)`.
   - Pre-fix, `widening_two_requested_below_optimal_delegates_to_parent` returned `[3400.0]`
     instead of `[1500.0, 1500.0]`.
   - Pre-fix, `widening_transition_propagation_preserves_two_bead_endpoint` returned 1 bead
     instead of the derived widths `[2450, 1500]` / locations `[1125, 2250]`.
   - Both now pass (`property_arachne_contracts`).
2. **The local maximum produced a seven-junction closed pre-stitch line.** Canonical
   `generateLocalMaximaSingleBeads` emits six junctions and `ExtrusionLine::ExtrusionLine` starts
   open.
   - The producer now emits six junctions with `is_closed = false`, and
     `arachne_local_maximum_pre_stitch_is_open` asserts it.
   - The post-stitch pentagon fixture (`arachne_local_maxima_single_beads`) now measures one open,
     odd, six-junction inset-1 line, because the tiny-poly gate rejects closing it.
   - That fixture was updated only to this measured canonical-correct shape, not to keep the former
     closed expectation.
3. **`LimitedBeadingStrategy::get_transition_thickness` lacked the
   `lower_bead_count >= max_bead_count` clamp.** At cap 4 it returned 18000.0 against canonical's
   19900.0 before the fix. Regression: `limited_transition_thickness_at_cap_matches_canonical_floor`.
4. **`ArachnePerimeters::build_walls` reversed `path.points` without reversing
   `width_profile.widths` / `feature_flags`.**
   - Before the fix, the regression
     `property_tests::build_walls_reversed_variable_width_keeps_profiles_and_bridge_flags_attached`
     failed with `width_profile.widths[1] = 0.6429204` against `path.points[1].width =
     0.84292036`.
   - It now covers reversal with unequal widths and asymmetric bridge flags.

**Retained, documented, not a defect: the over-cap sentinel superset.** `LimitedBeadingStrategy::
compute` generalises canonical's single over-cap sentinel pair to `bead_count - max_bead_count`
pairs. Canonical only reaches `max_bead_count + 1`, and that path is behaviourally unreachable
through `optimal_bead_count`. The retained `limited_inserts_sentinels_at_cap` fixture calls cap + 3,
so an assertion would fire. The superset is documented in the function comment.

## Manual literal review (AC-8)

`cargo xtask check-literals` cannot see `modules/core-modules/arachne-perimeters/src/property_tests.rs`,
so its result is not evidence for this file:

- `collect_enforced_files` (`xtask/src/check_literals.rs`) collects `crates/*/src` in `CfgTestOnly`
  mode and `modules/core-modules/*/tests` whole-file. It never collects `modules/core-modules/*/src`.
- `LiteralVisitor::visit_item_mod` delegates to `syn::visit::visit_item_mod` and never opens an
  out-of-line module file.

Watched types were re-derived (`pub` struct, ≥ 5 named fields, `crates/*/src`) by counting the
struct bodies on the implementation tree:

| Type | Defined in | Named fields | Watched |
|---|---|---:|---|
| `Point3WithWidth` | `crates/slicer-ir/src/slice_ir.rs` | 8 | yes |
| `ExtrusionPath3D` | `crates/slicer-ir/src/slice_ir.rs` | 5 | yes |
| `WallFeatureFlags` | `crates/slicer-ir/src/slice_ir.rs` | 6 | yes |
| `WallLoop` | `crates/slicer-ir/src/slice_ir.rs` | 6 | yes |
| `ArachneParams` | `crates/slicer-sdk/src/host.rs` | 23 | yes |
| `ExtrusionLine` | `crates/slicer-ir/src/slice_ir.rs` | 4 | no |
| `ExtrusionJunction` | `crates/slicer-ir/src/slice_ir.rs` | 2 | no |
| `WidthProfile` | `crates/slicer-ir/src/slice_ir.rs` | 1 | no |

`ArachneParams` here is `slicer_sdk::host::ArachneParams`, the SDK's host-side mirror struct, not
`slicer_core::arachne::pipeline::ArachneParams` (which is `host-algos`-gated and so not nameable
from this wasm guest crate). The mirror has the same 23 named fields; only the path is different.
The earlier version of this table named the core path, which this file never imports.

Inspected sites in `property_tests.rs`:

- `Point3WithWidth` in `point3`: FRU, `..Point3WithWidth::default()`.
- `ArachneParams` in `build_params`: FRU, `..ArachneParams::default()`.
- `WallFeatureFlags` in `sequence_wall`: FRU, `..WallFeatureFlags::default()`.
- `WallLoop`: no literal. `sequence_wall` builds it with `wall_loop_base` and then mutates fields.
- `ExtrusionPath3D`: no literal.

Result: every literal of a watched type uses FRU, and no `// exhaustive:` waiver is needed. The
completeness aid
`rg -n 'Point3WithWidth \{|ExtrusionPath3D \{|WallFeatureFlags \{|WallLoop \{' modules/core-modules/arachne-perimeters/src/property_tests.rs`
lists these three literal sites plus the `fn sequence_wall(…) -> WallLoop {` signature, which is not
a literal. The grep only helps check completeness; it is not the evidence. The evidence is the
site-by-site inspection above.

## G-ARACHNE disposition

Locally, every G-FAMILY obligation for the Arachne family is met:

- All six records run on Windows in both the routine and extended tiers, in the selected and the
  combined incremental lanes.
- Each has matching controller `begin`/`end` events, and accepted cases equal the profile's `cases`.
- Rejected test calls are zero, and `call_counts_complete = true`.
- A retained seed replays green.

The hosted `properties-windows` job in `.github/workflows/ci.yml` now asserts the six IDs. It was
**not** executed from this workstation, because it needs a push or PR. The local lanes above are the
evidence. Running the hosted job is the packet's remaining closure action.

## Post-closure follow-ups

1. **Hosted execution.** Run the `properties-windows` PR job and record its run ID, head and the
   six-ID assertion result here.
2. **Region order.** `WallToolPaths::getRegionOrder` differs between the two inspected checkouts.
   Any future property that depends on odd-line region ordering must first establish which
   reference applies.
3. **Scanner scope.** If `collect_enforced_files` ever collects `modules/core-modules/*/src` and
   follows out-of-line modules, replace the AC-8 manual review with the scanner result.

## Checks

| Check | Result |
|---|---|
| `rg -n 'H-ARACHNE-\|M-ARACHNE-\|inner.contour\|host-algos' docs/testing/properties.md` | 49 matching lines; all six `## <ID>` charter headings present |
| `rg -n 'G-FAMILY\|Windows\|source_revision\|canonical\|regression\|FRU\|exhaustive' docs/testing/proptest-arachne-evidence.md` | every alternative matches (`G-ARACHNE` also matches) |
| `rg -n 'H-ARACHNE\|M-ARACHNE\|inner.contour' docs/02_ir_schemas.md` | matches the new printable-versus-marker subsection and its two charter links |
| `rg -n 'M-ARACHNE\|property_testing\|host-algos\|check-literals' docs/05_module_sdk.md` | every alternative matches |
| Every `charter` anchor in `test-properties.toml` for `h-arachne-*`/`m-arachne-*` | all six resolve to `## ` headings in `docs/testing/properties.md` |
| `cargo build --workspace` | `BUILD_EXIT=0` (`closure-final-build.log`) |
| `cargo check --workspace --all-targets` | `CHECK_EXIT=0` (`closure-final-check.log`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | `CLIPPY_EXIT=0` (`closure-final-clippy.log`) |
| `cargo check`/`clippy -p slicer-core --all-targets --features host-algos,property_testing` | `CORE_CHECK_EXIT=0`, `CORE_CLIPPY_EXIT=0` (`closure-final-owner-gates.log`) |
| `cargo check`/`clippy -p arachne-perimeters --all-targets --features property_testing` | `MOD_CHECK_EXIT=0`, `MOD_CLIPPY_EXIT=0` (`closure-final-owner-gates.log`) |
| `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts` | 11 passed, 0 failed, `SDK_TEST_EXIT=0` (`closure-final-owner-gates.log`) |
| `cargo test -p xtask --bin xtask -- property_` | 23 passed, 0 failed, `XTASK_TEST_EXIT=0` (`closure-final-owner-gates.log`) |
| `cargo xtask check-literals` | 0 violations, `LITERALS_EXIT=0` (`closure-final-literals.log`); not evidence for `src/property_tests.rs` |
| `cargo xtask check-test-quality --report` | 0 findings, `TEST_QUALITY_EXIT=0` (`closure-final-test-quality.log`) |
| `cargo xtask check-fmt` | 62 packages clean, `FMT_EXIT=0` (`closure-final-fmt.log`) |
| `cargo xtask build-guests --check` | `GUESTS_EXIT=0`, `EXIT_FRESH` (`closure-final-guests.log`) |
