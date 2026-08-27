//! The drawn tree-support footprint must keep `support_object_xy_distance`
//! away from the model wall.
//!
//! Canonical `draw_circles` (`TreeSupport.cpp`) carves every drawn circle out
//! of its own local `get_collision` lambda, which returns
//! `offset_ex(m_layer_outlines[obj_layer_nr], scale_(m_xy_distance))` — the
//! object outline **inflated by `m_xy_distance`**, never the bare outline.
//!
//! This module prefers `SupportAnalysisView::model_occupancy` over the
//! `TreeVolumes` ladder for that carve because it is the exact per-layer
//! occupancy. `model_occupancy` carries the RAW `SliceIR` region polygons
//! (`support_analysis_producer` inserts `region.polygons` verbatim), so before
//! `inflate_model_occupancy` the carve degenerated into a difference against
//! the wall itself and the printed footprint ended up flush against the model.

use std::collections::HashMap;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportAnalysisCandidate, SupportAnalysisGeometryEntry,
    SupportAnalysisView, SupportGeometryView, SupportGeometryViewEntry,
};
use slicer_sdk::traits::PrepassModule;
use tree_support_planner::{carve_emitted_regions, point_inside_collision_volume};

/// Canonical `m_xy_distance` default, in mm — the planner's
/// `support_object_xy_distance` fallback.
const XY_DISTANCE_MM: f64 = 0.35;
/// Slack for the polyline approximation of the offset boundary and for the
/// `RESOLUTION`-tolerance simplify `draw_circles` applies to every drawn area.
const TOLERANCE_MM: f64 = 0.03;

const OBJECT_ID: &str = "wall-clearance";
const REGION_ID: &str = "0";
const LAYERS: u32 = 10;

fn planner_config() -> ConfigView {
    planner_config_with_diameter(2.0)
}

fn planner_config_with_diameter(branch_diameter: f64) -> ConfigView {
    let mut values: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    values.insert(ConfigKey::from("enable_support"), ConfigValue::Bool(true));
    values.insert(
        ConfigKey::from("support_type"),
        ConfigValue::String("tree(auto)".into()),
    );
    values.insert(
        ConfigKey::from("tree_support_branch_diameter"),
        ConfigValue::Float(branch_diameter),
    );
    ConfigView::from_map(values)
}

fn layer_plan() -> LayerPlanView {
    LayerPlanView {
        layers: (0..LAYERS)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: (i as f32 + 1.0) * 0.2,
                effective_layer_height: 0.2,
            })
            .collect(),
    }
}

fn regions() -> RegionSegmentationView {
    RegionSegmentationView {
        entries: (0..LAYERS)
            .map(|layer_index| RegionSegmentationViewEntry {
                object_id: OBJECT_ID.into(),
                layer_index,
                region_ids: vec![REGION_ID.into()],
            })
            .collect(),
        region_support_configs: vec![],
    }
}

/// A flat overhang plate at z = 1.8 spanning x, y in [0, 4].
fn overhang_object() -> MeshObjectView {
    MeshObjectView {
        object_id: OBJECT_ID.into(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.8],
            [4.0, 0.0, 1.8],
            [4.0, 4.0, 1.8],
            [0.0, 4.0, 1.8],
        ],
        triangles: vec![[1, 3, 2], [1, 4, 3]],
        paint_layers: vec![],
    }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: vec![],
    }
}

/// The model wall the branches run alongside: solid for x >= 2.0 on every
/// layer, overlapping the right half of the overhang the branches carry.
fn wall() -> ExPolygon {
    rect(2.0, -3.0, 9.0, 7.0)
}

fn analysis() -> SupportAnalysisView {
    SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: OBJECT_ID.into(),
            region_id: REGION_ID.into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![rect(0.0, 0.0, 4.0, 4.0)],
            ..Default::default()
        }],
        model_occupancy: (0..LAYERS)
            .map(|layer| SupportAnalysisGeometryEntry {
                global_support_layer_index: layer,
                object_id: OBJECT_ID.into(),
                region_id: REGION_ID.into(),
                polygons: vec![wall()],
            })
            .collect(),
        ..Default::default()
    }
}

fn run() -> SupportGeometryOutput {
    run_with(&analysis(), &SupportGeometryView::default())
}

fn run_with(
    analysis: &SupportAnalysisView,
    support_geometry: &SupportGeometryView,
) -> SupportGeometryOutput {
    run_with_config(analysis, support_geometry, &planner_config())
}

fn run_with_config(
    analysis: &SupportAnalysisView,
    support_geometry: &SupportGeometryView,
    config: &ConfigView,
) -> SupportGeometryOutput {
    let planner = tree_support_planner::SupportPlanner::from_config(config).expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[overhang_object()],
            &layer_plan(),
            &regions(),
            analysis,
            support_geometry,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

fn polygon_area(region: &ExPolygon) -> f64 {
    let points = &region.contour.points;
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64)
        .sum::<f64>()
        .abs()
        * 0.5
        / 100_000_000.0
}

fn point_segment_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    ((px - (ax + t * dx)).powi(2) + (py - (ay + t * dy)).powi(2)).sqrt()
}

