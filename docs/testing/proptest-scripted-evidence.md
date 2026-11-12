# Scripted geometry evidence: windows, profiles and the combined geometry disposition

This file records the **scripted** half of the proptest rollout: typed SCAD recipe
generation, independently witnessed conversion, shrink behaviour, measured
Windows profiles, whole-process supervision, and the combined direct + scripted
`G-GEOMETRY` decision.

It closes full `G-SCAD` **only** by combining the adapter-prerequisite evidence in
[`docs/scad/windows-feasibility.md`](../scad/windows-feasibility.md) with the
generation/render/shrink measurements and eligible profiles below. Neither
document alone is a full pass.

The engine is a **stimulus source, not an oracle**. Nothing here claims that a
successful render proves the slicer consumes geometry correctly, and nothing here
generalises the bounded scripted domains into unrestricted polygon generality.

## 1. Provenance

| Item | Value |
|---|---|
| `source_revision` | the final-run `source-state.json` under `target/property-testing/296007284f258f8e2622e20e4ea24ad0/` records the revision; the tree was dirty with the paths below |
| Working tree during measurement | dirty: `.github/workflows/ci.yml`, `test-properties.toml`, `xtask/src/properties.rs`, `xtask/src/properties/executor.rs`, `crates/slicer-sdk/Cargo.toml`, `crates/slicer-sdk/src/test_support/property_testing/{mod,run}.rs`, `crates/slicer-sdk/src/test_support/property_testing/scripted/{mod,emit,reference,strategy,artifacts}.rs`, `crates/slicer-sdk/tests/**`, `docs/05_module_sdk.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/scad/windows-feasibility.md`, `docs/testing/properties.md`, this file |
| OS | Windows 10.0.26300.9711 (x86_64) |
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| CMake | `cmake version 4.4.3` |
| MSVC | used by the vendored engine's CMake build; toolset selection recorded in `docs/scad/windows-feasibility.md` |
| Engine | base revision `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`, patched set per `vendor/openrscad/PATCHES.md`, backend `manifold_cpp` |
| Features under test | `slicer-sdk/property_testing_scad` (implies `property_testing`; enables `slicer-scad/openrscad`) |
| Seed | routine and extended runs used the executor's recorded seed; every attempt records it in `context.json` |
| Build profile | `dev` (`cargo xtask test`); no release measurements are claimed |

## 2. Feature boundary

`property_testing_scad` is default-off and native-only. Measured with resolved
metadata and `cargo tree`, not by scanning source:

- the default and `slicer-sdk/test` graphs contain neither `proptest` nor any
  `openrscad*` crate, and the product `pnp-cli` graph contains neither either;
- `slicer-sdk/property_testing` **does** enable `proptest` (that is its purpose)
  while staying engine-free; the isolation test asserts that as a positive
  control, and the tree selector uses the unqualified feature name because a
  qualified `slicer-sdk/x` argument is silently ignored by `cargo tree`;
- `slicer-sdk/property_testing_scad` resolves `slicer-scad`,
  `openrscad-eval`, `openrscad-geom`, `manifold-csg` and `proptest`
  (positive control);
- the behavioural half — engine identity present, a typed recipe emitted and
  evaluated to an exact canonical-unit contour — is
  `scripted_feature_isolation_and_union` in
  `crates/slicer-sdk/tests/property_scripted_contracts.rs`.

Ordinary property support stays engine-free; no guest manifest, production
prelude or self dev-dependency enables the scripted feature.

## 3. Witness reachability

Each family is reached by a deterministic template or by the reused direct
catalog, and every exposed value is re-validated after conversion. Reachability
is proven by `scripted_topology_and_solid_witnesses`, not by a large case count.

