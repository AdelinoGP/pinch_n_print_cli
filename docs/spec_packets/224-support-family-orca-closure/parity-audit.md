# Packet 224 — Adversarial OrcaSlicer Parity Audit

**Scope:** `d2a92e1e558e7a8c4a6447aa04814cea0a8688e8..HEAD` (33 commits) — tree support, traditional
support, support analysis, support aggregation.
**Method:** read-only swarm audit, 9 workers, one per surface plus a follow-up on the AC-6 no-Orca-read
gate. Canonical claims are cited by OrcaSlicer file + function only (`OrcaSlicerDocumented/`).
**Date:** 2026-08-20. **Guest WASM:** `cargo xtask build-guests --check` clean (no `STALE:`), so no
finding below is attributable to a stale guest artifact.

> Packet 224 docs (`design.md`, `tree-*.md`, `implementation-plan.md`) were authored by the same model
> that wrote the code. They are treated here as evidence to verify, never as truth. Where they are
> accurate this is stated explicitly.

**Commit classification (33):** 15 code-touching, 15 test-touching, 1 golden-only (`3c8d394e`),
13 docs-only.

---

## 1. Per-surface verdicts

| # | Surface | Verdict | Canonical refs checked | Key evidence |
|---|---------|---------|------------------------|--------------|
| 1 | `modules/core-modules/tree-support-planner/` | **DEVIATIONS FOUND** (8 HIGH, 6 MED) | `TreeSupport.cpp::{generate_contact_points, drop_nodes, calc_branch_radius, detect_overhangs, smooth_nodes}`, `TreeSupport.hpp` | RC-15 sampling port is faithful; radius clamp, merging, MST grouping, move pass, `to_buildplate` all deviate |
| 2 | `modules/core-modules/tree-support/` (+ planner render) | **DEVIATIONS FOUND** (2 CRIT, 4 HIGH) | `TreeSupport.cpp::{draw_circles, generate_toolpaths, calculate_collision, calculate_avoidance}` | Roof band capped per-object; body geometry cleared wholesale; hull-of-two-circles is not canonical's ellipse |
| 3 | `traditional-support{,-planner}/` | **DEVIATIONS FOUND** (2 HIGH, 6 MED) | `SupportCommon.cpp::generate_interface_layers`, `SupportMaterial.cpp::{detect_bottom_contacts, trim_support_layers_by_object}`, `SupportParameters.hpp` | Interface pitch omits flow spacing (~1.9x over-dense, measured vs Orca G-code); interface layer count off by one vs observed Orca |
| 4 | `slicer-runtime` support analysis / routing | **DEVIATIONS FOUND** (2 CRIT, 4 HIGH) | `SupportMaterial.cpp::detect_overhangs`, `PrintConfig.cpp::{support_threshold_angle, support_type}` | Threshold angle config unreachable; `support_angle` fallback reads the pattern-rotation key; auto/manual axis discarded |
| 5 | `slicer-ir` + `slicer-scheduler` role marshalling | **MOSTLY CONFIRMED** (1 HIGH, 2 MED) | `ExtrusionEntity.hpp` ExtrusionRole, `ExtrusionEntity.cpp::role_to_string` | WIT carries the role losslessly; `;TYPE:` strings match canonical exactly; `erSupportTransition` absent |
| 6 | `slicer-wasm-host` aggregation / marshal | **DEVIATIONS FOUND** (3 HIGH) | none (no canonical analogue — stated, not invented) | Routing-cell fix is correct; occupancy rejection silently deleted; **contract suite is red on HEAD** |
| 7 | All test changes in range | **DEVIATIONS FOUND** (7 HIGH) | `SupportCommon.cpp`, `TreeSupport::generate_toolpaths` | Two headline invariants are structurally unfireable; 19 call sites set a dead config key |
| 8 | `resources/golden/` replacement | **MOSTLY CONFIRMED** (1 HIGH, 3 MED) | `TreeSupport.cpp::generate_contact_points` | Deleted `*_orca_*` goldens were **never** Orca-derived; no oracle was lost there |
| 9 | AC-6 no-Orca-read gate (follow-up) | **DEVIATIONS FOUND** (4 HIGH) | `tmp/SupportTest_*_Orca.gcode` | `tmp/` is gitignored (narrow gate defensible); the AC's "no Orca-derived constant" clause is not |

---

## 2. Suspicion-list verdicts

### S-1 — `3c8d394e`: rename off `orca_parity`, golden deletion, branch count 9 -> 8

**Verdict: NOT the concealment it appears to be — but the new baseline is self-blessed.**

The deleted `benchy_tree_support_orca_*.txt` were **never OrcaSlicer reference data**. The commit that
created them (`e19493a5`) says so verbatim: *"deterministic ModularSlicer self-captures from the
synthetic single-object overhang fixture, not external OrcaSlicer reference data"*, and the file header
at `d2a92e1e` read *"Source: Pinch 'n Print self-capture ... The filename retains `_orca_` for stability
of test paths only."* They had already been rebaselined three times (5 -> 3 -> 9). **The rename and the
new "NOT parity evidence" header make the file more honest, not less.** No parity oracle was lost here.

