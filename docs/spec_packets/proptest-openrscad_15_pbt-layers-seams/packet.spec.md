---
packet: proptest-openrscad_15_pbt-layers-seams
status: draft
---

# Property Tests: Layer Planning, Seams, and Path Ordering

## Goal

Exercise the assigned layer, seam and path-ordering kernels with bounded, independently checked properties and reproducible Windows execution, without claiming more than the exercised paths prove.

## Why

The approved `PBT-LAYERS-SEAMS` family in `docs/specs/proptest-openrscad-adoption.md` §11 requires sequence/association, selection/geometry and ordering properties. Existing examples and source-copied tests are useful baselines, but are neither generated property coverage nor proof of upstream parity. In particular, source inspection found that the named seam comparator and spline are currently dormant: their tests cannot be reported as coverage of live region planning.

## Scope

- In: owner-local tests of `generate_object_layers` and `merge_layer_sequences`; `pick_seam_point` and `fit_cubic_bspline`; `select_seam_candidate`, `project_onto_wall_segment` and `rotate_wall_loop`; `nearest_neighbor_permutation` and `coalesce_locked_candidates`. Exercise their relevant local callers where necessary to prove paint selection, insertion, role grouping and locks. Add registry records, deterministic witnesses and negative controls, measured Windows profiles, documentation/evidence, and focused fixes **only** for confirmed SUT defects.
- Out: a new shared SDK API, SCAD, broad new guest/full-pipeline properties, redesigning the layer planner or seam algorithm, a general optimal-route solver, universal spline-accuracy or seam-quality claims, and arbitrary rotation-invariance claims. Do not edit another packet or the approved queue. Large or contract-changing fixes block affected acceptance and require appropriate authorization rather than being hidden as test maintenance.

## Depends on

- `docs/spec_packets/proptest-openrscad_01_pbt-foundation/` (**draft at authoring**): planned `slicer_sdk::test_support::property_testing::{PropertyRun, CaseGuard}`, registry, runner, artifacts and Windows lane. Implementation waits until it is implemented; recheck its exact exports, behavior and feature graph on disk.
- `docs/spec_packets/proptest-openrscad_02_pbt-domains/` (**draft at authoring**): planned direct domains, independent `oracles::{is_permutation, locked_groups_are_contiguous}`, and `regression::{ConcreteInput, format_regression_fixture}`. Implementation waits until it is implemented and the exact available shapes are rechecked. Do not assume a layer, seam, wall or support-specific shared generator exists.
- Queue `generated` means authoring only. Packet #11 is **not** a dependency: if a local fixture carries `variant_chain`, preserve the approved `(priority, semantic name)` axis ordering and the exact ordered vector as identity; never name-sort or normalize it for lookup. This is the approved source-plan decision and #11's planned reconciliation of `DEV-199`, not a claim that #11 has landed.

## Design notes

### Ownership, execution and feature boundaries

- All four are root workspace packages. Add default-off, empty owner selection features `property_testing = []` and workspace-managed Proptest **dev** dependencies to their `Cargo.toml` files. Extend each existing `slicer-sdk` **dev** dependency to include the planned `property_testing` feature along with `test`; do not put the SDK property feature on a normal dependency or guest manifest. Verify Cargo's actual dev-feature unification, discovery, and default/guest normal/build graphs after #1 lands. Native test-only code may use #2's actual direct domains and oracles; owner-specific fixtures/checkers remain private.
- Add `src/property_tests.rs` under each owner, registered from that owner's `src/lib.rs` with separate stacked `#[cfg(test)]` and `#[cfg(feature = "property_testing")]` attributes. No production visibility widening or copied source file in an integration test. The seam-planner root test module can access its `pub(crate)` `comparator` and `align` kernels. Confirm feature-enabled libtest discovery uses the full names in the table below.
- `PropertyRun::new(id: &'static str, full_test_name: &'static str) -> Result<Self, PropertyRunError>`, `bound(&self, name: &str) -> Result<u64, PropertyRunError>`, and `check<S, F>(&self, strategy: &S, test: F) -> Result<(), PropertyRunError>` are the **planned** #1 interfaces; `S: Strategy` and `F: Fn(S::Value, &mut CaseGuard) -> TestCaseResult`. `check` supplies a fresh `CaseGuard` per case and shrink attempt. An absent bound is an error. Do not nest guards or install a per-case panic hook. Plain Cargo invocation of a generated property without the registry's context is not acceptance: #1 deliberately rejects missing context. In-process properties use `timeout_ms = 0`; do not use parent-only counters as evidence for forked callbacks.
- `collect_enforced_files` (`xtask/src/check_literals.rs`) does not collect `modules/core-modules/*/src/**`; `LiteralVisitor` also does not follow an out-of-line `mod` into its file. The stacked attributes aid correct compilation and recognized inline scopes elsewhere, but **do not make these module source files scanner-covered**. Manually review every new test literal, including macro tokens, against `docs/21_data_defaults_and_fixtures.md`: watched public structs from `crates/*/src` with at least five named fields need FRU or an adjacent `// exhaustive: <reason>`. Review all new source tests against `docs/22_test_quality.md` as well. Record the manual review; a clean `check-literals`/`check-test-quality` run is not evidence it scanned these files.
- Use `prop::sample::select(modes).no_shrink()` or an equivalent genuinely nonshrinking Proptest 1.11 selector for every persistent semantic mode; never use `prop_oneof![Just(...), ...]` for that purpose. Parameters still shrink. Deterministic `ValueTree` probes must perform and observe a meaningful `simplify` and `complicate`, checking selected mode, domain validity and required witness at every current value. Do not assume a shrink completed merely because a limit was reached.
- Every property owns a nonempty positive observation counter, and a separate counter for each required positive subcase. Arrange the generated cases to exercise required positive subcases by construction rather than statistically hoping a random mode appears; deterministic companion witnesses are additional. Assert counters from generated SUT observations after `PropertyRun::check` succeeds; empty cases have separate exact assertions. Counters are local to the property, not shared with unrelated properties or simulated by fixtures.
- The exact registry records are the following **new planned** tests, not live exports. All use `target_kind = "lib"`, `generator_version = 1`, `requires_host_algos = false` for these owner kernels, `requires_scad = false`, both measured tiers, and the indicated single owner-qualified feature. Native SDK dependency feature unification may still enable host algorithms; verify the graph rather than claiming otherwise. `test_name` is the full libtest name; its final component starts `proptest_`. The listed bounds are the exact `PropertyRun::bound` keys to record, with positive and representable limits chosen from the Windows pilots.

