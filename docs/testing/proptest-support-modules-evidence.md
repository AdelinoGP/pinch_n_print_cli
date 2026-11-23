# Evidence: independently checked support planner and renderer properties

Packet `docs/spec_packets/proptest-openrscad_14_pbt-support-modules/` (queue row 14 of
`docs/specs/proptest-openrscad-adoption.md`). This file records measured execution, the
independently derived oracles, the exact controls, the manual scanner review, and the confirmed
defect disposition for the support-module properties.

All ten charters are implemented, registered and executed at both measured tiers. Two confirmed SUT
defects were reproduced, fixed and given no-RNG regressions, and one further branch was hardened
against a degenerate input. §9 records the executed commands
and §10 the coverage ledger. An authoring obligation is still not execution evidence, so every
claim below is either a recorded command result or an explicit source inspection labelled as such.

**Closure status.** Four full packet-scope review rounds have run. The first three returned CHANGES
REQUESTED and every finding in them was addressed; the fourth returned **APPROVED WITH NOTES**, with
AC-9 satisfied once PR #9's `Properties (Windows)` job passed (§9.7). The notes asked for correct
attribution of the two `Test`-job regressions; this packet then fixed both of them, plus the stale
`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` clause, rather than deferring them, so the branch has no
carried-over failure from this work. `status` is `implemented`.

## 1. Provenance

| Field | Value |
| --- | --- |
| Source revision | `a823db91ff58ee7ef3e3ce03fef3a8e5317ae49f` on branch `feature/proptest-openrscad` |
| Dirty paths at the baseline capture | none (`git status --porcelain` empty) |
| Toolchain | `cargo 1.96.0 (30a34c682 2026-05-25)`, `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Platform | Windows, native host build |
| `CARGO_TARGET_DIR` | unset; the workspace default `target/` for every command recorded below |
| Guest artifacts | `cargo xtask build-guests --check` exit `0` (`EXIT_FRESH`) before any test interpretation |
| Workspace Proptest | `proptest = "1.11"`, `default-features = false`, features `std`, `fork`, `timeout` |

The baseline logs are archived under `target/baseline-support/` (and were copied out before the
`target/` tree was reset; see §1.1). The dirty-path list is re-derived at each measured run and
written into the `PropertyRun` context/artifact record by the SDK harness — the context's
`source_revision`, `dirty_paths` and `source_state_digest` fields — so a replay cannot present a
stale provenance line as current.

### 1.1 Disk-space incident (affects which artifacts still exist, not which results are valid)

Mid-run the workspace drive reached **0 bytes free** (`D:` 448 GB used, 0 available) with
`target/` at **214 GB**. Two commands in one owner's session failed with
`error: linking with link.exe failed: exit code: 1140` and
`failed to write file ...dep-graph.part.bin: There is not enough space on the disk. (os error 112)`;
that owner re-ran both under a separate `CARGO_TARGET_DIR` and both passed there. `target/` was then
deleted to recover **213 GB**, so every build artifact recorded before that point is gone and the
later commands in §9 rebuild from scratch. The deletion is not a test-result change: no source file
was touched by it.

## 2. Predecessor handoffs actually in the tree

Packets 01, 02 and 11 are `status: implemented` at this revision. The handoffs this packet consumes
are the live ones:

| Handoff | Live identity |
| --- | --- |
| Harness | `slicer_sdk::test_support::property_testing::PropertyRun` (`new`, `bound`, `check`) and `CaseGuard` (`crates/slicer-sdk/src/test_support/property_testing/run.rs`, `.../property_testing/mod.rs`) |
| Domains | `...::property_testing::domains::{DomainError, GeometryBounds, GeometryDomain, GeometryCase, TopologyClass, BoundaryRule, geometry_cases, geometry_complexity}` (`crates/slicer-sdk/src/test_support/property_testing/domains/mod.rs`) |
| Oracles | `...::property_testing::oracles::{validate_geometry, point_in_region, segment_in_region, segment_relation, signed_area2, PointLocation, SegmentRelation}` (`crates/slicer-sdk/src/test_support/property_testing/oracles/mod.rs`) |
| Regression formatter | `...::property_testing::regression::{ConcreteInput, format_regression_fixture}` (`crates/slicer-sdk/src/test_support/property_testing/regression/mod.rs`) |
| Registry | `test-properties.toml` (`schema_version = 1`, `[[property]]` records with `routine`/`extended` profiles) and `cargo xtask test --properties` |

There is no support-specific SDK helper and no `MeshDomain` in the live tree. Every rectangle, raster,
flood, interval, graph and scene builder used by these properties is private to its owner test
module.

Territory and identity ownership is unchanged from #11: the host builds the `(priority, semantic
name)` chain, and this packet supplies already-ordered legal chains and preserves exact vectors.
The empty chain means chain-less, never a wildcard, and no test in this packet sorts a chain.

## 3. Native baseline before any edit

Each command below ran separately through the spec's status-preserving capture wrapper; the log for
each is archived under `target/baseline-support/`.

| # | Package | Command target | Result |
| --- | --- | --- | --- |
| 1 | `traditional-support-planner` | `--test agg_rasterizer_tdd rasterize_polygons_marks_any_covered_cell` | PASS (1 passed) |
| 2 | `traditional-support-planner` | `--test agg_rasterizer_tdd seed_fill_block_closes_gaps_within_a_block_but_not_across_blocks` | PASS (1 passed) |
| 3 | `traditional-support-planner` | `--test agg_rasterizer_tdd fill_holes_bridges_single_cell_holes_in_one_pass` | PASS (1 passed) |
| 4 | `traditional-support-planner` | `--test agg_rasterizer_tdd agg_path_still_declines_no_route_when_occupancy_closes_every_layer` | PASS (1 passed) |
| 5 | `traditional-support-planner` | `--test agg_rasterizer_tdd agg_does_not_decline_no_route_where_legacy_semantic_does` | PASS (1 passed) |
| 6 | `traditional-support-planner` | `--test traditional_family_tdd candidate_with_no_downward_route_is_declined` | PASS (1 passed) |
| 7 | `tree-support-planner` | `--lib tests::smooth_nodes_pulls_merge_absorbed_node_toward_the_column` | PASS (1 passed) |
| 8 | `tree-support-planner` | `--lib tests::build_roles_batched_matches_the_serial_path_per_item` | PASS (1 passed) |
| 9 | `tree-support-planner` | `--test tree_family_tdd disabled_and_declined` | PASS (1 passed) |
| 10 | `tree-support-planner` | `--test tree_family_tdd foreign_territory_bars_tree_roles_and_skeleton` | PASS (1 passed) |
| 11 | `tree-support` | `--test tree_family_tdd declined_or_empty_plan_has_no_fallback_paths` | PASS (1 passed) |
| 12 | `tree-support` | `--lib tests::narrow_region_gets_one_fill_line_when_pitch_has_no_scan_rows` | PASS (1 passed) |
| 13 | `traditional-support` | `--test support_fill_geometry_tdd scan_starts_at_rotated_bbox_min` | PASS (1 passed) |
| 14 | `support-surface-ironing` | `--test ironing_scanline_parity_tdd scan_starts_at_bbox_min` | PASS (1 passed) |
| 15 | `support-surface-ironing` | `--test ironing_tdd flow_rate_applied` | PASS (1 passed) |
| 16 | `slicer-sdk` | `--test property_domain_contracts domain_exact_predicates_counterexamples` | PASS (1 passed) |
| 17 | `slicer-sdk` | `--test property_harness case_guard_cleans_success_failure_and_unwind` | PASS (1 passed) |

All seventeen baseline commands were green before the first edit, so a later red cannot be
attributed to a pre-existing failure. No workspace test suite was run and none is an acceptance
criterion.

## 4. Owner and kernel boundaries re-checked at this revision

- `traditional-support-planner`: `rasterize_polygons`, `fill_grid_holes`,
  `dilate_trimming_region`, `seed_fill_block`, `contours_simplified`, `GridParams` and
  `SupportGrid::{new,for_test,params,grid,extract_islands,filter_islands_by_samples,extract_support}`
  are public in `modules/core-modules/traditional-support-planner/src/agg_raster.rs`.
  `SupportPlanner::plan_candidate` uses `SupportGrid::new`, `extract_islands` and
  `filter_islands_by_samples` on the `RasterizerMode::Agg` descent arm and does **not** call
  `extract_support`; the `extract_support` composition is therefore a public seam, never a
  production caller. `SupportPlanner::clip_to_territory` is private and is called in planning after
  extraction. `RasterizerMode::LegacySemantic` remains the default and AGG is opt-in.
- `tree-support-planner`: `smooth_nodes` is private and runs after pruning, before drawing and
  emitting. `carve_emitted_regions` and `build_roles` are `#[doc(hidden)]` public test seams; the
  serial `build_roles` has no production caller. Production emission goes through
  `flush_pending_emits` → `build_roles_batched` over private `RoleBuildInput`. The batched path
  additionally intersects `own_territory`; the serial path has no such argument.