The 9 -> 8 drop is **explained but not canonically justified**. `ad9019ee` (RC-15) genuinely changed
contact derivation from one point per overhang-triangle centroid to canonical corner + arc-walk + rotated
lattice sampling deduped on a `base_radius` grid, so both contact count and positions change — the drop
is a real consequence of a real port, not a silent regression. But no OrcaSlicer function was cited that
predicts the value 8, and `implementation-plan.md` only *predicted* the number would move ("will move the
branch count — regenerate once, in Step 8"). Prediction is not derivation. Two further facts matter:
"branch count" is a misnomer (it is `entries.len()`, i.e. layers-with-support), and with golden 9 the
+/-10% band was [8, 10] — **so 8 was already passing; the regeneration was not required to make it green.**

Residual concern: the new endpoints golden has 96 endpoints of which only 56 are unique — 40 exact
duplicates, several on degenerate integer coordinates. Coincident skeleton endpoints suggest collapsed
branch points, and nothing in the range investigates them.

### S-2 — `4d1848eb` "repair wrong-reason tests"

**Verdict: PARITY-CONFIRMED. These are genuine strengthenings.**
`invalid_body_degraded`'s old fixture really was dead — a single coplanar triangle at z=100 with no
cross-section at any Z, plus a `Transform3d::default()` all-zeros matrix. The repair substitutes a closed,
consistently-wound box and an explicit identity matrix, and *adds* a structured reason assertion plus a
retained-control body. `invalid_body_rejected` gained a new positive counterpart
(`support_body_straddling_absolute_cell_boundary_is_retained`).

**However** the same commit renamed the planner's config read to `tree_support_branch_angle` and updated
only *one* test file. **19 call sites across `orca_parity_tdd.rs`, `to_buildplate_tdd.rs` and
`diagnostics_tdd.rs` still set the now-unread `support_branch_angle_deg`.** They pass only because every
one sets 45.0 — exactly `DEFAULT_BRANCH_ANGLE_DEG`. No test varies branch angle at all, and the
angle-derived geometry assertions are testing a value the code never receives.

### S-3 — `4c67ccd9` wip "2 open failures"

**Verdict: PARITY-CONFIRMED — both fixed by real code changes, with one caveat.**
The two were `final_gcode_roles` and `interface_is_topmost_and_carved_out`. The first was fixed at
`4d1848eb` (a schema whose declared default 0.0 sat outside its own min 0.05). The second at `ed62090d`,
whose diff touches only `src/` files — `git show ed62090d -- support_family_closure.rs` is **empty**, so
the invariant itself was not edited. No deletion, no loosening, no golden regen was used to close them.

**Caveat:** the fix that made `interface_is_topmost_and_carved_out` pass is the `carved.clear()` defect
(F-3 below), which also made one third of that invariant permanently unreachable. The test went green
partly by having its assertion rendered unfireable.

### S-4 — `9f4540bd` swept-capsule math

**Verdict: DEVIATION (HIGH).** In-tree takes the convex hull of the two MST-endpoint circles on the *same*
layer. Canonical `TreeSupport.cpp::draw_circles` draws **one circle per node per layer**, distorted into
an ellipse along that node's own `node.movement` via the 2x2 movement matrix, then unions per layer.
`TreeSupport3D.cpp::extrude_branch` does a true 3D mesh extrusion + reslice. Neither is a hull between
nodes; this is a different construction, not a resolution difference. Compounding: 16 hardcoded circle
segments (canonical `CIRCLE_RESOLUTION` is 4 or 100; `SUPPORT_TREE_CIRCLE_RESOLUTION` is 25), and a
post-union `limit_contour_vertices(..., 16)` that truncates merged multi-branch contours to a fixed vertex
budget where canonical simplifies by a distance tolerance.

### S-5 — regen hatches, renames, loosened assertions, deleted fixtures

**Regen hatch: SAFE.** `SUPPORT_PLANNER_REGEN_GOLDEN` is read with `.is_ok()`, not a defaulted parse, so
the write-and-return branch is unreachable when the var is absent; missing goldens `panic!` rather than
silently regenerating. It is the only such env var in scope. Note the branch returns *before* asserting,
so a regeneration run can never fail — standard practice, but with no Orca oracle remaining it is the only
gate on tree-support geometry.

**Renames: SAFE.** Only one test was renamed off `orca_parity`, and its semantics did not change (the
pre-range doc comment already disclaimed parity).

**Loosened assertions: CONFIRMED, multiple.** Listed as F-22.

**Deleted fixtures/tests: CONFIRMED.** `enforcer_overrides_needs_support_false`,
`branching_pattern_present` (tree-vs-traditional distinguishing property), `density_affects_coverage`.
Partial compensation exists for the latter two; the enforcer-precedence gap is registered as G-17.

---

## 3. Findings by severity

### CRITICAL

**F-1 — Top-interface roof band is capped by a per-object counter, not per node**
`canonical_ref`: `TreeSupport.cpp::generate_contact_points` (`roof_layers` on `create_node`) and
`::drop_nodes` (`support_roof_layers_below` decrement).
`in_tree_symbol`: `roof_band_layers_emitted` / `node_roles` (`modules/core-modules/tree-support-planner/src/lib.rs`).

```rust
let mut roof_band_layers_emitted = 0u32;          // declared ONCE per object
let is_roof = top_n > roof_band_layers_emitted && node.dist_to_top < top_n;
```
Canonical carries the counter **per node**, initialised at each contact. Here it is per object and
increments on every interface-emitting layer, so once the first `top_n` such layers are past, `is_roof`
is false forever. **Any second, lower overhang on the same object receives no top interface at all.**
Independently found by two workers.

**F-2 — Support-threshold angle config is unreachable; the fallback key is the wrong setting**
`canonical_ref`: `PrintConfig.cpp` `support_threshold_angle` (coInt, default 30) vs `support_angle`
(coFloat, default 0 — support *pattern rotation*); `SupportMaterial.cpp::detect_overhangs`.
`in_tree_symbol`: `support_threshold_angle_deg` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`).

The function reads only `config.extensions`, but `support_overhang_angle` is a CLI-bound **typed** field;
`resolve_*` inserts into `extensions` only when `apply_cli_key` returns false, so the key is never there.
Every slice therefore uses the hardcoded `DEFAULT_SUPPORT_THRESHOLD_ANGLE_DEG = 45.0` and **the user's
overhang angle is silently ignored.** The typed field is never consulted. Separately, the 3MF-supplied
`support_threshold_angle` key is never looked for, and the declared fallback `support_angle` is Orca's
pattern-rotation angle (default 0) — if present it would be misread as a threshold of 0. Canonical default
is 30, not 45.

**F-3 — Branch body geometry is deleted wholesale on any interface layer**
`canonical_ref`: `TreeSupport.cpp::generate_toolpaths` / `draw_circles` — `base_areas = diff_ex(base_areas, roofs)`
subtracts the roof footprint and **keeps the remainder**.
`in_tree_symbol`: `build_roles` (`modules/core-modules/tree-support-planner/src/lib.rs`).

Verified verbatim on disk:
```rust
let mut carved = body;
for cut in [&roof, &floor] { ... carved = clip_polygons(&carved, cut, Difference); }
if !roof.is_empty() || !floor.is_empty() { carved.clear(); }   // discards the carve
```
The canonical subtraction is computed and then unconditionally thrown away: on any layer carrying *any*
roof or floor polygon, **all** branch-body geometry for that layer is dropped rather than carved.
Independently found by three workers. Introduced by `ed62090d` ("close the interface-invariant gauntlet").

This defect is self-concealing — see F-4.

### HIGH

**F-4 — The packet's two headline interface invariants are structurally unfireable**
`in_tree_symbol`: `interface_is_topmost_and_carved_out` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`);
`raft_and_interface_layers_emit_expected_entry_count` (`modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`).