| ID | package / Cargo lib target / feature | full test filter | SUT and bounds keys |
|---|---|---|---|
| `M-LAYERS-SEQ` | `layer-planner-default` / `layer_planner_default` / `layer-planner-default/property_testing` | `property_tests::proptest_layer_sequence` | `generate_object_layers`; `max_layer_steps`, `max_raft_layers`, `max_z_quanta` |
| `M-LAYERS-MERGE` | same layer package / target / feature | `property_tests::proptest_layer_merge` | `merge_layer_sequences`; `max_objects`, `max_layer_steps`, `max_raft_layers`, `max_z_quanta` |
| `M-SEAM-PLANNER-PICK` | `seam-planner-default` / `seam_planner_default` / `seam-planner-default/property_testing` | `property_tests::proptest_planner_pick` | `comparator::pick_seam_point`; `max_candidates`, `max_coord_quanta`, `max_score_quanta` |
| `M-SEAM-PLANNER-PAINT` | same seam-planner package / target / feature | `property_tests::proptest_planner_paint` | `region_candidates`, `choose_region_candidate`; `max_candidates`, `max_coord_quanta` |
| `M-SEAM-PLANNER-SPLINE` | same seam-planner package / target / feature | `property_tests::proptest_planner_spline` | `align::fit_cubic_bspline`; `max_observations`, `max_segments`, `max_coord_quanta`, `max_weight_quanta` |
| `M-SEAM-PLACER-SELECT` | `seam-placer` / `seam_placer` / `seam-placer/property_testing` | `property_tests::proptest_placer_select` | `select_seam_candidate`; `max_candidates`, `max_coord_quanta`, `max_score_quanta` |
| `M-SEAM-PLACER-PROJECT` | same seam-placer package / target / feature | `property_tests::proptest_placer_project` | `project_onto_wall_segment`, `insert_projected_point`; `max_wall_points`, `max_walls`, `max_coord_quanta` |
| `M-SEAM-PLACER-ROTATE` | same seam-placer package / target / feature | `property_tests::proptest_placer_rotate` | `rotate_wall_loop`; `max_wall_points`, `max_coord_quanta` |
| `M-PATH-NEAREST` | `path-optimization-default` / `path_optimization_default` / `path-optimization-default/property_testing` | `property_tests::proptest_path_nearest` | `nearest_neighbor_permutation`; `max_candidates`, `max_coord_quanta` |
| `M-PATH-LOCKS` | same path package / target / feature | `property_tests::proptest_path_locks` | `coalesce_locked_candidates`, `PathOptimizationDefault::group_then_nearest_neighbor`; `max_entities`, `max_coord_quanta` |
| `M-PATH-GROUPS` | same path package / target / feature | `property_tests::proptest_path_groups` | `PathOptimizationDefault::group_then_nearest_neighbor`; `max_entities`, `max_coord_quanta` |

Set the registry's single `production_symbol` and `production_path` for each record as follows; other named symbols in the table are exercised by that property, not additional registry fields:

