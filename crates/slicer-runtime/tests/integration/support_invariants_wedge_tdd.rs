#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanRole,
};
use slicer_runtime::run::PrepassContext;

use crate::common::support_wedge;

fn prepare_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context(true)
}

/// Tree-family wedge context.
///
/// The default wedge sets no `support_type`, which resolves to the
/// *traditional* family, and `traditional-support-planner` hardcodes
/// `skeleton: None`. Every assertion below that reads
/// `SupportPlanEntry::skeleton` must therefore run against this context or it
/// is asserting on data the fixture cannot produce.
fn prepare_tree_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context_tree(true)
}

fn owned_tree_plan_entries() -> Vec<SupportPlanEntry> {
    plan_entries(&prepare_tree_ctx()).to_vec()
}

fn plan_entries(ctx: &PrepassContext) -> &[SupportPlanEntry] {
    &ctx.blackboard
        .support_plan()
        .expect("support_plan must be committed")
        .entries
}

fn owned_plan_entries() -> Vec<SupportPlanEntry> {
    plan_entries(&prepare_ctx()).to_vec()
}

fn structural_points(entry: &SupportPlanEntry) -> impl Iterator<Item = &slicer_ir::Point3> {
    entry
        .skeleton
        .as_ref()
        .into_iter()
        .flat_map(|skeleton| skeleton.points.iter())
}

#[test]
fn support_plan_has_finite_branch_paths() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(!entries.is_empty());
    for entry in entries {
        assert!(!entry.family_id.is_empty());
        for point in structural_points(&entry) {
            assert!(point.x.is_finite() && point.y.is_finite() && point.z.is_finite());
        }
    }
}

#[test]
fn branch_endpoints_are_outside_support_collision_outlines() {
    let ctx = prepare_tree_ctx();
    let entries = plan_entries(&ctx);
    let structural = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none());
    assert!(structural.clone().next().is_some());
    for entry in structural {
        assert!(entry
            .skeleton
            .as_ref()
            .is_some_and(|skeleton| skeleton.points.len() > 1));
    }
}

#[test]
fn branch_points_match_entry_layer_z() {
    for entry in owned_plan_entries() {
        assert!(entry.anchor_z.is_positive() || entry.anchor_z == 0);
    }
}

#[test]
fn overhang_facets_have_wedge_layer_contacts() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(entries.iter().any(|entry| {
        entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody)
    }));
}

#[test]
fn branch_radii_stay_within_current_bounds() {
    for entry in owned_plan_entries() {
        for point in structural_points(&entry) {
            assert!(point.x.is_finite() && point.y.is_finite() && point.z.is_finite());
        }
    }
}

#[test]
fn disabled_raft_has_no_negative_entries() {
    assert!(owned_plan_entries()
        .iter()
        .all(|entry| entry.global_layer_index >= 0));
}

#[test]
fn support_disabled_produces_explicit_empty_plan() {
    let ctx = support_wedge::prepare_wedge_context(false);
    assert!(ctx
        .blackboard
        .support_plan()
        .expect("SupportPlanIR must be committed")
        .entries
        .is_empty());
}

#[test]
fn branch_points_carry_finite_nonnegative_dist_to_top_mm() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(entries.iter().all(|entry| entry.anchor_z >= 0));
}

#[test]
fn enabled_raft_config_is_emitted_as_raft_plan() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[
            ("support_raft_layers", ConfigValue::Int(2)),
            ("raft_first_layer_density", ConfigValue::Float(0.4)),
            ("base_raft_layers", ConfigValue::Int(1)),
            ("interface_raft_layers", ConfigValue::Int(1)),
        ],
    );
    assert_eq!(
        ctx.blackboard.support_plan().unwrap().raft_plan,
        Some(RaftPlan {
            raft_layers: 2,
            raft_first_layer_density: 0.4,
            base_raft_layers: 1,
            interface_raft_layers: 1,
        })
    );
}

#[test]
fn disabled_raft_config_has_no_raft_plan() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[("support_raft_layers", ConfigValue::Int(0))],
    );
    assert!(ctx.blackboard.support_plan().unwrap().raft_plan.is_none());
}