- `traditional-support`, `tree-support`, `support-surface-ironing`: `fill_expolygon`,
  `scan_fill_region` and `render_polygon_with_wall_count` are private, so their properties belong in
  a crate-internal `src/property_tests.rs`. Production callers are respectively
  `TraditionalSupport::run_support`, `TreeSupport::run_support` → `render_polygon_with_wall_count` →
  `scan_fill_region`, and `SupportSurfaceIroning::run_support_postprocess`.
  `TreeSupport::render_polygon` is a test-used wrapper and is not the production
  explicit-wall-count path. `vertical` in `scan_fill_region` is the production parameter
  `entry.anchor_layer_index % 2 != 0`.
- Kernel output is `ExtrusionPath3D` with `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`).
  Every checker walks **all** consecutive point pairs of every path, including closing edges.

## 5. Preserved divergences

`DEV-143` (tree smoothing f64 relaxation, round on commit), `DEV-165` (PnP own/foreign territory),
`DEV-166` (AGG block snapping may enlarge the extracted carry and may avoid a local blocker that
yields `NoRoute` in legacy mode), `DEV-142`/`DEV-144`/`DEV-156` (draw simplification, extra wall,
organic style) and `DEV-127`/`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE`/`D-209-IRONING-FILL-BEHAVIOUR-CHANGE`/
`D-209-TANGENTIAL-TOUCH-SPAN-SPLIT` are untouched. No identifier is allocated, closed or reworded by
this packet. No test asserts bit-exact upstream truncation, no test asserts a universal
extracted-subset-of-input relation, and no test asserts cross-layer area monotonicity.

`DEV-178` records that the `need_extra_wall` carrier has no renderer consumer. Nothing in this
packet asserts that a renderer turns the flag into extra walls; `M-TREE-RENDER` exercises the
explicit wall count as the kernel argument it is, not as evidence that the flag reached it.

## 6. Source-inspected candidate witnesses

The packet named two witnesses as source-inspected candidates. Both were executed against the
unmodified tree before any decision was taken:

- the `M-TRADITIONAL-SCAN` sub-pitch diamond, which returns an empty path set for the reasons in
  §8 and is **not** a defect;
- the `M-TREE-SCAN` tall vertical rectangle, which returned no strokes and **is** a defect (§7).

## 7. Confirmed SUT defect: `M-TREE-SCAN` vertical guard compares the wrong axis

`TreeSupport::scan_fill_region` (`modules/core-modules/tree-support/src/lib.rs`) gathers crossings
along the scan axis and pairs them into segments, then guarded each pair with
`pair[0] >= min_x && pair[1] <= max_x`. In `vertical` mode the pair holds **Y** coordinates and the
emitted segment runs along Y, but the guard compared them against the region's **X** extent.

Derived from the source and then executed: for the `[0, 10] x [0, 20]` mm rectangle with 2.5 mm
spacing and no holes, the scan positions are `1.25, 3.75, 6.25, 8.75` mm and every crossing pair is
`(0.0, 20.0)`, so `pair[1] <= max_x` is `20.0 <= 10.0 = false` and **no stroke was emitted**; the
mid-scan fallback at `x = 5.0` was rejected by the same comparison. `vertical` is the production
setting on every odd anchor layer, so every odd-anchor-layer tree region taller than it is wide lost
its fill entirely.

