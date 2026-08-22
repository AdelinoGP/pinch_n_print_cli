# OrcaSlicer divergences found during history squash

Newly found divergences between our support-family code and canonical OrcaSlicer
(`OrcaSlicerDocumented/src/libslic3r/Support/`) discovered while squashing
`parity/support-planners`. They are RECORDED here only — never fixed as part of the squash.
Canonical code is cited by file + function name, never line number.
Section numbering refers to the original squash groups; the original commit SHAs were dropped when the source branch was retired.

## Squashed commit 1 of 8 (2026-08-21)

1. **Tree top-Z gap mechanism: mm walk over actual layer Z vs canonical layer-count + virtual gap node.**
   - Ours: `contact_layer_after_top_gap` (`modules/core-modules/tree-support-planner/src/lib.rs`) walks
     `LayerPlanViewEntry.z` downward while `layers[i].z > overhang_plane_z - gap`, deliberately avoiding
     `effective_layer_height` (the host's two producers of that field disagree).
   - Canonical: `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) computes
     `z_distance_top_layers = round_up_divide(scale_(z_distance_top), scale_(layer_height)) + 1`
     ("Support must always be 1 layer below overhang") and inserts a virtual gap node
     (`distance_to_top=-1`); `TreeSupportSettings` ctor (`TreeSupportCommon.hpp`) uses
     `round(support_top_distance / layer_height)`.
   - Differs: PnP measures the gap in mm along actual layer Z; canonical converts the gap to a layer
     count (rounded up, +1) plus a virtual node. Agree under uniform layer heights; can land the top
     support layer on different layers under variable heights.

2. **Tree renderer fill-density model: `support_density` fraction key vs canonical spacing-derived densities.**
   - Ours: `render_polygon` (`modules/core-modules/tree-support/src/lib.rs`) uses
     `spacing = line_width / density.min(1.0)` from the `support_density` key, which
     `tree-support.toml` declares as a 0-100 percentage (max = 100.0, default 20.0) — any value >= 1
     clamps to 1.0, i.e. 100% fill (percent/fraction mis-scaling; the pre-chunk code divided by 100.0).
     Roof/floor interface regions go through the same density-based fill; the
     `tree_support_interface_spacing_mm` key was deleted in this chunk.
   - Canonical: `TreeSupport::generate_toolpaths` (`TreeSupport.cpp`) derives body fill density from
     spacing — `support_density = min(1., support_flow.spacing() / support_spacing)` with
     `support_spacing = support_base_pattern_spacing + support_flow.spacing()` — and interface fill
     density from `interface_density = min(1., interface_flow.spacing() / (support_interface_spacing +
     interface_flow.spacing()))`. No `support_density` percentage key exists for tree support.
   - Differs: PnP drives tree fill from a percentage `support_density` key (mis-scaled as a fraction);
     canonical derives body and interface fill density from `support_base_pattern_spacing` /
     `support_interface_spacing` (mm spacings).

## Squashed commit 2 of 8 (2026-08-21)

1. **Tree branch smoothing removed: canonical `TreeSupport::generate_toolpaths` calls `smooth_nodes()`
   (100 iterations) before `draw_circles()`; PnP no longer smooths.**
   - Ours: the `smooth_branches(&mut entries_in_order, 100)` call in `run_support_geometry`
     (`modules/core-modules/tree-support-planner/src/lib.rs`) was removed in this chunk ("Do not
     smooth after exact-Z collision validation"); `smooth_branches` (the `TreeSupport::smooth_nodes`
     port) is now production-dead, called only from `smooth_nodes_tdd.rs`.
   - Canonical: `TreeSupport::generate_toolpaths` (`TreeSupport.cpp`) calls `smooth_nodes()`
     immediately before `draw_circles()`.
   - Differs: canonical smooths every tree branch (100 iterations, max_move = support_line_width/2);
     PnP emits unsmoothed node positions. The smoothing port remains in-tree but unused.

2. **Body cleared entirely on any layer carrying roof/floor: canonical keeps base areas from
   non-roof nodes on the same layer.**
   - Ours: `build_roles` (`modules/core-modules/tree-support-planner/src/lib.rs`) carves roof/floor
     out of the body, then `if !roof.is_empty() || !floor.is_empty() { carved.clear(); }` — a layer
     with any interface geometry carries no body at all.
   - Canonical: `TreeSupport::draw_circles` (`TreeSupport.cpp`) classifies each node's circle by
     `support_roof_layers_below` into roof_1st_layer / roof_base_areas / roof_areas / base_areas,
     then `base_areas = diff_ex(base_areas, roofs)` — base and roof coexist on a layer (disjoint via
     diff, not via clearing).
   - Differs: on a layer where one branch is inside its roof band and another passes through,
     canonical emits both roof and body; PnP drops the passing branch's body capsule.

3. **Unioned body regions limited to 16 contour vertices: canonical draws 100-vertex circles.**
   - Ours: `structural_body_regions` unions merged capsules and `limit_contour_vertices` caps each
     contour at `BRANCH_CIRCLE_SEGMENTS` (16) (`modules/core-modules/tree-support-planner/src/lib.rs`).
   - Canonical: `TreeSupport::draw_circles` (`TreeSupport.cpp`) uses `CIRCLE_RESOLUTION = 100`
     (4 only in square mode when avg_node_per_layer > 200) and never unions node circles into a
     single body region.
   - Differs: PnP body contours are ≤16-vertex polygon approximations of capsule unions; canonical
     keeps full-resolution circles. (The 16-gon branch circle itself predates this chunk; the union
     plus the vertex limit is new.)

4. **Whole-edge rejection on swept-capsule collision: canonical clips per-circle and keeps the
   remainder.**
   - Ours: `body_segment_intersects` (`modules/core-modules/tree-support-planner/src/lib.rs`)
     rejects an entire MST edge when the swept capsule between its endpoints intersects model
     occupancy.
   - Canonical: `TreeSupport::draw_circles` (`TreeSupport.cpp`) clips each node circle against the
     collision via `avoid_object_remove_extra_small_parts` and keeps the largest non-colliding
     remainder; there is no node/edge drop gate.
   - Differs: a branch crossing a concave model corner can be dropped entirely in PnP where
     canonical would emit the clipped remainder.

## Squashed commit 3 of 8 (2026-08-21)

1. **`max_bridge_length` key undeclared; interior-grid step hardcodes 10.0.**
   - Ours: `sample_contact_points` (`modules/core-modules/tree-support-planner/src/lib.rs`) computes
     `step = point_spread.max(DEFAULT_MAX_BRIDGE_LENGTH_MM / 2.0)` with
     `DEFAULT_MAX_BRIDGE_LENGTH_MM = 10.0`; the key is not declared in `tree-support-planner.toml`.
   - Canonical: `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) uses
     `config.max_bridge_length.value` (user-settable, default 10.0).
   - Differs: matches the canonical default but ignores any profile override of `max_bridge_length`.

