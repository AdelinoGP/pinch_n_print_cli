//! TDD coverage for continuous aligned-seam projection.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamPosition,
    WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::PerimeterRegionViewBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

fn config_with_mode(mode: &str) -> ConfigView {
    let mut map = HashMap::new();
    map.insert(
        "seam_mode".to_string(),
        ConfigValue::String(mode.to_string()),
    );
    ConfigView::from_map(map)
}

fn ir_point(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn ir_flags(count: usize) -> Vec<WallFeatureFlags> {
    vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        };
        count
    ]
}

fn ir_wall(layer_z: f32, points: &[(f32, f32)]) -> WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
    let flags = ir_flags(path_points.len());
    let path = ExtrusionPath3D {
        points: path_points,
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}

fn aligned_region(walls: Vec<WallLoop>, resolved: Option<Point3WithWidth>) -> PerimeterRegionView {
    let mut region = PerimeterRegionView::default();
    region.set_object_id("obj-a".to_string());
    region.set_region_id(0);
    region.set_wall_loops(walls);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(resolved.map(|point| SeamPosition {
        point,
        wall_index: 0,
    }));
    region
}

#[test]
fn projects_onto_nearest_segment_point() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall = ir_wall(0.2, &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let regions = vec![aligned_region(vec![wall], Some(ir_point(1.5, 0.8, 0.2)))];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output.resolved_seam().expect("aligned seam must resolve");
    assert!((seam.point.x - 1.5).abs() < 0.001);
    assert!(seam.point.y.abs() < 0.001);

    let emitted = &output.rotated_wall_loops()[0].2;
    let first = emitted.path.points[0];
    assert!((first.x - 1.5).abs() < 0.001);
    assert!(first.y.abs() < 0.001);
    // Closed-loop metadata includes the explicit closing repeat and remains
    // parallel to path.points.
    assert_eq!(
        emitted.feature_flags.len(),
        emitted.path.points.len(),
        "feature_flags must be parallel to path.points including closing repeat"
    );
    assert_eq!(
        emitted.width_profile.widths.len(),
        emitted.path.points.len()
    );
    assert_eq!(emitted.path.points.last(), emitted.path.points.first());
}

#[test]
fn target_on_existing_vertex_does_not_insert() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall = ir_wall(0.2, &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let original_len = wall.path.points.len();
    let regions = vec![aligned_region(vec![wall], Some(ir_point(4.0, 0.0, 0.2)))];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let emitted = &output.rotated_wall_loops()[0].2;
    assert_eq!(emitted.path.points.len(), original_len);
    assert_eq!(emitted.path.points[0], ir_point(4.0, 0.0, 0.2));
}

#[test]
fn degenerate_empty_wall_loop_is_non_fatal_and_preserved() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![aligned_region(
        vec![ir_wall(0.2, &[])],
        Some(ir_point(1.0, 1.0, 0.2)),
    )];
    let mut output = PerimeterOutputBuilder::new();

    let err = module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect_err("degenerate empty wall loop must return a non-fatal error");

    // Code 7 is reserved for degenerate empty wall loops.
    assert_eq!(err.code, 7);
    assert!(!err.fatal);
    assert_eq!(
        err.message,
        "degenerate empty wall loop (no points) at wall_index=0"
    );
    assert!(output.resolved_seam().is_none());
    assert_eq!(output.rotated_wall_loops().len(), 1);
    assert!(output.rotated_wall_loops()[0].2.path.points.is_empty());
}

#[test]
fn aligned_no_wall_loops_is_silent() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![aligned_region(vec![], Some(ir_point(1.0, 1.0, 0.2)))];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("regions with no wall loops must be silent");

    assert!(output.resolved_seam().is_none());
    assert!(output.rotated_wall_loops().is_empty());
}