#[test]
fn branch_curvature_below_threshold() {
    let entries = owned_tree_plan_entries();
    let structural: Vec<_> = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .collect();
    assert!(
        !structural.is_empty(),
        "tree wedge must plan at least one non-declined entry"
    );
    // Was `map_or(true, ..)`, which passes for every `skeleton: None` entry the
    // traditional planner emits — i.e. it was vacuously green on the old
    // fixture. Every structural tree entry must actually carry a skeleton.
    assert!(structural
        .iter()
        .all(|entry| entry.skeleton.as_ref().is_some_and(|s| s.points.len() >= 2)));
}

#[test]
fn merge_geometry_symmetric_for_n_branches() {
    assert!(owned_plan_entries()
        .iter()
        .all(|entry| entry.body_ids.iter().all(|id| !id.is_empty())));
}

#[test]
fn build_plate_only_emits_no_to_model_branches() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[("support_on_build_plate_only", ConfigValue::Bool(true))],
    );
    assert!(plan_entries(&ctx)
        .iter()
        .all(|entry| entry.decline_reason != Some(SupportPlanDeclineReason::Blocked)));
}

#[test]
fn support_columns_are_contiguous_and_step_down_through_every_layer() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(!entries.is_empty(), "wedge must plan support entries");

    // The old form asserted `anchor_layer_index` was globally non-increasing
    // across `entries`, which encoded the planner's *emission* order. Packet
    // 223 made the aggregate sort entries ascending by the identity triple, so
    // no global anchor ordering survives. The invariant this test is named for
    // is per-column: within one (object, region, body) column the layers form
    // a contiguous run with single-layer steps.
    let mut columns: std::collections::BTreeMap<(String, u64, String), Vec<&SupportPlanEntry>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        for body_id in &entry.body_ids {
            columns
                .entry((entry.object_id.clone(), entry.region_id, body_id.clone()))
                .or_default()
                .push(entry);
        }
    }
    assert!(
        !columns.is_empty(),
        "every planned entry must carry at least one body_id so columns are identifiable"
    );

    for (key, mut column) in columns {
        column.sort_by_key(|entry| entry.global_layer_index);
        for pair in column.windows(2) {
            let step = pair[1].global_layer_index - pair[0].global_layer_index;
            assert!(
                step == 0 || step == 1,
                "column {key:?} is not contiguous: layer {} -> {} (step {step})",
                pair[0].global_layer_index,
                pair[1].global_layer_index
            );
        }
        for entry in &column {
            assert_eq!(
                entry.anchor_layer_index as i32, entry.global_layer_index,
                "column {key:?} anchor must step down in lockstep with its layer"
            );
        }
    }
}

#[test]
fn support_branch_widths_widen_monotonically_toward_the_plate() {
    let entries = owned_tree_plan_entries();
    let mut structural = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none());
    assert!(structural.clone().next().is_some());
    assert!(structural.all(|entry| {
        entry
            .skeleton
            .as_ref()
            .is_some_and(|skeleton| skeleton.points.len() > 1)
    }));
}

/// The model cross-sections the tree planner uses as its collision source,
/// unioned per global support layer index.
///
/// This is the same `SupportGeometryIR` the planner's `TreeVolumes` reads into
/// `m_layer_outlines` (`layer_outlines` in `tree-support-planner`), so a point
/// inside one of these polygons is a point the planner routed *into the model*.
/// The `u32::MAX` sentinel keys are intermediate model-resolution layers with
/// no `GlobalLayer` z to match against, so they are skipped.
fn model_outlines_by_layer(
    ctx: &PrepassContext,
) -> std::collections::BTreeMap<u32, Vec<slicer_ir::ExPolygon>> {
    let geometry = ctx
        .blackboard
        .support_geometry()
        .expect("support_geometry must be committed for the tree wedge");
    let mut by_layer: std::collections::BTreeMap<u32, Vec<slicer_ir::ExPolygon>> =
        std::collections::BTreeMap::new();
    for (key, polys) in &geometry.entries {
        if key.global_support_layer_index == u32::MAX {
            continue;
        }
        by_layer
            .entry(key.global_support_layer_index)
            .or_default()
            .extend(polys.iter().cloned());
    }
    by_layer
}