2. **Mesh-path overhang polygons from projected triangles vs canonical host-computed per-layer overhangs.**
   - Ours: `plan_for_object` (`modules/core-modules/tree-support-planner/src/lib.rs`) builds per-layer
     `ExPolygon`s by projecting overhang triangles downward (self-acknowledged "Legacy-path
     compatibility shim" for coplanar-plate fixtures).
   - Canonical: `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) samples
     `layer->loverhangs` — the slicer's per-layer overhang polygons.
   - Differs: for non-coplanar geometry a triangle's downward projection is not the layer's true
     overhang region; canonical consumes the host-computed polygons.

3. **Inner-grid erosion miter limit 0.0 vs canonical 3.0.**
   - Ours: `sample_contact_points` erodes with `host::offset_polygons(polygons, -base_radius,
     OffsetJoinType::Miter, 0.0)`.
   - Canonical: `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) uses
     `offset_ex(overhang, -radius_scaled)` with defaults `jtMiter`, `DefaultMiterLimit = 3.0`.
   - Differs: sharp-corner handling in the eroded containment region; minor.

## Squashed commit 4 of 8 (2026-08-21)

1. **Avoidance ladder keyed on the constant branch radius; canonical keys on the per-node tapered radius.**
   - Ours: `SupportPlanner::run_support_geometry` (`modules/core-modules/tree-support-planner/src/lib.rs`)
     calls `volumes.ensure_avoidance(branch_radius)` / `get_avoidance(branch_radius, cache_idx)` with
     `branch_radius = tree_support_branch_diameter / 2.0` — one constant radius for the whole run.
   - Canonical: `TreeSupport::move_nodes` (`TreeSupport.cpp`) calls
     `get_avoidance(next_radius, obj_layer_nr_next)` with
     `next_radius = calc_radius(node.dist_mm_to_top + height_next)` — the per-node tapered radius.
   - Differs: PnP's avoidance region is computed for one constant radius; canonical computes it per
     node radius. (The audit's F-16(d) described the pre-fix constant-radius inflation; this is the
     residual in the fix.)

