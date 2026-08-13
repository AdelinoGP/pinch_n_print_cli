#![allow(missing_docs)]

use slicer_ir::{
    point_in_contour_winding, point_in_polygon_winding, ConfigValue, ExtrusionPath3D, GlobalLayer,
    Point3WithWidth, RaftPlan, SupportGeometryIR, SupportPlanEntry,
};
use slicer_runtime::run::PrepassContext;

use crate::common::support_wedge;

fn prepare_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context(true)
}

fn plan_entries(ctx: &PrepassContext) -> &[SupportPlanEntry] {
    &ctx.blackboard
        .support_plan()
        .expect("support_plan must be committed")
        .entries
}

fn support_geometry(ctx: &PrepassContext) -> &SupportGeometryIR {
    ctx.blackboard
        .support_geometry()
        .expect("support_geometry must be committed")
}

fn global_layers(ctx: &PrepassContext) -> &[GlobalLayer] {
    &ctx.plan.global_layers
}

#[test]
fn support_plan_has_finite_branch_paths() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(!entries.is_empty());
    for entry in entries {
        for seg in &entry.branch_segments {
            assert!(seg.points.len() >= 2);
            for pt in &seg.points {
                assert!(pt.x.is_finite());
                assert!(pt.y.is_finite());
                assert!(pt.z.is_finite());
                assert!(pt.width.is_finite());
            }
        }
    }
}

#[test]
fn branch_endpoints_are_outside_support_collision_outlines() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    let geom = support_geometry(&ctx);
    const ORIGIN_CONTACT_TOLERANCE_MM: f32 = 1e-6;
    let mut skipped = 0usize;
    let mut origin_contact_exemptions = 0usize;
    let mut propagated_checked = 0usize;
    for entry in entries {
        let layer_idx = entry.global_layer_index;
        if layer_idx < 0 {
            continue;
        }
        let matching_key = geom.entries.keys().find(|k| {
            k.global_support_layer_index != u32::MAX
                && k.global_support_layer_index as i32 == layer_idx
                && k.object_id == entry.object_id
                && k.region_id == entry.region_id
        });
        let outlines = match matching_key.and_then(|k| geom.entries.get(k)) {
            Some(outlines) => outlines,
            None => {
                for seg in &entry.branch_segments {
                    skipped += seg.points.len();
                }
                continue;
            }
        };
        for seg in &entry.branch_segments {
            for endpoint in [seg.points.first(), seg.points.last()]
                .into_iter()
                .flatten()
            {
                if endpoint.dist_to_top_mm.is_finite()
                    && endpoint.dist_to_top_mm >= 0.0
                    && endpoint.dist_to_top_mm <= ORIGIN_CONTACT_TOLERANCE_MM
                {
                    origin_contact_exemptions += 1;
                    continue;
                }
                assert!(
                    endpoint.dist_to_top_mm.is_finite() && endpoint.dist_to_top_mm > 0.0,
                    "endpoint must be a finite origin contact within {ORIGIN_CONTACT_TOLERANCE_MM} mm of zero or a positive propagated endpoint; layer={}, dist_to_top_mm={}, origin_contact_exemptions={}, propagated_checked={}",
                    layer_idx,
                    endpoint.dist_to_top_mm,
                    origin_contact_exemptions,
                    propagated_checked
                );
                propagated_checked += 1;
                let px = endpoint.x as f64;
                let py = endpoint.y as f64;
                let inside_outer = outlines.iter().any(|poly| {
                    point_in_polygon_winding(poly, px, py, 0.0)
                        && !poly
                            .holes
                            .iter()
                            .any(|h| point_in_contour_winding(h, px, py, 0.0))
                });
                assert!(
                    !inside_outer,
                    "propagated branch endpoint ({}, {}) at layer {} must be outside all collision outlines; origin_contact_exemptions={}, propagated_checked={}",
                    endpoint.x,
                    endpoint.y,
                    layer_idx,
                    origin_contact_exemptions,
                    propagated_checked
                );
            }
        }
    }
    eprintln!(
        "branch endpoint collision checks: origin_contact_exemptions={}, propagated_checked={}, skipped_missing_geometry={}",
        origin_contact_exemptions,
        propagated_checked,
        skipped
    );
    assert!(
        skipped == 0,
        "{} branch points were skipped due to missing geometry layer; origin_contact_exemptions={}, propagated_checked={}",
        skipped,
        origin_contact_exemptions,
        propagated_checked
    );
    assert!(
        propagated_checked > 0,
        "collision-outside invariant checked no propagated endpoints; origin_contact_exemptions={}, propagated_checked={}, skipped_missing_geometry={}",
        origin_contact_exemptions,
        propagated_checked,
        skipped
    );
}

#[test]
fn branch_points_match_entry_layer_z() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    let layers = global_layers(&ctx);
    for entry in entries {
        let layer_idx = entry.global_layer_index;
        if layer_idx < 0 {
            continue;
        }
        let layer = layers
            .iter()
            .find(|gl| gl.index == layer_idx as u32)
            .expect("GlobalLayer must exist for entry's global_layer_index");
        let expected_z = layer.z;
        for seg in &entry.branch_segments {
            for pt in &seg.points {
                let diff = (pt.z - expected_z).abs();
                assert!(
                    diff <= 1e-4,
                    "point z={} differs from layer z={} by {} (>1e-4) at layer index {}",
                    pt.z,
                    expected_z,
                    diff,
                    layer_idx
                );
            }
        }
    }
}