Disposition: confirmed SUT defect, reproduced before the fix. The focused fix selects the pair
extent from the segment's own axis (`min_y`/`max_y` when `vertical`, `min_x`/`max_x` otherwise) and
changes nothing else. The no-RNG regression
`vertical_scan_fill_spans_the_full_y_extent_regression` lives in the existing
`#[cfg(test)] mod tests` of `modules/core-modules/tree-support/src/lib.rs`, needs neither RNGs nor
`PNP_PROPERTY_CONTEXT`, carries the root cause and the previously failing input in its doc comment,
and is run by §9's command set. The executed pre-fix and post-fix results are in §9.1.

## 7.1 Confirmed SUT defect: the centroid fallback drops the bbox-centre translation

`TraditionalSupport::fill_expolygon` (`modules/core-modules/traditional-support/src/lib.rs`)
rotates every edge into bbox-centre-relative coordinates once, scans there, and adds `refpt_x` /
`refpt_y` back when it converts a crossing to millimetres. The centroid fallback — the branch
reached when the scan loop emits no pair — performed the same rotation but omitted that
translation, so on any polygon whose bbox centre is not the origin it emitted a band around the
*relative* centroid, i.e. outside the polygon. Because the fallback ran before the
interface-contour branch, its bogus output also suppressed a correct printable fallback.

Executed: the downward-pointing triangle `(5,4), (6,4.5), (4,4.5)` mm at angle 0 with a 2 mm pitch
has a 0.5 mm height, so the only scanned row is the apex at `y = 4` mm and its two crossings
coincide. The required fallback band is `(4.5, 4.25) -> (5.5, 4.25)` mm; the pre-fix kernel
emitted `(-0.5, 0) -> (0.5, 0)` mm. Fix: add the same translation the scan-line branch adds, which
restores the documented behaviour rather than changing it. Regression
`narrow_translated_tip_fallback_is_placed_at_the_bbox_centre_regression`.

## 7.2 Degenerate-input hardening: the centroid fallback emitted zero-length strokes

The scan-line branch drops a crossing pair whose endpoints coincide (`x_start == x_end`); the
centroid fallback pushed every pair unconditionally. A two-point ring is traversed once in each
direction, so both of its centroid-row crossings always coincide and the branch emitted one
zero-length path whose start and end were the same point.

Executed: contour `(0,0), (1,1)` mm at angle 0 with a 2 mm pitch returned one path whose two
points were both `(0.5, 0.5)` mm. Fix: apply the scan branch's degeneracy rejection in the fallback
too. Regression `two_point_ring_emits_no_zero_length_fallback_regression`.

**Classification: hardening, not a confirmed production defect.** A two-point ring is not valid
geometry, so no reachable production input was demonstrated that loses material here. The change is
kept because the main branch already enforces the same invariant, the fallback emitting a
zero-length extrusion is wrong on its face, and the guard is four lines with no behavioural effect
on any non-degenerate input. `D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` itself lists "zero-length-span removal" among the behaviours
packet 209 intended, and `D-209-TANGENTIAL-TOUCH-SPAN-SPLIT` records the same intent for the scan
branch, so the guard restores a documented intention rather than inventing a new rule. It is still
recorded as hardening rather than a confirmed defect, because no reachable production input was
demonstrated to lose material, so that AC-N2's "confirmed defect" allowance is spent only on §7 and
§7.1.

## 8. Classification: the sub-pitch traditional diamond is not a defect

`TraditionalSupport::fill_expolygon` (`modules/core-modules/traditional-support/src/lib.rs`) scans
from the rotated bbox minimum in `line_spacing` steps and drops a pair when `x_start == x_end`. For
the diamond `(5,4.5), (5.5,5), (5,5.5), (4.5,5)` mm the single scan row at `y = 4.5` mm produces both
crossings at `x = 5.0` mm (both incident edges share that lower endpoint), so the pair is dropped and
the loop ends. The centroid fallback that follows requires an edge strictly straddling
`centroid_y`; at `y = 5.0` mm the diamond's widest row coincides with the bbox midpoint, so no edge
straddles it and the fallback emits nothing either.

The measured outcome is an **empty** path set. That is a permitted outcome for this input: the
packet's scope excludes an unconditional nonempty-fill guarantee, and the branch's precondition is
not met for this witness.

**Reconciliation (needed before §7.1 could be called a fix).** The packet required deciding whether
the fallback is part of the contract before touching it. The live source answers that twice: the
scan-fallback branch carries the comment "Drop a single horizontal segment across the polygon's
centroid so any non-empty support polygon yields at least one fill path", and a second, later
branch deliberately emits the contour when the region is an interface so "the contact-inclusive
interface layer" stays printable. Two independent branches exist to guarantee a non-empty printable
result, so the fallback **is** intended behaviour in this tree.
`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` records "removal of the centroid fallback", which contradicts
both the code and its comments. The correct reading is that the deviation row is stale relative to
the live implementation, not that the branch should be deleted. §7.1's fix therefore restores the
live contract (the missing bbox-centre translation) rather than changing it, which is why it was
allowed to proceed. Correcting the deviation row itself is a separate authorised action and is
**not** done here; this packet allocates and rewords no identifier.

## 9. Executed verification

### 9.1 The confirmed defect and its regression

Red/green proof, both runs on this tree. The pre-fix run was produced by toggling only the guard
expression back to `pair[0] >= min_x && pair[1] <= max_x` while the (then unused) segment-extent
binding stayed in place, so the compiler reported two `unused variable` warnings; the warning noise
is an artifact of the toggle and the executed predicate is byte-identical to the original guard.

| Command | Result |
| --- | --- |
| `cargo test -p tree-support --no-default-features --lib -- tests::vertical_scan_fill_spans_the_full_y_extent_regression --exact --nocapture` (pre-fix) | **FAILED** — `assertion left == right failed: expected four vertical strokes at x = 1.25, 3.75, 6.25, 8.75 mm, got []` with `left: 0`, `right: 4` |
| the same command (post-fix) | PASS — 1 passed, 0 failed (5 filtered out) |
| `cargo test -p tree-support --no-default-features --lib` | PASS — 6 passed |
| `cargo test -p tree-support --no-default-features --test tree_family_tdd` | PASS — 5 passed |
| `cargo test -p tree-support --no-default-features --test tree_support_tdd` | PASS — 18 passed |
| `cargo test -p tree-support --no-default-features --test enforcer_blocker_tdd` | PASS — 8 passed |
| `cargo test -p tree-support --no-default-features --test slicer_module_binding_tdd` | PASS — 1 passed |