| Required witness | Reachable via | Independent observation |
|---|---|---|
| Convex ring | direct catalog leaf; CSG `intersection` of two offset rectangles | no reflex turn on any converted outer ring |
| Concave ring | direct catalog leaf; CSG union of a rectangle and an offset arm | at least one reflex turn |
| Non-star-shaped | explicit 8-vertex opposing-wall leaf; CSG H (two arms + middle bar) | proven empty visibility kernel: a `+x` wall and a facing `-x` wall with overlapping vertical spans force contradictory half-planes |
| Holes | direct catalog leaf; CSG rectangle minus a faceted disk or rectangle | hole count ≥ 1, every hole vertex on its own hole boundary, exact signed-area agreement with the intended leaf |
| Disconnected sets | direct catalog leaf; CSG union of two separated rectangles | ≥ 2 components with disjoint closures |
| Thin features | direct catalog leaf | minimum bounding-box dimension ≥ 16 units and a non-adjacent edge clearance ≤ `min_dim / 16` |
| Near degeneracy | direct catalog leaf; scripted explicit leaf with a one-unit neck **and** an almost-collinear vertex | exactly one canonical unit between non-adjacent walls, plus a vertex within two units of the line through its neighbours but not on it |
| Contact/invalid | direct `invalid_geometry_cases` (crossing, duplicate vertex, collapsed, touching hole, misnested hole) | the direct packet's validator contract; scripted recipes never expose these as valid |
| Layered/3D | `Box`, faceted `Cylinder`, `linear_extrude(Recipe2d)`, translate, quarter turn, union/difference/intersection (all reachable from the generator, not only from fixed tests) | closed indexed mesh, positive signed volume, bounds/volume against an axis-aligned occupancy model, and the model's designated void/span/body probes re-checked **on the converted mesh** by ray casting |

Explicit `Empty` requires the `TopologyClass::Empty` domain and an empty `Union`;
the property rejects an empty output from any other recipe.

## 4. Independent references and negative controls

Contours: the reference is the *intended* region derived from the recipe's own
integral dimensions, compared at every ring vertex, edge midpoint, one-unit
neighbour and a coarse off-lattice grid using `oracles::point_in_region`
(exact integer predicates). Total signed area must agree exactly for explicit
polygon leaves. Faceted disks are compared against the analytic regular-polygon
area, and must stay measurably below the ideal circle.

Solids: the reference is a small axis-aligned occupancy built from dimension and
boundary intervals, plus `oracles::validate_indexed_mesh` and
`oracles::mesh_signed_volume_mm3`. Mesh bounds and volume must agree inside a
documented `1e-5 * M^3` budget for f32 narrowing. The model then chooses
designated probes — an enclosed void centre plus its six blocking samples, an
unsupported-span point plus its support columns, or two separated body centres
plus the gap between them — and the **converted mesh** must agree at exactly
those points: inside where the model has material and outside where it has none.
Volume and bounds agreement alone is not accepted as evidence that the mesh kept
a cavity, span or body separation.

Negative controls, all executed:

- removing a hole from a converted `Holed` case fails `validate_geometry`;
- removing a body from a converted `Disconnected` case fails `validate_geometry`;
- collapsing an overhang plate to its pillar footprint removes the unsupported
  span (independent mesh probe outside);
- removing a bridge beam removes the unsupported span;
- removing a cavity cut makes the former void centre inside the solid;
- overlapping two separated bodies removes the empty space between their probes;
- a `Difference` that exactly cancels its base evaluates to `Empty` and is
  rejected by every non-empty domain (the quantization-collapse control);
- a **converted** mesh that joins two separated bodies through a thin beam is
  rejected by the separated-body certification: the converted surface is one
  edge-connected component, checked by positional welding and shared edges, even
  though both body centres are inside and the gap between them is empty;
- a faceted cylinder is checked against the regular-polygon area and its radius
  bounds, so a generator family without an axis-aligned occupancy model still has
  an independent dimensional reference;
- a conversion-damaged mesh (dropped faces / lost holes) is rejected by
  `validate_indexed_mesh` / `validate_geometry`; the adapter's own
  `raw rings 1 and 0 intersect or touch` rejection was observed while building
  the holed-disk template, which is why that template now centres the disk.

## 5. Generation, rejection and evaluation attempts

The attempt journal is written **before** emission and before any adapter call.

| Run | Record | Cases | Attempt directories | Evaluated | Rejected | Failed |
|---|---|---|---|---|---|---|
| routine (`296007284f258f8e2622e20e4ea24ad0`, delivered tree) | `D-SCAD-2D` | 24 | 48 | 48 | 0 | 0 |
| routine (same) | `D-SCAD-3D` | 24 | 48 | 48 | 0 | 0 |
| extended (`e87fb1fee79fe255ba4ce39d9c9f7787`, delivered tree) | `D-SCAD-2D` | 96 | 192 | 192 | 0 | 0 |
| extended (same) | `D-SCAD-3D` | 96 | 192 | 192 | 0 | 0 |

