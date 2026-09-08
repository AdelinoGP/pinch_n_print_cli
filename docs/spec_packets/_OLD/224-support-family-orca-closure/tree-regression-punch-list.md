# Tree-family regression punch list (packet 224)

Read-only audit worker `w14`. Working tree at `647a7d0a`. No source edits, no commits.
All 10 failures reproduced locally with
`cargo test -p tree-support-planner --tests --no-fail-fast` and
`cargo test -p tree-support --tests --no-fail-fast` (log: `target/test-output.log`).
`--no-fail-fast` is mandatory: without it Cargo stops after `orca_parity_tdd` and
only 2 of the 10 are visible.

## Executive summary

The 8 "introduced" regressions are **not 8 independent defects**. They are three
root causes:

| # | Root cause | Tests |
|---|---|---|
| RC-A | `SupportPlanner::plan_for_object` lost its family-assignment fallback in `9f4540bd`; with an empty `SupportAnalysisView.family_assignments` every region `continue`s and **zero** `SupportPlanEntry` are emitted, silently (no diagnostic). | 5 tests |
| RC-B | `TreeSupport::render_polygon` now renders planned polygons as walls **+ scan fill**; `support_density` is consumed as a fraction but arrives as percent and is clamped by `.min(1.0)` → 100% solid, density-insensitive, all-horizontal paths. | 2 tests |
| RC-C | Self-captured golden baseline drifted 1.3 mm when the renderer/planner geometry changed. | 1 test |

Plus 2 inherited failures with their own causes (RC-D, RC-E).

**RC-A is the highest-value fix: one change clears 5 of 8.**

## Ordering rationale

The prompt asked for "most-likely-fixed-by-the-port first". The honest answer is
that the planned **contact-point-sampling port does not fix most of these** — the
dominant blocker (RC-A) sits *downstream* of contact generation and discards
entries no matter how many contacts exist. The list below is therefore ordered by
**expected clear-rate per unit of work**, which is the ordering a next session
actually wants. The port's contribution is called out per item.

---

## 1. RC-A group — family-assignment gate (5 tests, one fix)

`SupportPlanner::plan_for_object` gates emission on

```rust
let Some(support_family) = support_analysis.family_assignments.iter()
    .find(|a| a.object_id == obj.object_id && a.region_id == *region_id)
    .map(|a| canonical_support_family_alias(Some(&a.family_id)))
else { continue };
```

`9f4540bd` replaced a defaulting `let support_family = ...` with this `let ... else
{ continue }`, and changed `candidate_family`'s return from `String` (with
`.unwrap_or_else(|| "tree".to_string())`) to `Option<String>`. The in-crate
`mod tests` were migrated in the same commit (new `tree_analysis(object_id,
region_ids)` helper); the external `tests/` fixtures were not.
`run_support_geometry` forwards `&SupportAnalysisView::default()` and its
signature has **no** analysis parameter, so `to_buildplate_tdd.rs` cannot be
migrated at all without changing the API it is pinning.

**Verdict for all five: TEST-IS-RIGHT.** A floating plate must be planned for;
canonical `TreeSupport` has no host-side "family assignment" concept at all —
support type is a config (`support_type`), and this planner already reads
`support_family` in `from_config`. Emitting nothing *and* no diagnostic is a
silent-drop bug in its own right.

Recommended fix (production, not fixture): restore the fallback to the module's
own configured `support_family` when no assignment matches, and emit a diagnostic
if the fallback fires. Do **not** migrate the fixtures — that would delete the
only coverage of the no-assignment path.

| # | Test | Failing assertion |
|---|---|---|
| 1.1 | `to_buildplate_tdd::default_config_does_not_reject_to_model_contacts` | `AC-N1: ... Expected non-empty plan, got 0 entries. diagnostics=[]` |
| 1.2 | `to_buildplate_tdd::contact_xy_outside_footprint_sets_to_buildplate_true` | `AC-2: ... entries=0, diagnostics=[]` |
| 1.3 | `tree_family_tdd::disabled_and_declined` | `assertion failed: !disabled.entries().is_empty()` |
| 1.4 | `tree_family_tdd::distributed_contacts` | `planner must emit multiple layers` (`entries().len() >= 2`) |
| 1.5 | `tree_family_tdd::radius_aware_collision` | `non-colliding fixture body should remain emitted` |