2. **Emit collision gates: radius-free bucket + per-node disc test vs canonical radius-baked volume +
   point-in test.**
   - Ours: emit gates read `get_collision(0.0, l)` (outlines inflated by `m_xy_distance` alone) and add
     the node's tapered radius at test time via `body_intersects` → `body_overlaps_occupancy`
     (point-in plus distance-to-contour disc test).
   - Canonical: `TreeSupport::draw_circles` / `move_nodes` (`TreeSupport.cpp`) call
     `get_collision(radius, l)` — outlines inflated by `radius + m_xy_distance` — then a point-in test.
   - Differs: mechanism, not sum. The port documents the choice as deliberate (feeding
     `get_collision(branch_radius)` to the disc test double-counts the radius) and flags it as interim:
     the F-13 move-pass rewrite should switch to `get_collision(tapered_radius, l)` and drop the
     test-time inflation. The disc-distance test is also not exactly equivalent to a miter-inflated
     point-in at corners.

3. **Miter limit 2.0 vs canonical 3.0 on the TreeVolumes offsets.**
   - Ours: `TreeVolumes::ensure_collision`'s `batch_offset` and `ensure_avoidance`'s
     `host::offset_polygons` route through `slicer_core::polygon_ops::offset` → `inflate_once` with
     miter limit 2.0 (Clipper2 default); the host offset path exposes no miter-limit parameter.
   - Canonical: `TreeSupportData::calculate_collision` / `calculate_avoidance` (`TreeSupport.cpp`) use
     `offset_ex` with `DefaultMiterLimit = 3.0`.
   - Differs: sharp-corner handling on the inflated collision and eroded avoidance volumes. Same class
     as the previously recorded `sample_contact_points` erosion entry.

4. **Layer outlines not simplified at the 0.2 mm grid in the TreeVolumes constructor.**
   - Ours: `TreeVolumes::new` (`modules/core-modules/tree-support-planner/src/lib.rs`) stores raw
     `SupportGeometryView` outlines; `layer_outlines_below` unions the raw outlines.
   - Canonical: the `TreeSupportData` ctor (`TreeSupport.cpp`) simplifies each layer's `lslices` at
     `scale_(m_radius_sample_resolution)` and builds `m_layer_outlines_below` from the simplified
     outlines.
   - Differs: `outlines_at` / `outlines_below` consumers (to_buildplate and floor checks) see
     unsimplified geometry.

5. **`expolygons_simplify` skips canonical's final `union_ex`.**
   - Ours: guest-side `expolygons_simplify` (`modules/core-modules/tree-support-planner/src/lib.rs`)
     simplifies contour + holes and drops degenerate rings, keeping the contour/hole structure.
   - Canonical: `ExPolygon::simplify` (`ExPolygon.cpp`) is `union_ex(simplify_p(tolerance))` — the
     union can merge a hole into the contour or split an expolygon after simplification.
   - Differs: topology after simplification; minor.

6. **to_buildplate determination uses raw outlines; canonical uses `get_collision(0, l)`
   (xy_distance-inflated).**
   - Ours: `push_contact_with_demand` / `push_analysis_contact`
     (`modules/core-modules/tree-support-planner/src/lib.rs`) test
     `point_in_any_expoly(volumes.outlines_at(global_layer), x, y)` — raw outlines, no inflation.
   - Canonical: `TreeSupport::move_nodes` (`TreeSupport.cpp`) uses
     `!is_inside_ex(get_collision(0, obj_layer_nr), position)` — outlines inflated by `m_xy_distance`.
   - Differs: a contact within `support_object_xy_distance` of the model wall is classified
     to_buildplate in PnP but not in canonical. (Carried forward from the pre-fix code; the TreeVolumes
     port re-encodes it.)