`tree_support_tdd` contains the crate's path-count assertions, and it stayed green: the fix removes
suppressed fill on odd anchor layers and changes no other mode. Logs: `target/verify14/`.

### 9.2 `M-SUPPORT-IRONING`

`SupportSurfaceIroning::fill_expolygon` at
`modules/core-modules/support-surface-ironing/src/lib.rs`; module
`modules/core-modules/support-surface-ironing/src/property_tests.rs`. The module is constructed
through `from_config`, the speed factor is computed the way the production caller computes it
(`ironing_speed / BASE_SPEED`), and one `check_fill` function serves the property, the five
deterministic witnesses, every control and the shrink probe. It validates the fixture, checks all
path and point metadata, runs `segment_in_region` with `BoundaryRule::Inclusive` on every
consecutive segment, requires every output coordinate to recover to whole canonical units
bit-exactly, and only then compares the complete sorted `(y, x_start, x_end)` set against rows and
intervals derived from the fixture's own edges.

| Command | Result |
| --- | --- |
| `cargo test -p support-surface-ironing --no-default-features --features property_testing --lib -- property_tests::support_ironing_controls --exact --nocapture` | PASS — 1 passed, 0.10 s |
| `cargo test -p support-surface-ironing --no-default-features --features property_testing --lib -- property_tests::support_ironing_shrink_bounds --exact --nocapture` | PASS — 1 passed, 18.82 s |
| `cargo test -p support-surface-ironing --no-default-features --lib` | PASS — 1 passed |
| `cargo test -p support-surface-ironing --no-default-features --test ironing_tdd` | PASS — 11 passed |
| `cargo test -p support-surface-ironing --no-default-features --test ironing_scanline_parity_tdd` | PASS — 3 passed |
| `cargo check -p support-surface-ironing --all-targets --no-default-features --features property_testing` | PASS |
| `cargo clippy -p support-surface-ironing --all-targets --no-default-features --features property_testing -- -D warnings` | PASS |
| `cargo check -p support-surface-ironing --all-targets` (feature off) | PASS |

Registry-backed lanes, `cargo xtask test --summary --properties --property M-SUPPORT-IRONING --tier …`
(the runner owns its combined log; copies are in `target/verify14/property-*.log`):

| Tier | libtest | Accepted | Rejected | Controller events |
| --- | --- | --- | --- | --- |
| routine (64 cases) | 2.12 s | 64 | 0 | `begin` + `end`, `process_role = controller` |
| extended (256 cases) | 19.14 s | 256 | 0 | `begin` + `end`, `process_role = controller` |

Observed per-subcase counters, printed by the property after `check`:

| Tier | rect | notch | hole | degenerate | strokes = segments | split rows |
| --- | --- | --- | --- | --- | --- | --- |
| routine | 16 | 18 | 18 | 12 | 474 | 126 |
| extended | 57 | 61 | 66 | 72 | 14 402 | 2 956 |

Witness counters (each asserted `> 0`): charter holed `[4,6]^2` split 64 / 256, asymmetric-hole split
128 / 512, notch split 384 / 1 536, square rows 640 / 2 560, degenerate-empty modes 448 / 1 792.
`shrink_completed` is `null` in both tiers — the run does not claim a minimal counterexample.

Retained-seed replay: `--tier routine --seed 20261121` was run twice and produced **identical**
counters both times (`accepted_calls=64 rect=18 notch=18 hole=16 degenerate=12 strokes=455
segments=455 split_rows=112`, witnesses 64 / 128 / 384 / 640 / 448), so the stream is reproducible
from the recorded seed alone.

Whole-registry closure, both re-run on the finished tree (`f4ffe694`, the last commit before an
`rustfmt`-only pass, so the measured behaviour is that of the head):

| Command | Result |
| --- | --- |
| `cargo xtask test --summary --properties --tier routine` | **67 property binaries, 67 green, `VERDICT: PASS`**, exit 0 |
| `cargo xtask test --summary --properties --tier extended` | **67 property binaries, 67 green, `VERDICT: PASS`**, exit 0, no failing line |

Logs: `target/verify14/property-full-routine-final.log` and
`target/verify14/property-full-extended-final.log`. The head commit after these runs is
`ec75a181`, which changes formatting only (`cargo xtask fmt`); `cargo xtask check-fmt` reports all
62 packages clean at that commit and the three touched crates' suites and clippy gates were re-run
green afterwards.

The routine command is the exact one the required Windows PR job (`properties-windows` in
`.github/workflows/ci.yml`) runs, and that job now also asserts all ten of this packet's record IDs
appear in `target/test-output.log`, so a selector that silently stopped covering them would fail
the job rather than pass quietly. Both lanes execute every selected record, which is what makes the
ten IDs discovery-checked alongside the rest instead of only under a selective filter. Earlier in
this packet a 58-binary routine run was recorded from the checkpoint commit; it covered only one of
the ten records and has been superseded by the runs above.

Manual FRU and test-quality review of the new module-source file: `cargo xtask check-literals`
printed `0 violation(s) in 0 file(s)` and `cargo xtask check-test-quality --report` is clean, but
**neither gateway scanned this file** — `collect_enforced_files` omits
`modules/core-modules/*/src/**` and `LiteralVisitor::visit_item_mod` does not follow the out-of-line
declaration. The manual review found exactly two watched-type literal groups and two waivers:
`Point3WithWidth` literals in `stroke_like` use FRU over a real emitted point
(`..template.points[0]`, `..template.points[1]`); `GeometryBounds` and `GeometryDomain` each have one
literal carrying `// exhaustive: …` because every field derives from the live profile bounds or the
fixture's topology class. `ExtrusionPath3D` has no literal — mutants clone a real emitted path. The
controls are mutation controls over the same `check_fill`, each asserting the specific
`CheckFailure` class through `expect_failure`, so none is a success-only assertion.