Notes per test:

- **1.1 / 1.2** are a matched pair: one contact *inside* the footprint, one
  *outside*. Both return 0 entries, which is itself the proof that this is **not**
  a `to_buildplate` classification bug — a real one would break only one
  direction. `push_contact_with_demand`'s `to_buildplate = !point_in_any_expoly(...)`
  logic is correct as written.
- **1.5**: the code-1203 diagnostics assertion *passes* (the collision checks run
  before the family gate). Only the body-iteration fails, because there are no
  entries to iterate. The avoidance guard is not rejecting the survivor; the
  survivor's entry never exists.
- **1.4** additionally needs candidate-geometry sampling to run: the sampling loop
  is gated by `candidate_family(...) == Some("tree")`, so the 4×4 mm candidate
  `ExPolygon` is never handed to `candidate_contact_points`. Once the gate
  defaults, `candidate_contact_points` (polygon vertices + edge midpoints + 3×3
  bbox grid clipped to the polygon) already spans the corner / contour / interior
  classes the test demands.

**Port expectation:** `PARTIAL` for all five. The port changes *how many and where*
contacts are; it does not touch the gate. **Fix RC-A first, then re-run — some of
these five may already be green and must not be "fixed" twice.**

### Blocking hazard for the port (flagged, please read before porting)

All six planner fixtures (`overhang`, `two_overhangs`, `overhang_plate_fixture`,
`single_contact_fixture`, and the two inline meshes in `to_buildplate_tdd.rs`) are
**flat coplanar horizontal plates at z = 1.8**, with an unreferenced vertex
`[0,0,0]` present only to set `bmin[2]`. They have an **empty cross-section at
every Z**.

The current planner never slices — `detect_overhang_facets` reads
`MeshObjectView.triangles` directly — so the fixtures work today. But the planned
port samples "the per-layer overhang `ExPolygon`". **If that ExPolygon is derived
by slicing the mesh (canonical `curr_layer - offset(prev_layer)`), every one of
these fixtures yields an empty polygon and the port produces zero contacts,
turning 5 green tests red for a reason that looks like the port is wrong.** The
port must project the downward-facing triangles onto the layer plane, or the
fixtures must be rebuilt as closed solids first. Decide this before writing code.

## 2. RC-B — `tree-support` renderer: fill, density, direction (2 tests)

| # | Test | Failing assertion |
|---|---|---|
| 2.1 | `tree_support_tdd::density_affects_coverage` | `higher density should produce more paths: low=23, high=23` |
| 2.2 | `tree_support_tdd::branching_pattern_present` | `all angles are similar: [0.0, 0.0, ... ]` (21 entries) |

`TreeSupport::render_polygon` emits `wall_count` inset loops (via
`host::offset_polygons`) **plus** a `scan_fill_region` infill. `from_config` reads
`support_density` with no `/100.0`, and `render_polygon` computes
`spacing = (line_width / self.density.min(1.0))`.

### 2.1 `density_affects_coverage` — **VERDICT: TEST-IS-WRONG** (for the tree family, in its current form)

(a) Asserts that `support_density = 50` yields more support paths than
`support_density = 10`.
(b) `count_high > count_low` → `low=23, high=23`.
(d) Two defects are entangled here and must be separated:

- **A real production bug that must be fixed regardless of this test.**
  `tree-support.toml` declares `support_density` as `default = 20.0, max = 100.0`,
  `docs/15_config_keys_reference.md` agrees, real configs pass percent
  (`resources/test_config/benchy-tree-support.json` = 40.0), and the sibling
  `TraditionalSupport` converts correctly (`self.density / 100.0`). `TreeSupport`
  does not, and `.min(1.0)` saturates every percent value ≥ 1 to a solid fill.
  This is W3 in `tree-density-diagnosis.md`, already marked BUG and still
  unfixed. Fix it.
- **The test should still not exist in this form.** Canonical
  `tree_supports_generate_paths` renders branch bodies as **hollow concentric
  walls**; `tree-density-diagnosis.md` measures that roughly 60% of PnP's low-Z
  support length is interior fill canonical would not print. Once the hollow-wall
  port lands, path count is driven by `tree_support_wall_count` and is
  **independent of `support_density`** — so fixing the percent bug makes this test
  pass *today* and then breaks it again *after* the correct port. Chasing it green
  now buys a second regression later.

