# Packet 238c Handoff — Tree-Support Behaviour Parity

Snapshot: 2026-08-27 (session 2) · branch `parity/support-features` · HEAD `47559101` (238b close) ·
80 dirty paths in the working tree (238c implementation + both sessions' planner fixes,
all uncommitted). Re-derive any count/SHA below at point of use — they were true when
written.

## Mission

Close the visual-defect backlog blocking 238c approval: tree-tip geometry parity against
the Orca reference. Comparison fixture: `SupportTest.stl` sliced with
`tmp/support-family-config-tree-matched.json`, rendered via `pnp_cli visual-debug`,
compared layer-by-layer against `tmp/SupportTest_Tree_Orca.gcode` (user-exported Orca
reference; its embedded config header matches ours: `branch_diameter=5`, `branch_distance=5`,
`branch_diameter_angle=5`, top=2, z-distance 0.2, line_width 105% → 0.42).

Binding scope notes from the human approver: the rectangle on the left of every render is
the MODEL, not support — out of scope. Packet 241 owns AGG rasterization; 240 owns raft.

## The canonical-verified radius model (the session's core finding)

All citations verified 2026-08-27 against `OrcaSlicerDocumented/src/libslic3r/Support/`
by symbol grep. Re-verify narrowly (by symbol, never line number) before relying on them.

**Session-2 scope note:** items 1–4 below were verified against the *old* engine
(`TreeSupport.cpp` — `drop_nodes`/`draw_circles`/`contact_nodes`). The G-code reference
was produced by **OrcaSlicer 2.4.1**, whose organic engine (`TreeSupport3D` +
`TreeSupportMeshGroupSettings` in `TreeSupportCommon.hpp`) reads the `_organic` config
keys and uses a different radius model (`TreeSupportSettings::getRadius`: tip ramp
`min_radius → branch_radius` over `tip_layers`, then `tan(diameter_angle)·layer_height`
growth). With the organic values now in the config, the old-engine model reproduces the
reference's measured behaviour on this fixture (see Verified state), but exact tip-size
parity needs the ramp (Remaining delta 4).

1. **The port's old primary radius path mirrored canonical DEAD code.**
   `tapered_radius` (port) mirrors `TreeSupport::calc_branch_radius(base_radius,
   layers_to_top, tip_layers, diameter_angle_scale_factor)` — the layers-based overload,
   which has no call sites in canonical `TreeSupport.cpp`. The port recomputed it per
   layer at every emit site; that is the origin of both the original 0.4mm tips and,
   once a per-site raise was bolted on, the 24mm merged serpentines.