Both assert that body and interface geometry do not overlap. Because F-3 guarantees the body role is never
pushed on an interface layer (and the traditional planner emits exactly one role per entry), body and
interface can never both be non-empty on one entry. **The `intersection_ex` call and the nested overlap
loop are unreachable.** The "12/12 closure" of the interface gauntlet was achieved with its headline
assertion unable to fail. The planner-side test replaced a real quantitative assertion
(`interface_max > top_segs`) that *was* deleted in this range.

**F-5 — `slicer-wasm-host` contract suite is RED on HEAD**
**Measured this session**, guests verified fresh:
`cargo test -p slicer-wasm-host --test contract support_plan_validation` -> **2 passed, 3 failed**.
```
support_plan_validation                                   left: 2  right: 1
support_plan_validation::support_plan_validation          left: 2  right: 1
support_plan_aggregation_diagnoses_duplicate_identity     left: 0  right: 1
```
Cause (F-6): `ed62090d` deleted the exact-Z occupancy rejection, so the reason string
`"body rejected: exact-Z occupancy"` these tests assert on is no longer reachable, and the `colliding`
body now passes validation instead of being dropped. The packet was carried to a closure ceremony with
this suite failing. All four support module suites are green, which is why it was not noticed.

**F-6 — Exact-Z occupancy rejection silently deleted from the host validator**
`in_tree_symbol`: `validate_entry` / deleted `overlaps_any` (`crates/slicer-wasm-host/src/support_aggregation.rs`).
`.map(|query| { ... overlaps_any(body, &query.occupancy) ... })` became `.map(|_| None)`; `overlaps_any`
is gone. The exact-Z query result is now discarded — it only proves the object resolves. No production
consumer performs the intersection anywhere in `slicer-runtime` or `slicer-wasm-host`, so **support/model
separation now rests entirely on guest planners policing themselves**: a host trust boundary moved into
untrusted guest code. Deleted in `ed62090d` with no deviation-log row.

**F-7 — Traditional interface line spacing omits the flow-spacing term (~1.9x over-dense)**
`canonical_ref`: `SupportParameters.hpp` — interface spacing is
`support_interface_spacing.value + interface_flow.spacing()`.
`in_tree_symbol`: `TraditionalSupport::run_support` (`interface_line_spacing = mm_to_units(self.interface_spacing_mm)`).
Measured from the authoritative reference `tmp/SupportTest_Normal_Orca.gcode`, layer 123 interface:
X pitch 103.186 -> 103.943 -> 104.700 = **0.757 mm** = `support_interface_spacing (0.4) + flow spacing (0.357)`.
In-tree uses the configured 0.4 mm directly as the pitch. Over-extrusion on every traditional interface layer.

**F-8 — Traditional top interface emits one layer fewer than observed Orca**
`canonical_ref`: `SupportCommon.cpp::generate_interface_layers`; observed output of OrcaSlicer.
Per-layer scan of `tmp/SupportTest_Normal_Orca.gcode` (`support_interface_top_layers = 2`): layers 1-121
are `;TYPE:Support`, layers **122, 123, 124** are `;TYPE:Support interface` — three dense top layers.
In-tree produces exactly two. The mechanism producing Orca's third layer was not identified (the
base-interface path is excluded — `support_interface_filament = 0`), so the *cause* is unexplained, but
the output gap is measured against real Orca output.