Recommendation: fix the percent→fraction conversion; then **move the
density-affects-coverage assertion to the traditional-support family**, where
`support_density` legitimately governs fill pitch, and replace the tree-side test
with a wall-count assertion. If any tree-branch infill is retained for the base
region, it is interface/base-pattern driven, not `support_density` driven —
confirm against canonical before keeping a density knob on tree branches.
(e) Port expectation: **NO** — planner-side contact sampling has no bearing on the
renderer's fill pitch.

Residual **NEEDS-CANONICAL-CHECK**: whether OrcaSlicer's tree base region is ever
filled (a `with_infill` path exists in the organic-support code). This audit did
not have an OrcaSlicer checkout available. Marked UNKNOWN rather than asserted.

### 2.2 `branching_pattern_present` — **VERDICT: TEST-IS-STALE**

(a) Asserts that at least two emitted support paths differ in direction by >10°,
proving branches rather than parallel lines.
(b) `has_different_angles` false; all 21 angles are exactly `0.0`.
(d) The angle metric is `first_point → last_point` of each path. Wall loops are
closed (`wall[0]` is re-pushed), so first == last, `dx == dy == 0`, and every wall
is discarded by the `dx.abs() > 0.001 || dy.abs() > 0.001` filter. Only
`scan_fill_region`'s horizontal 2-point lines survive — hence uniformly `0.0`.
The test was written against the pre-rewrite architecture, where the **renderer**
built branch polylines (`fill_expolygon_tree`, deleted in `9f4540bd`). Post-224
the branch skeleton belongs to the **planner** (`SupportPlanEntry.skeleton`) and
the renderer only fills/walls planned polygons. Branch-direction variety is no
longer a property of the rendered layer paths, so this assertion cannot be
satisfied by a canonical-correct renderer.
Recommendation: retarget the assertion at `SupportPlanEntry.skeleton` in the
planner crate, or (minimally) make the angle extraction skip closed loops and
assert on wall-loop segment directions.
(e) Port expectation: **NO** — different module, different layer of the stack.

## 3. RC-C — self-captured golden drift (1 test)

### 3.1 `orca_parity_tdd::benchy_orca_parity_within_tolerance` — **VERDICT: TEST-IS-STALE**

(a) Asserts planner branch count within ±10% and skeleton-endpoint Hausdorff
distance ≤ 0.5 mm against a stored baseline.
(b) `AC-6 FAILED: Hausdorff distance 1.2998mm exceeds tolerance 0.5mm.` The branch
count check passed; only geometry moved.
(d) The goldens are explicitly **not** canonical. `resources/golden/benchy_tree_support_orca_branch_count.txt`
and `..._endpoints.txt` both open with
`# Source: Pinch 'n Print self-capture (synthetic overhang fixture, packet 31b)` /
`# Replace with real OrcaSlicer reference data ... before promoting to status: implemented.`
Per `CLAUDE.md` Test Discipline, a self-captured baseline may stay red and must
never be used to justify weakening the canonical implementation. It encodes
pre-rewrite PnP geometry by construction.
(e) Port expectation: **NO — worse.** The port raises contacts from 2 (triangle
centroids) to tens (contour corners + arc walk + interior grid) on this same flat
plate fixture, which will blow the ±10% branch-count check as well.
**Regenerate the goldens once, at the end, after RC-A and the port have both
landed** (`SUPPORT_PLANNER_REGEN_GOLDEN=1`) — not before, or they will be
regenerated twice against two different wrong algorithms.

## 4. Inherited failures

### 4.1 `tree_family_tdd::anchored_heights_and_termination` — **VERDICT: TEST-IS-RIGHT**