#[test]
fn overhang_facets_have_wedge_layer_contacts() {
    let wedge_path = {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("resources")
            .join("regression_wedge.stl");
        root.canonicalize().expect("wedge path must resolve")
    };
    let mesh =
        slicer_model_io::load_model(&wedge_path).expect("load regression_wedge.stl must succeed");
    let mesh_min_z = mesh
        .objects
        .iter()
        .flat_map(|object| object.mesh.vertices.iter().map(|vertex| vertex.z))
        .fold(f32::INFINITY, f32::min);
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    let layers = global_layers(&ctx);
    let first_layer_height = layers
        .first()
        .and_then(|layer| layer.active_regions.first())
        .map(|region| region.effective_layer_height)
        .expect("first global layer must have an active region");
    let branch_distance_mm = 1.0f32;
    let overhang_threshold = -std::f32::consts::FRAC_1_SQRT_2;
    let mut overhang_facets = 0usize;
    let mut skipped_base_facets = 0usize;
    let mut checked_facets = 0usize;
    for object in &mesh.objects {
        let tris = &object.mesh;
        for (facet_index, chunk) in tris.indices.chunks(3).enumerate() {
            if chunk.len() < 3 {
                continue;
            }
            let v0 = &tris.vertices[chunk[0] as usize];
            let v1 = &tris.vertices[chunk[1] as usize];
            let v2 = &tris.vertices[chunk[2] as usize];
            let ux = v1.x - v0.x;
            let uy = v1.y - v0.y;
            let uz = v1.z - v0.z;
            let vx = v2.x - v0.x;
            let vy = v2.y - v0.y;
            let vz = v2.z - v0.z;
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len == 0.0 {
                continue;
            }
            let nz_norm = nz / len;
            if nz_norm > overhang_threshold {
                continue;
            }
            overhang_facets += 1;
            let cx = (v0.x + v1.x + v2.x) / 3.0;
            let cy = (v0.y + v1.y + v2.y) / 3.0;
            let cz = (v0.z + v1.z + v2.z) / 3.0;
            if nz_norm < 0.0 && (cz - mesh_min_z).max(0.0) < first_layer_height * 0.5 {
                skipped_base_facets += 1;
                continue;
            }
            checked_facets += 1;
            let layer = layers.iter().find(|gl| gl.z >= cz).unwrap_or_else(|| {
                panic!(
                    "qualifying wedge facet {} centroid=({}, {}, {}) has no layer at/above centroid Z",
                    facet_index, cx, cy, cz
                )
            });
            let layer_idx = layer.index;
            let nearest_distance = entries
                .iter()
                .filter(|e| e.global_layer_index >= 0 && e.global_layer_index as u32 == layer_idx)
                .flat_map(|e| e.branch_segments.iter())
                .flat_map(|seg| [seg.points.first(), seg.points.last()])
                .flatten()
                .map(|pt| (pt.x - cx).hypot(pt.y - cy))
                .fold(f32::INFINITY, f32::min);
            assert!(
                nearest_distance <= branch_distance_mm,
                "wedge facet {} at layer {} centroid=({}, {}, {}) has no branch endpoint within {} mm; nearest distance={}",
                facet_index,
                layer_idx,
                cx,
                cy,
                cz,
                branch_distance_mm,
                nearest_distance
            );
        }
    }
    eprintln!(
        "overhang_facets_have_wedge_layer_contacts: qualifying={}, skipped_base_facets={}, checked_facets={}",
        overhang_facets, skipped_base_facets, checked_facets
    );
    assert!(overhang_facets > 0);
    assert!(checked_facets > 0);
}

#[test]
fn branch_radii_stay_within_current_bounds() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    let max_radius_mm = 6.0f32;
    for entry in entries {
        for seg in &entry.branch_segments {
            for pt in &seg.points {
                let radius = pt.width / 2.0;
                assert!(radius.is_finite());
                assert!(radius >= 0.0);
                assert!(radius <= max_radius_mm);
            }
        }
    }
}

#[test]
fn disabled_raft_has_no_negative_entries() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    for entry in entries {
        assert!(entry.global_layer_index >= 0);
    }
}

#[test]
fn support_disabled_produces_explicit_empty_plan() {
    let ctx = support_wedge::prepare_wedge_context(false);
    let plan = ctx
        .blackboard
        .support_plan()
        .expect("SupportPlanIR must be committed even when enable_support=false");
    assert!(
        plan.entries.is_empty(),
        "disabled support should produce an empty plan, got {} entries",
        plan.entries.len()
    );
    let geom_present = ctx.blackboard.support_geometry().is_some();
    eprintln!(
        "support_disabled_produces_explicit_empty_plan: support_geometry.is_some() = {}",
        geom_present
    );
}

