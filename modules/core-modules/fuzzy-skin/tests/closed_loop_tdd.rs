//! Regression: when fuzzy-skin processes a closed wall loop (5 points with
//! last == first, OrcaSlicer `is_closed()` convention), the closing edge is
//! perturbed exactly like every other edge. Without the explicit closing
//! repeat in the wall path, fuzzy-skin's segment loop (`for seg_idx in
//! 0..points.len() - 1`) skips the implicit closing segment — producing a
//! visibly straight bottom on a fuzzy 10mm square. See plan Phase A1.4.

use std::collections::HashMap;

use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth, WallBoundaryType,
    WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::ConfigViewBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use fuzzy_skin::FuzzySkinModule;

fn fuzzy_flag(on: bool) -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: on,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

fn pt(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

/// Build a closed-square outer wall with N+1 = 5 points (explicit closing repeat).
fn closed_square_outer_wall(fuzzy: bool) -> WallLoop {
    let first = pt(0.0, 0.0);
    let points = vec![first, pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0), first];
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: points.clone(),
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4; points.len()],
        },
        feature_flags: vec![fuzzy_flag(fuzzy); points.len()],
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

fn region_with(wall: WallLoop) -> PerimeterRegionView {
    let mut tmp = PerimeterRegionView::default();
    tmp.set_object_id("obj-0");
    tmp.set_region_id(0);
    tmp.set_wall_loops(vec![wall]);
    tmp
}

fn config_apply_to_all(thickness_mm: f32, point_distance_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .float("thickness", thickness_mm as f64)
        .float("point_distance", point_distance_mm as f64)
        .bool("apply_to_all", true)
        .build()
}

/// Returns the perpendicular distance from `p` to the infinite line through `a`-`b` (in mm).
fn perpendicular_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    ((dy * p.0 - dx * p.1 + b.0 * a.1 - b.1 * a.0).abs()) / len2.sqrt()
}

#[test]
fn fuzzy_skin_perturbs_closing_edge_of_closed_loop() {
    // Input is a closed 10mm square with N+1 points (5 entries, last == first).
    // With apply_to_all = true and non-trivial thickness, every segment —
    // including the closing edge from (0,10) → (0,0) — must produce
    // perturbation points off the straight-line path.
    let wall = closed_square_outer_wall(true);
    assert!(
        wall.path.is_closed(),
        "fixture must be closed (5 points, last == first); got {} points",
        wall.path.points.len()
    );
    let module = FuzzySkinModule::on_print_start(&config_apply_to_all(0.5, 0.8)).unwrap();
    let region = region_with(wall);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &[region], &mut output, &config_apply_to_all(0.5, 0.8))
        .unwrap();

    let walls = output.wall_loops();
    assert_eq!(walls.len(), 1, "expected exactly one fuzzed wall loop");
    let pts = &walls[0].path.points;
    assert!(
        pts.len() > 5,
        "fuzzed wall must have more than 5 points (intermediate perturbations); got {}",
        pts.len()
    );

    // The closing edge runs from (0, 10) back to (0, 0): a vertical line at x = 0.
    // After fuzzy-skin perturbation, points whose original-line was this closing
    // edge should have x != 0 (perpendicular displacement). Scan the tail of the
    // path (the closing-edge perturbations land between the last corner (0, 10)
    // and the closing repeat (0, 0)).
    let closing_corner_idx = pts
        .iter()
        .rposition(|p| p.x.abs() < 0.001 && (p.y - 10.0).abs() < 0.001)
        .expect("path must visit the (0, 10) corner");
    let after_corner = &pts[closing_corner_idx + 1..pts.len().saturating_sub(1)];
    assert!(
        !after_corner.is_empty(),
        "fuzzy-skin produced no intermediate points on the closing edge — \
         the closing edge was emitted as a straight line. \
         The N+1-point closed-loop input convention is broken."
    );

    // At least one intermediate point on the closing edge must be off the
    // straight line from (0, 10) to (0, 0) (i.e. its x must differ from 0).
    let line_a = (0.0_f32, 10.0_f32);
    let line_b = (0.0_f32, 0.0_f32);
    let max_perp = after_corner
        .iter()
        .map(|p| perpendicular_distance((p.x, p.y), line_a, line_b))
        .fold(0.0_f32, f32::max);
    assert!(
        max_perp > 0.01,
        "closing-edge intermediate points must be perpendicular-displaced from \
         the straight (0,10)→(0,0) line; got max_perp = {max_perp} mm"
    );
}

#[test]
fn fuzzy_skin_output_preserves_closing_repeat_invariant() {
    // Output of fuzzy-skin on a closed input must also be closed: the final
    // point must equal the first (so downstream emitter / future post-processors
    // see the same convention).
    let wall = closed_square_outer_wall(true);
    let module = FuzzySkinModule::on_print_start(&config_apply_to_all(0.3, 0.8)).unwrap();
    let region = region_with(wall);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &[region], &mut output, &config_apply_to_all(0.3, 0.8))
        .unwrap();

    let pts = &output.wall_loops()[0].path.points;
    let first = pts.first().expect("non-empty");
    let last = pts.last().expect("non-empty");
    assert!(
        (first.x - last.x).abs() < 0.001 && (first.y - last.y).abs() < 0.001,
        "fuzzy-skin output must preserve the closing-repeat invariant: \
         first=({},{}) last=({},{})",
        first.x,
        first.y,
        last.x,
        last.y
    );
}