(a) Asserts two merged demand candidates produce a plan reaching the plate layer,
where every entry carries an anchor Z, a skeleton, and printable geometry under
some role.
(b) Currently: `assertion failed: !output.entries().is_empty()` (masked by RC-A).
At `5a38fdce`, before RC-A existed, it failed later and differently:
`entry at layer 8 carries no printable geometry under any role: []`.
(d) An emitted `SupportPlanEntry` with zero role regions is a phantom layer —
canonical never plans a support layer it cannot print. The assertion was
deliberately *widened* in packet 224 (the in-file comment explains that roof/floor
carving can leave only `TopInterface`/`BottomInterface`), so it is already as
permissive as canonical requires; an empty `roles: []` is not a legitimate outcome.
(e) Port expectation: **PARTIAL**. RC-A will unmask it, then the original
empty-roles defect at layer 8 resurfaces and needs its own fix. Denser contact
sampling may incidentally populate layer 8, but do not rely on that — treat
"entry emitted with no printable geometry" as a separate bug.

### 4.2 `enforcer_blocker_tdd::default_ineligible_region_generates_zero_support` — **VERDICT: TEST-IS-STALE**

(a) Asserts a region with `needs_support = false` and no enforcer/blocker paint
produces zero support paths.
(b) `assertion left == right failed: needs_support=false with no paint must yield
zero support paths / left: 6 / right: 0`.
(d) Introduced by **`eff6b91a`**, not `9f4540bd`. That commit deliberately removed
```rust
SupportPaintPolicy::DefaultEligible => { if !region.needs_support() { continue; } }
```
so the renderer now honours the plan regardless of the flag — which matches
canonical, where the planner owns eligibility and the toolpath generator prints
what was planned. The 6 paths are 2 wall loops + 4 fill lines over the fixture's
10 mm square (`spacing = 0.4 / 0.2 = 2.0`), because `paint_view_with_annotations`
always attaches a non-declined `SupportPlanIR` entry with `family_id: "tree"`.
The `Blocked` policy is still honoured, so blocker/enforcer precedence is intact.
Recommendation: move the eligibility assertion to the planner (assert no
`SupportPlanEntry` is produced for an ineligible region), and retarget this test
at what the renderer still owns.
**Coverage warning:** its sibling `default_eligible_region_generates_support` now
passes *for the wrong reason* — the flag is ignored in both directions, so neither
test currently proves anything about `needs_support`. Fix them as a pair.
(e) Port expectation: **NO** — renderer-side, unrelated to contact sampling.

## Empty-cross-section trap check

Asked for explicitly. Result: **no test among the 10 is defective by the
`Transform3d::default()` all-zeros route.** The only `ObjectMesh` literal in these
six files (`validation_mesh` in `tree-support-planner/tests/tree_family_tdd.rs`)
sets an explicit identity matrix. `MeshObjectView` carries no transform at all and
the tree planner never reads `ObjectMesh.transform`.

The coplanar-plate hazard *is* present in all six planner fixtures but is currently
latent, because the planner does not slice. See the blocking hazard note under
item 1 — it becomes live the moment the port introduces slicing.

## Verdict roll-up

| # | Test | Verdict | Port fixes? |
|---|---|---|---|
| 1.1 | `to_buildplate_tdd::default_config_does_not_reject_to_model_contacts` | TEST-IS-RIGHT | PARTIAL |
| 1.2 | `to_buildplate_tdd::contact_xy_outside_footprint_sets_to_buildplate_true` | TEST-IS-RIGHT | PARTIAL |
| 1.3 | `tree_family_tdd::disabled_and_declined` | TEST-IS-RIGHT | PARTIAL |
| 1.4 | `tree_family_tdd::distributed_contacts` | TEST-IS-RIGHT | PARTIAL |
| 1.5 | `tree_family_tdd::radius_aware_collision` | TEST-IS-RIGHT | PARTIAL |
| 2.1 | `tree_support_tdd::density_affects_coverage` | TEST-IS-WRONG | NO |
| 2.2 | `tree_support_tdd::branching_pattern_present` | TEST-IS-STALE | NO |
| 3.1 | `orca_parity_tdd::benchy_orca_parity_within_tolerance` | TEST-IS-STALE | NO (regenerate last) |
| 4.1 | `tree_family_tdd::anchored_heights_and_termination` | TEST-IS-RIGHT | PARTIAL |
| 4.2 | `enforcer_blocker_tdd::default_ineligible_region_generates_zero_support` | TEST-IS-STALE | NO |

**Net: 6 tests are right and the code must change; 3 fixtures are stale; 1 test is
wrong for this family. Zero tests require weakening.**