| ID | `production_symbol` | `production_path` |
|---|---|---|
| `M-LAYERS-SEQ` | `layer_planner_default::generate_object_layers` | `modules/core-modules/layer-planner-default/src/lib.rs` |
| `M-LAYERS-MERGE` | `layer_planner_default::merge_layer_sequences` | `modules/core-modules/layer-planner-default/src/lib.rs` |
| `M-SEAM-PLANNER-PICK` | `seam_planner_default::comparator::pick_seam_point` | `modules/core-modules/seam-planner-default/src/comparator.rs` |
| `M-SEAM-PLANNER-PAINT` | `seam_planner_default::region_candidates` | `modules/core-modules/seam-planner-default/src/lib.rs` |
| `M-SEAM-PLANNER-SPLINE` | `seam_planner_default::align::fit_cubic_bspline` | `modules/core-modules/seam-planner-default/src/align.rs` |
| `M-SEAM-PLACER-SELECT` | `seam_placer::select_seam_candidate` | `modules/core-modules/seam-placer/src/lib.rs` |
| `M-SEAM-PLACER-PROJECT` | `seam_placer::project_onto_wall_segment` | `modules/core-modules/seam-placer/src/lib.rs` |
| `M-SEAM-PLACER-ROTATE` | `seam_placer::rotate_wall_loop` | `modules/core-modules/seam-placer/src/lib.rs` |
| `M-PATH-NEAREST` | `path_optimization_default::nearest_neighbor_permutation` | `modules/core-modules/path-optimization-default/src/lib.rs` |
| `M-PATH-LOCKS` | `path_optimization_default::coalesce_locked_candidates` | `modules/core-modules/path-optimization-default/src/lib.rs` |
| `M-PATH-GROUPS` | `path_optimization_default::PathOptimizationDefault::group_then_nearest_neighbor` | `modules/core-modules/path-optimization-default/src/lib.rs` |

The symbol strings identify private kernels; they are metadata, not new public Rust exports. Set each record's `charter` to its own heading in `docs/testing/properties.md`. The foundation registry, not an extra hand-maintained list, owns discovery/execution, context and profile identity. The registry command cannot accept separate Cargo package/target/filter arguments; the tables supply those exact child selections and #1 requires their validation against Cargo metadata and `--exact` discovery.

For the ordinary generated domain, numeric quanta represent `1/1024 mm` for float geometry/Z and `1/1024` for scalar scores/weights; planner paint's integer XY coordinates instead use native `Point2` units. These are bounded construction domains, not runtime budgets. Validate every bound before conversion/allocation: `max_objects <= 4`; `max_layer_steps`, `max_candidates`, `max_wall_points`, `max_walls`, `max_entities`, `max_observations`, and `max_segments <= 32`; `max_raft_layers <= 8`; each numeric `max_*_quanta <= 65536`. Profiles must also be large enough for their required positive witnesses, and `max_segments <= max_observations`. Check combined raft/top, squared distance, weight and conversion intermediates against these bounds; reject inconsistent profiles rather than silently narrowing them. Explicit rounding-boundary probes are separate from the ordinary dyadic generator. Preserve required nonempty blocks, interior segments and paint/fit cases when their numeric parameters shrink.

### Layer sequence and associations

`generate_object_layers`, `merge_layer_sequences`, `merge_raft_sequences`, `raft_top` and `explicit_schedule_step` are private in `modules/core-modules/layer-planner-default/src/lib.rs`; `DefaultLayerPlanner::run_layer_planning` invokes them. `layer_planning_objects` (`crates/slicer-runtime/src/run.rs`) forwards `layer_top_zs` from `query_z_grid` (`crates/slicer-config/src/resolution.rs`); an explicit schedule selects the general merge. Exercise explicit schedules and the uniform fallback separately; do not claim the fast path covers the production explicit-schedule path.

Build a small **independent event/interval model**, not a call to the SUT, for unique object IDs, finite positive bounded heights, first heights and steps, bounded raft counts and strictly increasing finite explicit tops. Prefer bounded dyadic millimeter quanta with representable intermediate sums and gaps greater than the `1e-6` matching threshold; derive the `f32` conversion envelope, and separately probe selected boundary cases rather than pretending float equality is universal. Bound maximum steps before creating a sequence. Model, compare and label these currently source-derived rules:

1. Uniform local tops start at `first_layer_height`, advance by `layer_height`, and include an endpoint only while the top is at most `height + 1e-6`; explicit nonempty `layer_zs` are forwarded as authored. A first height above the end can give no native tops. Do not replace this with `ceil(height/layer_height)`.
2. A raft-bearing object's raft tops are `first + j * layer_height` for `0 <= j < raft_layers`; the greatest final raft top is the common `raft_top` added to **all** objects' model tops, including raft-free objects. Raft events precede model events; check object association, first/subsequent credits and `is_raft` through `run_layer_planning` or the actual raft helper, not by inventing a flag on `MergedLayer`.
3. In the general merge, compare the union of native model tops and each object's actual `RegionLayerProposal` (`object_id`, `region_id == "0"`, `effective_layer_height`, `is_catchup`, `catchup_z_bottom`). Each still-active object without a native event gets a catch-up from its own previous participating global Z, initialized to zero, if the gap exceeds `1e-6`; native and catch-up are distinct. The first **explicit native** event retains `first_layer_height` credit even after a preceding catch-up; subsequent explicit credits are differences of adjacent authored tops. Do not invent LCM-created planes or assert `is_sync_layer`.
4. The uniform fast merge compares the `f64` base to the end; the general merge compares the `f32` global Z converted back to `f64` minus `raft_top`. Preserve this distinction as a **characterization**, including a deliberately constructed rounding-boundary probe, until parity and the applicable contract are checked. An emitted plane must have a nonempty region population; regions and planes must be ordered and valid within the declared domain, and an ended object must not acquire later events.