**F-9 — Branch radius capped 40% below canonical, justified by a non-existent symbol**
`canonical_ref`: `TreeSupport.hpp` `MAX_BRANCH_RADIUS = 10.0`; `TreeSupport.cpp::calc_branch_radius`.
`in_tree_symbol`: `MAX_BRANCH_RADIUS_MM = 6.0` (`modules/core-modules/tree-support-planner/src/lib.rs`).
Verified directly: canonical `TreeSupport.hpp` declares `const coordf_t MAX_BRANCH_RADIUS = 10.0;`. The
in-tree doc comment attributes 6.0 to *"OrcaSlicer's `TreeSupportData::max_radius` hard upper bound"* —
**that symbol does not exist**; grep for `max_radius` under `Support/` returns only
`TreeModelVolumes::get_collision_lower_bound_area`'s parameter. A fabricated canonical citation.

**F-10 — Interface-layer radius floor dropped**
`canonical_ref`: `TreeSupport.cpp::calc_branch_radius` — `if (support_interface_top_layers.value > 0) radius = std::max(radius, base_radius);`
`in_tree_symbol`: `tapered_radius`. With the module's own default `support_interface_top_layers = 2`,
canonical never tapers below `base_radius` (2.5 mm at default diameter 5.0); in-tree tapers tips to 0.4 mm.
The test `tapered_radius_no_longer_floors_at_branch_radius` pins the removal of exactly this floor.

**F-11 — Branch merging uses a flat invented threshold with no leaf test**
`canonical_ref`: `TreeSupport.cpp::drop_nodes` first pass.
Canonical merges only when `neighbours.size()==1 && vsize2_with_unscale(...) < get_max_move_dist(p_node,2)
&& mst.adjacent_nodes(neighbours[0]).size()==1` (**both** endpoints MST leaves), creates a **new** node at
the midpoint, and has a separate multi-neighbour branch gated on `dist_mm_to_top` ordering (STUDIO-6326).
In-tree: `if *d < self.merge_distance_mm { drop[*a.max(b)] = true; }` — a flat constant, no leaf-degree
test, no midpoint node; the higher-index endpoint is simply deleted.

**F-12 — Single global MST instead of canonical per-part spanning trees**
`canonical_ref`: `TreeSupport.cpp::drop_nodes` (`nodes_per_part` / `spanning_trees`).
Canonical partitions layer nodes into group 0 (`to_buildplate`) plus one group per
`m_layer_outlines_below` part and builds one MST per group. In-tree builds one Prim MST over all
`active_nodes`. Nodes on opposite sides of the model can become MST neighbours, merge, and pull each other
across the object. Canonical's guard against exactly this — `is_line_cut_by_contour` — has no in-tree equivalent.