No production defect was found in this kernel, so it has no fix and no regression.

### 9.3 `M-TRADITIONAL-SCAN`

`TraditionalSupport::fill_expolygon` at
`modules/core-modules/traditional-support/src/lib.rs`; module
`modules/core-modules/traditional-support/src/property_tests.rs`. One `check_fill` serves the
property, the five deterministic witnesses, every control and the shrink probe. It compares the
complete sorted `(row, x_start, x_end)` set for angle-zero modes, checks whole-segment containment
on every consecutive pair for all modes, requires every output coordinate to recover to a whole
canonical unit, and asserts the six degenerate classes' actual empty outcome.

| Command | Result |
| --- | --- |
| `cargo test -p traditional-support --no-default-features --features property_testing --lib -- property_tests::traditional_renderer_controls --exact --nocapture` | PASS — 1 passed, 0.07 s |
| `cargo test -p traditional-support --no-default-features --features property_testing --lib -- property_tests::traditional_renderer_shrink_bounds --exact --nocapture` | PASS — 1 passed, 4.27 s |
| `cargo test -p traditional-support --no-default-features --lib` | PASS — 4 passed, including both new regressions |
| `cargo clippy -p traditional-support --all-targets --no-default-features --features property_testing -- -D warnings` | PASS |
| `cargo check -p traditional-support --all-targets` (feature off) | PASS |

Red/green proof for both defects on this tree (the pre-fix runs toggle only the fixed expression):

| Command | Result |
| --- | --- |
| centroid-translation regression, pre-fix | **FAILED** — `left: (-0.5, 0.0)`, `right: (4.5, 4.25)` |
| the same command, post-fix | PASS |
| zero-length regression, pre-fix | **FAILED** — one path with both points at `(0.5, 0.5)` mm |
| the same command, post-fix | PASS |

Registry-backed lanes, `cargo xtask test --summary --properties --property M-TRADITIONAL-SCAN --tier …`:

| Tier | libtest | Accepted | Rejected | Mode mix (rect/notch/hole/rotated/degenerate) | strokes = segments | rows | split rows |
| --- | --- | --- | --- | --- | --- | --- | --- |
| routine (64 cases) | 2.28 s | 64 | 0 | 12 / 11 / 16 / 10 / 15 | 449 | 389 | 60 |
| extended (256 cases) | 3.9 s | 256 | 0 | 50 / 55 / 65 / 41 / 45 | 1 896 | 1 606 | 290 |

`witness_rotated_split` is 192 routine / 768 extended, i.e. the 180-degree cut-carrying witness ran
in every callback and split three rows each time; the sampler alone does not guarantee that
subcase, so it is checked deterministically. Those two figures are re-measured at `18e8cdb5`
(`target/verify14/ts-r.log`, `target/verify14/ts-e.log`), the commit that introduced the witness.

A bare run of the property without `PNP_PROPERTY_CONTEXT` fails with
`missing_property_context`, which is the harness's documented behaviour and not a test failure;
the six other lib tests in that binary pass bare.

### 9.4 `M-SUPPORT-RASTER`, `M-SUPPORT-HOLES`, `M-SUPPORT-GRID`, `M-SUPPORT-TERRITORY`

`modules/core-modules/traditional-support-planner/src/property_tests.rs` (new) against
`agg_raster.rs` and `SupportPlanner::clip_to_territory`. One checker per property serves the
property, the deterministic witnesses and the mutation controls; `traditional_planner_controls`
and `traditional_planner_shrink_bounds` are the two unregistered lib tests.

| Command | Result |
| --- | --- |
| `cargo test -p traditional-support-planner --no-default-features --features property_testing --lib -- property_tests::traditional_planner_controls --exact --nocapture` | PASS — 1 passed, 0.03 s |
| `cargo test -p traditional-support-planner --no-default-features --features property_testing --lib -- property_tests::traditional_planner_shrink_bounds --exact --nocapture` | PASS — 1 passed, 17.30 s |
| `cargo check` / `cargo clippy -D warnings` (feature on, `--all-targets`) | PASS |
| `cargo check -p traditional-support-planner --all-targets` (feature off) | PASS |
| `cargo test -p traditional-support-planner --no-default-features` | PASS — 50 passed across 4 binaries, 0 failed |

Registry-backed lanes (identical selective command with `--tier routine` / `--tier extended`):

| ID | routine accepted | extended accepted | representative routine counters |
| --- | --- | --- | --- |
| `M-SUPPORT-RASTER` | 64 | 256 | `rect=18 notch=17 hole=6 multi=14 no_work=9 cells=1824 set=586` |
| `M-SUPPORT-HOLES` | 64 | 256 | `grid=23 tiny=19 short=22 cells=594 filled=23` |
| `M-SUPPORT-GRID` | 64 | 256 | `proven_blocks=74 proven_seeded=70 proven_masked=16 filled=422 islands=… selected=…` |
| `M-SUPPORT-TERRITORY` | 64 | 256 | `own=7 base=9 no_partition=9 own_family_only=15 empty_subject=11 fully_removed=13 kept=121 removed=95 clearance=20` |

All four ran 0 rejected test calls at both tiers, and every witness counter was positive.

### 9.4a `M-TREE-SMOOTH` and `M-TREE-ROLES`

`modules/core-modules/tree-support-planner/src/property_tests.rs` (new) against the private
`smooth_nodes` and the hidden `carve_emitted_regions` / `build_roles` seams plus the production
`build_roles_batched`. `tree_planner_controls` and `tree_planner_shrink_bounds` are the two
unregistered lib tests.

| Command | Result |
| --- | --- |
| `cargo test -p tree-support-planner --no-default-features --features property_testing --lib -- property_tests::tree_planner_controls --exact --nocapture` | PASS — 1 passed, 0.00 s |
| `cargo test -p tree-support-planner --no-default-features --features property_testing --lib -- property_tests::tree_planner_shrink_bounds --exact --nocapture` | PASS — 1 passed (8.00 s for both lib tests together) |
| `cargo check` / `cargo clippy -D warnings` (feature on, `--all-targets`) | PASS |
| `cargo check -p tree-support-planner --all-targets` (feature off) | PASS |
| `cargo test -p tree-support-planner --no-default-features` | PASS — 146 passed across 11 binaries, 0 failed |