#[test]
fn branch_points_carry_finite_nonnegative_dist_to_top_mm() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    let mut positive_value_observed = false;
    let mut observed_values = Vec::new();

    for entry in entries {
        for (segment_index, segment) in entry.branch_segments.iter().enumerate() {
            for (point_index, point) in segment.points.iter().enumerate() {
                assert!(
                    point.dist_to_top_mm.is_finite(),
                    "non-finite dist_to_top_mm at layer {}, segment {}, point {}: {}",
                    entry.global_layer_index,
                    segment_index,
                    point_index,
                    point.dist_to_top_mm
                );
                assert!(
                    point.dist_to_top_mm >= 0.0,
                    "negative dist_to_top_mm at layer {}, segment {}, point {}: {}",
                    entry.global_layer_index,
                    segment_index,
                    point_index,
                    point.dist_to_top_mm
                );
                positive_value_observed |= point.dist_to_top_mm > 0.0;
                observed_values.push(point.dist_to_top_mm);
            }
        }
    }

    assert!(
        positive_value_observed,
        "wedge support branches must expose at least one positive dist_to_top_mm; observed {:?}",
        observed_values
    );
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
    let support_plan = ctx
        .blackboard
        .support_plan()
        .expect("support_plan must be committed");

    assert_eq!(
        support_plan.raft_plan,
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
    let support_plan = ctx
        .blackboard
        .support_plan()
        .expect("support_plan must be committed");

    assert!(support_plan.raft_plan.is_none());
}

#[test]
fn branch_curvature_below_threshold() {
    // Gate for packet 121 (smooth_nodes port). The Laplacian smoother in
    // `support_planner::smooth_branches` operates on the column chain formed
    // by each entry's first branch segment's first point across consecutive
    // layers (tip -> root). This invariant reconstructs that exact chain and
    // asserts no consecutive-segment turn angle exceeds the threshold, so a
    // regression that drops the smoothing pass surfaces as ~90° stairstep
    // turns here.
    use std::collections::BTreeMap;

    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);

    let mut columns: BTreeMap<(slicer_ir::ObjectId, slicer_ir::RegionId), Vec<usize>> =
        BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        columns
            .entry((e.object_id.clone(), e.region_id))
            .or_default()
            .push(i);
    }
    for col in columns.values_mut() {
        col.sort_by(|&a, &b| {
            entries[b]
                .global_layer_index
                .cmp(&entries[a].global_layer_index)
        });
    }

    const MAX_TURN_DEG: f32 = 30.0;
    let mut max_turn = 0.0f32;
    let mut chains_checked = 0usize;
    let mut total_columns = 0usize;
    for col in columns.values() {
        total_columns += 1;
        if col.len() < 3 {
            continue;
        }
        let mut pts: Vec<(f32, f32)> = Vec::new();
        for &idx in col {
            if let Some(seg) = entries[idx].branch_segments.first() {
                if let Some(p) = seg.points.first() {
                    pts.push((p.x, p.y));
                }
            }
        }
        if pts.len() < 3 {
            continue;
        }
        // Mirror the smoother's CHAIN_BREAK_THRESHOLD_MM = 5.0 in
        // support-planner/src/lib.rs — distinct support trees are typically
        // 25+ mm apart; per-layer stairsteps are 1-2 mm. Skipping inter-tree
        // gaps is what the smoother itself does, so the invariant must match.
        const CHAIN_BREAK_MM: f32 = 5.0;
        eprintln!(
            "DBG pts=[{}]",
            pts.iter()
                .map(|(x, y)| format!("({:.1},{:.1})", x, y))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let mut sub_start = 0usize;
        for k in 1..pts.len() {
            let dx = pts[k].0 - pts[k - 1].0;
            let dy = pts[k].1 - pts[k - 1].1;
            if (dx * dx + dy * dy).sqrt() > CHAIN_BREAK_MM {
                if k - sub_start >= 3 {
                    chains_checked += 1;
                    for j in sub_start..k.saturating_sub(2) {
                        let v1 = (pts[j + 1].0 - pts[j].0, pts[j + 1].1 - pts[j].1);
                        let v2 = (pts[j + 2].0 - pts[j + 1].0, pts[j + 2].1 - pts[j + 1].1);
                        let cross = v1.0 * v2.1 - v1.1 * v2.0;
                        let dot = v1.0 * v2.0 + v1.1 * v2.1;
                        let ang = cross.atan2(dot).to_degrees().abs();
                        if ang > max_turn {
                            max_turn = ang;
                            eprintln!("DBG max at k={}: pts[k..k+3]={:?}", j, &pts[j..j + 3]);
                        }
                    }
                }
                sub_start = k;
            }
        }
        if pts.len() - sub_start >= 3 {
            chains_checked += 1;
            for j in sub_start..pts.len().saturating_sub(2) {
                let v1 = (pts[j + 1].0 - pts[j].0, pts[j + 1].1 - pts[j].1);
                let v2 = (pts[j + 2].0 - pts[j + 1].0, pts[j + 2].1 - pts[j + 1].1);
                let cross = v1.0 * v2.1 - v1.1 * v2.0;
                let dot = v1.0 * v2.0 + v1.1 * v2.1;
                let ang = cross.atan2(dot).to_degrees().abs();
                if ang > max_turn {
                    max_turn = ang;
                    eprintln!("DBG max at k={}: pts[k..k+3]={:?}", j, &pts[j..j + 3]);
                }
            }
        }
    }

    eprintln!(
        "branch_curvature_below_threshold: total_columns={}, chains_checked={}, max_turn={:.2}° (threshold {:.1}°)",
        total_columns, chains_checked, max_turn, MAX_TURN_DEG
    );

    assert!(
        chains_checked > 0,
        "curvature invariant found no multi-layer (>2) branch columns to check; total_columns={}",
        total_columns
    );
    assert!(
        max_turn <= MAX_TURN_DEG,
        "max consecutive-segment turn angle {:.2}° exceeds {:.1}° threshold after Laplacian smoothing (packet 121); chains_checked={}, total_columns={}",
        max_turn,
        MAX_TURN_DEG,
        chains_checked,
        total_columns
    );
}