**F-13 — Move pass diverges from canonical displacement rule**
`canonical_ref`: `TreeSupport.cpp::drop_nodes` second pass —
`movement = normal(direction_to_outer, scale_(get_max_move_dist(&node)))`, i.e. always a step of exactly
`min(tan_angle*height, support_extrusion_width)`. In-tree steps toward a 1/d^2-weighted neighbour aggregate
capped at `tan_angle * effective_height * wall_count` with **no `support_extrusion_width` cap**, then
post-hoc clamps out of avoidance. Absent entirely: `DO_NOT_MOVER_UNDER_MM` (5 mm no-move zone),
`max_converge_distance`, `is_line_cut_by_contour`, and the STUDIO-7883 radius clamp. (The 1/d^2 direction
itself *is* equivalent to canonical's `sum_direction += direction * (1/dist2_to_neighbor)`.)

**F-14 — `to_buildplate` computed once and never recomputed**
`canonical_ref`: `TreeSupport.cpp::generate_contact_points` (`insert_point`) + `::drop_nodes`.
Canonical sets `to_buildplate = true` **unconditionally** at contact creation, then recomputes it for
every descendant: `to_buildplate = !is_inside_ex(m_layer_outlines[obj_layer_nr_next], next_layer_vertex)`.
In-tree computes it once from the collision test and copies it unchanged down every propagation step. The
in-tree doc comment claims this *is* canonical's initial assignment — canonical's initial assignment is
the constant `true`. Canonical's `unsupported_branch_leaves` pruning under `support_on_buildplate_only`
has no equivalent.

**F-15 — Tree overhang detection uses mesh-facet normals, not canonical 2D slice difference**
`canonical_ref`: `TreeSupport.cpp::detect_overhangs` — `diff_ex(curr_polys, lower_layer_offseted)`.
`in_tree_symbol`: `detect_overhang_facets`. In-tree classifies raw triangles by normal-z
(`nz_unit <= -sin(45deg)`) and unions their XY projections. The source comment *"OrcaSlicer uses the same
z-normal threshold in `detect_overhangs`"* is **false**. Labelled a "legacy-path compatibility shim" but
it runs unconditionally whenever `obj.vertices` is non-empty — it is not gated behind the host
`SupportAnalysisView` path. Also absent: sharp-tail clustering, `support_remove_small_overhang`,
`max_bridge_length` bridge exemption, `enforce_support_layers`, `support_critical_regions_only`.

**F-16 — Avoidance/collision volumes are not canonically constructed** (guard *sense* is now correct)
`canonical_ref`: `TreeSupport.cpp::TreeSupportData::{calculate_collision, calculate_avoidance}`.
The `acf9fa1d` inversion fix is correct — canonical avoidance is a *forbidden* region and
`move_out_expolys` pushes nodes out of it. But: (a) `avoid_inflate` uses
`tree_support_branch_distance / 2.0`, which is canonical's contact-point `point_spread`, **not** an XY
clearance — canonical uses `m_xy_distance` (`support_object_xy_distance`), absent from both tree
manifests; (b) `collision_polys` are raw outlines with **zero** inflation, so tree support is clipped
flush to the model with no XY gap; (c) in-tree avoidance is per-layer, canonical is recursive
(`offset_ex(get_avoidance(radius, layer-1), -max_move)`) — the recursion is what prevents a branch being
trapped; (d) inflation uses a constant radius, not the per-node tapered radius.

**F-17 — Tree renderer fills interface at body density**
`canonical_ref`: `TreeSupport.cpp::generate_toolpaths` —
`interface_density = min(1., interface_flow.spacing()/interface_spacing)`, plus a separate
`bottom_interface_density`. In-tree `run_support` calls the same `render_polygon` for `SupportBody`,
`TopInterface` and `BottomInterface`; spacing is always `line_width / density`. Neither
`support_interface_spacing` nor `support_bottom_interface_spacing` exists in `tree-support.toml`, and
the planner's own `tree_support_interface_spacing_mm` is never consumed by the renderer.

**F-18 — Bottom-interface (floor) promotion is single-point, not surface-band**
`canonical_ref`: `TreeSupport.cpp::draw_circles` floor pass —
`intersection_ex(comp_poly, band_ex)` against the model's `stTop`/`stBottom` surfaces, plus
`bottom_gap_height`. In-tree tests **one point** (`point_in_any_expoly(collision, node.x, node.y)`) on
layers below and promotes the whole node segment to floor. `support_bottom_z_distance` has no in-tree
counterpart. (The `support_interface_bottom_layers < 0 -> mirror top_n` rule *does* match
`SupportParameters.hpp::number_of_support_interface_bottom_layers` exactly.)

**F-19 — Auto/manual support axis discarded**
`canonical_ref`: `PrintConfig.cpp` `s_keys_map_SupportType` {normal(auto), tree(auto), normal(manual),
tree(manual)}; `PrintConfig.hpp::is_auto`; `SupportMaterial.cpp::detect_overhangs`
(`auto_normal_support = support_type == stNormalAuto`).
`canonical_support_family` maps `tree*`/`hybrid*` -> tree, everything else -> traditional. The **family**
axis is right (and the legacy `hybrid(auto)` migration is handled correctly), but auto/manual is dropped
entirely — `slicer_ir::SupportType` has only `Traditional | Tree`. Canonical manual modes generate support
**only** from enforcers; in-tree the producer emits auto-detected candidates unconditionally with
`enforced: false, blocked: false` hardcoded, and canonical's `auto_normal_support` gate has no counterpart.

**F-20 — AC-6 was amended, in this packet, into a prohibition that forecloses the fix**
`in_tree_symbol`: `assert_no_test_reads_orca_gcode`, `task_163b_disposition`
(`crates/slicer-runtime/tests/integration/support_family_closure.rs`); `packet.spec.md` AC-6.

The narrow rule is **defensible**: `git check-ignore -v` resolves both reference files to `.gitignore:25:tmp/`,
so a test reading them would fail on a clean clone. Three things are not:
1. AC-6 was amended by `289a2056` — *in this packet* — to require that no read of `tmp/SupportTest_*_Orca.gcode`
   **and no Orca-derived constant** appear in any test; `c645ed9a` then implements exactly what `289a2056`
   had just required. The "no Orca-derived constant" clause bars the obvious workaround.
2. The workaround has in-repo precedent: `crates/slicer-runtime/tests/fixtures/multi_color_cube.orca.gcode`
   is **tracked** (76 lines) as a distilled Orca reference. A comparable distillation here would be ~1 KB.
3. The gate's doc comment gives a *policy* ("Orca comparison is a recorded manual inspection") rather than
   the verifiable *fact* (tmp/ is gitignored). Its predecessor was itself vacuous — the disposition comment
   admits "the previous body probed two `tmp/*.gcode` paths and ran an empty `if`". **The remedy for a
   vacuous check was to forbid the check.**

Feasibility is not in doubt. Extracted this session in seconds: tree — 122 `;TYPE:Support`, 2
`Support interface`, 150 distinct Z, 124 Z carrying support; normal — 121 / 3 / 150 / 124. `design.md`
records these **accurately** (independently confirmed), and also records live divergences: PnP tree 123 vs
Orca 122 support blocks, PnP traditional 2 vs Orca 3 interface layers (gap G-18). **None of these numbers
appears in any assertion**, and a future agent who distils them into a test now hard-fails the suite.

**F-21 — Dead config key leaves 19 tests asserting on a value the code never receives**
Detailed under S-2. `support_branch_angle_deg` is set at 19 call sites and read nowhere;
all 19 set 45.0 = `DEFAULT_BRANCH_ANGLE_DEG`.

**F-22 — Weakened assertions (7 sites)**

| Site | Before | After |
|---|---|---|
| `tree_support_family` (runtime) | `paths.len() >= 3` "trunk must have two wall passes plus fill" | `!paths.is_empty()` |
| `tree_support_family` (runtime) | — | `family_id.is_empty() \|\| family_id == "tree"` — **an unattributed entry passes**, the exact failure this packet exists to close |
| `radius_aware_collision` | `local_radius >= 0.39` | `>= 0.3` (comment states measured value is 0.3366 -> ~10% unexplained slack) |
| `tree_family_tdd::distributed_contacts` | `any(r.role == SupportPlanRole::SupportBody)` | `any(!role.regions.is_empty())` |
| `tree_family_tdd::anchored_heights_and_termination` | ditto | ditto |
| `traditional_family_tdd::contact_area_planning` | ditto | ditto (this one added a compensating suite-level check) |
| `traditional_family_tdd::anchored_termination` | ditto | ditto |

The four role-assertion rewrites now pass with *any* role, including one not yet defined. Root cause is F-3.

**F-23 — Regenerated golden contains degenerate output**
96 endpoints, 56 unique — 40 exact duplicates, several on degenerate integer coordinates
(0,0,1.6 / 0,2,1.6 / 2,0,1.6). Duplicate skeleton endpoints indicate coincident or collapsed branch
points. The +/-10% band and 0.5 mm Hausdorff are then measured against this set. No investigation appears
in the range.

### MEDIUM

- **F-24** Per-region rather than whole-lower-layer overhang difference. Canonical subtracts the **union**
  of all lower-layer regions (`lslices`); in-tree diffs region R layer n against region R layer n-1 only.
  A region first appearing at layer k atop a *different* region emits its entire cross-section as contact.
  Multi-material objects get spurious full-area supports. (`SupportMaterial.cpp::detect_overhangs` caller.)
- **F-25** Canonical post-diff steps absent: expand-back
  (`diff(intersection(expand(diff_polygons, lower_layer_offset, SUPPORT_SURFACES_OFFSET_PARAMETERS), layerm_polygons), lower_layer_polygons)`),
  the tiny-spot collapse filter, and `xy_expansion`. Join type is `Miter` where canonical
  `SUPPORT_SURFACES_OFFSET_PARAMETERS` is `jtSquare` — same mismatch in the traditional XY trim.
- **F-26** Zero-threshold semantics wrong. Canonical angle 0 uses `support_threshold_overlap`, not
  "support everything"; in-tree does a plain difference and its comment claims this matches canonical.
  `support_threshold_overlap` is read nowhere. Canonical's `+1` inclusivity and `min(angle, 89)` clamp absent.
- **F-27** Post-union re-validation can drop legitimately merged bodies: same-`body_id` bodies far apart
  union into an envelope exceeding `ROUTING_CELL_SIZE` (104.8576 mm) and are dropped — a cap canonical's
  unbounded `union_` does not impose. The cell label also migrates mid-loop as the envelope grows.
- **F-28** Untagged-origin marshal fallback collapses interface and raft paths to `SupportRole::SupportBody`
  via `..Default::default()`. Latent in production (both shipped renderers call `begin_region`) and G-code
  impact is nil because per-path `ExtrusionRole::SupportInterface` survives, but entry-level attribution is lost.
- **F-29** `erSupportTransition` has no in-tree representation (`ExtrusionEntity.hpp`). Impact bounded — no
  producer of it was found under `Support/` — but canonical treats it distinctly for speed/cooling and tool ordering.
- **F-30** Three config keys are **read but not declared** in `tree-support-planner.toml`
  (`support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `support_type`/`support_family`).
  Verified: `lib.rs` = 1/2/n hits, manifest = 0. `ConfigView::from_declared` drops undeclared keys, so merge
  distance is permanently pinned at 0.8 mm and the cap at 1024 — **the same class of bug as the RC-11 defect
  this packet fixed**. Conversely `support_layer_height_mm` is declared and read nowhere (manifest = 1, lib.rs = 0).
- **F-31** `tree_support_wall_count` manifest range is min 1 / max 10 / default 1 where canonical is
  `[0, 2]` with **0 = auto**; auto is unrepresentable and max is 5x canonical. `from_config` defaults to 2,
  contradicting the manifest's 1.
- **F-32** `support_angle` (Orca's pattern-rotation key) deleted from `tree-support.toml` with the field
  removed but its `#[allow(dead_code)]` left behind, now silently suppressing lints on `support_speed`.
  `scan_fill_region` is hardcoded axis-aligned — pattern rotation is unimplemented with no schema record.
- **F-33** Canonical `smooth_nodes` stage is never called in production. `smooth_branches` is a complete
  port with its own TDD file and **zero** production call sites; canonical calls it unconditionally between
  `drop_nodes` and `draw_circles`.
- **F-34** Top-Z gap placement: canonical places the contact at `layer_nr - 1` (always one layer) plus a
  **virtual** gap node of `height = z_distance_top`; in-tree walks real layer Z, dropping ~2 layers at a
  0.2 mm gap with 0.1 mm layers. `max(z_distance_top, min_layer_height)` and the thin-object early return absent.
- **F-35** RC-15 lattice under-covers the rotated bbox: canonical spans `rotated_dims = (w*cos+h*sin, w*sin+h*cos)/2`
  (~1.33x at 22deg); in-tree derives indices from the **unrotated** bbox, dropping interior contacts near
  bbox corners — the exact failure the function's own doc comment claims to avoid.
- **F-36** Traditional bottom interface uses the whole layer, not canonical's
  `intersection(top, supports_projected)` expanded by one flow width; a column landing partly on model and
  partly on plate marks its entire cross-section BottomInterface.
- **F-37** Interface regularization absent (`closing` + `smooth_outward` in `generate_interface_layers`);
  no base-interface (`num_top_base_interface_layers`) role.
- **F-38** `traditional-support-planner/src/lib.rs` has **no AGPLv3 attribution header** despite containing
  logic its own comments attribute to `generate_base_layers`,
  `bottom_contact_layers_and_layer_support_areas`, and `trim_support_layers_by_object`. Required by
  `docs/ORCASLICER_ATTRIBUTION.md`. (All other ported files in scope carry it correctly.)
- **F-39** Missing `support_bottom_z_distance` (canonical default 0.2) — model-landing columns print flush.
  Pre-existing, not introduced here.
- **F-40** `SupportGridPattern` projection not implemented: canonical prints a grid-snapped
  `expansion_to_slice` area while propagating a smaller `expansion_to_propagate`, and merges nearby columns
  via the grid. In-tree propagates and prints the same unexpanded carry. Documented in-code as a deferral.
- **F-41** Fixtures re-spaced to survive RC-15 dedup (`to_buildplate_tdd` 0.4 -> 4.0 mm;
  `diagnostics_tdd` cap-overflow tile pitch 0.4 -> 2.4 mm). Consequence: **the dense/adjacent-overhang case —
  the realistic trigger for the branch cap and for MST clustering — is no longer exercised anywhere.**
  `single_contact_fixture` shrank 4x4 mm -> 0.2x0.2 mm (400x area) to keep yielding one node.
- **F-42** Deleted coverage: `enforcer_overrides_needs_support_false` (enforcer precedence — zero automated
  coverage remains; registered as G-17), `branching_pattern_present` (>10deg angular variance, the property
  distinguishing tree from traditional), `density_affects_coverage`. Partial compensation exists for the latter two.
- **F-43** `eff6b91a` (render regardless of `needs_support`) is **canonically justified** — canonical has no
  per-region eligibility predicate; the contact-node set *is* the determination. But it masks a real stub:
  `needs_support` is a hardcoded `true` literal at both host marshal sites
  (`crates/slicer-wasm-host/src/marshal/in_.rs`, `crates/slicer-wasm-host/src/host.rs`), never derived from
  `OverhangRegion::needs_support`. Since it is hardcoded true, the commit message's claim that the gate
  "skipped every planned polygon" is not reproducible from the host path as written.
- **F-44** Painted-region `variant_chain` inconsistency: `backfill_active_region_configs` deliberately falls
  back to the smallest matching chain, but the family-assignment lookup in the same commit builds
  `variant_chain: Vec::new()` with `.unwrap_or_else(|| "traditional")`. A painted tree region is routed to
  the tree planner by the executor but recorded as `traditional` in `SupportAnalysisIR.family_assignments`.

### LOW

- **F-45** Non-saturating `max_x - min_x` reintroduced by `ed62090d` after `2afa4cf9` shipped
  `saturating_sub` — debug-build panic on a malformed guest plan with `min_x` near `i64::MIN`.
- **F-46** Citation-style violation introduced by this series: `// wall_count multiplier — fall back to 1
  per OrcaSlicer line 2632` — a bare line-pinned citation with no file and no function, against
  `CLAUDE.md`'s MUST-follow rule. The canonical site is `TreeSupport.cpp::generate_toolpaths`.
- **F-47** `docs/15_config_keys_reference.md` drift (>=8 rows): stale
  `support_filament`/`support_interface_filament`/`support_overhang_angle` rows;
  `support_top_z_distance_mm` shown 0.0 where manifests now say 0.2; `support_layer_height_mm` min 0.05
  where the manifest is now 0.0; a `tree_support_interface_spacing_mm` row nothing declares; no row for the
  newly added `support_object_xy_distance`.
- **F-48** Stale/misplaced doc comments: `tree-support/src/lib.rs` still asserts "not a port of OrcaSlicer's
  TreeSupport ... from-scratch grid-MST design" while carrying the TreeSupport.cpp attribution header (the
  grid-MST filler was deleted in this series); the `clamp_to_avoidance` crate doc still describes the
  pre-fix inverted behaviour; `overhang_plate_fixture`'s doc edit landed on the wrong function; a new test
  in `diagnostics_tdd.rs` was inserted between `small_overhang_fixture`'s doc comment and its `fn`.
- **F-49** Top interface band excludes the plate layer for columns shorter than `support_interface_top_layers`;
  canonical membership depends only on Z distance from the top contact. Edge case only.

---

## 4. What the packet got right

Recording these so the report is not read as uniformly negative — each was verified, not assumed:

- **RC-15 contact sampling (`ad9019ee`) is a faithful port.** All three canonical streams match
  `TreeSupport.cpp::generate_contact_points`: the corner test (`v1.dot(v2) > -0.7`), the EdgeCache arc walk
  at `point_spread`, and the 22deg-rotated interior lattice with
  `step = max(point_spread, max_bridge_length/2)` filtered against the eroded overhang. Dedup cell
  `mm_to_units(base_radius).max(1)+1` matches canonical `pt / (radius_scaled + 1)`.
- **The coordinate-system contract holds everywhere.** Every numeric literal added across all four modules
  and the runtime was swept: all are mm, degrees, dimensionless, or diagnostic codes. No Orca `scale_()`
  value was transplanted unconverted. `DEFAULT_MAX_BRIDGE_LENGTH_MM = 10.0` matches `max_bridge_length`;
  `MIN_BRANCH_RADIUS = 0.4` matches `TreeSupport.hpp`. `clamp_to_avoidance` round-trips correctly through
  `SCALING_FACTOR = UNITS_PER_MM = 10_000`. **No divide-by-100 defect found anywhere.**
- **The avoidance guard inversion was real and is correctly fixed** (`acf9fa1d`), matching
  `TreeSupport.cpp::drop_nodes`'s `!is_inside_ex(avoidance_next, node.position)` sense.
- **The routing-cell fix (`2afa4cf9`) is correct and the old code was genuinely wrong.** Absolute-cell
  containment rejected arbitrarily small bodies for straddling a grid line (528 rejections at gap 0.2 vs 0
  at gap 0.0); the extent test is translation-invariant. Cell indices use `div_euclid`, so there is no
  toward-zero bug at negative coordinates. No grid is materialised, so extent cannot drive allocation.
- **Aggregation is deterministic** — a total sort precedes all iteration; no HashMap is iterated. Goldens
  will not flake from ordering.
- **The interface role crosses WIT losslessly**, discriminated by an explicit guest-supplied bool
  (`push-interface-path: func(path, is-top-interface: bool)`), with exhaustive 4-arm matches and no `_ =>`
  arm at every conversion point. `;TYPE:Support` / `;TYPE:Support interface` match
  `ExtrusionEntity.cpp::role_to_string` **exactly**.
- **`SupportEntry` has no `#[serde(default)]`**, so an old fixture fails loudly rather than deserialising
  with a wrong role.
- **The XY-clearance trim (`3a361521`) matches canonical** `trim_support_layers_by_object` in order and
  algebra, with correct mm units and a default (0.35) matching `PrintConfig.cpp`.
- **Plate-aware bottom-interface suppression is correct**, confirmed against the reference G-code: with
  `support_interface_bottom_layers = 2`, Orca emits zero interface at the bottom of a plate-resting column.
- **The net deletion in `traditional-support-planner` removed a *non-canonical* algorithm** (mesh-facet
  normal threshold) that duplicated and contradicted the host's 2D detection. That deletion moves toward parity.
- **The overhang-angle formula itself is canonical**: `lower_layer_height / tan(threshold)` then
  `difference_ex`, measured from horizontal, using the lower layer's height — matching
  `SupportMaterial.cpp::detect_overhangs`. (It is the config *plumbing* that is broken — F-2.)
- **The regen hatch is not vacuous**, no test gained `#[ignore]`, no `required-features` or
  `#![cfg(feature` blindness exists in any in-scope crate, and the struct-literal churn gate is satisfied.
- **`design.md`'s recorded Orca numbers are accurate** — independently re-extracted and matched exactly,
  including the divergences it self-reports (G-18).

---

## 5. Overall verdict

# DEVIATIONS FOUND

Counts across 9 workers, ~110 raw findings deduplicated to 49 distinct issues (three workers
independently found F-3; two found F-1):

| Verdict | Count |
|---|---|
| PARITY-CONFIRMED | 31 |
| DEVIATION | 46 |
| UNVERIFIED | 3 |

**By severity:** 3 CRITICAL, 20 HIGH, 21 MEDIUM, 5 LOW.

**UNVERIFIED (3):** the branch-count value 8 has no canonical derivation (only a prediction);
`single_contact_fixture`'s pre-224 behaviour could not be established without running the planner;
`SupportGridPattern` divergence magnitude was not measured.

### Assessment

The packet is not a fabrication — the coordinate-system contract holds everywhere, the RC-15 port is
faithful, several fixes (avoidance inversion, routing-cell bounding, XY trim, plate-aware bottom interface)
are genuinely correct, and the golden rename is more honest than what it replaced. The suspicion that the
Orca goldens were deleted to hide a regression is **disproved**: they were never Orca data.

But three findings should block closure:

1. **F-5** — the `slicer-wasm-host` contract suite is **red on HEAD** (3 failures, guests verified fresh).
   The packet was carried through a closure ceremony with a failing suite; all four support module suites
   are green, which is why it went unnoticed.
2. **F-1, F-2, F-3** — three CRITICAL correctness defects, each of which silently produces wrong support
   geometry rather than failing: a second overhang on an object gets no top interface; the user's overhang
   angle is ignored on every slice; branch bodies vanish on every interface layer.
3. **F-4** — the packet's two headline interface invariants are **structurally unfireable**, and F-3 is
   precisely what makes them so. The "12/12 closure" of the interface gauntlet measured nothing. Combined
   with F-21 (19 tests asserting on a dead config key) and F-22 (7 weakened assertions), the green suite
   is a substantially weaker signal than its size suggests.

**F-20 is the finding with the longest half-life.** The narrow no-Orca-read rule is defensible on CI
grounds — `tmp/` is gitignored. But AC-6 was amended *within this packet* to also bar "any Orca-derived
constant", the packet then satisfied the criterion it had just written, and that clause forecloses the
one workaround with in-repo precedent (`multi_color_cube.orca.gcode` is tracked). `design.md` records
accurate Orca numbers *and* live divergences (PnP 123 vs Orca 122 support blocks; PnP 2 vs Orca 3
traditional interface layers), none of which is asserted anywhere — and a future agent who distils them
into a test now hard-fails the suite. For a packet named *orca-closure*, the mechanism that would detect
tomorrow's parity regression has been made a test failure.

---

*Read-only audit. No code, test, golden, or fixture was modified. This file is the only artifact created.*
