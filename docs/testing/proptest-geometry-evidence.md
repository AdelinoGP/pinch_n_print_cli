# Property-testing evidence: geometry and mesh properties

Packet: `proptest-openrscad_09_pbt-geometry`. This document is the execution evidence surface for
the direct `H-BOOL-*`, `H-OFFSET-*`, `H-MESH-*` and `H-HELPERS-SPLIT` records, and the disposition
record for the conditional scripted enrichment (`H-BOOL-SCAD`, `H-MESH-SCAD`).

Every number below comes from an archived log under `target/evidence/` and from the per-run
`context.json`; none is inferred and no Windows CI run is claimed that was not executed.

## Provenance

- **Baseline revision.** `779afc030c584be9d2138d23260595af498d6194` (`source_revision`), working
  tree clean (`git status --porcelain` empty) immediately before the packet's first edit.
- **Dirty-content note.** This packet's own work is uncommitted, so every measurement below was taken
  on a dirty tree. The per-record `context.json` under `target/property-testing/<run-token>/<ID>/`
  is authoritative for each measurement's exact `source_revision`, `dirty_paths`,
  `source_state_digest` and `cargo_target_dir`; this document deliberately does not freeze a
  workspace-wide dirty digest, because it changes with every edit and would be stale on arrival. The
  hashes quoted for specific re-runs are the run tokens of those context directories.
- **Log archive.** Numbers attributed to a named `target/evidence/<slug>.log` come from that file.
  Three timings were originally read from the terminal rather than archived: the 315.81 s and 9.19 s
  extended re-runs and the 9.52 s / 0.76 s SCAD routine run. The SCAD routine output survives as
  `target/evidence/review09-prior-scad-routine-test-output.log`; the two extended re-runs were later
  re-measured with archived logs (`review09-boolset-extended.log`, `review09-scad-extended.log`), and
  where the two disagree this document keeps the archived figure.
- **Baseline outcomes.** Eight existing narrow targets, captured before any edit, archived as
  `target/evidence/step1-*.log`:

  | Command | Result |
  |---|---|
  | `cargo test -p slicer-core --features host-algos --test polygon_ops_tdd` | 14 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --test polygon_ops_clip_batched_tdd` | 10 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --test offset2_ex_tdd` | 2 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --test offset2_ex_collapse_tdd` | 2 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --test triangle_mesh_slicer_tdd` | 11 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --test aabb_tree_tdd` | 6 passed, 0 failed |
  | `cargo test -p slicer-core --features host-algos --lib -- smooth_outward::tests` | 8 passed, 0 failed, 185 filtered out |
  | `cargo test -p slicer-helpers --test split_tdd` | 2 passed, 0 failed |

- **Toolchain.** `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`,
  Windows host, `dev` profile, default workspace `target/`.
- **Guest freshness at baseline.** `cargo xtask build-guests --check` exited `0` (`EXIT_FRESH`),
  judged by exit code, not by grepping output.
- **Guest freshness after the manifest change.** The packet's required `[features]` and `[[test]]`
  additions to `crates/slicer-core/Cargo.toml` stale every guest, because
  `guest_closure_input_paths` (`xtask/src/build_guests.rs`) fingerprints each closure crate's
  `Cargo.toml` bytes and `slicer-core` is in every guest's closure. This is the documented
  rebuild-then-recheck path, not a defect; see `## Deviations` item 3 of the packet for the
  measured consequence of the rejected optional-normal `slicer-scad` edge.
  **A fresh checkout of this branch therefore reports `STALE` (exit `1`) until `cargo xtask
  build-guests` runs once.** The closure review reproduced exactly that: exit `1` with 49/49 stale at
  `2eb60366`, then exit `0` after the rebuild. The `EXIT_FRESH` results recorded elsewhere in this
  document are post-rebuild results, not a claim that the committed tree is fresh without one.
- **Dirty paths.** This run is uncommitted; the per-record `context.json` is authoritative for the
  exact dirty set each measurement saw.

## Profiles