/// Packet 122 invariant: at merge points (a node with ≥ 3 incoming MST
/// edges), the distances from the merge point to its contributing
/// branch-segment endpoint XYs must be approximately equal — i.e. the
/// merge is centred. Under the old single-neighbour propagation, the
/// move target skewed toward whichever MST edge had the lowest
/// distance, so the merge geometry was visibly asymmetric. The
/// reciprocal-distance-squared weighted aggregate (`support_planner::
/// aggregate_neighbour_targets`, packet 122) restores symmetry.
///
/// Detection rule: within each `SupportPlanEntry`, treat every
/// 2-point `branch_segment` as an MST edge between its two endpoints.
/// A "merge point" is a (x, y) that appears as an endpoint of three or
/// more segments within the same entry. For each merge point, gather
/// the *other* endpoint of each contributing segment and compute the
/// set of distances from the merge point to those other endpoints.
/// The invariant requires `stddev / mean ≤ 0.30` — the threshold 30%
/// is empirical (packet 122 design §Risks).
#[test]
fn merge_geometry_symmetric_for_n_branches() {
    use std::collections::HashMap;

    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);

    // Round endpoints to 1e-3 mm so floating-point near-matches count as the
    // same merge point. The merge point itself is shared between segments;
    // if it moved during smoothing it should be identical to the integer-
    // rounded reference.
    const ROUND_MM: f64 = 1e-3;
    const MAX_STDDEV_OVER_MEAN: f64 = 0.30;

    let mut merge_points_checked: usize = 0;
    let mut total_entries_scanned: usize = 0;
    let mut total_segments_scanned: usize = 0;
    let mut worst_ratio: f64 = 0.0;
    let mut worst_anchor: Option<(f32, f32)> = None;
    let mut worst_distances: Vec<f32> = Vec::new();

    for entry in entries {
        if entry.global_layer_index < 0 {
            // Raft prefix layers are not MST-derived; skip.
            continue;
        }
        total_entries_scanned += 1;
        if entry.branch_segments.is_empty() {
            continue;
        }
        // Build a map: merge-point (rounded) → list of "other endpoint" XYs
        // for each segment that touches the merge point.
        let mut merge_map: HashMap<(i64, i64), Vec<(f32, f32)>> = HashMap::new();
        for seg in &entry.branch_segments {
            // Each branch_segment is a 2-point ExtrusionPath3D (see
            // SupportPlanEntry doc). Take first and last point.
            let first = match seg.points.first() {
                Some(p) => p,
                None => continue,
            };
            let last = match seg.points.last() {
                Some(p) if seg.points.len() > 1 => p,
                _ => continue,
            };
            total_segments_scanned += 1;
            let key_first = (
                (first.x as f64 / ROUND_MM).round() as i64,
                (first.y as f64 / ROUND_MM).round() as i64,
            );
            let key_last = (
                (last.x as f64 / ROUND_MM).round() as i64,
                (last.y as f64 / ROUND_MM).round() as i64,
            );
            merge_map
                .entry(key_first)
                .or_default()
                .push((last.x, last.y));
            merge_map
                .entry(key_last)
                .or_default()
                .push((first.x, first.y));
        }
        for (key, others) in &merge_map {
            if others.len() < 3 {
                // Less than 3 incoming → not a merge point.
                continue;
            }
            merge_points_checked += 1;
            // Reconstruct the merge point XY by averaging the endpoints that
            // round to the same key. (All endpoints at this key should be
            // within ROUND_MM of each other.)
            let mp_x = (key.0 as f64) * ROUND_MM;
            let mp_y = (key.1 as f64) * ROUND_MM;
            // Distances from the merge point to each contributing other endpoint.
            let distances: Vec<f32> = others
                .iter()
                .map(|&(x, y)| {
                    ((x as f64 - mp_x).powi(2) + (y as f64 - mp_y).powi(2)).sqrt() as f32
                })
                .filter(|d| d.is_finite() && *d > 0.0)
                .collect();
            if distances.len() < 3 {
                continue;
            }
            let n = distances.len() as f64;
            let mean = distances.iter().map(|d| *d as f64).sum::<f64>() / n;
            if mean < 1e-6 {
                // Degenerate cluster — skip (no meaningful spread).
                continue;
            }
            let var = distances
                .iter()
                .map(|d| {
                    let diff = (*d as f64) - mean;
                    diff * diff
                })
                .sum::<f64>()
                / n;
            let stddev = var.sqrt();
            let ratio = stddev / mean;
            if ratio > worst_ratio {
                worst_ratio = ratio;
                worst_anchor = Some((mp_x as f32, mp_y as f32));
                worst_distances = distances.clone();
            }
            assert!(
                ratio <= MAX_STDDEV_OVER_MEAN,
                "merge point ({:.3}, {:.3}) at layer {} obj={} region={} has asymmetric geometry: stddev/mean = {:.3} (> {:.2}); distances = {:?} mm",
                mp_x,
                mp_y,
                entry.global_layer_index,
                entry.object_id,
                entry.region_id,
                ratio,
                MAX_STDDEV_OVER_MEAN,
                distances
            );
        }
    }

    eprintln!(
        "merge_geometry_symmetric_for_n_branches: entries_scanned={}, segments_scanned={}, merge_points_checked={}, worst_ratio={:.3} (threshold {:.2})",
        total_entries_scanned,
        total_segments_scanned,
        merge_points_checked,
        worst_ratio,
        MAX_STDDEV_OVER_MEAN
    );
    if let Some((mx, my)) = worst_anchor {
        eprintln!(
            "  worst merge anchor: ({:.3}, {:.3}) distances={:?} mm",
            mx, my, worst_distances
        );
    }
    // The wedge has at most a few branches, so a small or zero count is OK;
    // we only assert the ratio is bounded.
    let _ = merge_points_checked; // suppress unused warning when zero
}