/// Global-layer z heights in mm, keyed by layer index.
fn layer_z_mm(ctx: &PrepassContext) -> std::collections::BTreeMap<u32, f32> {
    ctx.blackboard
        .layer_plan()
        .expect("layer_plan must be committed")
        .global_layers
        .iter()
        .map(|layer| (layer.index, layer.z))
        .collect()
}

/// True when `(x_mm, y_mm)` is in the material of `outlines` — inside a
/// contour and not inside one of its holes.
fn point_is_in_material(outlines: &[slicer_ir::ExPolygon], x_mm: f64, y_mm: f64) -> bool {
    outlines.iter().any(|poly| {
        slicer_ir::point_in_polygon_winding(poly, x_mm, y_mm, 0.0)
            && !poly
                .holes
                .iter()
                .any(|hole| slicer_ir::point_in_contour_winding(hole, x_mm, y_mm, 0.0))
    })
}

/// Coarse penetration depth in mm: the largest ladder rung `m` for which the
/// point *and* the four `±m` cross samples around it are all in material.
///
/// `0.0` means the point is outside, or on/near the outline boundary — the
/// distinction that matters here, because canonical contact placement puts
/// nodes exactly on a wall face (measured on this wedge: the layer-13 node
/// `(15.902, 50.000)` sits exactly on the outline's `y = 50.000` edge, where a
/// plain winding test with zero tolerance reports "inside"). Anything above
/// [`PENETRATION_TOLERANCE_MM`] is a branch routed *through* material.
fn penetration_depth_mm(outlines: &[slicer_ir::ExPolygon], x_mm: f64, y_mm: f64) -> f64 {
    const LADDER_MM: [f64; 7] = [0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0];
    if !point_is_in_material(outlines, x_mm, y_mm) {
        return 0.0;
    }
    let mut depth = 0.0;
    for m in LADDER_MM {
        let all_in = [(m, 0.0), (-m, 0.0), (0.0, m), (0.0, -m)]
            .into_iter()
            .all(|(dx, dy)| point_is_in_material(outlines, x_mm + dx, y_mm + dy));
        if !all_in {
            break;
        }
        depth = m;
    }
    depth
}

/// Penetration at or below this is boundary contact, not routing into the
/// model. It is the first rung of `penetration_depth_mm`'s ladder, i.e. the
/// finest resolution the predicate can distinguish — not a tuned fudge.
const PENETRATION_TOLERANCE_MM: f64 = 0.0;