## Squashed commit 5 of 8 (2026-08-21)

1. **`move_out_expolys` projects onto the original ring and aborts on budget exceed; canonical
   dilates via `offset_ex`, projects onto the dilated ring, and clamps to the budget.**
   - Ours: `move_out_expolys` (`modules/core-modules/tree-support-planner/src/lib.rs`) finds the
     closest point on the ORIGINAL polygon rings (contours + holes), steps outward analytically by
     `min_dist`, and returns the ORIGINAL point unchanged when the displacement would exceed
     `max_dist` (the move is aborted). Returns the new position.
   - Canonical: `move_out_expolys` (`TreeSupport.cpp`) computes
     `polys_dilated = union_ex(offset_ex(polygons, scale_(distance)))`, projects onto the DILATED
     ring, and when the projection exceeds `max_move_distance` clamps to
     `pt_max = from + normal(outward_dir, scale_(max_move_distance))`; returns bool success.
   - Differs: (a) projection target — original ring + analytic step vs dilated ring; (b) budget
     handling — abort (return original) vs clamp to `pt_max`. The PnP comment claims "Canonical
     restores `from0` when the push-out exceeds the budget"; in the local checkout canonical clamps
     to `pt_max` and `from0` is saved but unused. Affects the branch-A group-0 push-out, the
     STUDIO-4252 retry, and the F-13 move-pass escape.