There are two attempt directories per accepted case because `isolation = "fork"`
runs the callback in a worker process that regenerates the certified tree; the
controller and the worker each record their own pre-execution input set. The
The delivered-tree runs rejected no candidate. An earlier run did record eight
rejected attempts: duplicated controller/worker evaluations of 11-, 13- and
15-facet cylinders whose bound check wrongly assumed a symmetric ideal circle.
That check now uses the signed regular-polygon extrema and the committed
reference test covers 10-, 13- and 15-facet cylinders as positive controls.
Rejections are always recorded beside the inputs with their reason and bounded by
`max_local_rejects` / `max_global_rejects` plus the per-tree proposal and
evaluation budgets; they never become a passing sample.

Adapter failures are **not** discarded samples. A real
`partial`/`degraded`/`approximate`/unclassified outcome returns
`ScriptedError::Adapter`, is recorded as a generator fault, and fails the run as
`generator_or_oracle`; only a domain/witness mismatch is an ordinary rejection.

## 6. Structural shrink trace

Measured on a `Holed` domain with `max_evaluations = 128` (one tree). Every line
below is copied verbatim from `target/pbt-scripted-measure.log`; complexity is
`(nodes, explicit vertices + facets, sum of absolute integral parameters)`.

```text
trace initial: complexity nodes=1 vertices+facets=50 sum_abs=1993469 components=4 holes=8 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=49 sum_abs=1953588 components=4 holes=8 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=48 sum_abs=1913706 components=4 holes=8 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=44 sum_abs=1754230 components=4 holes=7 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=40 sum_abs=1594746 components=4 holes=6 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=36 sum_abs=1435254 components=4 holes=5 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=32 sum_abs=1275754 components=4 holes=4 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=28 sum_abs=1116246 components=4 holes=3 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=24 sum_abs=956730 components=4 holes=2 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=20 sum_abs=797206 components=4 holes=1 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=16 sum_abs=637822 components=3 holes=1 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=12 sum_abs=478430 components=2 holes=1 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=319030 components=1 holes=1 area_mm2=0.000005
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=319018 components=1 holes=1 area_mm2=0.000003
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=319012 components=1 holes=1 area_mm2=0.000002
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=319008 components=1 holes=1 area_mm2=0.000001
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=319006 components=1 holes=1 area_mm2=0.000001
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177494 components=1 holes=1 area_mm2=0.000001
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177464 components=1 holes=1 area_mm2=0.000000
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177450 components=1 holes=1 area_mm2=0.000000
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177442 components=1 holes=1 area_mm2=0.000000
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177438 components=1 holes=1 area_mm2=0.000000
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=177436 components=1 holes=1 area_mm2=0.000000
trace simplify: complexity nodes=1 vertices+facets=8 sum_abs=24 components=1 holes=1 area_mm2=0.000000
```

Optional holes and bodies are removed down to the required minimum, then
coordinates shrink. The required hole survives, every intermediate value
re-validates, and every accepted step strictly decreases `RecipeComplexity`. The
last steps keep `vertices+facets` constant while `sum_abs` falls, which is why
the tuple has three components rather than a single scalar. This is a
deterministic bounded search, **not** a claim of global minimality.

3D scenes shrink structurally too: optional cuts and union members are dropped,
dimensions/radii/heights halve, facets and translations reduce. Each candidate is
re-evaluated and re-certified, so a shrink that loses the cavity, bridge span,
overhang or a separated body is rejected. The retained solid's volume never
grows.

## 7. Completed versus interrupted minimization

| Situation | Observed outcome |
|---|---|
| Passing run, no failure | `outcome = "passed"`, `shrink_completed = null` (no shrink was needed) |
| All-rejected callback population | `outcome = "exhausted"`, distinct from an assertion failure |
| Known failing probe, one-evaluation tree budget | `outcome = "failed"`, `failure.json.shrink_completed = false` |
| Fatal artifact failure outside any callback | `outcome = "failed"`, `error_kind = "generator_or_oracle"` |
| Shrink search exhausted inside its limits | `shrink_completed = true` on the end event |
| Deadline kill of the whole libtest child | nonzero run status, `property_process_timeout` in the combined log, prepared attempts retained, no minimization claim |
| Containment cannot be created or assigned | the run fails closed: an uncontained execution child is refused instead of being run |
| Process killed while a candidate render is pending | the retained attempt is `prepared` (input set complete, no fabricated `evaluated` state) and replays from its retained bytes |
| Kill during a **generation** render and during a **shrink** candidate render | both phases are exercised: the fixture pauses on evaluation 1 and on evaluation 2 respectively, and each killed run retains a `prepared` attempt |

A process-killed `prepared` attempt is evidence of an interrupted input, not
proof that it was the globally smallest failure.