/// Distance in mm from `(x, y)` to the boundary of `poly`.
fn distance_to_boundary(poly: &ExPolygon, x: f64, y: f64) -> f64 {
    let points = &poly.contour.points;
    let n = points.len();
    let mut best = f64::MAX;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        best = best.min(point_segment_distance(
            x,
            y,
            a.x as f64 / 10_000.0,
            a.y as f64 / 10_000.0,
            b.x as f64 / 10_000.0,
            b.y as f64 / 10_000.0,
        ));
    }
    best
}

#[test]
fn drawn_footprint_keeps_xy_distance_from_the_model_wall() {
    let output = run();
    let wall = wall();

    let mut checked = 0_usize;
    let mut nearest = f64::MAX;
    let mut nearest_at = (0.0_f64, 0.0_f64, 0_i32);
    for entry in output.entries() {
        for role in &entry.roles {
            for region in &role.regions {
                for point in &region.contour.points {
                    let (x, y) = (point.x as f64 / 10_000.0, point.y as f64 / 10_000.0);
                    checked += 1;
                    assert!(
                        !slicer_ir::point_in_polygon_winding(&wall, x, y, 0.0),
                        "role vertex ({x:.4}, {y:.4}) on layer {} is inside the model wall",
                        entry.global_layer_index
                    );
                    let distance = distance_to_boundary(&wall, x, y);
                    if distance < nearest {
                        nearest = distance;
                        nearest_at = (x, y, entry.global_layer_index);
                    }
                }
            }
        }
    }

    assert!(
        checked > 0,
        "fixture emitted no role regions — the clearance check would be vacuous"
    );
    // Non-vacuity: the branches must actually reach the wall, otherwise the
    // carve is never exercised and the test passes for the wrong reason.
    assert!(
        nearest <= XY_DISTANCE_MM + 0.4,
        "no emitted role region comes near the wall (nearest {nearest:.4} mm) — \
         the fixture does not exercise the `draw_circles` carve"
    );
    assert!(
        nearest >= XY_DISTANCE_MM - TOLERANCE_MM,
        "role vertex ({:.4}, {:.4}) on layer {} sits {nearest:.4} mm from the model wall; \
         canonical `draw_circles` carves the drawn circle out of \
         `offset_ex(m_layer_outlines[l], m_xy_distance)`, so the printed footprint \
         must keep {XY_DISTANCE_MM} mm",
        nearest_at.0,
        nearest_at.1,
        nearest_at.2
    );
}

#[test]
fn collision_carve_keeps_only_the_largest_surviving_part() {
    let drawn = rect(0.0, 0.0, 10.0, 2.0);
    let collision = vec![rect(1.0, -1.0, 2.0, 3.0), rect(3.0, -1.0, 4.0, 3.0)];

    let carved = carve_emitted_regions(&[drawn], &collision);

    assert_eq!(
        carved.len(),
        1,
        "small disconnected carve remnants must be dropped"
    );
    assert!(
        (polygon_area(&carved[0]) - 12.0).abs() < 0.01,
        "the retained component must be the 6x2 mm largest part; got {} mm^2",
        polygon_area(&carved[0])
    );
}

fn ladder_wall(x0: f32) -> SupportGeometryView {
    SupportGeometryView {
        entries: (0..LAYERS)
            .map(|layer| SupportGeometryViewEntry {
                global_support_layer_index: layer,
                object_id: OBJECT_ID.into(),
                region_id: REGION_ID.into(),
                outlines: vec![rect(x0, -3.0, 9.0, 7.0)],
            })
            .collect(),
    }
}

#[test]
fn emit_gate_uses_radius_baked_collision_point_in_semantics() {
    // A node at x=2 is outside the radius-free bucket but inside the bucket
    // pre-inflated for its 0.4 mm tapered radius. Production passes the latter
    // `get_collision(radius, layer)` result to this point-in predicate.
    let radius_free = rect(2.15, -3.0, 9.0, 7.0);
    let radius_baked = rect(1.75, -3.0, 9.0, 7.0);
    assert!(!point_inside_collision_volume(&[radius_free], 2.0, 2.0));
    assert!(point_inside_collision_volume(&[radius_baked], 2.0, 2.0));
}

#[test]
fn avoidance_clearance_is_keyed_by_each_nodes_stored_radius() {
    let geometry = ladder_wall(4.5);
    let mut analysis = analysis();
    for occupancy in &mut analysis.model_occupancy {
        occupancy.polygons = vec![rect(6.0, -3.0, 9.0, 7.0)];
    }
    let small = run_with_config(&analysis, &geometry, &planner_config_with_diameter(2.0));
    let large = run_with_config(&analysis, &geometry, &planner_config_with_diameter(4.0));
    let nearest_node = |output: &SupportGeometryOutput| {
        output
            .entries()
            .iter()
            // Contact layers have different surviving seed counts for the two
            // diameters. The plate layer compares fully propagated descendants.
            .filter(|entry| entry.global_layer_index == 0)
            .filter_map(|entry| entry.skeleton.as_ref())
            .flat_map(|skeleton| skeleton.points.iter())
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let small_nearest = nearest_node(&small);
    let large_nearest = nearest_node(&large);
    assert!(small_nearest.is_finite() && large_nearest.is_finite());
    assert!(
        large_nearest < small_nearest,
        "larger stored radii must query a larger avoidance bucket: small={small_nearest}, large={large_nearest}"
    );
}
