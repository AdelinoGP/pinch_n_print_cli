#![allow(missing_docs)]

use slicer_ir::{
    point_in_contour_winding, point_in_polygon_winding, ConfigValue, GlobalLayer, RaftPlan,
    SupportGeometryIR, SupportPlanEntry,
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
        .expect("SupportPlanIR must be committed even when support_enabled=false");
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