Deterministic nonempty witnesses include: an explicit A sequence `[0.25, 0.5, 0.75, 1.0]`, height `1.0` and first credit `0.25`, and B sequence `[0.375, 0.75]`, height `0.75` and first credit `0.375`, so B's `0.25` event is catch-up from zero and its `0.375` native event retains first credit; and a raft object with `first=0.25`, `step=0.125`, two raft tops `0.25, 0.375` and first model top `0.625`, paired with a raft-free object. Independently construct the expected event tables and inspect all associations. Also run empty/no-native, one-object, equal-height fast-path, mixed explicit/uniform, exact-end and boundary-neighbor witnesses. No unbounded or non-finite scalar is sent into the potentially nonterminating raw loop.

The delegated source read found canonical `generate_object_layers` (`Slicing.cpp`) uses a center probe, whereas this module's fallback is top-based; those different inputs/boundaries are **not** proof of parity. Before treating an end rule as canonical correctness, trace the host's explicit schedule and corresponding upstream contract. A confirmed divergence requires the canonical-correct fix and an independently justified concrete regression, or an explicitly authorized disposition; do not protect an incorrect rule merely because this event model describes current source. Source-inspected non-finite-loop and fast/general rounding risks are hypotheses, not reproduced failures; isolate any hang probe in a killable process.

### Seam planner: distinct live and dormant paths

`comparator::pick_seam_point` (`modules/core-modules/seam-planner-default/src/comparator.rs`) scans a **nonempty, in-bounds** range and returns an index in that range; strict ties retain the first index. An empty range returns its unchecked start, not an `Option`; never index or advertise that result as a valid candidate. Test a nonzero-start singleton and a nonempty mixed-paint range with independently obvious best choice. In valid paint data, `Enforced > Neutral > Blocked`, with the aligned central-enforcer preference checked separately; do not assume an invalid `central_enforcer`/paint combination. `pick_random_seam_point` in that file is a dormant position-seeded sibling: for finite nondegenerate valid ranges, test reproducible membership and an interpolated point on the selected edge, not distribution or seam quality. It and `align::fit_cubic_bspline` are not invoked by the current `run_region_planning_entries` production path; report those properties as dormant-kernel coverage only.

For real planner paint behavior, exercise private `region_candidates` and `choose_region_candidate` (`modules/core-modules/seam-planner-default/src/lib.rs`). Exact `Custom("seam_blocker")`/`Custom("seam_enforcer")` `Flag(true)` annotations classify contour vertices; blockers are removed, central enforcers score `2`, other enforcers `1`, neutral `0`, and a mode chooses only among maximum-score candidates. Empty selection returns `None`; check a nonempty enforcer-versus-neutral witness, blocked-versus-enforced precedence, multi-vertex enforcer run, no paint, singleton and empty. Random selection uses `layer_index` as the deterministic modulo choice among the eligible candidates. It has no external RNG seed: test repeatability at fixed layer indices and use the registry seed for case generation/replay. Hole vertices are neutral under the current contour annotation convention. Do not credit this direct selection test as end-to-end paint delivery.

For `fit_cubic_bspline` (`modules/core-modules/seam-planner-default/src/align.rs`), use nonempty equal-length arrays, finite sorted observation parameters, positive finite weights, `1 <= segments <= observations.len()`, and bounded finite intermediate arithmetic. Test a positive-span, independently well-conditioned constant-data family at observed/interior parameters, plus a same-parameter weighted-mean degeneracy and a single observation; e.g. equal parameters, observations `[[0,0],[10,0],[0,10]]`, weights `[1,2,1]` have mean `[5,2.5]`. Require finite output and derive a dimensionally valid error envelope from the restricted basis/conditioning and floating-point operations before choosing bounds. Do not demand arbitrary interpolation, affine exactness for all samples, extrapolation, or a universal fit error. Empty arrays and zero segments are not valid fit-domain inputs; no new rejection API is implied.

### Seam placer: selection, projection and rotation

`select_seam_candidate`, `project_onto_wall_segment`, `insert_projected_point`, `interpolate_point` and `rotate_wall_loop` are private in `modules/core-modules/seam-placer/src/lib.rs`; their production caller is the `LayerModule::run_wall_postprocess` implementation for `SeamPlacer`. For finite nonempty candidates, independently check actual membership and local choice: `Nearest` and the defensive aligned arms minimize `score + reason_bonus`, breaking ties by lowest Y then lowest X; bonuses are `UserForced = -1.0`, `Concave = -0.5`, `Sharp = -0.2`, `Aligned = 0.0`. `Rear` maximizes Y, then prefers lower effective score and higher X; `Random` chooses `layer_index % candidates.len()`. Test singleton, exact ties and all modes. Empty selection must safely produce no candidate and preserve the documented fallback, including `Random`. Source inspection shows that Random currently computes a modulo by candidate count before `get`; reproduce the empty case before calling it a confirmed defect, then fix it if confirmed. The deterministic `layer_index` cycling is not a claim of statistical randomness.