Both tiers are `in_process`, `rng_algorithm = "chacha"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero local/global/flat-map rejection budgets, and total generators (no
`prop_assume`). `max_default_size_range` is 32 routine and 128 extended; `max_shrink_iters` is 64
and 256.

| Record | cases (routine/extended) | bounds (routine) | bounds (extended) |
|---|---|---|---|
| `H-BOOL-SET` | 64 / 512 | abs 60000, verts 32, holes 4, comps 3, shrink 32, queries 64 | abs 120000, verts 64, holes 8, comps 4, shrink 64, queries 128 |
| `H-BOOL-CLIP` | 64 / 512 | the six shared keys plus polylines 12 | plus polylines 16 |
| `H-OFFSET-MITER` | 64 / 512 | abs 120000, side 8 mm, offset 3 mm | abs 200000, side 16 mm, offset 5 mm |
| `H-OFFSET-SMOOTH` | 64 / 512 | abs 120000, side 40000, clip 5000 | abs 200000, side 80000, clip 8000 |
| `H-MESH-SLICE` | 64 / 512 | extent 8 mm, translation 8 mm, planes 6 | extent 16, translation 16, planes 10 |
| `H-MESH-QUERY` | 64 / 512 | extent 8 mm, translation 8 mm, queries 8 | extent 16, translation 16, queries 16 |
| `H-HELPERS-SPLIT` | 64 / 512 | components 4, faces 12, abs 8 mm | components 8, faces 24, abs 16 mm |

The bound values are not free parameters: each property validates them against the minima its named
witnesses need and fails before sampling on a zero, overflowing or out-of-range value. Two
constraints worth recording: `H-BOOL-CLIP` floors `max_polylines` at 10 because nine deterministic
witness paths are always present, and `H-OFFSET-MITER` requires
`2 * max_abs_units/10_000 >= 4*max_offset_mm + max_side_mm`, which is why the routine absolute bound
is 120000 rather than the 60000 used by the Boolean rows.

## Independent oracles and their limits

| Record | Reference | Independent of | Explicit limit |
|---|---|---|---|
| `H-BOOL-SET` | exact open-cell truth table for rectangle pairs; sampled `point_in_region` for general sets | Clipper, self-union, ring order | general membership is proved only at sampled points |
| `H-BOOL-CLIP` | analytic `offset(R,1)` universe; exact 1D interval partition for axis-aligned inputs | Clipper, result order | non-axis coverage is sampled; distinct non-axis inputs closer than `2*BAND` can share one emitted piece |
| `H-OFFSET-MITER` | algebraic bound staging, exact `i128` double area | Clipper | rectangles and single-hole rectangular annuli only |
| `H-OFFSET-SMOOTH` | `k = floor(chord/√2)` from an exact integer search; independently built chamfer ring | the SUT | obtuse branch only; tiny-ring deletion is one concrete case |
| `H-MESH-SLICE` | dimension-derived section plus exact integer clearance sampling | a second slice call, Clipper | box section exact; prism section bounded by a derived envelope |
| `H-MESH-QUERY` | coordinate sign/swap plus translation; dimension-derived bounds and ray intersections | matrix product, second tree query | unit-length rays only; no normalization promise |
| `H-HELPERS-SPLIT` | pairwise nested-loop adjacency plus union-find over oriented position triples | production's edge map and DFS | face partition only, never a vertex layout |

## Measurements

Execution is libtest wall time from the archived runs. Compilation is **not** tabulated per record:
the seven direct records share two compile units (`property_geometry`, `property_helpers`) and the two
scripted records share a third (`property_geometry_scad`), and with Cargo's incremental builds a
per-record compile number would measure cache state rather than the target. The compile evidence that
does exist is the gate logs (`s2b-core-check`, `s3-core-check`, `s4-core-check`, `s6-core-check`),
which each report `Finished` on the whole target.

Extended-tier wall times are now recorded per record from the full 30-record lane
(`target/evidence/r8-full-extended-scad.log`), in registry order:

| Record | Extended | Record | Extended | Record | Extended |
|---|---:|---|---:|---|---:|
| `H-IR-ANCHORS` | 8.74 s | `H-CONFIG-BOUNDS` | 1.44 s | `H-BOOL-SET` | 255.24 s |
| `D-SCALARS` | 0.48 s | `H-IR-SERDE` | 6.14 s | `H-BOOL-CLIP` | 66.90 s |
| `D-GRAPHS` | 0.89 s | `H-CONFIG-INTERN` | 1.39 s | `H-OFFSET-MITER` | 0.23 s |
| `D-IR` | 43.66 s | `H-CHAINS` | 0.36 s | `H-OFFSET-SMOOTH` | 0.19 s |
| `D-GEOMETRY` | 8.10 s | `H-DAG` | 1.68 s | `H-MESH-SLICE` | 11.51 s |
| `D-MESH` | 0.46 s | `H-CLAIMS` | 1.91 s | `H-MESH-QUERY` | 0.15 s |
| `D-SCAD-2D` | 10.70 s | `H-REGISTRY` | 0.96 s | `H-HELPERS-SPLIT` | 0.20 s |
| `D-SCAD-3D` | 1.54 s | `H-RUNTIME-DISPATCH` | 0.79 s | `H-BOOL-SCAD` | 41.04 s |
| `H-COORD` | 0.27 s | `H-RUNTIME-SELECTION` | 2.87 s | `H-MESH-SCAD` | 2.55 s |
| `H-IDS` | 0.32 s | `H-RUNTIME-MODIFIERS` | 77.29 s | `H-RUNTIME-PARTITION` | 50.69 s |

**Granularity limit, stated rather than implied.** Compile, generation and shrink are *not* separated
per record and cannot be at this granularity: the harness reports one libtest wall time per record,
which includes the strategy construction and every case, and Proptest exposes no generation-versus-shrink
split. Compilation is per Cargo target (`property_geometry`, `property_helpers`,
`property_geometry_scad`), not per record, and a per-record compile figure would measure incremental
cache state rather than the target. No shrink time appears in the table because no shrink campaign ran
in any of these lanes.

| Record | Cases | accepted | rejected | shrink_completed | libtest wall | Log |
|---|---:|---:|---:|---|---:|---|
| `H-BOOL-SET` | 32 | 32 | 0 | `null` | 8.92 s | `s2-harness-bool.log` |
| `H-BOOL-CLIP` | 64 | 64 | 0 | `null` | 1.91 s | `s2-harness-clip.log` |
| `H-BOOL-CLIP` (wide pilot) | 512 | 512 | 0 | `null` | 61.61 s | `s2b-harness-clip-wide.log` |
| `H-OFFSET-MITER` | 16 | 16 | 0 | `null` | 0.02 s | `s3-harness-offset.log` |
| `H-OFFSET-SMOOTH` | 16 | 16 | 0 | `null` | 0.02 s | `s3-harness-smooth.log` |
| `H-MESH-SLICE` | 16 | 16 | 0 | `null` | 0.22 s | `s4-harness-slice.log` |
| `H-MESH-QUERY` | 16 | 16 | 0 | `null` | 0.02 s | `s4-harness-query.log` |
| `H-HELPERS-SPLIT` | 64 | 64 | 0 | `null` | 0.04 s | `s5-helper-controls.log` (run) and the harness event in the step report |

`shrink_completed` is `null` everywhere because Proptest 1.11 cannot report whether minimization
finished; this document therefore never calls a retained case minimal.

**Extended tier, first run.** The first `--tier extended` run of all seven records reported
`sut_failure` for `H-BOOL-SET` and `H-MESH-SLICE`. Both were diagnosed as **checker** defects, not
production defects, and both were fixed in the test-side checkers:

- `H-BOOL-SET` rejected a legitimate `xor` output because the strict simple-ring predicate
  (#2 `validate_ring`) refuses a ring that revisits a vertex. Clipper's `NonZero` output may
  legitimately **pinch**, so a ring passing through the same point twice is a correct representation
  of a self-touching region. The general family now uses a **weakly-simple** predicate over the exact
  integer oracles (`orientation`, `segment_relation`): non-adjacent edges may be disjoint or touch at
  a single point, while proper crossings, collinear overlaps of positive length and backtracking
  spikes are rejected. Strict `validate_ring` is retained only where the expected result is a single
  simple rectangle.
- `H-MESH-SLICE` reported a vacuous inside population: dividing the **longest** axis into eight
  lattice steps left a thin prism's only candidate exactly on its hypotenuse. The sampler now always
  includes one analytically guaranteed interior point (rectangle centre, prism centroid) and four
  guaranteed exterior points, so a nonempty layer always yields a nonempty inside population
  regardless of lattice spacing.

**Extended tier, after the fix.** Re-run green: `H-BOOL-SET` 315.81 s and `H-MESH-SLICE` 9.19 s,
512 cases each, `VERDICT: PASS`, and both later re-measured on the final tree at 255.24 s and 11.51 s
in the full 30-record lane below. The other five records passed in the same run and were unaffected by
those two test-side changes.

**This packet did surface one production defect.** The tightened ring predicate is what exposed the
`xor` collinear-spike defect recorded under "Defects and regressions"; the pre-fix demonstration
(failing the two new regressions with `polygon_ops.rs` reverted) is in the review log. What did *not*
happen is a shrink campaign: every failure was a checker or predicate fault rather than a minimised
SUT input, so `shrink_completed` stayed `null` throughout. Shrinking is exercised deliberately by the
two owned-shrink-witness controls instead (`geometry_controls_owned_shrink_witnesses`,
`helper_controls_owned_shrink_witnesses`), which drive the real value trees through `current`,
successful `simplify`, a `false` `simplify` and repeated `complicate` while re-validating the witness
at every exposed state.

Pilot bounds in the harness runs above are the smaller pilot profiles, not the registry profiles;
the registry profiles are the measured-and-chosen values in `test-properties.toml`. Both are recorded
so the difference is visible rather than implied.

## Hosted Windows runs

PR [#8](https://github.com/AdelinoGP/pinch_n_print_cli/pull/8) produced the two hosted lanes this
packet needs. Run `44182873399`, head `2eb60366`:

| Lane | Conclusion | Duration | Records covered |
|---|---|---|---|
| `Properties (Windows)` (job `134391555404`) | **pass** | 11m41s | 26 (plain `--tier routine` selector) |
| `Scripted SCAD properties (Windows)` (job `134391555486`) | **pass** | 13m51s | 30 (`--with-scad`) |

The record counts reconcile exactly with `test-properties.toml`: 30 records total, 4 with
`requires_scad`, so 26 for the plain selector and 30 once the engine lane is enabled. The earlier run
on head `3504585f` was also fully green (9 of 9 checks; the same two lanes at 27m33s and 23m58s).

**Attribution is asserted, not inferred.** A green lane exit code does not prove which records ran —
an empty or mis-scoped selector exits 0, and this packet's own first `--with-scad`-less lane silently
covered 26 of 30. The per-record execution lines live in the runner's `target/test-output.log`, which
is uploaded only on failure, so a green run otherwise leaves no attribution in the PR.
`.github/workflows/ci.yml` therefore now asserts the IDs with the same `Select-String` idiom the SCAD
lane already used for `D-SCAD-2D`/`D-SCAD-3D`:

- `properties-windows` asserts all seven direct IDs (`H-BOOL-SET`, `H-BOOL-CLIP`, `H-OFFSET-MITER`,
  `H-OFFSET-SMOOTH`, `H-MESH-SLICE`, `H-MESH-QUERY`, `H-HELPERS-SPLIT`) and fails listing any that are
  missing;
- `properties-scad-windows` asserts `H-BOOL-SCAD` and `H-MESH-SCAD` alongside the two older IDs.

Both assertions passed on `2eb60366`, so the hosted logs prove by name that this packet's records
executed. The guard was validated locally in both directions before it was pushed (exit 0 against a
real run log, exit 1 naming all seven against a non-property log) rather than discovered in CI.

The `Test` job (workspace suite) was still running when this section was written and is **not**
claimed here; it passed on the parent head `3504585f`, and the only delta to `2eb60366` is the
workflow file.

## Controls and rejected mutations

The deterministic controls run under plain `cargo test` with no registry context:

- `geometry_controls_boolean_and_clip_mutations` — rejects a union returning only operand A, a
  difference that fills a hole (with both the rectangle and the general checker), a chord shortcut
  through a notch, a span left inside a notch, an omitted interior span, an endpoint outside the
  allowance, and the endpoints-and-midpoint-inside counterexample.
- `geometry_controls_offset_and_smooth_mutations` — rejects a reversed sign, the swapped pass order,
  a lost hole, a hole left unmoved, a setback equal to the chord, a rounded-up setback, a missing
  chamfer and an unchanged tiny CW ring.
- `geometry_controls_slice_and_query_mutations` — rejects a filled bottom, an empty top, an omitted
  interior section, a filled exterior section, a transposed transform, a missing exit hit and a
  fabricated hit.
- `geometry_controls_vertex_plane_and_hole_witnesses` — the cube top/bottom ownership pair, the
  through-vertex analytically expected triangle (double area `100_000_000`), and an annulus whose
  hole must survive a real `difference`.
- `geometry_controls_owned_shrink_witnesses` — the #2 `GeometryValueTree` audit described above.
- `geometry_controls_profile_bounds_and_envelopes` — zero, overflowing, inexact-float and
  out-of-range profile bounds are rejected before sampling; valid bounds convert losslessly.

Every arm runs the real SUT on a named fixture (which must pass the same checker) and then proves the
checker rejects a specific wrong result, naming the reason so a failure identifies the arm.

## Defects and regressions

- **One production defect was found and fixed.** `slicer_core::polygon_ops::xor` could emit a ring
  containing a **collinear backtracking spike**: three consecutive vertices `a → b → c` with
  `orientation(a,b,c) == 0` and `(b−a)·(c−b) < 0`, i.e. the boundary ran out to `b` and returned over
  itself, leaving a zero-width excursion. Such a ring fails the repo's own strict `validate_ring`
  with "ring edges intersect beyond their shared endpoint". Trigger: a redundant collinear vertex of one
  operand sitting on a boundary segment coincident with the other operand, with a component gap
  forcing the boundary to split there. Concrete fixture: chamfered octagon
  `(-95848,-88869), (-95840,-88869), (-95632,-88869), (-95632,-88669), (-95640,-88661), (-95840,-88661), (-95848,-88669)`
  plus a component `[-95624,-95616] × [-88869,-88661]`, against `[-95880,-95608] × [-88869,-88661]`.
  Fixed by normalising at the single ring-construction funnel (`oriented_ring` inside
  `collect_expolygon_from_tree`, which every tree-based output passes through, so all four wrappers
  and the offset paths share the fix): a vertex is deleted exactly when its neighbour vectors are
  collinear and reverse; forward collinear vertices are kept, consecutive and closing-seam
  duplicates are removed, and the removal iterates to
  a fixed point bounded by the vertex count, and a fully collapsed ring is dropped rather than emitted
  empty. The region is provably unchanged — the excursion has zero area and creates no vertex — and
  the regression asserts the exact analytic double area (`20096`) plus a nine-point membership truth
  table. Regression: `xor_redundant_collinear_vertex_emits_no_backtracking_spike`
  (`crates/slicer-core/tests/polygon_ops_tdd.rs`), RNG-free, no engine, no `PropertyRun` context.
  The defect was invisible until the ring predicate was tightened; the earlier tolerant predicate
  accepted it.
- Everything else agreed with its independent reference on every witness and generated case:
  `intersection`, `difference`, `union`, `clip_polylines`, `offset`, `offset2_ex`,
  `smooth_outward_polygon`, `slice_mesh_ex`, `transform_point3`, `AabbTree` and
  `split_connected_components`. No other production change was made and no other production
  regression is claimed.
- **One checker defect was found and fixed** (no production change): `clip::near_input`'s
  perpendicular test was `cross² > len_sq`, a one-unit tolerance, while the span tests in the same
  function already used `BAND²·len_sq` with `BAND = 2`. It therefore asserted a claim one unit
  stricter than `clip_polylines` documents and wrongly rejected a real 1.41-unit output. The test is
  now `BAND`-consistent and is pinned in both directions by control arms: the exact finding witness is
  accepted, a 2.159-unit displaced piece is rejected, and `near_input` accepts exactly 2.0 and rejects
  2.2 units on a 3-4-5 line. The wide pilot then passed 512/512. This is recorded as a checker fix,
  not a production-bug claim, per AC-N2.
- **One generator defect was found and fixed** in the offset step: drawing the annulus shape before
  its deltas left almost every annulus at `d1 = d2 = 0`. The generator now draws the cumulative deltas
  first and sizes the band and hole from them; no tolerance or assertion was changed.
- **Two further checker defects were found by the extended tier** and fixed (no production change):
  the `H-BOOL-SET` general ring predicate wrongly rejected a legitimate pinched `xor` output, and the
  `H-MESH-SLICE` sampler produced no interior candidate for a thin prism. Both are described in the
  Measurements section above. Neither is a production-bug claim.
- **Retained concrete regressions** (unchanged, re-run green):
  `test_shared_edge_with_opposite_windings_produces_closed_loop` and
  `test_slice_through_mesh_vertex_still_forms_closed_triangle` (`triangle_mesh_slicer_tdd`),
  `offset2_ex_collapse_returns_empty_no_panic` (`offset2_ex_collapse_tdd`),
  `clip_polylines_line_through_hole_split_around_hole` (`polygon_ops_tdd`) and
  `split_keeps_tiny_fragment` (`split_tdd`).

## Scripted enrichment disposition

Prerequisites #3 (`scad-adapter`) and #4 (`pbt-scripted`) are implemented, so the packet's condition
for the scripted family is met and the family **is implemented in this run**:

- `crates/slicer-core/tests/property_geometry_scad.rs` exists as a standalone target gated on
  `slicer-core/property_testing_scad`; its two modules
  (`property_geometry_common/{scad_boolean,scad_mesh}.rs`) are mounted behind the same `cfg`, so the
  direct targets never compile the engine.
- `H-BOOL-SCAD` (`proptest_scripted_boolean_membership`) and `H-MESH-SCAD`
  (`proptest_scripted_mesh_cross_sections`) are registered with `requires_scad = true`,
  `isolation = "fork"`, nonzero `timeout_ms` and a positive `bounds.test_process_timeout_ms`, so
  recipe generation and shrink rendering are supervised by the whole-process deadline and not only by
  a callback timeout.
- Both charters exist at `docs/testing/properties.md#h-bool-scad` and `#h-mesh-scad`.
- Registry lane, routine tier, `--with-scad`: **PASS**, 24 cases each, fork isolation, 9.52 s and
  0.76 s. Pilot with 8 cases: 2.14 s and 0.30 s, 0 rejected calls, `shrink_completed = null`.