/// Axis-aligned `(width, depth)` in mm of the model bbox unioned with the
/// given `(x, y)` skeleton points.
fn footprint_extent_mm(
    bbox: &slicer_ir::BoundingBox3,
    points: impl Iterator<Item = (f64, f64)>,
) -> (f64, f64) {
    let (mut min_x, mut max_x) = (bbox.min.x as f64, bbox.max.x as f64);
    let (mut min_y, mut max_y) = (bbox.min.y as f64, bbox.max.y as f64);
    for (x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (max_x - min_x, max_y - min_y)
}

/// The printer bed's axis-aligned extent `(width_mm, depth_mm)`, derived from
/// the resolved `printable_area` polygon (interleaved `[x0, y0, x1, y1, ...]` mm).
fn bed_extent_mm(config: &slicer_ir::ResolvedConfig) -> (f64, f64) {
    let pts = &config.printable_area;
    assert!(
        pts.len() >= 6 && pts.len().is_multiple_of(2),
        "printable_area must be at least 3 interleaved points; got {pts:?}"
    );
    let xs: Vec<f64> = pts.iter().step_by(2).copied().collect();
    let ys: Vec<f64> = pts.iter().skip(1).step_by(2).copied().collect();
    let span = |v: &[f64]| {
        let min = v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        max - min
    };
    let (w, d) = (span(&xs), span(&ys));
    assert!(
        w.is_finite() && d.is_finite() && w > 0.0 && d > 0.0,
        "printable_area encloses no area; got {pts:?}"
    );
    (w, d)
}

/// Canonical containment invariant for planned branch centrelines.
///
/// **Amended by packet 224.** This test used to assert that every skeleton
/// point lay inside `MeshIR::build_volume` (the model bbox) + 1 mm, on the
/// premise — stated in its old comment — that "branch centrelines are clamped
/// inside the overhang footprint". Packet 224 deleted that centreline clamp as
/// non-canonical when it landed the canonical move pass (F-13/F-14): canonical
/// `TreeSupport` moves nodes out of `get_collision(r, l) = outlines ⊕
/// (ceil_radius(r) + m_xy_distance)`, so a branch running down a wall
/// legitimately stands off *outside* the model bbox by its own radius plus the
/// xy clearance. Measured on this wedge at layer 0: the outline spans
/// y ∈ [0.000, 50.000] and the lowest node sits at y = -5.249 with radius
/// 4.731 mm, i.e. exactly the canonical clearance (4.8 + 0.35, plus the
/// 0.2 mm `radius_sample_resolution` push margin). Asserting the old bbox
/// bound would require re-introducing the deleted clamp.
///
/// The two properties the old assertion was groping at, and which do hold
/// canonically, are asserted instead:
///
/// 1. **No branch is routed into the model.** No skeleton point may lie inside
///    the model cross-section at its own layer. This is a *necessary*
///    condition for canonical collision-freedom rather than the full one:
///    `SupportPlanSkeleton` carries no per-point radius, so the
///    radius-inflated collision volume cannot be reconstructed here; the raw
///    outline is the `r = 0`, `xy_distance = 0` subset of it. "Inside" means
///    *strictly interior* (see `penetration_depth_mm`) — a node sitting
///    exactly on a wall face is boundary contact, not penetration, and the
///    current plan does emit such nodes (e.g. `(15.902, 50.000)` at z = 2.8,
///    on the outline's `y = 50.000` edge).
/// 2. **The support stays printable.** Branch z stays between the plate and
///    the top of the model, and the combined model + support footprint fits
///    inside the bed. The bound is on the *extent*, not on absolute position,
///    because this fixture loads `regression_wedge.stl` with its raw STL
///    coordinates — the model sits flush against the bed's x = 0 / y = 0 edges
///    with no plate placement, so a canonical standoff necessarily reads as
///    negative y. Extent-fit is the tightest bound that is invariant to that
///    fixture artefact; it still fails any branch that flies off the plate.
#[test]
fn support_segments_stay_outside_the_model_and_within_the_build_volume() {
    let ctx = prepare_tree_ctx();
    let entries = plan_entries(&ctx);

    let point_count = entries.iter().flat_map(structural_points).count();
    assert!(
        point_count > 0,
        "tree wedge must plan at least one structural skeleton point"
    );

    let outlines = model_outlines_by_layer(&ctx);
    assert!(
        outlines.values().any(|polys| !polys.is_empty()),
        "collision source is empty — the model-containment check would be vacuous"
    );
    let layer_z = layer_z_mm(&ctx);
    let layers: Vec<(u32, f32)> = outlines
        .keys()
        .filter_map(|idx| layer_z.get(idx).map(|z| (*idx, *z)))
        .collect();
    assert!(
        !layers.is_empty(),
        "no support-geometry layer maps to a planned global layer z"
    );

    // Property 1: never inside the model.
    for entry in entries {
        for point in structural_points(entry) {
            let (idx, z) = layers
                .iter()
                .copied()
                .min_by(|a, b| (a.1 - point.z).abs().total_cmp(&(b.1 - point.z).abs()))
                .expect("layers is non-empty");
            let polys = &outlines[&idx];
            let depth = penetration_depth_mm(polys, point.x as f64, point.y as f64);
            assert!(
                depth <= PENETRATION_TOLERANCE_MM,
                "skeleton point {point:?} is routed {depth} mm inside the model \
                 cross-section at layer {idx} (z = {z} mm)"
            );
        }
    }

    // Property 2: printable — z between plate and model top, and the combined
    // model + support footprint fits the bed.
    let bbox = ctx.blackboard.mesh().build_volume;
    let (bed_w, bed_d) = bed_extent_mm(&ctx.default_resolved_config);
    const Z_EPS_MM: f32 = 1e-3;
    for entry in entries {
        for point in structural_points(entry) {
            assert!(
                point.z >= -Z_EPS_MM && point.z <= bbox.max.z + Z_EPS_MM,
                "skeleton point {point:?} leaves the build volume in z \
                 (plate 0.0 mm .. model top {} mm)",
                bbox.max.z
            );
        }
    }
    let (fw, fd) = footprint_extent_mm(
        &bbox,
        entries
            .iter()
            .flat_map(structural_points)
            .map(|p| (p.x as f64, p.y as f64)),
    );
    assert!(
        fw <= bed_w && fd <= bed_d,
        "model + support footprint {fw:.3} x {fd:.3} mm does not fit the bed \
         {bed_w:.3} x {bed_d:.3} mm"
    );
}

/// Negative control for the two predicates
/// `support_segments_stay_outside_the_model_and_within_the_build_volume`
/// asserts on. An invariant test that cannot fail is worse than none, and the
/// wedge plan is (correctly) clean, so the defect is injected here instead: a
/// skeleton point placed in the interior of the model cross-section (measured:
/// the layer-0 contour centroid `(27.500, 30.000)` reads a penetration depth of
/// 2.0 mm, the ladder cap, against a tolerance of 0.0), and one
/// placed off the plate.
#[test]
fn amended_containment_invariant_still_catches_a_routed_into_model_defect() {
    let ctx = prepare_tree_ctx();
    let outlines = model_outlines_by_layer(&ctx);
    let (idx, polys) = outlines
        .iter()
        .find(|(_, polys)| polys.iter().any(|p| p.contour.points.len() >= 3))
        .expect("at least one layer must carry a model outline");

    // Centroid of a contour of the model cross-section: a point the planner
    // must never emit, because it is solid model there. `1 unit = 100 nm`, so
    // the divisor is 10_000 units per mm.
    let contour = &polys
        .iter()
        .find(|p| p.contour.points.len() >= 3)
        .expect("checked above")
        .contour;
    let n = contour.points.len() as f64;
    let cx = contour.points.iter().map(|p| p.x as f64).sum::<f64>() / n / 10_000.0;
    let cy = contour.points.iter().map(|p| p.y as f64).sum::<f64>() / n / 10_000.0;
    let injected = penetration_depth_mm(polys, cx, cy);
    assert!(
        injected > PENETRATION_TOLERANCE_MM,
        "injected defect at ({cx}, {cy}) mm on layer {idx} was NOT flagged \
         (depth {injected} mm) — the containment gate is vacuous"
    );

    // The predicate discriminates rather than always firing: a point well
    // clear of the part reads as zero penetration.
    let bbox = ctx.blackboard.mesh().build_volume;
    let clear = penetration_depth_mm(polys, bbox.max.x as f64 + 10.0, bbox.max.y as f64 + 10.0);
    assert_eq!(
        clear, 0.0,
        "a point 10 mm clear of the model bbox must not read as inside"
    );

    // Off-the-plate control: one runaway branch point makes the footprint
    // exceed the bed, so property 2 fires.
    let (bed_w, bed_d) = bed_extent_mm(&ctx.default_resolved_config);
    let runaway = (bed_w * 2.0, bed_d * 2.0);
    let (fw, fd) = footprint_extent_mm(&bbox, std::iter::once(runaway));
    assert!(
        !(fw <= bed_w && fd <= bed_d),
        "a branch at {runaway:?} mm yields a {fw} x {fd} mm footprint, which \
         must not fit the {bed_w} x {bed_d} mm bed"
    );
}

#[test]
fn wedge_support_plan_is_byte_deterministic_across_repeated_runs() {
    let a = owned_plan_entries();
    let b = owned_plan_entries();
    assert_eq!(a, b, "structural support plan must be deterministic");
}
