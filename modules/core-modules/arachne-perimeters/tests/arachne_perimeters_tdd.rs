//! TDD tests for Arachne variable-width perimeter generation.
//!
//! Tests the ArachnePerimeters LayerModule implementation for the
//! Layer::Perimeters stage. Unlike classic perimeters (constant-width insets),
//! Arachne produces variable-width wall loops that adapt to local geometry.
//!
//! Per OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp and
//! OrcaSlicerDocumented/generated_documentation/pseudocode_arachne_straight_skeleton.md.

use std::collections::HashMap;

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, ExPolygon, ExtrusionRole, LoopType, Polygon, WallBoundaryType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Create a square ExPolygon centered at origin with given side length in mm.
fn make_square(side_mm: f32) -> ExPolygon {
    square_polygon(0.0, 0.0, side_mm)
}

/// Create a narrow wedge/rectangle thinner than 2*line_width (0.8mm).
/// This 0.6mm wide rectangle should result in fewer walls than wall_count.
fn make_narrow_rect(width_mm: f32, height_mm: f32) -> ExPolygon {
    rect_polygon(0.0, 0.0, width_mm, height_mm)
}

/// Create a config with specified wall_count and line_width.
fn wall_config(wall_count: u32, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("line_width", line_width)
        .build()
}

#[rustfmt::skip]
/// Create a config with speed settings and optional min_feature_size.
fn make_config_full(wall_count: u32, line_width: f64, outer_speed: f64, inner_speed: f64) -> ConfigView {
    ConfigViewBuilder::new().int("wall_count", wall_count as i64).float("line_width", line_width).float("outer_wall_speed", outer_speed).float("inner_wall_speed", inner_speed).build()
}

#[rustfmt::skip]
/// Create a SliceRegionView with a single polygon.
fn make_region_from_poly(poly: ExPolygon, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new().object_id("obj-1").region_id(1).add_polygon(poly).effective_layer_height(0.2).z(z).has_nonplanar(false).build()
}

/// Create a SliceRegionView with a single square polygon.
fn square_slice_region(side_mm: f32, z: f32) -> SliceRegionView {
    make_region_from_poly(make_square(side_mm), z)
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
    (area.abs() / 2.0) / (10_000.0 * 10_000.0)
}

// ---- Tests ----

#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    // Default: wall_count=3
    // R2 (P105): inner/outer wall widths are now read per-invocation in run_perimeters,
    // not cached as struct fields. Wall count remains cached (can't change mid-layer).
    assert_eq!(module.wall_count(), 3);
}

#[test]
fn on_print_start_custom() {
    let config = make_config_full(4, 0.5, 40.0, 80.0);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    // R2 (P105): wall_count is cached; line widths are per-invocation.
    assert_eq!(module.wall_count(), 4);
}

#[test]
fn single_square_region() {
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        !walls.is_empty(),
        "Expected at least 1 WallLoop for a 10mm square"
    );
    // Each wall loop should have non-empty path points
    for wall in walls {
        assert!(
            !wall.path.points.is_empty(),
            "Wall loop should have non-empty path points"
        );
    }
}

#[test]
fn variable_width_profile() {
    // A narrow wedge-like region should produce walls with varying widths.
    // We use a narrow rectangle (0.6mm wide) which is between 1x and 2x line_width.
    // Arachne should adapt wall widths rather than use uniform width.
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let narrow = make_narrow_rect(0.6, 10.0);
    let regions = vec![make_region_from_poly(narrow, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        !walls.is_empty(),
        "Narrow region should still produce walls"
    );

    // At least one wall should have a width_profile with a width != line_width (variable)
    let has_variable = walls.iter().any(|w| {
        w.width_profile
            .widths
            .iter()
            .any(|&width| (width - 0.4).abs() > 0.01)
    });
    assert!(
        has_variable,
        "Arachne should produce variable widths for narrow regions, \
         but all widths matched nominal line_width"
    );
}

#[test]
fn thin_region_fewer_walls() {
    // Region narrower than 2*line_width should produce fewer walls than wall_count.
    let config = wall_config(3, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    // 0.6mm wide: fits ~1 wall, definitely not 3
    let narrow = make_narrow_rect(0.6, 10.0);
    let regions = vec![make_region_from_poly(narrow, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        walls.len() < 3,
        "Thin region (0.6mm) should produce fewer than 3 walls with 0.4mm line_width, got {}",
        walls.len()
    );
}

#[test]
fn zero_walls_config() {
    let config = wall_config(0, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.wall_loops().len(),
        0,
        "No wall loops when wall_count=0"
    );
    // All input becomes infill
    assert!(
        !output.infill_areas().is_empty(),
        "Infill should be the input polygons when wall_count=0"
    );
}

#[test]
fn empty_regions_no_output() {
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-1".to_string());
    region.set_region_id(1);
    region.set_polygons(Vec::new());
    region.set_infill_areas(Vec::new());
    region.set_effective_layer_height(0.2);
    region.set_z(1.0);
    region.set_has_nonplanar(false);
    let regions = vec![region];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(output.wall_loops().len(), 0);
    assert_eq!(output.infill_areas().len(), 0);
}

#[test]
fn outer_wall_role() {
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty());
    let outer = &walls[0];
    assert_eq!(
        outer.perimeter_index, 0,
        "First wall should be outer (index 0)"
    );
    assert_eq!(outer.loop_type, LoopType::Outer);
    assert_eq!(outer.path.role, ExtrusionRole::OuterWall);
    assert_eq!(outer.boundary_type, WallBoundaryType::ExteriorSurface);
}

#[test]
fn inner_wall_role() {
    let config = wall_config(3, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(walls.len() >= 2, "Need at least 2 walls");

    for wall in walls.iter().skip(1) {
        assert_eq!(wall.loop_type, LoopType::Inner);
        assert_eq!(wall.path.role, ExtrusionRole::InnerWall);
        assert_eq!(wall.boundary_type, WallBoundaryType::Interior);
    }
}

#[test]
fn infill_areas_set() {
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let infill = output.infill_areas();
    assert!(
        !infill.is_empty(),
        "10mm square with 2 walls should have infill area"
    );

    let infill_area: f64 = infill.iter().map(|p| polygon_area_mm(&p.contour)).sum();
    assert!(infill_area > 0.0, "Infill area should be positive");
    assert!(
        infill_area < 100.0,
        "Infill area should be smaller than input 10x10mm"
    );
}

#[test]
fn seam_candidates_generated() {
    let config = wall_config(2, 0.4);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![square_slice_region(10.0, 1.0)];
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
    for (pos, score) in seams {
        assert!(*score > 0.0, "Seam score should be positive");
        assert!((pos.z - 1.0).abs() < 0.01, "Seam Z should match layer Z");
    }
}