## 8. Costs (measured, this machine, `dev` profile)

Separated entry point:
`cargo test -p slicer-sdk --features property_testing_scad --test property_scripted_contracts -- scripted_measure_representative_costs --ignored --nocapture`
(5 representative recipes — rectangle, faceted disk, hole difference, cavity
solid, bridge solid — and 8 shrink trees).

| Phase | Measurement |
|---|---|
| Recipe emission (`emit_scad`, 100 samples) | ≈ 9.7 µs per recipe (0.97 ms total in the recorded run) |
| Recipe-strategy generation (`ScriptedStrategy::new_tree`, 16 trees: family selection + recipe construction + first render + certification) | ≈ 36.6 ms per certified tree (0.59 s total in the recorded run) |
| Candidate render alone (`evaluate_recipe`, 100 samples) | ≈ 0.53 ms per recipe in the recorded run (only the recorded run is quoted as evidence) |
| Structural shrink (8 trees, 170 accepted steps) | ≈ 26 ms per accepted step (4.41 s total); each step re-emits, re-renders and re-certifies across all candidates, so the cost is not per accepted step alone |
| SDK+engine compilation | the engine edge is the measured cold cost in `docs/scad/windows-feasibility.md` (≈ 4 m 26 s for a full cold `slicer-scad --features openrscad` build); no separate cold figure is claimed here |
| Selected routine run (delivered tree) | `D-SCAD-2D` 1.52 s, `D-SCAD-3D` 0.34 s |
| Selected extended run (delivered tree) | `D-SCAD-2D` 8.85 s, `D-SCAD-3D` 1.37 s |

Attribution is measured, not assumed. Fork isolation spawns **one** worker per
property process, not one per case; the retained journals show a single worker
PID for all 24 routine and all 96 extended callbacks. The 2D/3D gap is instead
per-case cost inside that worker: the planar record validates up to ~96 vertices,
8 holes and 8 components with exact predicates and dozens of region probes on
every exposed value, while a 3D witness scene is a handful of axis-aligned cells.
The recorded strategy-generation figure (≈ 37 ms per certified contour tree) is
the measured dominant term; `emit_scad` alone is ~10 µs.

## 9. Eligible profiles and why

Both records use the same finite profile shape. Registry values are in
`test-properties.toml`; the reasons:

| Key | Routine | Extended | Reason |
|---|---|---|---|
| `cases` | 24 | 96 | covers every `prop_oneof!` family with headroom; extended adds sampling depth, not new families |
| `max_local_rejects` | 32 | 128 | a CSG template may be rejected by the domain validator (for example a `min_holes` request the template cannot meet); the measured population rejects none |
| `max_global_rejects` | 128 | 512 | bounded; an all-rejected population must report `exhausted` |
| `max_default_size_range` | 32 | 128 | recipe parameters are chosen from explicit templates, not from size scaling |
| `max_shrink_iters` | 64 | 256 | the measured trace needs 15 accepted steps for a 7-component/5-hole start |
| `max_shrink_time_ms` | 0 | 0 | iteration-bounded shrinking; no elapsed-time cutoff hides a slow step |
| `isolation` | `fork` | `fork` | callback faults in the engine path must not take down the controller |
| `timeout_ms` | 30000 | 60000 | per-case callback budget, measured render cost is ≈ 0.55–2.5 ms per recipe |
| `max_abs_units` | 20000 (2 mm) | 40000 (4 mm) | keeps `max_abs_coordinate_mm` and every translation/extrusion intermediate inside the same envelope |
| `max_vertices` / `max_holes` / `max_components` | 64 / 4 / 4 | 96 / 8 / 8 | the catalog's required minima fit; larger bounds only widen the sampled population |
| `max_shrink_candidates` | 32 | 64 | bounds proposals per tree without turning rejection into success |
| `max_recipe_nodes` / `max_recipe_depth` | 24 / 6 | 32 / 8 | fits every template, including the 4-node H and 3-node bridge |
| `max_facets` | 16 | 24 | fits the recipe `u16` and the adapter `u32`; the adapter default is 360 |
| `max_source_bytes` | 16384 | 32768 | a 96-vertex leaf is ≈ 2 KiB; the AST capacity derivation is floored by this value |
| `max_evaluations` | 48 | 128 | per-tree initial retries plus shrink proposals |
| `max_scad_vertices` / `max_scad_triangles` | 200000 / 400000 | 400000 / 800000 | explicit mesh capacity, far below the adapter defaults but finite |
| `evaluation_steps` | 5000000 | 20000000 | explicit finite fuel; `0`/`u64::MAX` are rejected by the adapter |
| `worker_stack_bytes` | 268435456 | 268435456 | the adapter's measured safe default (≥ 64 MiB floor) |
| `test_process_timeout_ms` | 120000 | 600000 | whole libtest child, including generation and shrink rendering; measured selected runs are 2.30 s / 28.99 s |