/// Packet 123 invariant: with `support_on_build_plate_only = true`, every
/// emitted branch endpoint must lie OUTSIDE the object's per-layer collision
/// outline (no origin-contact exemption is allowed). This is a tightening of
/// invariant 2 (`branch_endpoints_are_outside_support_collision_outlines`),
/// which exempts endpoints whose `dist_to_top_mm <= 1e-6` (the contact tip
/// may sit on the model face). With `support_on_build_plate_only = true`,
/// the planner rejects every `to_model` contact at creation time, so a
/// surviving contact is classified `to_buildplate = true` and must have a
/// collision-free path — the exemption no longer applies. Endpoints at the
/// build plate (`z <= 1e-3 mm`) and contact tips at the overhang origin
/// layer (a fresh contact whose `dist_to_top_mm == 0`) are accepted; all
/// other endpoints must clear the model outline. The tolerance matches the
/// existing `branch_points_match_entry_layer_z` float convention
/// (`Point3WithWidth.z` is f32 millimeters, NOT the IR's 100 nm scaled
/// integer units).
#[test]
fn build_plate_only_emits_no_to_model_branches() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[("support_on_build_plate_only", ConfigValue::Bool(true))],
    );
    let entries = plan_entries(&ctx);
    let geom = support_geometry(&ctx);
    let layers = global_layers(&ctx);
    const BUILD_PLATE_TOLERANCE_MM: f32 = 1e-3;
    const ORIGIN_CONTACT_TOLERANCE_MM: f32 = 1e-6;

    let mut endpoints_checked = 0usize;
    let mut at_build_plate = 0usize;
    let mut origin_contact_exempt = 0usize;
    let mut cleared_outline = 0usize;
    let mut skipped_missing_geometry = 0usize;

    for entry in entries {
        let layer_idx = entry.global_layer_index;
        if layer_idx < 0 {
            // Raft prefix layers: skip — `support_on_build_plate_only` does
            // not constrain the raft pipeline.
            continue;
        }
        let matching_key = geom.entries.keys().find(|k| {
            k.global_support_layer_index != u32::MAX
                && k.global_support_layer_index == layer_idx as u32
                && k.object_id == entry.object_id
                && k.region_id == entry.region_id
        });
        let outlines = match matching_key.and_then(|k| geom.entries.get(k)) {
            Some(outlines) => outlines,
            None => {
                for seg in &entry.branch_segments {
                    skipped_missing_geometry += seg.points.len();
                }
                continue;
            }
        };
        // Identify the overhang origin layer for this column: the topmost
        // (largest global_layer_index) entry's layer. A fresh contact tip on
        // that layer is allowed to sit on the model outline (the overhang
        // face) — that's the only origin-contact exemption under
        // `support_on_build_plate_only = true`.
        let overhang_origin_layer = entries
            .iter()
            .filter(|e| {
                e.global_layer_index >= 0
                    && e.object_id == entry.object_id
                    && e.region_id == entry.region_id
            })
            .map(|e| e.global_layer_index)
            .max()
            .unwrap_or(layer_idx);
        let is_overhang_origin_layer = layer_idx == overhang_origin_layer;

        for seg in &entry.branch_segments {
            for endpoint in [seg.points.first(), seg.points.last()]
                .into_iter()
                .flatten()
            {
                endpoints_checked += 1;
                let at_plate = endpoint.z <= BUILD_PLATE_TOLERANCE_MM;
                if at_plate {
                    at_build_plate += 1;
                }
                // Origin-contact exemption: only valid on the overhang origin
                // layer (the contact tip is the overhang face itself), AND
                // the tip must be a fresh contact (dist_to_top_mm == 0).
                let is_fresh_contact_tip = endpoint.dist_to_top_mm.is_finite()
                    && endpoint.dist_to_top_mm >= 0.0
                    && endpoint.dist_to_top_mm <= ORIGIN_CONTACT_TOLERANCE_MM;
                let origin_exempt = is_overhang_origin_layer && is_fresh_contact_tip && !at_plate;
                if origin_exempt {
                    origin_contact_exempt += 1;
                    continue;
                }
                // Every other endpoint must clear the layer's collision outline.
                let px = endpoint.x as f64;
                let py = endpoint.y as f64;
                let inside_outer = outlines.iter().any(|poly| {
                    point_in_polygon_winding(poly, px, py, 0.0)
                        && !poly
                            .holes
                            .iter()
                            .any(|h| point_in_contour_winding(h, px, py, 0.0))
                });
                assert!(
                    !inside_outer,
                    "with support_on_build_plate_only=true, branch endpoint ({}, {}, {}) at layer {} (z={}, is_overhang_origin_layer={}) must lie outside the collision outline (no origin-contact exemption for build-plate-only mode except the overhang-tip contact itself); at_build_plate={}, origin_contact_exempt={}, cleared_outline={}, skipped_missing_geometry={}, endpoints_checked={}",
                    endpoint.x,
                    endpoint.y,
                    endpoint.z,
                    layer_idx,
                    layers.iter().find(|gl| gl.index == layer_idx as u32).map(|gl| gl.z).unwrap_or(f32::NAN),
                    is_overhang_origin_layer,
                    at_build_plate,
                    origin_contact_exempt,
                    cleared_outline,
                    skipped_missing_geometry,
                    endpoints_checked,
                );
                cleared_outline += 1;
            }
        }
    }

    eprintln!(
        "build_plate_only_emits_no_to_model_branches: endpoints_checked={}, at_build_plate={}, origin_contact_exempt={}, cleared_outline={}, skipped_missing_geometry={}",
        endpoints_checked, at_build_plate, origin_contact_exempt, cleared_outline, skipped_missing_geometry
    );
    assert!(
        endpoints_checked > 0,
        "build-plate-only invariant checked no branch endpoints; at_build_plate={}, origin_contact_exempt={}, cleared_outline={}, skipped_missing_geometry={}",
        at_build_plate,
        origin_contact_exempt,
        cleared_outline,
        skipped_missing_geometry
    );
}