Construct small finite closed loops with explicit closing duplicates, plus open polylines, distinct per-point widths and distinct flags. Use an independent clamped dot-product/segment-distance calculation for the nearest **actual** segment with bounded `f32` arithmetic; include interior, endpoint, zero-length, equal-distance and empty-wall cases. Account for the source's `1e-5` parameter snap to an endpoint and `t = 0` for zero-length segments; use points away from the snap boundary for the ordinary oracle, with separately derived boundary witnesses. Equal-distance ties choose the lower wall index then segment start. For an open path, last-to-first is not an actual segment; preserve open geometry rather than introducing an implicit closing chord. Current projection/insertion source wraps open paths and can append a closing point: reproduce and classify this before changing it. Some existing projection examples use open fixtures and expect closure; if a confirmed contract/canonical fix changes them, update those examples to express the corrected justified behavior without deleting or weakening coverage.

For interior insertion, compare every field against the source-supported contract: XY/Z, point width, `flow_factor` and `dist_to_top_mm` interpolate linearly; point `overhang_quartile` and `overhang_distance_mm` and the entire `WallFeatureFlags` value choose the start at `t <= 0.5`, otherwise the end; `width_profile.widths` interpolates independently. Check the half-way boundary and values on both sides. The source does **not** linearly interpolate boolean flags. Require a genuinely interior insertion and aligned point/flag/width counts; endpoints do not add points.

For closed rotation, compare the effective cyclic point sequence and paired flags/widths by the independently chosen start index, then one closing repeat. Open paths remain unchanged in every field by `rotate_wall_loop`; rotating a closed sequence changes the starting index, not the geometry or orientation. Distinguish the repeated endpoint from the effective vertices and count correspondence by vertex identity, not by lengths alone. `ExtrusionPath3D::is_closed` (`crates/slicer-ir/src/slice_ir.rs`) uses XY, while projection currently tests full point equality; include a same-XY/different-metadata closing witness, inspect the accepted representation, and resolve any confirmed mismatch without silently treating the trailing record as an additional independent vertex. Use exact comparisons for retained metadata and a derived bounded `f32` error for calculated coordinates/widths.

### Path ordering and ADR-0062

`nearest_neighbor_permutation`, `coalesce_locked_candidates`, and `PathOptimizationDefault::group_then_nearest_neighbor` are private in `modules/core-modules/path-optimization-default/src/lib.rs`; `run_path_optimization` calls the latter. Use finite bounded positions and the production `OrderedEntityView` identity convention. Check candidate-index and original-index bijections independently with #2's planned `is_permutation` or a local seen-set, including empty and singleton. The greedy cursor begins at the origin and advances to the chosen candidate's end; its source documents an `0.001 mm` tie window and earlier candidate index. Exact two-candidate tie and separated-distance witnesses may assert the selected first index using an independent distance calculation; do not require a globally shortest route or a full copied greedy oracle.

For direct coalescing, an untagged item is a singleton and each maximal adjacent equal `Some(order_lock)` run becomes a candidate with original indices in authored order, first start and last end. Check empty, singleton, adjacent distinct tags and a same-tag-separated-by-`None` characterization without claiming that malformed separated tags meet ADR-0062. For a valid locked block in one `(layer, object, region)`, compare output positions against input using the independent `locked_groups_are_contiguous` predicate plus explicit `false` reversal flags. The block may move as a unit, never split or reverse. Include a nearer unrelated singleton and a witness where using the first instead of the last endpoint changes the next choice.

ADR-0062 does not limit a valid lock to one role or tool. Include an authored contiguous, same-region multi-role block and a multi-tool block when the actual producer/view can represent them; if a genuine upstream constraint rules one out, document the concrete constraint rather than assuming the optimizer's partitions are input preconditions. Locks take precedence when a role/tool grouping would split one; verify the real module proposal and independently check contiguity/direction. For ordinary unlocked or compatible-block inputs, assert ascending tool clusters, skirt before the staged wall block, walls preserved in staged order, then the documented non-wall role groups. Never assert that this optimizer re-sorts `InnerWall` before `OuterWall`. The archived `docs/spec_packets/_OLD/61_path-optimization-role-sequencing.md` says new infill roles such as `InternalBridgeInfill` belong with infill; exercise an `InternalBridgeInfill`/`SupportMaterial` witness, investigate the current wildcard group, and fix only if the applicable contract is confirmed. Do not derive expected roles by calling `role_group` itself.

### Negative controls, profiles and failure lifecycle

Each owner's new `property_tests::packet15_witnesses_controls_and_shrinking` is a deterministic ordinary unit test, independent of `PropertyRun` context. It calls the real SUT on every required witness, asserts the same checker accepts the unmodified nonempty observation, then supplies the following individual mutated observations and asserts that same checker rejects each. A panic, compilation failure, or mutation of only test scaffolding is not a kill.

