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
  (`EXIT_FRESH`), judged by exit code, and later runs did not rebuild. The closure gate log
  `target/proptest-arachne-evidence/closure-xtask-gates.log` records `build-guests --check`
  `EXIT=0`.

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
| `M-ARACHNE-BUILD` | `max_junctions = 8`, `max_abs_mm_units = 20000`, `max_spacing_units = 2000`, `max_height_units = 500` | `16`, `40000`, `8000`, `2000` |
| `M-ARACHNE-SEQUENCE` | `max_walls = 6` | `12` |

The zero rejection budgets are honest because every generator is construction-first and total. A
rejection would show that a generator changed, and the run would abort as `exhausted`. Each family
validates its bounds before it builds a strategy and rejects zero, non-representable, overflowing
and impossible-witness values. Those checks are pinned by `arachne_property_domain_rejections` and
`property_tests::arachne_module_domain_rejections`.

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

## Independent oracles and their limits

| Record | Reference | Independent of | Explicit limit |
|---|---|---|---|
| `H-ARACHNE-BEADING` | closed-form bead arithmetic on dyadic inputs; Higham `gamma(m)` absolute+relative envelope from the operation budget (≤ 2.0e-9 width units, ≤ 8.0e-9 location units at extended bounds) | every strategy's `compute` | equal-spacing `t = n*w` only for `n >= 3`; widening conservation deliberately not asserted |
| `H-ARACHNE-GRAPH` | indexed relation checker `check_relations` | the production compactor | relations checked at construction stage only; NaN radii are allowed sentinels |
| `H-ARACHNE-PIPELINE` | `check_output` per declared outcome mode | the pipeline's own closure and index logic | closure equality only in the explicit-repeat domain; no wall count, containment or golden output |
| `M-ARACHNE-CLASSIFY` | `expected_classification` truth table | `classify_line` | PnP ThinWall labelling, not an upstream role identity |
| `M-ARACHNE-BUILD` | real-number `spacing + height*(1-π/4)` with derived f32 envelope `width_envelope`; bridge membership from `point_in_region` / analytic inequalities | `PerimeterSpatialContext::is_bridge`, `build_wall_flags` | the controlled rectangle scenes only |
| `M-ARACHNE-SEQUENCE` | `expected_sequence_order` identity mapping | `commit_wall_sequence` | exact mode order only for a unique endpoint outer; otherwise a lossless permutation only |

## Measurements

Execution is libtest wall time per record. Compilation is not tabulated per record. The three host
records share one compile unit (`property_arachne`) and the three module records share another (the
`arachne_perimeters` lib test binary). With incremental builds, a per-record compile figure would
measure cache state, not the target. Proptest exposes no generation-versus-shrink split, and no
shrink campaign ran, so the table has no shrink time.

| Record | Selected routine | Selected extended | Combined routine | Combined extended |
|---|---:|---:|---:|---:|
| `H-ARACHNE-BEADING` | 0.03 s | 0.44 s | 0.03 s | 0.31 s |
| `H-ARACHNE-GRAPH` | 0.23 s | 7.61 s | 0.29 s | 6.43 s |
| `H-ARACHNE-PIPELINE` | 0.34 s | 8.02 s | 0.33 s | 6.65 s |
| `M-ARACHNE-CLASSIFY` | 0.03 s | 0.55 s | 0.05 s | 0.31 s |
| `M-ARACHNE-BUILD` | 0.12 s | 3.46 s | 0.14 s | 2.08 s |
| `M-ARACHNE-SEQUENCE` | 0.05 s | 0.96 s | 0.06 s | 1.06 s |
| Log | `routine-six.log` | `extended-six-full.log` | `combined-routine-full.log` | `combined-extended-full.log` |

All logs are under `target/proptest-arachne-evidence/`. Per lane:

- **Six-record routine selection**
  (`cargo xtask test --summary --properties --property H-ARACHNE-BEADING … --property
  M-ARACHNE-SEQUENCE --tier routine`):
  - Total `real 0m23.678s`, recorded in the archived digest.
  - `VERDICT: PASS`.
  - 64/64 accepted per record, 0 rejects, `call_counts_complete = true`,
    `completion_eligible = true`, `shrink_completed = null`.
  - The archived file is the `--summary` digest. It keeps the six `test result:` lines and the
    verdict but not the per-record event lines. `VERDICT: PASS` is emitted only after the runner's
    evidence check requires a controller begin/end pair per record with
    `accepted_cases >= profile.cases` and `completion_eligible == true`, so the 64/64 claim is
    enforced by the runner, not asserted from this table.
- **Six-record extended selection** (same command, `--tier extended`, run token
  `ccdbdbf653748fb44d20fbb7b60ee03e`):
  - Six matching controller `begin`/`end` events.
  - Every `end` event: `accepted_cases = 1024`, `rejected_test_calls = 0`,
    `call_counts_complete = true`, `completion_eligible = true`, `shrink_completed = null`,
    `outcome = passed`.
  - Total `real 0m43.704s`, archived in `extended-six-summary.log`.
- **Retained-seed replay** (`--property H-ARACHNE-BEADING --tier routine --seed
  14268583537219271429`, run token `29d43e1e20aa98fce6670d40b9413df3`):
  - `outcome = passed`, 64/64 accepted, 0 rejects, libtest 0.03 s.
  - Total `real 0m15.462s`, archived in `replay-beeding-summary.log`.
  - The archived log is `replay-beeding-routine.log`. That filename is the real one on disk,
    misspelling included.
- **Combined incremental routine** (`cargo xtask test --summary --properties --tier routine`, run
  token `da9e43a45ded39e150265d4ae5d518a1`):
  - 32 records. The four `requires_scad` records are skipped without `--with-scad`.
  - 32 `begin` and 32 `end` events. The six Arachne `end` events report 64 accepted and 0
    rejected.
  - `VERDICT: PASS` and a total of `real 5m10.607s`, archived in `combined-routine-summary.log`.
- **Combined incremental extended** (`--tier extended`, run token
  `84cd58e0a7bde2f90b466e58efb81869`):
  - 32 records, 32 `begin` and 32 `end` events. The six Arachne `end` events report 1024 accepted
    and 0 rejected.
  - `VERDICT: PASS` and a total of `real 16m4.651s`, archived in
    `combined-extended-summary.log`.
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
- **Domain rejections and shrink witnesses.** `arachne_property_domain_rejections` and
  `property_tests::arachne_module_domain_rejections` reject unusable bounds before sampling. The
  latter also runs `shrink_retains_geometry_and_mode_witness`.

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
| `ArachneParams` | `crates/slicer-core/src/arachne/pipeline.rs` | 23 | yes |
| `ExtrusionLine` | `crates/slicer-ir/src/slice_ir.rs` | 4 | no |
| `ExtrusionJunction` | `crates/slicer-ir/src/slice_ir.rs` | 2 | no |
| `WidthProfile` | `crates/slicer-ir/src/slice_ir.rs` | 1 | no |

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
| `cargo build --workspace` | `BUILD_EXIT=0` (`closure-build-check-clippy.log`) |
| `cargo check --workspace --all-targets` | `CHECK_EXIT=0` (`closure-build-check-clippy.log`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | `CLIPPY_EXIT=0` (`closure-build-check-clippy.log`) |
| `cargo check`/`clippy -p slicer-core --all-targets --features host-algos,property_testing` | both exit 0 (`closure-prereq-owner-gates.log`) |
| `cargo check`/`clippy -p arachne-perimeters --all-targets --features property_testing` | both exit 0 (`closure-prereq-owner-gates.log`) |
| `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts` | 11 passed, 0 failed (`closure-prereq-owner-gates.log`) |
| `cargo test -p xtask --bin xtask -- property_` | 23 passed, 0 failed (`closure-prereq-owner-gates.log`) |
| `cargo xtask check-literals` | 0 violations (`closure-xtask-gates.log`); not evidence for `src/property_tests.rs` |
| `cargo xtask check-test-quality --report` | 0 findings (`closure-xtask-gates.log`) |
| `cargo xtask check-fmt` | 62 packages clean (`closure-xtask-gates.log`) |
| `cargo xtask build-guests --check` | exit `0`, `EXIT_FRESH` (`closure-xtask-gates.log`) |