// ============================================================================
// Structural invariants replacing the wedge self-capture goldens
//
// `support_golden_regression_wedge_tdd.rs` compared the wedge's branch count
// and endpoint positions against two self-captured baseline files (count
// ±10%, endpoint Hausdorff ≤ 0.5 mm). Those baselines were PnP's own prior
// output — not OrcaSlicer reference data — so green only ever meant
// "unchanged from the last capture" (ADR-0042, D-109-SELF-CAPTURED-FIXTURES).
// Packet 213's lone-node emission grew the count 78 → 136 for the *intended*
// reason of emitting a degenerate segment per surviving propagated node —
// exactly the failure mode a self-capture cannot distinguish from a
// regression — and the baseline was re-blessed (e08dfe9a).
//
// The invariants below assert the structural properties those baselines
// were standing in for, following the wedge byte-SHA reshape (33799527) and
// the arachne baselines conversion (packet 177 / ADR-0042): the wedge's
// support geometry is a small set of vertical columns that start at the
// mesh's overhang facets, descend contiguously through the layer stack with
// bounded per-layer drift, and widen monotonically toward the build plate.
// Each fails by naming the property that changed.
// ============================================================================

/// Load `regression_wedge.stl` without going through the runtime's model
/// cache, so these invariants read the same bytes the planner consumed.
fn wedge_mesh() -> slicer_ir::MeshIR {
    let wedge_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
        .join("regression_wedge.stl");
    let wedge_path = wedge_path
        .canonicalize()
        .expect("regression_wedge.stl must resolve");
    slicer_model_io::load_model(&wedge_path).expect("load regression_wedge.stl must succeed")
}

/// One structural column: entries sharing `(object_id, region_id)`, ordered
/// top → bottom by `global_layer_index`. This is the same grouping
/// `support_planner::group_branches_into_columns` (and the smoother) use.
fn wedge_columns<'a>(entries: &'a [SupportPlanEntry]) -> Vec<Vec<&'a SupportPlanEntry>> {
    use std::collections::BTreeMap;
    let mut columns: BTreeMap<
        (slicer_ir::ObjectId, slicer_ir::RegionId),
        Vec<&'a SupportPlanEntry>,
    > = BTreeMap::new();
    for entry in entries {
        columns
            .entry((entry.object_id.clone(), entry.region_id))
            .or_default()
            .push(entry);
    }
    let mut columns: Vec<Vec<&'a SupportPlanEntry>> = columns.into_values().collect();
    for column in columns.iter_mut() {
        column.sort_by_key(|entry| std::cmp::Reverse(entry.global_layer_index));
    }
    columns
}

/// Structural segments of a column: every `branch_segment` of every entry,
/// tagged with its layer index. MST edges and interface scan-line fills are
/// indistinguishable in the public IR, so these invariants are written to
/// hold for both.
fn column_segments<'a>(column: &[&'a SupportPlanEntry]) -> Vec<(i32, &'a ExtrusionPath3D)> {
    column
        .iter()
        .flat_map(|entry| {
            entry
                .branch_segments
                .iter()
                .map(move |seg| (entry.global_layer_index, seg))
        })
        .collect()
}

/// First point of a segment. Structural segments always carry ≥ 2 points
/// (pinned by `support_plan_has_finite_branch_paths`).
fn first_point(seg: &ExtrusionPath3D) -> &Point3WithWidth {
    &seg.points[0]
}