- Engine provenance recorded by the artifacts: backend `manifold_cpp`, engine revision
  `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`, patch digest
  `c029707967be77f41f5bab502f0a1b1bbad24990c02fa91096a203548baa4024`.

Two limitations are recorded rather than smoothed over:

1. **#4's contour catalog is not scaled to the adapter envelope.** One third of contour draws come
   from `geometry_cases`, whose dimensions are capped independently of the profile, so a converted
   component can be one or two canonical units wide and contain no integer point at the 2-unit
   margin. Those components are enumerated exhaustively over their integer bounding box (bounded at
   65 536 points) and counted as `proved_unobservable` — a classification, not a skip: the observable
   components in the same case must still supply witnesses, and the exterior and rectangle-only
   populations are still asserted. Measured: 4 of 8 pilot cases had at least one such component and
   1 case had all components unobservable while 21 rectangle-only and 11 outside-both samples were
   still checked against all four operations. This is #4's contract, not a production defect, and no
   `slicer-sdk`/`slicer-scad` change was made.
2. **A small `cases` count does not reach every solid witness.** Seed 37 with 8 cases reached Bridge,
   ClosedSolid, Overhang and Cavity but not SeparatedBodies. Each witness is therefore also evaluated
   deterministically, so `SeparatedBodies` is covered by a named two-component witness regardless of
   the sampled distribution.