| Checker | Required mutation controls, each separately rejected |
|---|---|
| Layer sequence/merge | drop a native top; omit global raft offset for the raft-free object; replace a catch-up bottom or height; swap object association; insert an empty-region plane or move a raft plane after a model plane; replace first explicit credit after catch-up with the later step |
| Planner pick/paint/spline | return an out-of-range or lower-priority candidate; retain a blocked vertex or demote the only enforcer; replace `[5,2.5]` weighted mean by the unweighted mean or non-finite fit |
| Placer selection/projection/rotation | return a nonmember; select wrong tie winner; put projection off the actual segment; alter interpolated width or discrete flag; rotate points without their matching width/flag; lose the closing repeat or close an open path |
| Path ordering | duplicate/drop an index; split/reverse a locked block or set one reversal flag; substitute the first block end for the last; reorder staged walls; put support before the documented infill witness; violate an exact tie winner |

For selected in-process generated profiles require positive local counters for nonempty layers and associations, painted eligible selections, nondegenerate fits, selections, interior projections, closed rotations, index permutations, locked blocks and role/tool groups. Separate deterministic empty cases cannot satisfy these counters. Mark repeat-call comparisons as determinism pins with a `// test-quality:` reason, not independent correctness; identify negative-control waivers by the checker they protect when applicable.