| ID | routine accepted | extended accepted | representative routine counters |
| --- | --- | --- | --- |
| `M-TREE-SMOOTH` | 64 | 256 | routine `straight=20 bent=17 short=13 mixed=14 interior=380 straight_interior=145 short_nodes=56 flagged=282`; extended `straight=79 bent=63 short=54 mixed=60 interior=3904 straight_interior=1895 short_nodes=242 flagged=2958` |
| `M-TREE-ROLES` | 64 | 256 | routine `split=13 swallow=19 no_collision=18 territory=14 items=129 body_cells=615 interface_cells=1243 territory_removed=663 swallowed=60 split_regions=86`; extended `split=60 swallow=67 no_collision=65 territory=64 items=669 body_cells=7087 interface_cells=23770 territory_removed=10166 swallowed=377 split_regions=801` |

0 rejected test calls at both tiers. The `role_simplify_tolerance` domain is pinned by the controls:
`None` for `avg_node_per_layer` `0..=200` at line widths `1/64`, `0.4` and `2.0` mm for both
`is_base_area` values, and the first `Some(2000.0)` at `avg = 201`, width `0.4` mm, base area — which
is what makes the rectangular role model exact.

### 9.5 `M-TREE-SCAN` and `M-TREE-RENDER`

`modules/core-modules/tree-support/src/property_tests.rs` (new) against
`TreeSupport::scan_fill_region` and `TreeSupport::render_polygon_with_wall_count`;
`tree_renderer_controls` and `tree_renderer_shrink_bounds` are the two unregistered lib tests.

| Command | Result |
| --- | --- |
| `cargo test -p tree-support --no-default-features --features property_testing --lib -- property_tests::tree_renderer_controls --exact --nocapture` | PASS — 1 passed, 0.29 s |
| `cargo test -p tree-support --no-default-features --features property_testing --lib -- property_tests::tree_renderer_shrink_bounds --exact --nocapture` | PASS — 1 passed, 83.46 s |
| `cargo check` / `cargo clippy -D warnings` (feature on, `--all-targets`) | PASS |
| `cargo check -p tree-support --all-targets` (feature off) | PASS |
| `cargo test -p tree-support --no-default-features` | PASS — 38 passed across 6 binaries, 0 failed, including the vertical-guard regression |

| ID | routine accepted | extended accepted | representative routine counters |
| --- | --- | --- | --- |
| `M-TREE-SCAN` | 64 | 256 | `rect=9 notch=12 hole=12 fallback=14 degenerate=17 vertical_cases=31 strokes=262 split_rows=51` |
| `M-TREE-RENDER` | 64 | 256 | `walls=12 holed_walls=13 zero_wall=15 zero_wall_holed=14 collapse=10 wall_loops=69 fill_strokes=267 split_rows=48` |

0 rejected test calls at both tiers. `M-TREE-SCAN`'s `witness_vertical_strokes=256` is the
corrected four-stroke witness firing on all 64 routine calls, which is the property-level
counterpart of §7's regression.

### 9.5a Why these bounds

Every owner validates its own bound map before the property runs, so a value that cannot hold the
deterministic witnesses fails rather than silently narrowing the domain. The recorded values are
the smallest sets that pass those validators:

- `M-SUPPORT-RASTER` uses `80` absolute units, pixel `10` and an `8x8` grid. Those are exactly the
  minima `raster_limits` enforces (`RASTER_WITNESS_ABS` / `RASTER_WITNESS_DIM`) while still
  admitting the packet's witness — a rectangle `[11,12] x [11,19]` grid units on pixel size 10 —
  and the generator does scale with every one of them, so a larger ceiling would produce larger
  scenes. Raising them adds cases, not new classes; the extended tier therefore keeps them and
  spends its extra budget on more cases instead.
- `M-SUPPORT-HOLES` is `6x5` / `25` cells. They are the minima `hole_limits` enforces while still
  admitting the 5x5 cascading witness plus its one-cell and two-cell gap variants, and
  `build_hole_case` scales with them too. As above, larger values buy cases rather than classes.
- `M-SUPPORT-GRID` needs `max_blocks = 2` for the two-block isolation witness and
  `max_abs_units = 40_000` because the AGG grid is built from polygons, not from a cell rectangle;
  `max_pixel_units = 5_000` is the largest `COMBOS` pixel the generator chooses from.
- `M-SUPPORT-TERRITORY` needs `100_000` absolute units for the `[0,10]^2` subject and `10_000` for
  the 1 mm clearance; `121` cells is the classification grid the witness extent needs.
- `M-TREE-SMOOTH` is `24` nodes / `12` layers (`max_layers` must be in `5..=256`) and an even
  `10_000`-unit width, which the validator requires because the flag threshold is
  `width_units / 2`.
- `M-TREE-ROLES` needs `300_000` absolute units because its second fixture is translated by 20 mm
  and reaches x = 30 mm, and `max_cells = 289` is `(2 * (max_regions + 3) - 1)^2` for `max_regions
  = 6` — the grid the cell-partition model needs.
- `M-TRADITIONAL-SCAN` needs `200_000` absolute units for the 10 mm witnesses, `10_000` for the
  1 mm spacing, and `16` rows, which is also the generator's rotated-width cap.
- `M-TREE-SCAN` needs `200_000` absolute units for the `[0,10] x [0,20]` mm vertical witness and
  `25_000` spacing units for its 2.5 mm pitch.
- `M-TREE-RENDER` needs `100_000` for the 10 mm square and `5_000` width units for the charter's
  0.5 mm line width.
- `M-SUPPORT-IRONING` shares the 10 mm / 1 mm / 10-row witness frame; its extended tier widens
  `max_abs_units` and `max_rows` because the 1 mm-spacing fixture is generated at a larger scale
  there.