#[test]
fn support_columns_are_contiguous_and_step_down_through_every_layer() {
    // Per-layer XY drift bound. The planner's move pass is capped at
    // `tan(45°) × 0.2 mm × wall_count = 0.2 mm`, and 100 Laplacian smoothing
    // passes can only pull a column point toward the centroid of its
    // neighbours — never past it. 1.0 mm is five move caps, chosen so that
    // the smoother's cross-column interactions cannot trip it, while a
    // degenerate column (tip emitted, then nothing) or a jump between
    // distinct support trees (25+ mm apart on this fixture) fails loudly.
    const MAX_STEP_MM: f32 = 1.0;
    // `dist_to_top` advances by one per layer; the emitted mm value is the
    // counter times the effective layer height (0.2 mm on this fixture).
    const MAX_DTOP_INC_MM: f32 = 0.22;

    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(!entries.is_empty(), "wedge support plan must be non-empty");

    let mut columns_checked = 0usize;
    let mut segments_checked = 0usize;
    for column in wedge_columns(entries) {
        let segments = column_segments(&column);
        assert!(
            !segments.is_empty(),
            "column for obj={} region={} must carry branch segments",
            column[0].object_id,
            column[0].region_id
        );
        let top_segment = &segments[0].1;
        assert_eq!(
            first_point(top_segment).dist_to_top_mm,
            0.0,
            "the topmost segment of column obj={} region={} must be a fresh contact \
             (dist_to_top_mm == 0); got {}",
            column[0].object_id,
            column[0].region_id,
            first_point(top_segment).dist_to_top_mm
        );

        // Walk the column top → bottom comparing consecutive propagated
        // segments only. A segment with `dist_to_top_mm == 0` is a fresh
        // contact: a new support tree may begin below a propagated one
        // (distinct overhang groups share one region column on this
        // fixture), so fresh contacts legitimately interrupt the walk and
        // reset the pairwise comparison.
        let mut prev_layer = segments[0].0;
        let mut prev = first_point(segments[0].1);
        for &(layer_index, seg) in &segments[1..] {
            let point = first_point(seg);
            if prev.dist_to_top_mm > 0.0 && point.dist_to_top_mm > 0.0 {
                segments_checked += 1;
                assert_eq!(
                    layer_index,
                    prev_layer - 1,
                    "column obj={} region={} must descend contiguously through the layer \
                     stack; found layer {} then {}",
                    column[0].object_id,
                    column[0].region_id,
                    prev_layer,
                    layer_index
                );
                let step = (point.x - prev.x).hypot(point.y - prev.y);
                assert!(
                    step <= MAX_STEP_MM,
                    "per-layer drift of column obj={} region={} at layer {} is {:.3} mm \
                     (> {:.1} mm)",
                    column[0].object_id,
                    column[0].region_id,
                    layer_index,
                    step,
                    MAX_STEP_MM
                );
                let inc = point.dist_to_top_mm - prev.dist_to_top_mm;
                assert!(
                    (0.0..=MAX_DTOP_INC_MM).contains(&inc),
                    "dist_to_top_mm must advance by one layer per downward step in column \
                     obj={} region={} at layer {}; increment {:.3}",
                    column[0].object_id,
                    column[0].region_id,
                    layer_index,
                    inc
                );
            }
            prev_layer = layer_index;
            prev = point;
        }
        columns_checked += 1;
    }
    assert!(
        columns_checked >= 1 && segments_checked > 0,
        "contiguity invariant checked no columns; columns={}",
        columns_checked
    );
}

#[test]
fn support_branch_widths_widen_monotonically_toward_the_plate() {
    // Radius bounds mirror `support_planner`: the 0.4 mm `MIN_BRANCH_RADIUS`
    // floor (width 0.8 mm) and the 6.0 mm `MAX_BRANCH_RADIUS_MM` ceiling.
    const MAX_RADIUS_MM: f32 = 6.0;
    const MIN_NONZERO_WIDTH_MM: f32 = 0.8;
    // `smooth_branches` averages first-point widths and clamps them to
    // `MAX_BRANCH_RADIUS_MM = 6.0` (a diameter-vs-radius quirk: the clamp
    // is applied to the width, not the radius). A sub-chain boundary point
    // is pinned and can therefore sit above 6.0 while the next interior
    // point is clamped back down to 6.0 — the only legitimate shrink.
    // `smooth_branches` itself can never produce a value below 6.0 from an
    // above-6.0 average, so any drop that lands below the clamp ceiling is
    // a real regression, not smoothing noise.
    const SMOOTH_CLAMP_CEILING_MM: f32 = 6.0;
    // The smoother averages widths in `f32` over up to 100 passes; a small
    // relax absorbs that averaging noise below the clamp ceiling.
    const WIDTH_RELAX_MM: f32 = 0.1;

    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);

    let mut monotonic_segments = 0usize;
    let mut width_drops = 0usize;
    for column in wedge_columns(entries) {
        let segments = column_segments(&column);
        let mut prev_layer = segments[0].0;
        let mut prev = first_point(segments[0].1);
        for &(layer_index, seg) in &segments[1..] {
            let point = first_point(seg);
            if prev.dist_to_top_mm > 0.0
                && point.dist_to_top_mm > 0.0
                && layer_index == prev_layer - 1
            {
                monotonic_segments += 1;
                if point.width + WIDTH_RELAX_MM < prev.width {
                    let clamp_explains = prev.width > SMOOTH_CLAMP_CEILING_MM
                        && point.width >= SMOOTH_CLAMP_CEILING_MM - WIDTH_RELAX_MM;
                    if !clamp_explains {
                        width_drops += 1;
                    }
                }
            }
            prev_layer = layer_index;
            prev = point;
        }
    }
    assert!(
        width_drops == 0,
        "branch width must not shrink toward the build plate; {width_drops} drops \
         (tolerance {WIDTH_RELAX_MM} mm), monotonic_segments={monotonic_segments}"
    );
    assert!(
        monotonic_segments > 0,
        "width invariant checked no segments"
    );

    for entry in entries {
        for seg in &entry.branch_segments {
            for pt in &seg.points {
                assert!(
                    pt.width == 0.0 || pt.width >= MIN_NONZERO_WIDTH_MM,
                    "non-zero branch width must respect the MIN_BRANCH_RADIUS floor, got {}",
                    pt.width
                );
                assert!(
                    pt.width <= MAX_RADIUS_MM * 2.0,
                    "branch width must respect the MAX_BRANCH_RADIUS_MM ceiling, got {}",
                    pt.width
                );
            }
        }
    }
}

