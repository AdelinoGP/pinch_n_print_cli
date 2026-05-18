//! TDD tests for classic perimeter generation.
//!
//! Tests the ClassicPerimeters LayerModule implementation for the
//! Layer::Perimeters stage per docs/01_system_architecture.md.

use std::collections::HashMap;

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{
    mm_to_units, ConfigValue, ConfigView, ExPolygon, ExtrusionRole, LoopType, Point2, Polygon,
    WallBoundaryType,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Create a square ExPolygon centered at origin with given side length in mm.
fn make_square(side_mm: f32) -> ExPolygon {
    let half = side_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: mm_to_units(-half),
                    y: mm_to_units(-half),
                },
                Point2 {
                    x: mm_to_units(half),
                    y: mm_to_units(-half),
                },
                Point2 {
                    x: mm_to_units(half),
                    y: mm_to_units(half),
                },
                Point2 {
                    x: mm_to_units(-half),
                    y: mm_to_units(half),
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// Create a config with specified wall_count and line_width.
fn make_config(wall_count: u32, line_width: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "wall_count".to_string(),
        ConfigValue::Int(wall_count as i64),
    );
    fields.insert("line_width".to_string(), ConfigValue::Float(line_width));
    ConfigView::from_map(fields)
}

/// Create a config with speed settings too.
fn make_config_with_speeds(
    wall_count: u32,
    line_width: f64,
    outer_speed: f64,
    inner_speed: f64,
) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "wall_count".to_string(),
        ConfigValue::Int(wall_count as i64),
    );
    fields.insert("line_width".to_string(), ConfigValue::Float(line_width));
    fields.insert(
        "outer_wall_speed".to_string(),
        ConfigValue::Float(outer_speed),
    );
    fields.insert(
        "inner_wall_speed".to_string(),
        ConfigValue::Float(inner_speed),
    );
    ConfigView::from_map(fields)
}

/// Create a SliceRegionView with a single square polygon.
fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    {
        let mut tmp = SliceRegionView::default();
        tmp.set_object_id("obj-1".to_string());
        tmp.set_region_id(1);
        tmp.set_polygons(vec![make_square(side_mm)]);
        tmp.set_infill_areas(Vec::new());
        tmp.set_effective_layer_height(0.2);
        tmp.set_z(z);
        tmp.set_has_nonplanar(false);
        tmp
    }
}

#[test]
fn single_square_two_walls() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert_eq!(walls.len(), 2, "Expected 2 wall loops (outer + inner)");

    // Infill area should be non-empty and smaller than input
    let infill = output.infill_areas();
    assert!(!infill.is_empty(), "Infill areas should be computed");
}

#[test]
fn outer_wall_is_index_zero() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty());
    assert_eq!(walls[0].perimeter_index, 0, "Outer wall should be index 0");
    assert_eq!(
        walls[0].loop_type,
        LoopType::Outer,
        "First wall should be Outer"
    );
}

#[test]
fn inner_walls_correct_type() {
    let config = make_config(3, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(walls.len() >= 3, "Expected at least 3 wall loops");

    for (i, wall) in walls.iter().enumerate().skip(1) {
        assert_eq!(
            wall.loop_type,
            LoopType::Inner,
            "Wall {} should be Inner",
            i
        );
        assert_eq!(
            wall.perimeter_index, i as u32,
            "Wall {} should have perimeter_index {}",
            i, i
        );
    }
}

#[test]
fn infill_area_computed() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let infill = output.infill_areas();
    assert!(!infill.is_empty(), "Infill areas should be non-empty");

    // Infill area should be smaller than original polygon
    // Original is 10x10=100mm^2, after 2 walls + half width inset, much smaller
    let infill_area: f64 = infill.iter().map(|p| polygon_area_mm(&p.contour)).sum();
    assert!(
        infill_area < 100.0,
        "Infill area ({}) should be smaller than input (100mm^2)",
        infill_area
    );
    assert!(infill_area > 0.0, "Infill area should be positive");
}

#[test]
fn empty_polygon_no_output() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-1".to_string());
    region.set_region_id(1);
    region.set_polygons(Vec::new());
    region.set_infill_areas(Vec::new());
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    let regions = vec![region];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        0,
        "No wall loops for empty input"
    );
    assert_eq!(output.infill_areas().len(), 0, "No infill for empty input");
}

#[test]
fn wall_count_zero() {
    let config = make_config(0, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        0,
        "No wall loops with wall_count=0"
    );
    // Infill areas should be the input polygons themselves
    assert!(
        !output.infill_areas().is_empty(),
        "Infill should be input polygons"
    );
}

#[test]
fn seam_candidates_generated() {
    let config = make_config(2, 0.4);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let seams = output.seam_candidates();
    assert!(
        !seams.is_empty(),
        "Seam candidates should be generated from outer wall corners"
    );
    // All seam candidates should have positive scores and correct Z
    for (pos, score) in seams {
        assert!(*score > 0.0, "Seam score should be positive, got {}", score);
        assert!((pos.z - 0.2).abs() < 0.01, "Seam Z should match layer Z");
    }
}

#[test]
fn speed_factor_from_config() {
    let config = make_config_with_speeds(2, 0.4, 30.0, 60.0);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(walls.len() >= 2);

    // Outer wall: 30/50 = 0.6
    let outer = &walls[0];
    assert_eq!(outer.path.role, ExtrusionRole::OuterWall);
    assert!(
        (outer.path.speed_factor - 0.6).abs() < 0.01,
        "Outer speed_factor should be 30/50=0.6, got {}",
        outer.path.speed_factor
    );

    // Inner wall: 60/50 = 1.2
    let inner = &walls[1];
    assert_eq!(inner.path.role, ExtrusionRole::InnerWall);
    assert!(
        (inner.path.speed_factor - 1.2).abs() < 0.01,
        "Inner speed_factor should be 60/50=1.2, got {}",
        inner.path.speed_factor
    );

    // Verify boundary types
    assert_eq!(outer.boundary_type, WallBoundaryType::ExteriorSurface);
    assert_eq!(inner.boundary_type, WallBoundaryType::Interior);
}

/// Helper: compute signed area of a polygon in mm^2 from scaled i64 coords.
fn polygon_area_mm(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut area: f64 = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += (pts[i].x as f64) * (pts[j].y as f64);
        area -= (pts[j].x as f64) * (pts[i].y as f64);
    }
    // Convert from scaled units^2 to mm^2
    (area.abs() / 2.0) / (10_000.0 * 10_000.0)
}