Seven records use the same bounds at both tiers, and the reason is cost, not capability: both
oracles are exact for any size, so wider bounds would legitimately generate larger scenes. The
extended tier is already the slow lane — `M-TREE-RENDER` 18.77 s and `M-SUPPORT-IRONING` 18.78 s,
the two heaviest of the ten, both from `target/verify14/property-all-extended.log` — and
widening geometry there multiplies the per-case cost of every generated scene, while the case count
already rises fourfold. The three records that *do* widen — `M-SUPPORT-IRONING`, `M-TREE-SMOOTH`
and `M-TREE-ROLES` — are the ones whose extended bounds were measured to admit a larger witness
set without that cost.

### 9.5b One item left unverified

`tree-support`'s `CLIPPER_ALLOW = 2` canonical units is a **derived** allowance, not a measured
epsilon: the derivation bounds the offset delta as exact, the vertex rounding at 0.5 unit, the scan
re-derivation at 0.5 unit plus an `f32` relative error below 0.06 unit, and then doubles the total.
The one input that was not independently checked is the backend's actual rounding rule for an
offset vertex. The port's own comment says Clipper2 rounds to nearest, and the derivation budgets
0.5 unit for that; a rule that truncated instead could be off by up to one unit, which the
factor-of-two margin still covers. The rounding rule is therefore a source reading of the port's
comment, not a verification against the library. The zero-wall mode calls no Clipper and is
compared with allowance 0, so a wrong allowance cannot hide a wall-band defect there.

### 9.6 Workspace gates and guest freshness