Record source revision and dirty-source digest, toolchain and Windows environment, target/features, exact commands/status/log paths, measured compile cost separately from execution, bounds, cases, rejected/exhausted cases, shrink limits, observations/counters, witness and mutation results, finite-intermediate/tolerance derivations, and a retained-seed replay in **new** `docs/testing/proptest-layers-seams-evidence.md`. Set routine/extended registry profiles only after Windows pilots; do not invent budgets or loosen bounds/assertions to hide a defect. Required routine Windows PR execution uses #1's `properties-windows` lane; extended must pass for this packet's completion. On failure classify generator/oracle/contract/SUT first. Only a confirmed SUT defect gets a focused canonical-correct fix and named concrete, minimized, RNG/context-independent regression with root cause and failing input. Otherwise record none found. Preserve target-local artifacts and no source-adjacent seed history.

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`). Layer Z and seam path floats are millimeters; exact integer-grid comparisons do not imply exact float round trips.

<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).

<!-- snippet: orca-delegation -->
## OrcaSlicer references

The author delegated direct source reads. `generate_object_layers` (`Slicing.cpp`, available local OrcaSlicer checkout) uses a center probe; its relation to the host-supplied PnP schedule needs the comparison specified above. `fit_curve` (`OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp`) provides a source-backed positive segment/weight domain for the restricted fit tests. These observations do not establish parity of the current PnP implementations. Delegate further direct reads of named upstream functions before any additional canonical behavioral claim or parity fix; cite file and function, never line numbers. If source is unavailable, report the gap rather than infer behavior from a fixture name. Any actual C++ port into a new file requires the attribution header in `docs/ORCASLICER_ATTRIBUTION.md`.

## Acceptance criteria

- **AC-1** `M-LAYERS-SEQ` and `M-LAYERS-MERGE` under `P` call the real kernels and compare independently derived native tops, raft prefix/offset, per-object native/catch-up associations, first credits and end behavior on the bounded domain; both explicit and uniform modes, the dyadic catch-up and raft-free-object witnesses, and each required positive counter execute. Fast/general boundary differences are characterized and canonical correctness resolved before pinning an end rule.
- **AC-2** `M-SEAM-PLANNER-PICK`, `M-SEAM-PLANNER-PAINT` and `M-SEAM-PLANNER-SPLINE` under `P` check nonempty range membership, ties, live paint/empty selection and position/layer-seeded reproducibility, plus finite supported constant and weighted-mean fits. The report distinguishes dormant comparator/spline from live planner selection.
- **AC-3** `M-SEAM-PLACER-SELECT`, `M-SEAM-PLACER-PROJECT` and `M-SEAM-PLACER-ROTATE` under `P` establish empty/member selection, actual-segment projection, interior metadata interpolation, closed cyclic point/flag/width correspondence and open-path preservation, with explicit duplicated-endpoint witnesses.
- **AC-4** `M-PATH-NEAREST`, `M-PATH-LOCKS` and `M-PATH-GROUPS` under `P` prove index bijections, source-backed local tie behavior, first-start/last-end coalescing, ADR-0062 contiguity/forward direction including valid cross-group blocks, and applicable role/tool grouping with staged wall order. They do not assert route optimality.
- **AC-N1** All four `C` commands run their owner-local deterministic witness/control/shrink test. Every unmodified positive SUT observation is accepted; every separately named mutation above is rejected by its same checker; meaningful `simplify` and `complicate` transitions retain the selected mode and witness. Empty input cases are asserted separately and positive counters cannot be satisfied by them.
- **AC-N2** `select_seam_candidate`'s empty Random case and an open-polyline implicit-closing-edge witness are executed and classified; the valid documented no-candidate/open-geometry behavior passes after any confirmed focused fix. Existing affected examples pass with their substantive coverage retained. A suspected defect is not presented as confirmed from source inspection alone.
- **AC-5** `P` passes in both measured Windows tiers with exact discovered tests, matching controller begin/end events and accepted case counts, and the existing Windows routine lane includes all eleven records. Retained-seed replay reproduces concrete pre-shrink inputs; failed/zero/exhausted execution is never a pass. The evidence file contains the specified measurements, counters, manual lint review, and confirmed regressions or explicit none-found.
- **AC-6** The four owner `property_testing` features and planned SDK dev feature compile all selected lib tests; default/guest normal/build graphs remain free of property dependencies. Relevant existing examples under `B`, guest freshness by exit code, workspace build/check/clippy and the literal/test-quality gates pass. No production API, WIT or IR schema is widened merely for tests.

## Verification

These are **implementation acceptance commands**, not authoring-session pass claims. `P` selects exactly the eleven registry rows above; #1's runner supplies the recorded package/target/feature/full filter to each child and owns combined logging and child status. Run the command once with each listed tier, reading/archiving `target/test-output.log` and `target/property-testing/` before the next run:

```bash
cargo xtask test --summary --properties --tier routine --property M-LAYERS-SEQ --property M-LAYERS-MERGE --property M-SEAM-PLANNER-PICK --property M-SEAM-PLANNER-PAINT --property M-SEAM-PLANNER-SPLINE --property M-SEAM-PLACER-SELECT --property M-SEAM-PLACER-PROJECT --property M-SEAM-PLACER-ROTATE --property M-PATH-NEAREST --property M-PATH-LOCKS --property M-PATH-GROUPS
cargo xtask test --summary --properties --tier extended --property M-LAYERS-SEQ --property M-LAYERS-MERGE --property M-SEAM-PLANNER-PICK --property M-SEAM-PLANNER-PAINT --property M-SEAM-PLANNER-SPLINE --property M-SEAM-PLACER-SELECT --property M-SEAM-PLACER-PROJECT --property M-SEAM-PLACER-ROTATE --property M-PATH-NEAREST --property M-PATH-LOCKS --property M-PATH-GROUPS
```

For the replay, run the following twice and retain both artifacts and logs, comparing the concrete input stream rather than merely the printed seed:

```bash
cargo xtask test --summary --properties --property M-SEAM-PLACER-SELECT --tier routine --seed 42
```

`C` is each exact command below. `B0` and `B` are the following exact existing baselines. For every plain Cargo test command set up Bash with `set -euo pipefail` and `mkdir -p target`; the commands include `2>&1 | tee target/test-output.log`. Archive/read that log after **each** invocation before running the next. In PowerShell retain the native `$LASTEXITCODE` across `Tee-Object`. No standalone command may be counted as green if its exact test was not discovered/executed.

```bash
cargo test -p layer-planner-default --features property_testing --lib -- property_tests::packet15_witnesses_controls_and_shrinking --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-planner-default --features property_testing --lib -- property_tests::packet15_witnesses_controls_and_shrinking --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-placer --features property_testing --lib -- property_tests::packet15_witnesses_controls_and_shrinking --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p path-optimization-default --features property_testing --lib -- property_tests::packet15_witnesses_controls_and_shrinking --exact --nocapture 2>&1 | tee target/test-output.log
```

`B0`, before adding the new owner features or changing the SUT:

```bash
cargo test -p layer-planner-default --no-default-features --test layer_planning_tdd -- test_first_layer_height_respected --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p layer-planner-default --no-default-features --test layer_planning_tdd -- explicit_first_layer_credit_survives_a_preceding_catch_up --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-planner-default --no-default-features --test seam_canonical_spline_tdd -- canonical_spline_tests::single_observation_fit_returns_that_observation --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-placer --no-default-features --test seam_continuous_projection_tdd -- projects_onto_nearest_segment_point --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-placer --no-default-features --test seam_degraded_fallback_tdd -- missing_plan_emits_non_fatal_and_preserves_walls --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p path-optimization-default --no-default-features --lib -- tests::locked_block_never_split_or_reversed --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p path-optimization-default --no-default-features --lib -- tests::committed_wall_sequence_is_not_reordered_by_role_priority --exact --nocapture 2>&1 | tee target/test-output.log
```

`B`, after adding the features and after any affected production fixes:

```bash
cargo test -p layer-planner-default --features property_testing --test layer_planning_tdd -- test_first_layer_height_respected --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p layer-planner-default --features property_testing --test layer_planning_tdd -- explicit_first_layer_credit_survives_a_preceding_catch_up --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-planner-default --features property_testing --test seam_canonical_spline_tdd -- canonical_spline_tests::single_observation_fit_returns_that_observation --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-placer --features property_testing --test seam_continuous_projection_tdd -- projects_onto_nearest_segment_point --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p seam-placer --features property_testing --test seam_degraded_fallback_tdd -- missing_plan_emits_non_fatal_and_preserves_walls --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p path-optimization-default --features property_testing --lib -- tests::locked_block_never_split_or_reversed --exact --nocapture 2>&1 | tee target/test-output.log
cargo test -p path-optimization-default --features property_testing --lib -- tests::committed_wall_sequence_is_not_reordered_by_role_priority --exact --nocapture 2>&1 | tee target/test-output.log
```

Run the exact owner feature checks and clippy gates:

```bash
cargo check -p layer-planner-default --all-targets --features property_testing
cargo check -p seam-planner-default --all-targets --features property_testing
cargo check -p seam-placer --all-targets --features property_testing
cargo check -p path-optimization-default --all-targets --features property_testing
cargo clippy -p layer-planner-default --all-targets --features property_testing -- -D warnings
cargo clippy -p seam-planner-default --all-targets --features property_testing -- -D warnings
cargo clippy -p seam-placer --all-targets --features property_testing -- -D warnings
cargo clippy -p path-optimization-default --all-targets --features property_testing -- -D warnings
```

Run the repository's non-test gates `cargo build --workspace`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, and `cargo xtask build-guests --check` (rebuild on exit 1, stop on 3). No workspace test criterion is required.

## Docs impact

- Planned #1 `docs/testing/properties.md`: add a complete charter and measured tier/profile for each of the eleven IDs in the table, including owner/path, domain, oracle independence, tolerance, shrinking, nonempty witness, registry features and replay. Check `rg -n 'M-LAYERS-SEQ|M-SEAM-PLANNER-SPLINE|M-SEAM-PLACER-PROJECT|M-PATH-LOCKS' docs/testing/properties.md` and inspect all eleven entries.
- New `docs/testing/proptest-layers-seams-evidence.md`: retain baseline/after statuses, source and Windows profile provenance, exact commands, controls, counts, shrink evidence, manual lint review, dependency-graph evidence and confirmed defect dispositions. Check `rg -n 'Windows|M-LAYERS|M-SEAM|M-PATH|manual' docs/testing/proptest-layers-seams-evidence.md`.
- Conditional on a confirmed fix changing the corresponding normative contract: update `docs/01_system_architecture.md` §§ Catch-Up Layer Semantics and Seam-First Aligned Default, `docs/02_ir_schemas.md` §§ IR 3, IR 7 and IR 9c, and/or `docs/05_module_sdk.md` §§ Seam-candidate generation convention and Continuous wall projection and degraded fallback. Inspect each section actually changed, using respectively `rg -n 'Catch-Up Layer Semantics|Seam-First Aligned' docs/01_system_architecture.md`, `rg -n 'IR 3 — LayerPlanIR|IR 7 — PerimeterIR|IR 9c — SeamPlanIR' docs/02_ir_schemas.md`, and `rg -n 'Seam-candidate generation convention|Continuous wall projection' docs/05_module_sdk.md`. Do not change WIT/IR/ADR or deviation records without the applicable authorization.

## Open questions

- [FWD] Recheck the implemented #1/#2 exports, registry selectors and feature unification on disk before beginning; if they differ, reconcile within the approved design or stop rather than inventing a replacement API.
- [FWD] Resolve the source-observed top-based versus upstream center-probe/end-boundary question against the actual host schedule before declaring parity. Reproduce and classify the Random-empty, open-path, cross-group-lock and newer-infill-role witnesses; a genuine architectural conflict blocks the affected criterion rather than silently narrowing its domain.
- [FWD] Derive the fit/projection numerical envelopes and measure Windows routine/extended bounds, case/rejection/shrink settings and compile/execution effects. Hand exact registry IDs and evidence to the rollout audit; no counts or runtimes are claimed at authoring.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | The owner kernels, actual callers, SDK/IR fields, live-versus-dormant seam paths and candidate defects were read at the paths above; proposed test names/features are explicitly new. |
| 2 | WIT/IR identifiers | PASS | `RegionLayerProposal`, `OrderedEntityView.order_lock`, `Point3WithWidth` and ordered `variant_chain` are existing Rust fields; no WIT/IR change is requested. |
| 3 | Schema versions | PASS | No WIT/IR version bump or future-version literal. |
| 4 | ADRs | PASS | ADR-0062 supplies atomic ordered, non-reversed lock semantics; ADR-0046's amendments and `docs/22_test_quality.md` govern seam/test scope. No new ADR slot is reserved. |
| 5 | Deviation IDs | PASS | `DEV-199` is identified as an existing planned reconciliation only; no ID is allocated, closed or modified here. |
| 6 | Prerequisites | PASS | #1 and #2 are accurately marked draft, implementation waits, and planned exports must be rechecked; #11 is not added as a dependency. |
| 7 | Test targets | PASS | Offline Cargo metadata identified four lib targets and baseline test targets; exact new test registration, separate cfg gates, full filters and manual scanner blind-spot review are specified. |
| 8 | Criteria | PASS | All eleven exact registry IDs and owner/target/feature/filters, deterministic controls, empty cases, negative paths and measured tier evidence are named; no workspace test criterion. |
| 9 | Fallout | PASS | No existing production struct field, WIT or schema version is added; owner features/dev dependencies and test literals are explicitly reviewed. |
| 10 | Docs impact | PASS | Named property charter and evidence documents have checks; normative docs change only with a confirmed applicable contract fix. |
| 11 | Completeness | PASS | No unresolved scope decision or template placeholder; implementation-time parity investigation, prerequisite recheck and measurements are explicit forward work. |

**Verdict: AUTHOR SELF-PREFLIGHT PASS.** This is source/metadata authoring evidence, not an implementation or test pass.