2. **Enforcer contacts are generated only under manual support type; canonical adds them under
   auto too.**
   - Ours: `commit_support_analysis_builtin`
     (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) routes geometry through
     `enforcer_contacts` only when `!support_type.is_auto()`; under auto, `detect_support_contacts`
     runs without the enforcer set.
   - Canonical: `detect_contacts` (`SupportMaterial.cpp`) runs the enforcer branch whenever
     `has_enforcer` (the layer's `annotations.enforcers_layers` is non-empty), with no support-type
     gate; the `auto_normal_support` gate applies only to `detect_overhangs`' thresholded branch.
   - Differs: under `normal(auto)` with painted enforcers, canonical emits auto-detected overhangs
     PLUS enforcer contacts; PnP ignores the enforcers. The `SupportType::NormalAuto` doc comment
     (`crates/slicer-ir/src/slice_ir.rs`) claims "auto-detected overhangs plus enforcers",
     contradicting the producer's routing. Related: the tree planner's legacy mesh-path contact
     sampling is likewise not gated on auto (F-15 already documents the unconditional shim).

3. **`detect_support_contacts` omits five canonical `detect_overhangs` steps.**
   - Ours: `detect_support_contacts` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
     implements diff → expand-back → blockers → tiny-spot filter → XY expansion → union_ex, and
     self-documents as "Not modelled": sharp-tail detection, `bridge_no_support`,
     `buildplate_covered`, the cantilever pass, and `enforce_support_layers`.
   - Canonical: `detect_overhangs` (`SupportMaterial.cpp`) additionally appends sharp-tail
     overhangs under `g_config_support_sharp_tails`, subtracts `buildplate_covered` under
     buildplate-only, calls `remove_bridges_from_contacts` under `bridge_no_support`, runs the
     post-union cantilever pass, and forces `lower_layer_offset = 0` below `enforce_support_layers`.
   - Differs: five canonical behaviours absent. Not in the gap register or parity-audit.

4. **Flow-derived widths replaced by config keys.**
   - Ours: `resolve_contact_params` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`)
     resolves `fw` as `outer_wall_line_width` extension → typed `line_width` → 0.4; the tree
     planner's `get_max_move_dist` cap uses a new manifest key `support_line_width` (plain mm float,
     default 0.35, min 0, max 2) read in `SupportPlanner::from_config`
     (`modules/core-modules/tree-support-planner/src/lib.rs`).
   - Canonical: `detect_overhangs` / `detect_contacts` (`SupportMaterial.cpp`) take
     `fw = layerm->flow(frExternalPerimeter).scaled_width()`; `TreeSupport::generate_toolpaths`
     (`TreeSupport.cpp`) derives `support_extrusion_width` from
     `Flow::auto_extrusion_width(frSupportMaterial, nozzle_diameter)`, with `support_line_width` a
     `coFloatOrPercent` (percent over nozzle diameter, default 0 = auto).
   - Differs: PnP has no flow model, so widths come from config keys. The tree planner's
     `support_line_width` has no percent semantics and defaults to 0.35 mm where canonical defaults
     to 0 (auto).

5. **Branch-A merge seeds the roof counter with the max of both nodes; canonical inherits the
   parent's counter.**
   - Ours: the F-11 branch-A collapse (`plan_for_object`,
     `modules/core-modules/tree-support-planner/src/lib.rs`) seeds the merged node's
     `support_roof_layers_below` with `max(id, nid)` minus the decrement; the comment cites
     canonical `insert_dropped_node`.
   - Canonical: `drop_nodes` (`TreeSupport.cpp`) branch A uses
     `node_parent->support_roof_layers_below - (node_parent->distance_to_top >= 0 ? 1 : 0)` — the
     PARENT's counter (parent = the node with the larger `dist_mm_to_top`). `insert_dropped_node`'s
     max-merge is the same-position dedup path, not branch A.
   - Differs: when the two collapsing nodes carry different roof counters, PnP's merged node
     inherits the max; canonical inherits the parent's.

6. **Branch-A merged-node `to_buildplate` uses raw outlines; canonical uses `get_collision(0, l)`.**
   - Ours: branch A computes `to_buildplate = !is_inside_ex(volumes.outlines_at(next_cache_idx), ...)`
     — raw outlines, no inflation.
   - Canonical: `drop_nodes` (`TreeSupport.cpp`) branch A computes
     `to_buildplate = !is_inside_ex(get_collision(0, obj_layer_nr_next), next_position)` — outlines
     inflated by `m_xy_distance`.
   - Differs: a midpoint within `support_object_xy_distance` of the model wall is classified
     to_buildplate in PnP but not canonical. Same class as the previously recorded to_buildplate
     entry, new call site. (The F-14 per-descendant recompute correctly uses raw outlines —
     canonical's move pass uses `m_layer_outlines` there too.)

7. **`smsTreeStrong` / `smsTreeHybrid` support styles unmodeled.**
   - Ours: the F-13 move pass always uses the 1/d² neighbour weighting (`neighbour_direction_sum`)
     and the `normal(direction_to_outer, max_move)` movement; `TreeNodeType::Polygon` is never
     minted; `SupportPlanner::from_config` distinguishes only slim.
   - Canonical: `drop_nodes` (`TreeSupport.cpp`) under `is_strong` (support_style == smsTreeStrong)
     uses unweighted neighbour sums and `movement = direction_to_outer + move_to_neighbor_center`
     with a dot-product gate; `generate_contact_points` under smsTreeHybrid mints ePolygon nodes
     with their own merge/move handling.
   - Differs: `support_style = tree_strong` / `tree_hybrid` users get non-strong behaviour. Feature
     gap, not a defect in the ported path.

## Squashed commit 6 of 8 (2026-08-21)

1. **`support_bottom_interface_spacing` recorded as PnP-invented (DEV-139), but canonical declares
   and uses the same key.**
   - Ours: `[config.schema.support_bottom_interface_spacing]` in
     `modules/core-modules/traditional-support/traditional-support.toml` and
     `modules/core-modules/tree-support/tree-support.toml`, default -1.0 (negative mirrors the top
     gap). The chunk's commit message and DEV-139 (`docs/DEVIATION_LOG.md`) both claim the key has
     no canonical counterpart and that canonical derives ONE interface spacing for both bands.
   - Canonical: `PrintConfig.cpp` declares `support_bottom_interface_spacing` (coFloat, default 0.5,
     min 0); `SupportParameters` (`SupportParameters.hpp`) computes
     `bottom_interface_spacing = support_bottom_interface_spacing + support_material_interface_flow.spacing()`;
     `TreeSupport::generate_toolpaths` (`TreeSupport.cpp`) computes the same pair for tree support.
   - Differs: the recorded deviation's premise is false — the key is canonical and canonical uses it
     exactly as PnP does (gap + flow spacing). The real difference is narrower: PnP defaults the key
     to -1.0 (mirror-top convention) where canonical defaults to 0.5 mm, so an untouched PnP config
     produces a bottom pitch equal to the top pitch while an untouched canonical config produces
     0.5 + flow-spacing. DEV-139 needs correction during packet revision.

## Squashed commit 7 of 8 (2026-08-21)

1. **Emit carve keeps ALL surviving parts of a drawn circle; canonical
   `avoid_object_remove_extra_small_parts` keeps only the largest.**
   - Ours: the emit pass (`modules/core-modules/tree-support-planner/src/lib.rs`) drops a node or
     region only when its drawn footprint lies ENTIRELY inside the collision volume
     (`swallowed_by_collision` / `node_swallowed`), and `build_roles` carves the footprint by
     difference against the same collision set, keeping every surviving part.
   - Canonical: `TreeSupport::draw_circles` (`TreeSupport.cpp`) computes
     `avoid_object_remove_extra_small_parts(circle, get_collision(...))` — `diff_ex` against the
     collision, then keeps ONLY the largest-area surviving part.
   - Differs: small disconnected slivers of a branch cross-section that survive the collision
     difference are printed in PnP and dropped by canonical. This supersedes the previously
     recorded whole-edge rejection (`body_segment_intersects`, deleted in this chunk): the drop
     gate now matches canonical `drop_nodes`' "completely inside" rule, but the carve does not
     replicate the largest-part selection.

2. **STUDIO-4252 retry call site passes `radius_sample_resolution + EPSILON` as the dilation
   argument; canonical passes `max_move_between_samples` for both arguments.**
   - Ours: the F-13 move pass (`modules/core-modules/tree-support-planner/src/lib.rs`) calls
     `move_out_expolys(&collision_next, pos, RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM,
     max_move + RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM)`.
   - Canonical: `TreeSupport::drop_nodes` (`TreeSupport.cpp`) computes
     `max_move_between_samples = max_move_distance + radius_sample_resolution + EPSILON` and passes
     it as BOTH the dilation and the max-distance argument of `move_out_expolys`.
   - Differs: the in-tree dilation is smaller by `max_move_distance`; affects only the escape
     direction, since the step is renormalized to `max_move` afterwards. Spotted in this chunk's
     commit message ("Unrelated parity nit ... deliberately NOT changed"); the call site predates
     the chunk and was left unchanged.

## Squashed commit 8 of 8 (2026-08-21)

1. **Emit simplify applied unconditionally to every role region at 0.0125 mm; canonical simplifies
   only `base_areas`, only under `SQUARE_SUPPORT`, at `line_width / 2`.**
   - Ours: `build_roles` (`modules/core-modules/tree-support-planner/src/lib.rs`) now runs
     `expolygons_simplify` at `DRAW_CIRCLES_RESOLUTION_MM` (0.0125 mm) on every role region (body,
     roof, floor) BEFORE the carve, and its comment cites canonical `draw_circles`' simplify as
     unconditional and its post-simplify `diff_ex(base_areas, trimming)` as a collision re-diff.
   - Canonical: `TreeSupport::draw_circles` (`TreeSupport.cpp`) simplifies only `base_areas`, only
     when `SQUARE_SUPPORT` (`avg_node_per_layer > 200`, the same fallback that drops
     `CIRCLE_RESOLUTION` to 4), at `scale_(line_width / 2)` (`support_line_width / 2`, ~0.2 mm).
     The later `ts_layer->base_areas = diff_ex(ts_layer->base_areas, trimming)` is the Orca
     bottom-Z clearance trim via `get_trim_support_regions` (gap_object_support), not a re-diff
     against collision.
   - Differs: in the normal case (avg_node_per_layer <= 200) canonical emits unsimplified
     100-vertex circles; PnP simplifies every role region at a much finer tolerance. The PnP
     comment's canonical justification ("simplifying first is the equivalent with one clip instead
     of two") does not match the local checkout.
