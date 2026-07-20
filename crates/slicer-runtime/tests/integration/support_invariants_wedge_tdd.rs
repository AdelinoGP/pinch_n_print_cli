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