2. **Canonical's live model is inheritance + linear growth** (`SupportNode` constructor,
   `TreeSupport.hpp`):
   - contact node: `radius = clamp(overhang_bbox_radius, MIN_BRANCH_RADIUS=0.4,
     base_radius=2.5)`, `dist_mm_to_top = 0`;
   - child: `dist_mm_to_top = parent.dist_mm_to_top + parent.height`;
     `radius = parent.radius + (dist_mm_to_top - parent.dist_mm_to_top) *
     diameter_angle_scale_factor` (tan(5°)/mm growth from the CONTACT's radius);
   - STUDIO-7883 never-shrink clamp on the normal-child path only:
     `next_node->radius = max(node.radius, min(next_node->radius, dist_to_outer))`
     where `dist_to_outer` is clearance to the collision boundary
     (`projection_onto(get_collision(0, layer))`). Branch-A merge creation has no clamp.
   - `draw_circles` draws at `node.radius` (`scale = node.radius / branch_radius`).

3. **The G-13 raise is fallback-only.** The mm-to-top overload
   `calc_branch_radius(base_radius, mm_to_top, angle, use_min_distance)` ends with
   `if (support_interface_top_layers > 0) radius = max(radius, base_radius)` — but it is
   reached only via `get_radius` when `node->radius == 0` (parentless chains; radius is
   otherwise cached before first use in `get_max_move_dist`). The earlier "apply the raise
   at every emit site" reading of design.md §G-13 is a mis-generalization. The packet's
   AC-4 text stays satisfied because the raise exists in the mm-to-top helper with
   `support_interface_top_layers = 0 ⇒ unchanged`.

4. **Seeding** (`generate_contact_points`): `point_spread = tree_support_branch_distance`;
   `sample_step = max(point_spread, max_bridge_length / 2)` — **no halving**. Contacts per
   overhang part: convex corners (`v1·v2 > -0.7`), contour walk every `point_spread` mm,
   inset inner grid (`offset(overhang, -radius)` membership), hash-cell dedup.
   Per-part radius = `clamp(bbox_radius, 0.4, 2.5)`. The port already had all four
   mechanisms + per-part radius; only the two `* 0.5` halvings were wrong.

5. **Tip carve**: canonical `draw_circles` diffs each drawn circle against the layer
   collision (`avoid_object_remove_extra_small_parts(ExPolygon(circle),
   get_collision(is_sharp_tail && distance_to_top <= 0))`) — this is why Orca's tips are
   model-trimmed crescents, not closed rings.

## What changed this session (all uncommitted)

`modules/core-modules/tree-support-planner/src/lib.rs`:
- `run_support_geometry`: child `dist_mm_to_top` accumulation; children grow from the
  stored parent radius at `tan_diameter_angle`; STUDIO-7883 clamp (normal path only);
  move + emit passes consume the **stored node radius** (no per-layer taper recompute,
  no `interface_adjusted_radius`/`interface_band_raise` wrappers).
- Raise moved into the mm-based `calc_radius` (canonical's home). `interface_band_raise`
  and its test deleted. `interface_adjusted_radius` + `tapered_radius` kept, tests pinned.
- Avoidance bucket keyed by each node's prospective stored radius.
- Both `sample_step * 0.5` halvings removed (canonical spacing restored).
- `build_roles`: per-footprint collision carve before role union; largest-fragment
  selection per footprint; post-simplification cleanup preserves selected components.
- `MAX_BRANCH_RADIUS_MM` 6.0 → 10.0 (closes open AC-3).

Session 2 (2026-08-27, intermediate-layer segmentation):
- **Avoidance ladder now keyed by `calc_radius(dist_mm_to_top + height_next)`** at the
  F-13 move pass and the branch-A push-out — canonical `drop_nodes` queries the
  avoidance/collision ladders with the global base-radius taper while the child's
  *stored* radius stays the ctor inheritance (verified against
  `OrcaSlicerDocumented/.../TreeSupport.cpp` `drop_nodes` + `calc_radius`). Byte-inert
  on this fixture alone, but canonical-correct.
- **Same-layer swept-capsule fusion removed from the emitted roles** (`build_roles` /
  `structural_body_regions`): the capsules were a port addition ("this port's addition",
  per the old comment); both canonical engines emit per-node cross-sections only.
  Distinct-point MST edges are now skeleton-only; degenerate per-node disc fallbacks
  still emit. Skeleton contract unchanged (renderer uses it only for
  `skeleton_wall_count`). Tests updated to pin the canonical behaviour
  (`structural_regions_exclude_mst_edges_but_keep_node_fallbacks`).
- **Config corrected to the organic engine's effective values**
  (`tmp/support-family-config-tree-matched.json`): branch_diameter 5→2, branch_distance
  5→1, branch_angle 45→40. The reference was sliced by **OrcaSlicer 2.4.1**, whose
  organic engine (`TreeSupportMeshGroupSettings`, `TreeSupportCommon.hpp`) reads the
  `_organic` keys (`branch_diameter_organic=2` → branch_radius 1.0,
  `branch_distance_organic=1`, `branch_angle_organic=40`, `tree_support_tip_diameter=0.8`
  → min_radius 0.4); the non-organic `tree_support_branch_diameter=5` /
  `branch_distance=5` keys the earlier session matched are **not read** by the engine
  that produced the reference. Our port reads the non-organic key names, so the file now
  carries organic-equivalent values under those names.

Tests: `tests/wall_clearance_tdd.rs` (`avoidance_clearance_is_keyed_by_each_nodes_stored_radius`
fixture reworked to compare plate-layer descendants — contact layers had unequal surviving
seed counts), `tests/tree_family_tdd.rs` (band-raise test removed, `calc_radius` raise
coverage added), `tests/structural_*` updated to pin per-node role regions (session 2).
Golden: `resources/golden/benchy_tree_support_regression_{branch_count,endpoints}.txt`
regenerated in session 1 — intentional E3-classified drift (7 branches, 154 endpoints);
regen env var `SUPPORT_PLANNER_REGEN_GOLDEN=1` (see `orca_parity_tdd.rs`). Session 2's
capsule removal did NOT drift the goldens (skeleton topology unchanged).

Earlier in the session (pre-radius-model, still in the tree): removed `drop`-filter at
`layer_records` active-node collection in `run_support_geometry` — **now re-verified
(session 2): CANONICAL-CORRECT.** Canonical `draw_circles` iterates
`contact_nodes[layer_nr]` including merge-invalid (`valid=false`) nodes; only
`is_processed` nodes are erased (matching our `retain(!is_processed)`). The pending
re-verification item is closed.

## Verified state (2026-08-27, session 2 close)

- `cargo test -p tree-support-planner`: 107 passed / 0 failed (11 binaries).
- `cargo test -p tree-support --test tree_support_tdd`: 18 passed.
- `cargo clippy -p tree-support-planner --all-targets -- -D warnings`: clean.
- `cargo xtask check-literals`: clean.
- `cargo xtask build-guests --check`: exit 0.
- Current slice `tmp/support_test_tree_238c_v11.gcode` (config-corrected, capsule-free;
  byte-identical to v10). 124 `;TYPE:Support` blocks; interface blocks unchanged.

Measured per-path bbox-span harness — **calibrated against this handoff's baseline:
6/6 path counts exact, 5/6 max spans exact** (the old "Z16.2 ≤11.92" span was actually
the Z16.4 one; the harness splits at non-extruding moves, counts paths with ≥2 points,
drops single-point dots). Script: `tmp/measure_paths.py`.

```text
# v11 (session 2 close) vs Orca:
#   Z13.2  ours  6 paths ≤3.74 | Orca 10 paths ≤7.58
#   Z16.2  ours  8 paths ≤3.21 | Orca 17 paths ≤6.09
#   Z16.4  ours  8 paths ≤3.17 | Orca 16 paths ≤6.19
#   Z24.4  ours 73 paths ≤1.81 | Orca 73 paths ≤2.67
# (v7 session-1 baseline was: 3/6.75, 3/11.67, 7/18.97)
```

ACCEPTANCE (session 2): met. Z16.2 ≥8 discrete paths ≤7mm ✓ (8/3.21). Z24.4 visually
clustered with solid filled tips ✓ (73-tip dense field, count matches Orca exactly).
Planner + tdd suites green ✓. build-guests --check exit 0 ✓. Refreshed bundle:
`tmp/vd-238c/user-ours-v11/` (request `tmp/vd-238c/user-ours-request.json` → v11) —
l80 shows 8 discrete tips (was 2 fused multi-lobe clusters), l121 a dense tip field,
l65 6 discrete tips; compare against `tmp/vd-238c/user-ref/`.

## Session 3 (2026-08-27): per-layer cross-section union

**Symptom (human-reported, side-view preview):** branch "stepping" between layers and
overlapping perimeters printed through the inside of support branches, most visible at
z 11–13, 17–18, 19–20.

**Root cause:** `build_roles` (`modules/core-modules/tree-support-planner/src/lib.rs`)
emitted each node's carved cross-section as its own role region and ran the final
collision gate one region at a time, explicitly *not* unioning adjacent cross-sections
(a session-2 over-correction that went past the capsule removal). Canonical
`draw_circles` appends each carved circle into `base_areas` / `roof_areas` and then
runs the collection through Clipper boolean ops — `diff_ex(base_areas, roofs)`,
`intersection_ex(base_areas, m_machine_border)`, and `diff_clipped(closing_ex(...))`
for the interfaces — every one of which returns non-overlapping `ExPolygons`. Adjacent
node circles therefore come out **fused** in canonical. Ours printed one full wall loop
per circle, so a fused branch pair carried a duplicate perimeter through its interior,
and the branch silhouette popped between layers as neighbouring circles drifted in and
out of contact.

**Fix:** `union_expolys` on the carved regions before role simplification, and the final
collision gate is now canonical's set-wide `Difference` instead of a per-region loop.
Per-circle carve + largest-fragment selection (canonical
`avoid_object_remove_extra_small_parts`, verified: it keeps only the max-area fragment)
is unchanged and still runs *before* the union.

**Regression test:** `build_roles_merges_overlapping_node_cross_sections_into_one_outline`
(same file) — two 1.0mm-radius node circles 1.0mm apart must produce exactly one body
region. Red before the fix (2 regions), green after.

**Measured (v11 → v12, `tmp/measure_paths.py`, vs `tmp/SupportTest_Tree_Orca.gcode`):**

```text
#           ours v11        ours v12        Orca
#   Z13.2   6 paths ≤3.74   4 paths ≤5.44   10 paths ≤7.58
#   Z16.2   8 paths ≤3.21   6 paths ≤4.78   17 paths ≤6.09
#   Z16.4   8 paths ≤3.17   6 paths ≤4.91   16 paths ≤6.19
#   Z24.4  73 paths ≤1.81  12 paths ≤19.28  73 paths ≤2.67
```

Layer 94 (z19.0) render `tmp/vd-step/v12/` now shows fused peanut outlines matching
`tmp/vd-step/ref/`; `tmp/vd-step/ours/` (v11) shows the same nodes as separate crossing
loops. Current slice: `tmp/support_test_tree_238c_v12.gcode`. Gates: planner suite
108 passed / 0 failed, `tree-support --test tree_support_tdd` 18 passed, clippy clean,
`check-literals` clean, `build-guests --check` exit 0, `cargo check --workspace
--all-targets` clean. Goldens did not drift.

**Delta the union exposed (pre-existing, NOT caused by the fix):** at z24.4 the contact
seeding is border-heavy. Measured nearest-neighbour distance between tip centroids:
ours median 0.52mm (min 0.00) against a ~1.74mm tip diameter — i.e. the contour-walk
tips already overlapped in v11 and were merely being drawn as separate loops; Orca's
median is 1.83mm with the same 73-tip count and a *uniform* interior distribution
(compare `tmp/vd-step/v12top/` against `tmp/vd-step/reftop/`). The union now renders the
border chain as one 19.28mm outline. This is the contact-distribution half of remaining
deltas 4–6 below, not a new defect.

## Session 3b: config/engine mismatch (REAL, but NOT the stepping fix)

> **Corrected in session 3c by human verification.** The config change below fixes the
> branch *structure* (region counts and extrusion mass converge on Orca Strong) but the
> stepping is still present in `tmp/support_test_tree_strong.gcode`. Do not read this
> section as the stepping fix. It stands only as a fixture/config correction.

**Human evidence that cracked it:** Orca's Tree Slim and Tree Strong do not show the
stepping. Verified structurally: `TreeSupport::generate()` forks to
`generate_tree_support_3D` **only** for `smsTreeOrganic`; Slim/Strong/Hybrid all run the
`TreeSupport.cpp` engine this port implements. So the old engine demonstrably produces
smooth branches and the stepping could not be blamed on an engine-model gap.

**Correction to the session-2 config change.** Session 2 correctly determined the
reference is organic (`support_style = default` + `tree(auto)` →
`smsTreeOrganic`, `SupportParameters.hpp`) and therefore rewrote
`tmp/support-family-config-tree-matched.json` to organic-equivalent values under the
non-organic key names. But **this port implements the OLD engine**, which reads those
non-organic keys literally. The result was the old engine driven by organic parameters:

| key | Orca Slim/Strong | our "matched" config | effect |
|---|---|---|---|
| `tree_support_branch_diameter` | 5 | 2.0 | branch radius 2.5x too small |
| `tree_support_branch_distance` | 5 | 1.0 | contact seeding 5x too dense |
| `tree_support_branch_angle`    | 45 | 40 | — |

Many thin, densely-seeded branches that never fuse into a trunk. A branch only ~1 bead
wide, leaning up to `max_move_dist` per layer, terraces; a fused trunk does not.

**Evidence (no code change, config only — `tmp/support-family-config-tree-strong.json`,
slice `tmp/support_test_tree_strong.gcode`):**

```text
#            ours(organic params)   ours(Strong params)   Orca Strong
#   Z12.0    3 regions,  42.8mm     1 region,  31.6mm     1 region,  76.5mm
#   Z15.0    5 regions,  56.9mm     2 regions, 91.0mm     3 regions, 143.2mm
#   Z19.0    8 regions,  80.5mm     2 regions,157.3mm     2 regions, 151.1mm
```

Envelope-jump layers >0.25mm over Z[2,24]: ours-Strong-params 11/110, Orca Strong 13/110.
Z19 render `tmp/vd-step/ours-strong19/` shows two fused double-walled masses, matching
`tmp/vd-step/strong19/`.

**Metrics that did NOT discriminate** (recorded so the next session does not re-derive
them): per-layer lateral move (ours p90 0.166 sits between Slim 0.187 and Strong 0.287 —
there is no movement bug; the earlier "ours 0.122 vs 0.048" was a cross-engine comparison
against organic and is misleading), global envelope jumps, local outline overhang, and
cross-section circularity. All four rank the port at or better than Slim/Strong.

**Open product question:** `TreeSupportStyle::from_config` maps `support_style = default`
to `Default` (old engine); canonical maps `default` + `tree(auto)` to **organic**. For the
default tree config this port and canonical run different engines by construction. Either
ship non-organic defaults for the old engine, or implement the organic engine. Not decided.

**Same-engine references now in tree** (from the human): `tmp/SupportTest_Tree_Orca_Strong.gcode`,
`tmp/SupportTest_Tree_Orca_Slim.gcode`; the original organic export is renamed
`tmp/SupportTest_Tree_Orca_Organic.gcode`. Harnesses: `tmp/envelope.py`,
`tmp/move_detect.py`, `tmp/overhang_step.py`, `tmp/fill_density.py` (its floodfill leaks
through open loops — printed area is trustworthy, enclosed is not).

## Session 3c: branch "stepping" localized to a per-layer MOVEMENT FREEZE (superseded)

**Status: superseded by session 3d — the freeze framing was wrong; the probe measured 0 frozen move-pass transitions in the stepping window. Root cause and fix in session 3d.** The human confirmed the union fix (session 3) is correct
and that stepping persists in `tmp/support_test_tree_strong.gcode`. Human-reported
location: **Z 9.20mm to 13mm**, trunk surface.

### The measurement that finally localized it

Outer-outline bbox span, layer by layer, in the reported window
(`tmp/support_test_tree_strong.gcode` vs `tmp/SupportTest_Tree_Orca_Strong.gcode`):

```text
# Orca Strong deltas: +0.30 +0.29 +0.30 +0.29 +0.29 +0.28 ... then steady +0.13..+0.19
# OURS deltas:        +0.31 +0.26 +0.04 +0.36 +0.31 +0.06 +0.07 +0.32 +0.10 +0.57
#                     +0.46 +0.04 +0.10 +0.07 +0.39 +0.78
```

Growth stalls for 2-3 layers, then jumps 0.3-0.8mm. That is the staircase.

### Mechanism (measured)

Per-layer node movement, matched by nearest centroid, Z[6,22]:

```text
#                        ~0 (<0.02mm)   0.02-0.11mm   >0.11mm
#   ours               38/121  (31%)     69  (57%)    14 (12%)
#   Orca Strong         5/169  ( 3%)    123  (73%)    41 (24%)
#   Orca Slim          18/402  ( 4%)    211  (52%)   173 (43%)
```

**31% of our node transitions do not move at all vs 3-4% in Orca.** Branches freeze, then
jump a full `get_max_move_dist` step.

In the F-13 move pass (`modules/core-modules/tree-support-planner/src/lib.rs`,
`run_support_geometry`) a node only freezes when BOTH terms vanish:
`direction_to_outer == (0,0)` AND `move_to_neighbor_center == (0,0)`. For a trunk ~12mm
clear of the model wall `direction_to_outer` is legitimately zero, so **every frozen layer
is a layer where the neighbour term collapsed.** Canonical zeroes it far less often.

### Three candidates, ranked (untested)

1. **MST adjacency too sparse** - if active nodes have no surviving MST edge,
   `neighbours_of[i]` is empty and the term is zero by construction. Canonical's MST
   connects all nodes within a part (`nodes_this_part`).
2. **`is_line_cut_by_contour` over-triggering** in the neighbour filter - it discards each
   candidate neighbour; a spurious true (e.g. wrong outline set) zeroes the term.
3. **The `neighbours.len() == 1 && first_d2 >= max_move_dist^2` gate** - canonical
   excludes a lone about-to-collapse neighbour; a unit/sign error in `first_d2` would
   exclude far more than canonical does.

**Proposed next probe:** emit per-layer counts of each zero-condition via the existing
`push_diagnostic` channel (no new plumbing), slice, and read the counts off stderr. That
separates the three without guessing.

### Ruled out (do NOT re-derive - all measured against SAME-ENGINE Slim/Strong)

Ours scores equal or better than Orca Slim/Strong on every one of these, so none is the
stepping signal:

| dimension | ours | Strong | Slim |
|---|---|---|---|
| global envelope jump (layers >0.25mm) | 11/110 | 13/110 | 8/110 |
| local outward step, filled masks (>0.35mm) | 24/80 | 26/80 | - |
| per-layer lateral move p90 | 0.166 | 0.287 | 0.187 |
| surface ribbing RMS residual | 0.104mm | 0.664mm | 0.220mm |
| cross-section circularity (closed loops only) | 0.962 | 0.453 | 0.982 |
| per-layer flow E/mm, `;HEIGHT`, dZ | 0 variance, 0.2 exact | - | - |

Also ruled out: `SQUARE_SUPPORT` (`avg_node_per_layer > 200` gate and `contact_stats` both
match canonical); gcode-writer/path-optimization decimation (none exists in our pipeline);
`smooth_nodes` (100 Jacobi iterations, fixed head, `need_extra_wall` predicate all match);
`move_out_expolys` (returns false and leaves the point untouched when outside, equivalent
to canonical's explicit `else { direction_to_outer = 0 }`).

**Methodology warning that cost this session a lot of time:** Orca emits tree support walls
as *anchored polylines*, not closed loops. Any metric that treats a support path as a
polygon (point-in-polygon, shoelace, circularity, corner-seeded floodfill) is INVALID on
Orca files. Use rasterized masks + `scipy.ndimage.binary_fill_holes` instead. The earlier
"ours moves 0.122 vs Orca 0.048 mm/layer" claim was a cross-engine comparison against the
organic export and is wrong; against same-engine Slim/Strong there is no movement-rate bug.

### Separate real gap found (not the stepping cause)

Our emitted support outlines carry **32-44 vertices** where Orca Strong carries **112-173**
on a comparable perimeter (~0.85mm vs ~0.35mm per segment). No decimation exists in our
config (`resolution` unset; Orca uses 0.001) or in `path-optimization-default` /
`machine-gcode-emit`, so the coarseness originates in the planner's emitted polygons.
Suspect `BRANCH_CIRCLE_SEGMENTS = 16` in `swept_region` (the degenerate per-node disc
fallback) versus `CIRCLE_RESOLUTION_FINE = 100` used by `node_ellipse`. Unverified.

Also: Orca uses `support_line_width = 80%` (0.32mm beads); ours emits 0.42mm - our support
E/mm is 26% higher. Appearance only, not stepping.

### Artifacts (session 3c)

- Same-engine references (from the human): `tmp/SupportTest_Tree_Orca_Strong.gcode`,
  `tmp/SupportTest_Tree_Orca_Slim.gcode`; original organic export renamed
  `tmp/SupportTest_Tree_Orca_Organic.gcode`.
- Our slices: `tmp/support_test_tree_238c_v12.gcode` (union fix, organic-valued config),
  `tmp/support_test_tree_strong.gcode` (union fix + old-engine params, config
  `tmp/support-family-config-tree-strong.json`) - **the current reproduction**.
- Harnesses (all take `<gcode> <zmin> <zmax>`): `tmp/envelope.py`, `tmp/move_detect.py`,
  `tmp/ledge_detect.py`, `tmp/overhang_step.py`, `tmp/layer_step.py` (mask-based, the only
  one valid on Orca files), `tmp/ribbing.py`, `tmp/circularity.py`, `tmp/fill_density.py`
  (floodfill leaks through open loops - printed area trustworthy, enclosed is NOT),
  `tmp/branch_profile.py` (shared parser), `tmp/side_view.py`, `tmp/beadstack.py`.

## Session 3d (2026-08-27): stepping ROOT-CAUSED and FIXED — merge-absorbed nodes were never smoothed

**Status: fixed, pending human visual confirmation of `tmp/support_test_tree_strong_v13.gcode`.**

### The probe that killed the freeze hypothesis

Per-layer counters on every zero-condition in the F-13 move pass (session 3c's three
ranked candidates), emitted via `slicer_sdk::host::log_warn` (note: `push_diagnostic`
output is drained into host audits and never reaches stderr — use `log_warn` for probe
lines): in Z 9.2–22 **frozen = 0 on every layer**; every node passed the neighbour gate
and moved a full step (`gate_open == nodes`, `no_neigh/lone_close/conv_empty/far/cut` all
0). The only frozen node in Z[6,22] is a single trunk below Z 9.0 with no MST neighbour —
legitimately plumb. The "31% ~zero G-code centroid transitions" was an emit-side artifact,
not planner kinematics. None of session 3c's three candidates was the cause.

### Actual root cause (measured, then verified against canonical)

Span probes at three pipeline stages localized the terrace: raw drop-pass node span grows
steadily; the **smoothed** node span stalls 2–4 layers then jumps; emitted regions track
the smoothed nodes (≤0.1mm). Per-layer extreme-node identity then showed every jump layer's
extreme node was `is_processed=false, valid=false` — a **merge-absorbed node pinned at its
raw drop-pass position**, sticking 0.6–0.7mm past the smoothed column, while stall layers'
extreme node was a smoothed trunk node crawling.

Canonical `SupportNode` ctor (`TreeSupport.hpp`) runs
`for (auto& neighbor : parent->merged_neighbours) { neighbor->child = this;
parents.push_back(neighbor); }` — so in canonical every merge-absorbed node gains a
`child` (the surviving column's descendant), becomes a fixed-head chain **interior** node
in `smooth_nodes`, and is pulled onto the smoothed column. Our `create_node`
(`modules/core-modules/tree-support-planner/src/lib.rs`) wired only `parent.child` /
`parents = [parent]`, so absorbed nodes started their own chain with **no fixed head**,
stayed pinned (`branch[0]`), and popped the silhouette outward on every merge layer.

### Fix (canonical-faithful, three parts, same file)

1. `create_node`: the canonical ctor loop — every `parent.merged_neighbours` entry gets
   `child = new node` and joins the new node's `parents`.
2. F-11 Branch A: the faded twin is pushed into `parent_id.merged_neighbours` **before**
   `create_node` (canonical `node_parent->merged_neighbours.push_front(...)`), replacing
   the previous manual `parents.push` / `child` wiring at the call site.
3. F-11 Branch B absorb: the absorbed node's own `merged_neighbours` are spliced into the
   keeper's list (canonical `node.merged_neighbours.insert(end, ...)`), so `child`
   reassignment reaches transitively-absorbed nodes.

Side effect (canonical-correct): `parents.len() > 1` in `smooth_nodes` can now be true, so
merge nodes can earn `need_extra_wall` via that clause, as canonical intends.

**Regression tests (red before, green after, same file):**
`create_node_wires_merged_neighbours_child_and_parents` (ctor seam) and
`smooth_nodes_pulls_merge_absorbed_node_toward_the_column` (user-visible seam: absorbed
node must be smoothed interior and pulled toward the surviving column).

### Measured (Z 9.2–13.6 outer-span deltas, `tmp/support_test_tree_strong_v13.gcode`)

```text
# pre-fix dx:  +0.09 +0.10 +0.02 +0.45 +0.27 +0.35 +0.13 +0.31 +0.05 +0.63 +0.46
#              +0.04 +0.10 +0.08 +0.39 +0.77 ...
# post-fix dx: +0.09 +0.10 +0.10 +0.15 +0.14 +0.19 +0.16 +0.17 +0.15 +0.21 +0.27
#              +0.21 +0.22 +0.13 +0.22 +0.25 +0.24 +0.26 +0.26 +0.25 +0.24 +0.23
# Orca Strong: +0.06 +0.06 +0.09 +0.09 +0.09 +0.09 -0.00 +0.07 +0.16 +0.14 +0.14
#              +0.13 +0.18 +0.16 +0.16 +0.20 +0.19 +0.18 +0.18 +0.18 +0.17 +0.17
```

Also: `tmp/layer_step.py 6 22` max ledge 0.566 (pre-fix 0.825), layers>0.35mm 13 (pre-fix
24, Orca Strong 26); `tmp/move_detect.py 6 22` max centroid jump 0.091 (pre-fix 0.360).

**Gates (all green, post-instrumentation-removal):** planner suite 110 passed
(108 + 2 new), `tree-support --test tree_support_tdd` 18, clippy `-D warnings` clean,
`check-literals` clean, `build-guests --check` exit 0, `slicer-runtime --test integration
-- support` 39 passed. All `[DBG-238C-*]` instrumentation removed (grep clean).

### Artifacts (session 3d)

- Fixed slice: `tmp/support_test_tree_strong_v13.gcode` (config
  `tmp/support-family-config-tree-strong.json`) — **awaiting human visual check**.
- Pre-fix reproduction retained: `tmp/support_test_tree_strong.gcode`.

### Still open

- ~~The session-3b product question~~ **RESOLVED (session 3e, human-decided via
  grilling, 2026-08-27):** canonically-organic styles (`default`, `organic`, `grid`,
  `snug` on a tree family) alias to **Strong** in `TreeSupportStyle::from_config`;
  explicit `organic` gets a once-per-slice code-1005 Warn, plain `default` is silent;
  non-tree families resolve `Default`. `style_movement_for`'s live Strong composition
  was removed (canonical's `is_strong` block in `drop_nodes` is dead code — its result
  is unconditionally overwritten by the `normal(direction_to_outer)` chain), so Strong
  now differs from Default only in the neighbour-sum weighting. DEV-156 filed (renumbered from DEV-150 in the 2026-08-28 origin/master merge); the
  organic engine port (`TreeSupport3D.cpp`) is queued as row 7 (TASK-441) in
  `docs/specs/support-generation-remediation-plan.md`. Contract suite
  `tests/tree_style_styles_tdd.rs` rewritten canonical (E3: intentional product
  decision + canonical dead-code correction).
- Session 3c's "separate real gap" items (outline vertex coarseness suspect
  `BRANCH_CIRCLE_SEGMENTS = 16`; `support_line_width` 80% vs ours 0.42) remain unverified
  and untouched.
- Noted, not changed (ours-vs-canonical delta, deliberate-looking): our Branch B absorb
  maxes `distance_to_top` and merges roof counters; canonical Branch B does neither (only
  the ePolygon merge branch does). Also ours does not max `dist_mm_to_top` on absorb where
  canonical's ePolygon branch does. Left as-is; flag if mid-height radius deltas matter.

## Remaining deltas, prioritized (session 2 close)

1. ~~Intermediate-layer segmentation~~ **RESOLVED (session 2)**: organic config values +
   avoidance-ladder keying + same-layer capsule removal. Z16.2 now 8 paths ≤3.21
   (Orca 17 ≤6.09); l80's tip field is discrete like the reference.
2. ~~Path fragmentation at Z24.4~~ **RESOLVED (session 2)**: the 19.28mm tangent-joined
   chain is gone; ours emits 73 fragments ≤1.81 (Orca 73 ≤2.67 — count matches).
3. ~~Re-verify the removed `drop`-filter~~ **CLOSED (session 2)**: canonical-correct
   (see "What changed" above).
4. **Tip-radius ramp (new-engine `getRadius`)**: the organic engine ramps tip radius
   `min_radius(0.4) → branch_radius(1.0)` over `tip_layers` (≈5 here), then grows
   `tan(5°)·layer_height`/layer (`TreeSupportSettings::getRadius`, `TreeSupportCommon.hpp`).
   Our port grows from the contact radius (bbox-clamped) with no ramp. Measured effect is
   small on this fixture (Z24.4 ours ≤1.81 vs Orca ≤2.67 — ours slightly thinner) but
   implementing the ramp would align top-layer tip sizes exactly. The port also does not
   read `tree_support_tip_diameter` at all.
5. **Organic key selection**: the port reads `tree_support_branch_{diameter,distance}` /
   `branch_angle` names; the organic engine reads the `_organic` variants. The config file
   now carries organic values under non-organic names (works, but fragile). Consider
   having the planner select the organic keys for `tree(auto)` or exposing the organic
   keys in the module manifest schema.
6. **Column-collapse curve**: ours 68 columns at the contact layer → 8 by z16.2; Orca
   ~17 by z16.2. The old-engine convergence gate (`max_converge_distance` =
   `tan(angle)·(print_z − DO_NOT_MOVE_UNDER) + bottom_radius`, `drop_nodes`) admits every
   neighbor at organic 1mm seeding; the organic engine merges via influence-area overlap.
   A gate at cross-section overlap (d ≤ r_a + r_b) would be the organic-faithful shape.
   Not needed for the acceptance; needed for closer mid-height counts (8 vs 17).
7. **Interface block Z placement**: ours Z24.6+Z24.8, Orca Z24.6+Z24.6 (deferred, unprompted).
8. **Pre-existing, unrelated**: repeated `ERR_MALFORMED_LAYER_MARKER` (code 12) warnings
   from `machine-gcode-emit` on every slice (~108/slice; session 2 confirmed count) —
   confirm out of packet scope, file if needed.

## Artifacts

- G-code iterations: `tmp/support_test_tree_238c{,_v2,_v3,_v4,_v5,_v6,_v7}.gcode`
  (session 1); `tmp/support_test_tree_238c_v8.gcode` (avoidance-ladder fix, byte-inert),
  `_v9.gcode` (organic config), `_v10.gcode` (capsule removal), `_v11.gcode` (probe
  cleanup, byte-identical to v10 — **current**). Orca reference:
  `tmp/SupportTest_Tree_Orca.gcode`.
- Visual-debug bundles: `tmp/vd-238c/user-ours-v11/` (**current ours**, request
  `tmp/vd-238c/user-ours-request.json` → v11) and `tmp/vd-238c/user-ref/` (reference);
  older: `user-ours-v7/` (session 1), `user-ours{,-v2,-v4,-v6}/`. Prepass probe bundle
  `tmp/vd-238c/prepass-l80-82/` (`PrePass::SupportGeometry` tap, model mode).
  Layers of interest: z 9.0/13.2/13.4/13.6/16.2/16.4/24.4 → l44/l65/l66/l67/l80/l81/l121,
  `filled_areas` + `filament_lines`, `resolution_scale: 2`, `gcode_line_width_mm: 0.42`.
- Harness: `tmp/measure_paths.py` (calibrated 6/6 vs the baseline below); calibration
  scratch `tmp/calibrate_harness{,2,3}.py`.
- Config: `tmp/support-family-config-tree-matched.json` (organic-equivalent values,
  session 2). Model:
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.

## Binding workflow rules (violated once already — do not repeat)

- Every slice: `--module-dir modules/core-modules` AND `--model <path>`; else no support
  module registers and the slice silently drops support.
- The planner is a GUEST WASM: after touching `modules/*/src/**` run
  `cargo xtask build-guests` (then `--check` → exit 0) BEFORE re-slicing. A stale guest
  produced a byte-identical "v3" once — the slice looked unchanged because the guest was
  never rebuilt, not because the fix was inert.
- `cargo build --bin pnp_cli --release` is NOT needed for planner-only changes.
- Tee every test/verification run to `target/test-output.log`; read the file, never re-run.
- Narrow runs: `cargo test -p tree-support-planner` compiles all its test binaries;
  `--no-fail-fast` goes BEFORE the `--` separator where applicable.
- Do not weaken assertions or regenerate goldens without E3-classifying the drift.
- Cite canonical by file + function name only; cite in-tree code by symbol + crate path.