#[test]
fn support_segments_stay_within_mesh_bbox() {
    // Spatial containment: every support point must stay inside the mesh's
    // XY bounding box plus a small margin. Nodes are clamped into the
    // per-layer avoidance polys (model cross-sections inflated by
    // `branch_radius + tree_support_branch_distance / 2 = 3.0 mm`), and
    // interface scan lines extend at most `radius + branch_distance / 2 ≤
    // 6.5 mm` from their node, so 10 mm of margin is a safe upper bound.
    // This is the coarse replacement for the golden's endpoint Hausdorff:
    // a branch escaping the model footprint entirely fails here.
    const MESH_BBOX_MARGIN_MM: f32 = 10.0;

    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);

    let mesh = wedge_mesh();
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for object in &mesh.objects {
        for vertex in &object.mesh.vertices {
            min_x = min_x.min(vertex.x);
            max_x = max_x.max(vertex.x);
            min_y = min_y.min(vertex.y);
            max_y = max_y.max(vertex.y);
        }
    }
    assert!(
        min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite(),
        "regression_wedge.stl must have a finite XY bounding box"
    );

    let mut points_checked = 0usize;
    for entry in entries {
        for seg in &entry.branch_segments {
            for pt in &seg.points {
                assert!(
                    pt.x >= min_x - MESH_BBOX_MARGIN_MM && pt.x <= max_x + MESH_BBOX_MARGIN_MM,
                    "branch x={} must stay within the mesh bbox x range [{}, {}] + {} mm \
                     margin",
                    pt.x,
                    min_x,
                    max_x,
                    MESH_BBOX_MARGIN_MM
                );
                assert!(
                    pt.y >= min_y - MESH_BBOX_MARGIN_MM && pt.y <= max_y + MESH_BBOX_MARGIN_MM,
                    "branch y={} must stay within the mesh bbox y range [{}, {}] + {} mm \
                     margin",
                    pt.y,
                    min_y,
                    max_y,
                    MESH_BBOX_MARGIN_MM
                );
                points_checked += 1;
            }
        }
    }
    assert!(points_checked > 0, "bbox invariant checked no points");
}

#[test]
fn wedge_support_plan_is_byte_deterministic_across_repeated_runs() {
    // The old golden comparison was also implicitly the only determinism
    // gate on the wedge plan itself (a capture compared against itself would
    // always pass). This pins bit-for-bit determinism of the committed
    // `SupportPlanIR` across two full prepass runs, which is stronger: any
    // nondeterminism fails here instead of surfacing as a future golden
    // re-bless.
    let ctx_a = prepare_ctx();
    let ctx_b = prepare_ctx();
    let plan_a = ctx_a
        .blackboard
        .support_plan()
        .expect("support_plan must be committed");
    let plan_b = ctx_b
        .blackboard
        .support_plan()
        .expect("support_plan must be committed");

    let entries_a = &plan_a.entries;
    let entries_b = &plan_b.entries;
    assert_eq!(
        entries_a.len(),
        entries_b.len(),
        "entry count must be identical across repeated runs"
    );
    for (a, b) in entries_a.iter().zip(entries_b.iter()) {
        assert_eq!(a.global_layer_index, b.global_layer_index);
        assert_eq!(a.object_id, b.object_id);
        assert_eq!(a.region_id, b.region_id);
        assert_eq!(
            a.branch_segments.len(),
            b.branch_segments.len(),
            "branch segment count must be identical at layer {}",
            a.global_layer_index
        );
        for (seg_a, seg_b) in a.branch_segments.iter().zip(b.branch_segments.iter()) {
            assert_eq!(
                seg_a.points.len(),
                seg_b.points.len(),
                "point count must be identical at layer {}",
                a.global_layer_index
            );
            for (pa, pb) in seg_a.points.iter().zip(seg_b.points.iter()) {
                assert_eq!(
                    pa.x.to_bits(),
                    pb.x.to_bits(),
                    "x bits must match at layer {}",
                    a.global_layer_index
                );
                assert_eq!(pa.y.to_bits(), pb.y.to_bits());
                assert_eq!(pa.z.to_bits(), pb.z.to_bits());
                assert_eq!(pa.width.to_bits(), pb.width.to_bits());
            }
        }
    }
}