| Gate | Result |
| --- | --- |
| `cargo build --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| per-owner `cargo check` / `cargo clippy --all-targets --no-default-features --features property_testing` (5 crates) | PASS |
| per-owner `cargo check --all-targets` (feature off, 5 crates) | PASS |
| `cargo xtask check-literals` | PASS — `0 violation(s) in 0 file(s)` |
| `cargo xtask check-test-quality --report` | PASS — `0 finding(s) in 0 file(s)` |
| `cargo xtask build-guests --check` before rebuild | exit **1**, one stale guest (`traditional-support-planner-guest`) |
| `cargo xtask build-guests` | exit 0, `built 1 guest(s)` |
| `cargo xtask build-guests --check` after rebuild | exit **0** (`EXIT_FRESH`) |

Both xtask gates are **blind to these files**, verified directly in the scanners:
`collect_enforced_files` in `xtask/src/check_literals.rs` and `xtask/src/check_test_quality.rs`
walk `crates/*/tests`, `crates/*/benches`, `crates/*/src` and `modules/core-modules/*/tests` — never
`modules/core-modules/*/src` — and `LiteralVisitor::visit_item_mod` only descends into an inline
module body, so an out-of-line `mod property_tests;` is never followed. The manual FRU/waiver
review for all five new module-source files is recorded in each file's own header and summarised
here: `Point3WithWidth` literals use FRU over a real emitted
point in the three fill/ironing modules (`traditional-support`, `tree-support` and
`support-surface-ironing` each have one in a `stroke_like` helper); `GeometryBounds` and
`GeometryDomain` literals carry `// exhaustive:` notes derived from the live profile;
`traditional-support-planner` builds a FRU `SupportAnalysisView` literal and one waived
`SupportAnalysisGeometryEntry`, both recorded in that file's own header; `tree-support-planner`
builds no watched-type literal at all, because its segment lists are empty and distinct-point
segments are not assumed to create roles. No other watched-type literal exists.

Feature-off suites for the other three owners, for completeness:
`tree-support-planner` 146 passed / 11 binaries, `traditional-support` 30 passed / 7 binaries,
`support-surface-ironing` 16 passed / 5 binaries, all with 0 failures.

A bare (context-less) run of any registered property fails with `missing_property_context`, which
is the harness's documented behaviour: the feature-on `--lib` runs report
`64 passed; 2 failed` for `tree-support-planner` and `2 passed; 4 failed` for
`traditional-support-planner`, and every failure is that single error kind.

### 9.7 CI execution

PR #9 (`https://github.com/AdelinoGP/pinch_n_print_cli/pull/9`) runs the required Windows
property lanes against this branch for the first time; the branch had no pull request before, so
every earlier packet on it had only local runs.

| Job | Result |
| --- | --- |
| **Properties (Windows)** — `cargo xtask test --summary --properties --tier routine` | **success** |
| Scripted SCAD properties (Windows) | success |
| Format (`cargo xtask check-fmt`) | success |
| Clippy | success |
| Docs staleness guard | success |
| Dist editions | success |
| SCAD adapter / ingestion (Windows) | success |
| Test | **success**, after the two branch regressions fixed below |

The `Properties (Windows)` success was first observed at head `3e7f533c`; the `Test` failure at the
same head is what the two repairs below close. Once both landed, **all nine checks on PR #9
passed**. The packet's commit is a squash whose tree is byte-identical to `6626ca95`, the head
that ran them (`git diff 6626ca95 HEAD` is empty), so the recorded results apply unchanged.

The `Properties (Windows)` job is the assertion that matters for AC-9: its step
"Assert the 32 required property records executed" now includes all ten of this packet's IDs
(`.github/workflows/ci.yml`), and it passed.

**The `Test` job's two failures were this branch's, and this packet fixes them.** Both were
regressions introduced on the branch after master's last green run (`e191f31b`, run
`44454168187`), by the packets PR #9 also carries; both reproduced deterministically and are now
green:

- `lightning_migration_tdd::legacy_contract_is_retired` is a documentation guard requiring
  `docs/02_ir_schemas.md`'s "Lightning legacy IR retirement" section to state the exact sentence
  "The legacy path is retired." Packet 11 (`a23b5f9d`, found with
  `git log -S'The legacy path is retired.' -- docs/02_ir_schemas.md`) reworded it to "The legacy
  **host** path is retired." and broke the guard. **Fix:** the pinned sentence is restored
  verbatim; the host-specific detail it displaced is already carried by the following paragraph,
  which names the deleted `commit_lightning_tree_ir_builtin` producer and slot. The guard itself
  was not touched.
- `preparation_runtime_lifecycle_tdd::shared_selection_all_target_kinds` asserted the composed
  paint chain as `[("fuzzy_skin", Flag(true)), ("material", ToolIndex(1))]` while production
  returns `[("material", …), ("fuzzy_skin", …)]`. **Production is correct.** ADR-0018's
  "Shared priority/name chain identity" amendment (packet 11) fixes the canonical axis sequence as
  ascending `(priority, semantic name)`; this test's own manifest declares `material` priority
  **100** and `fuzzy_skin` priority **200**, and
  `crates/slicer-core/src/algos/paint_segmentation/mod.rs` states the same example explicitly
  ("`material` (100) comes before `fuzzy_skin` (200) even though "fuzzy_skin" sorts first by
  name"). **Fix:** the stale alphabetical `expected_chain` in the test is reordered to the
  canonical sequence, with the priority reasoning recorded in a comment. Production was not
  changed, and no assertion was weakened — the same exact-vector equality is still asserted.

| Command | Result |
| --- | --- |
| `cargo test -p slicer-runtime --test e2e -- preparation_runtime_lifecycle_tdd::shared_selection_all_target_kinds --exact` | PASS — 1 passed (80.26 s) |
| `cargo test -p slicer-runtime --test e2e -- lightning_migration_tdd::legacy_contract_is_retired --exact` | PASS — 1 passed (241.11 s) |

`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` is also corrected here rather than carried forward: its
"removal of the centroid fallback" clause is retired and quoted verbatim in an amendment, because
the fallback is live contract in both of its branches (§7 and §8) and this packet restored its
bbox-centre translation and zero-length rejection. `cargo xtask check-deviations` regenerated
doc 07's Open Deviation Map from the amended row, and the row's own text now records that no
support-fill output changed except where the pre-fix band was outside the region or zero-length.

## 10. Coverage ledger

| Property ID | Owner package | Status |
| --- | --- | --- |
| `M-SUPPORT-IRONING` | `support-surface-ironing` | implemented, registered, executed at both tiers |
| `M-SUPPORT-RASTER` | `traditional-support-planner` | implemented, registered, executed at both tiers |
| `M-SUPPORT-HOLES` | `traditional-support-planner` | implemented, registered, executed at both tiers |
| `M-SUPPORT-GRID` | `traditional-support-planner` | implemented, registered, executed at both tiers |
| `M-SUPPORT-TERRITORY` | `traditional-support-planner` | implemented, registered, executed at both tiers |
| `M-TREE-SMOOTH` | `tree-support-planner` | implemented, registered, executed at both tiers |
| `M-TREE-ROLES` | `tree-support-planner` | implemented, registered, executed at both tiers |
| `M-TRADITIONAL-SCAN` | `traditional-support` | implemented, registered, executed at both tiers; one confirmed fallback defect fixed and regressed, plus one degenerate-input hardening change |
| `M-TREE-SCAN` | `tree-support` | implemented, registered, executed at both tiers; its kernel defect is fixed and regressed |
| `M-TREE-RENDER` | `tree-support` | implemented, registered, executed at both tiers |

Consequences for the packet's acceptance criteria:

- AC-1: satisfied — raster, hole-fill and grid compare actual grids and supported extraction
  results against their independent bounded references, and `traditional_planner_controls` rejects
  centre-only rasterization, a filled hole, in-place cascading fill, a macro-block crossing, mask
  propagation from and into a blocked cell, a dropped sample and missing trim. The 5x5 control
  yields `(1,2)=1` and `(2,2)=0`.
- AC-2: satisfied — own intersection and base-minus-expanded-foreign cells are checked by exact
  full-source identity, and the controls reject the wrong chain, omitted clearance and a false
  all-empty own result.
- AC-3: satisfied — live graph references, fixed endpoints, unchanged short chains, bounded relaxed
  positions and the merge extra-wall trigger, with the invalid-reference, moved-endpoint and
  missing-flag mutants rejected.
- AC-4: satisfied — actual carve, serial `build_roles` and production `build_roles_batched` against
  independent rectangle-cell labels, including unique largest-part selection, nonempty
  body/interface populations, collision clearance and batched own-territory clipping; the
  collision-material, retained-overlap, omitted-territory, swapped-item and all-empty mutants are
  rejected, and no universal interface-disjointness claim is made.
- AC-5: satisfied — all three fill owners compare every required row/interval and every whole
  segment including holes and metadata, with the hole-crossing segment, the notch chord, a dropped
  row and a false all-empty output rejected by the same checker. The vertical 10x20 mm tree witness
  produces its four strokes at the specified x positions over the full y span.
- AC-6: satisfied — two distinct square wall bands, fill clearance and holed geometry are checked on
  the stated feasible domain, and the coincident-wall, missing-hole-wall and un-inset-fill mutants
  are rejected. The sub-pitch fallback and the collapse/no-work modes are observed, not assumed.
- AC-7: satisfied — the blocked AGG/default/legacy `NoRoute`/1203 outcomes, the accepted
  local-blocker AGG difference, the tree blocked/disabled cases and the declined-renderer
  no-fallback behaviour all execute and stay distinct.
- AC-8: satisfied — each owner's `*_shrink_bounds` control drives real simplify/complicate
  transitions, validates every exposed input and observes a permitted reduction, and every property
  has its own deterministic positive witness with counters checked after `check`.
- AC-9: **satisfied.** The local half: all ten exact IDs run under both measured tiers with
  matching controller events, 64/256 accepted cases and zero rejected calls, and the required
  Windows PR job's exact command completed with 67 property binaries green for both tiers (§9.2).
  The CI half: the `Properties (Windows)` job **passed** on PR #9 at head `3e7f533c`
  (`actions/runs/44976563310`), and that job's executed-records assertion now lists all ten of this
  packet's IDs, so a selector that stopped covering them would have failed it. See §9.7.
- AC-N1: satisfied — the checker controls reject the named wrong outputs through the same checker the
  properties use.
- AC-N2: satisfied — two confirmed SUT defects (§7 and §7.1) each have a focused fix and a named,
  RNG- and context-independent regression that fails on the old code. The third change (§7.2) is
  recorded as degenerate-input hardening, not as a confirmed defect, so it does not draw on this
  criterion's allowance.

All ten charters are also recorded in `docs/testing/properties.md` (sections `M-SUPPORT-RASTER`
through `M-SUPPORT-IRONING`), one per registered record and each naming that record's exact
identity; `packet.spec.md`'s "Property charters" section remains the authoritative design source.