`max_ast_nodes`, `max_ast_depth` and `max_geometry_nodes` are **derived** from
the recipe bound rather than left permissive: eight AST nodes per recipe node
plus sixteen, floored by `max_source_bytes` (a polygon leaf contributes one AST
node per vertex, and every evaluated CSG node comes from a statement, which
occupies at least one byte); and two AST levels per recipe level plus eight.
`scripted_cases` and `evaluate_recipe` reject any smaller adapter capacity before
a render, so a mis-sized profile fails loudly.

## 10. Process containment

Registered `requires_scad` records must carry a positive
`bounds.test_process_timeout_ms`; the registry validator rejects a record without
one. The executor supervises the whole libtest execution child: it drains both
pipes on reader threads, polls `try_wait`, and on deadline terminates the
**Windows process tree** with `taskkill /T /F`, then joins the child and reports
a nonzero status with `property_process_timeout` in the combined log. Killing
only the controller would leave Proptest worker processes running.

Proven by `property_scad_supervision_covers_generation_shrink_and_descendants`
(`cargo test -p xtask --bin xtask -- property_`): a fixture that blocks **before
any callback** and keeps a live descendant process is ended inside its 1500 ms
deadline, the descendant PID is gone afterwards, the log records
`property_process_timeout`, and a real bounded `D-SCAD-3D` run passes under the
same supervised path with `status: Some(0) (timed_out: false)`.

## 11. Full `G-SCAD` closure

| Obligation | Evidence |
|---|---|
| Pin and build the engine on Windows | `docs/scad/windows-feasibility.md` |
| Default-off and enabled builds; no engine from workspace membership alone | `docs/scad/windows-feasibility.md`; §2 here |
| Execute parse/evaluate/render and convert holed 2D contours and a 3D mesh | `docs/scad/windows-feasibility.md`; §3 and §4 here (holed disk difference, extruded solids, cavities, bridges) |
| Parameter expressions with the shared budget/exact mode | `docs/scad/windows-feasibility.md` |
| Strict diagnostic classification, including nonempty partial/degraded outputs | `docs/scad/windows-feasibility.md`; here, a touching-ring conversion is a hard `invalid_contours`, never a passed sample |
| Finite/range/topology checks after conversion | `docs/scad/windows-feasibility.md`; §4 here |
| Reproducibility for controlled self-contained programs and fresh case state | §12 here: same explicit seed reproduces geometry and provenance exactly, twice; a direct replay of a retained case matches source, options and provenance |
| Process-level timeout/abort handling (not a thread timeout) | §10 here |
| Measure representative generation/render/shrink costs and set eligible profiles | §8 and §9 here |

Full `G-SCAD` is therefore recorded as **passed** for the bounded scripted
property surface. It does not claim unrestricted SCAD program generality, and it
does not make the engine an oracle.

Unsafe-path evidence includes a directory where a file is expected, a
non-directory replay root and an oversized file. The probe also attempts a
directory junction and a file symlink where a regular file is expected; on this
machine's workspace volume both `mklink` forms are refused by Windows ("Local
volumes are required", "The specified path is invalid"), so that execution
witness is **unverified here** — the probe reports both failures instead of
passing silently. The refusal itself is implemented in `read_checked` (symlink
metadata plus a canonical-containment check).

Native-risk fixture and replay bodies run in supervised children of the test
binary (one child per contract body, deadline-killed on expiry), not on the
parent thread.

Explicit-seed sequence evidence: the routine `D-SCAD-2D` record ran with seed
`6892392065927971396` (run token `05be38e61a464da0934e114ebe22c77f`, PASS in
1.29 s), and the same seed was then replayed through the registry
(`cargo xtask test --summary --properties --property D-SCAD-2D --tier routine
--with-scad --seed 6892392065927971396`; run token
`84809a1a2af13a9816405faa64e175d5`, PASS in 1.27 s). Both contexts record the
same seed, `source_state_digest`, profile and feature set; the comparison is in
`target/pbt-scad-seed-replay-evidence.log`.