The combined G-SCAD / G-GEOMETRY rollout **is claimed for this packet**, on hosted evidence rather than
local evidence: `properties-scad-windows` ran and passed on head `2eb60366` (job `134391555486`,
13m51s) over all 30 records, and its lane now asserts that both scripted IDs executed. Both records
also pass at extended tier (96 cases each, `VERDICT: PASS`), and the scripted checkers' ability to
reject a filled hole, a dropped body and an all-outside answer is covered by deterministic control
arms.

Two attributions are worth stating precisely rather than glossing: hosted attribution comes from the
`Select-String` assertion over the runner's `target/test-output.log` (a printed per-record line is not
in the summary digest), and on the mesh side the all-outside case is detected by `check_plane`'s
component-count comparison rather than by a dedicated arm — an arm exists for the boolean side only.
That is a coverage note, not a behavioural gap: an empty layer where the model expects curves cannot
pass.

## Checks

| Check | Result |
|---|---|
| `cargo check -p slicer-core --all-targets --features property_testing,host-algos` | PASS |
| `cargo clippy -p slicer-core --all-targets --features property_testing,host-algos -- -D warnings` | PASS |
| `cargo check -p slicer-helpers --all-targets --features property_testing` | PASS |
| `cargo clippy -p slicer-helpers --all-targets --features property_testing -- -D warnings` | PASS |
| `cargo tree -p slicer-helpers -e normal` | 0 `slicer-core` and 0 `slicer-sdk` lines |
| `cargo xtask check-literals` | 0 violations |
| `cargo xtask check-test-quality --report` | 0 findings |
| `cargo xtask check-fmt` | 62 packages clean |
| `cargo test -p slicer-core --features property_testing,host-algos --test property_geometry_controls` | 6 passed, 0 failed |
| `cargo test -p slicer-helpers --features property_testing --test property_helper_controls` | 2 passed, 0 failed |
| `cargo xtask test --summary --workspace` — first attempt, fail-fast | 354 binaries, 4506 passed, 5 failed. **Superseded**: fail-fast stopped at the first failing binary, so its "nothing else failed" implication was unsupported |
| `cargo xtask test --summary --properties --tier routine --with-scad` — **all 30 records** | **PASS** (30 of 30 `Run status: Some(0)`) |
| `cargo xtask test --summary --properties --tier extended --with-scad` — **all 30 records** | **PASS** (30 of 30) |
| `cargo xtask test --summary --workspace --no-fail-fast` | 486 binaries, **5640 passed / 6 failed** in exactly two binaries: `slicer-sdk --test property_domains` (5, pre-existing) and `xtask --bin xtask` (1, this packet's own registry fallout — fixed, now 228/228) |
| `cargo test -p xtask --bin xtask` after the roster fix | **228 passed, 0 failed** |

**The `--with-scad` flag is load-bearing and was initially missed.** A plain
`--tier routine`/`--tier extended` selector covers only **26** of the 30 records; the four scripted
records (`D-SCAD-2D`, `D-SCAD-3D`, `H-BOOL-SCAD`, `H-MESH-SCAD`) are silently omitted without it.
An earlier revision of this document claimed "all 30 records" for the plain selector; that was wrong
and is corrected here. Both tiers were re-run with `--with-scad` and report 30 of 30.

**One workspace-sweep failure was this packet's own doing and is fixed.** The first fail-fast run
stopped at `slicer-sdk --test property_domains`, hiding a second: `xtask`'s
`properties::executor::tests::property_selector_rejects_conflicts_and_unknown_ids` pins the live
registry's `requires_scad` set to exactly `["D-SCAD-2D", "D-SCAD-3D"]`, so this packet's two SCAD
records broke it. The expectation now names the real four records (with a comment explaining that the
pin is deliberate), and the `xtask` binary is 228/228. This is registry fallout of the kind
`docs/spec_packets/README.md` gotcha 6 describes, and it was only visible because the sweep was
re-run with `--no-fail-fast`.

**An infrastructure failure occurred during the first extended-lane attempt and is recorded rather than
hidden.** Two `slicer-sdk` domain records (`D-IR`, `D-GEOMETRY`) failed with
`PropertyRunError { kind: "harness", message: "saving input before callback: There is not enough space on the disk. (os error 112)" }`.
The cause was the host: `D:` was 100% full, with `target/property-testing/` holding **32.9 GB** across
1712 per-run artifact directories. That directory is generated diagnostics, not source. 1705
unreferenced run tokens were pruned (every token cited by this document or by a charter was kept),
reclaiming 32 GB, and the lane was re-run to the PASS above. No code, assertion or profile was
changed to work around it, and the original failure is a resource error rather than a test result.

**The workspace sweep's five failures are pre-existing and not attributable to this packet.**
All five are `slicer-sdk --test property_domains` (`proptest_direct_{scalar,graph,ir,geometry,mesh}_domains`)
and every one panics in `PropertyRun::new` with
`PropertyRunError { kind: "missing_property_context" }` — these are registry-backed properties that
require the xtask harness context and fail identically whenever it is absent, before reaching any
geometry code. `property_domains` exercises the domain catalog, not `polygon_ops`, so the
`oriented_ring` change cannot affect them. Their supported entry point is
`cargo xtask test --properties`, not a bare workspace run. The guest freshness gate reported
`EXIT_FRESH`. Which *other* binaries failed could not be established from that fail-fast run — that
is what the `--no-fail-fast` row above establishes, and it did find one more failure, which was this
packet's own registry fallout and is fixed. This is recorded rather than excluded so a future reader does not
mistake a known-red harness precondition for a regression.