Reproducibility evidence: `write_scripted_artifacts` followed by
`replay_scripted_case` returns a case with identical recipe, source, options and
geometry; two independent evaluations with the same explicit seed produce equal
`geometry` and `provenance`; a changed source/recipe/bounds/options/engine
identity, a changed generator version, a missing/truncated/oversized file or a
directory where a file belongs are all refused as `ReplayMismatch`/`Artifact`
rather than silently replayed with defaults.

## 12. Combined `G-GEOMETRY` decision

Disposition: **`hybrid_sufficient`**.

Stated bounded domain: canonical-unit coordinates within `max_abs_units`
(2 mm routine / 4 mm extended), at most 64/96 explicit vertices, 4/8 holes, 4/8
components, recipe nodes ≤ 24/32, facets ≤ 16/24, and sources ≤ 16/32 KiB. Inside
that domain the bounded direct catalog plus the scripted recipes reach every
required class and shrink meaningful failures while retaining the required
witness. **This is not unrestricted simple-polygon generality**: an arbitrary
simple polygon outside those bounds, or one needing more than the catalog's
construction templates, is not generated.

| §6.2 obligation | Direct source | Scripted source | Combined evidence |
|---|---|---|---|
| Convex ring | `geometry_cases(Convex)` | intersection of offset rectangles; contained union | `domain_geometry_topology_witnesses` + `scripted_topology_and_solid_witnesses`; independent no-reflex observation |
| Concave ring (incl. non-star) | `geometry_cases(Concave)`, `geometry_cases(NonStarShaped)` | union with an offset arm; H template | proven empty visibility kernel for both; independent reflex/kernel checks; `scripted_generator_reaches_every_advertised_family` proves the CSG templates are actually generated |
| Holes | `geometry_cases(Holed)`, `min_holes` | rectangle minus disk / rectangle | hole count, hole-boundary probes, exact intended area; required hole survives shrinking |
| Disconnected sets | `geometry_cases(Disconnected)` | two separated rectangles | ≥ 2 components throughout; optional bodies are removed only above the required minimum |
| Thin features | `geometry_cases(Thin)` | — (direct only) | measured `min_dim / 16` clearance |
| Near degeneracy | `geometry_cases(NearDegenerate)` | explicit leaf with a one-unit neck **and** an almost-collinear vertex | one-unit non-adjacent clearance plus a vertex ≤ 2 units off the line through its neighbours, both observed on converted geometry |
| Contact/invalid cases | `invalid_geometry_cases` (5 named faults) | adapter rejection of touching/crossing output | both sides reject; the adapter path returns `invalid_contours`, never a valid case |
| Layered/3D geometry | `analytical_meshes`, `rigid_transforms` | extrusion, cavities, bridges/overhangs, separated bodies, dimension-controlled cross-sections; every closed-solid, cavity and separated-body family has multiple construction variants (box, cylinder, extrusion, quarter turn, intersection) | closed mesh + occupancy volume/bounds + the model's probes re-checked on the converted mesh by a multi-direction ray cast (shared-edge hits are discarded rather than double counted), plus converted-surface connectivity for separated bodies and a regular-polygon reference for cylinders; the committed counterexample joins two bodies with a real converted beam and is rejected |

Shrinking obligations:

- every exposed value passes the domain/witness checks (`scripted_topology_and_solid_witnesses`
  re-validates every `current()` during shrinking);
- deletion of optional holes/bodies, dimension/facet/translation reduction and
  Boolean simplification are exercised (§6 trace);
- revalidation happens after conversion, not just after AST editing;
- the complexity ordering is stated and strictly decreasing, with no global
  minimality claim;
- limits are reported as `shrink_completed = false` and never as a pass;
- the required witness is retained by construction (certification is the
  acceptance gate for every candidate).

`general_generator_blocked` is **not** recorded: no required class is missing, and
no broad gap was demonstrated that a bounded direct family could fill. The
scripted surface deliberately does not generate arbitrary polygons; that is a
stated boundary of the disposition, not an unmet obligation of the classes above.

## 13. What is not claimed

- No host or module SUT coverage credit: these are helper properties.
- No unrestricted polygon generality, no arbitrary vertex-deletion validity, no
  global minimality.
- No claim that a successful render proves slicer correctness.
- No release-profile cost figures and no CI-observed figures; every number here
  is from this Windows development machine and is labelled with its context.
- No claim that an in-engine render can be interrupted from outside at this API
  surface. The supervision seam pauses at the exact pre-render window; the
  executor kills the whole process tree, so any in-engine phase is contained by
  the same mechanism.
