//! AC-1: outer/inner width and spacing contract (T-051/T-052, packet 105).
//!
//! Given an ExPolygon square of side 10 mm with outer_wall_line_width=0.5 mm,
//! inner_wall_line_width=0.4 mm, wall_count=3:
//! - Outer wall (index 0) has every vertex width=0.5 mm
//! - Inner walls (indices 1,2) have every vertex width=0.4 mm
//! - Radial gap between outer and first-inner = ext_perimeter_spacing2, the mean
//!   of the two *rounded-cross-section spacings* (NOT the mean of the widths)
//! - Radial gap between walls 1 and 2 = perimeter_spacing, the spacing derived
//!   from `inner_wall_line_width`
//!
//! Spacings are derived here by calling `slicer_core::flow::line_width_to_spacing`
//! rather than transcribed as decimals, so the test tracks the canonical
//! `Flow::spacing()` formula instead of a frozen snapshot of it.

use classic_perimeters::ClassicPerimeters;
use slicer_core::flow::line_width_to_spacing;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

#[test]
fn outer_inner_width_and_spacing() {
    let outer_w = 0.5_f32;
    let inner_w = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert_eq!(walls.len(), 3, "Expected 3 wall loops");

    // AC-1 width assertions
    let outer = &walls[0];
    for pt in &outer.path.points {
        assert!(
            (pt.width - outer_w).abs() < 0.005,
            "Outer wall vertex width {} != {}",
            pt.width,
            outer_w
        );
    }

    for wall in &walls[1..] {
        for pt in &wall.path.points {
            assert!(
                (pt.width - inner_w).abs() < 0.005,
                "Inner wall vertex width {} != {}",
                pt.width,
                inner_w
            );
        }
    }

    // AC-1 spacing assertions.
    // square_polygon creates a square centered at origin with half-side=5mm.
    // The right edge of the contour is at X=5mm.
    // Outer wall centerline is inset by outer_width/2 from the contour.
    let half_side = 5.0_f32;
    let layer_height = 0.2_f32;
    // Canonical `process_classic`: ext_perimeter_spacing2 is the mean of the two
    // flows' spacings (non-precise branch); the i>=2 inset delta is
    // perimeter_spacing. The first loop's inset stays ext_perimeter_width / 2.
    let ext_perimeter_spacing = line_width_to_spacing(outer_w, layer_height).unwrap();
    let perimeter_spacing = line_width_to_spacing(inner_w, layer_height).unwrap();
    let ext_perimeter_spacing2 = 0.5 * (ext_perimeter_spacing + perimeter_spacing);

    let expected_outer_right = half_side - outer_w / 2.0;
    let expected_first_inner_right = expected_outer_right - ext_perimeter_spacing2;
    let expected_second_inner_right = expected_first_inner_right - perimeter_spacing;

    let outer_x = find_max_x(&outer.path.points);
    let first_inner_x = find_max_x(&walls[1].path.points);
    let second_inner_x = find_max_x(&walls[2].path.points);

    assert!(
        (outer_x - expected_outer_right).abs() < 0.005,
        "Outer wall right edge X {} != {}",
        outer_x,
        expected_outer_right
    );

    assert!(
        (first_inner_x - expected_first_inner_right).abs() < 0.005,
        "First inner wall right edge X {} != {}",
        first_inner_x,
        expected_first_inner_right
    );

    assert!(
        (second_inner_x - expected_second_inner_right).abs() < 0.005,
        "Second inner wall right edge X {} != {}",
        second_inner_x,
        expected_second_inner_right
    );

    // Verify the gaps between walls
    let gap_outer_to_first = outer_x - first_inner_x;
    let expected_gap_outer_to_first = ext_perimeter_spacing2;
    assert!(
        (gap_outer_to_first - expected_gap_outer_to_first).abs() < 0.005,
        "Gap outer→first inner {} != ext_perimeter_spacing2 {}",
        gap_outer_to_first,
        expected_gap_outer_to_first
    );

    let gap_first_to_second = first_inner_x - second_inner_x;
    assert!(
        (gap_first_to_second - perimeter_spacing).abs() < 0.005,
        "Gap first→second inner {} != perimeter_spacing {}",
        gap_first_to_second,
        perimeter_spacing
    );
}

/// A width/layer-height combination whose rounded-cross-section spacing is <= 0
/// must surface as a FATAL `ModuleError` (code 1), never a panic and never a
/// silent `Ok`. Mirrors `arachne-perimeters`' `ERR_NEGATIVE_SPACING` contract.
#[test]
fn negative_spacing_config_is_a_fatal_module_error() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .float("layer_height", 2.0)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 2.0)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_perimeters(0, &regions, &paint, &mut output, &config);

    let err = result.expect_err("negative spacing must be a fatal module error, got Ok");
    assert_eq!(
        err.code, 1,
        "expected ERR_NEGATIVE_SPACING (code 1), got {err:?}"
    );
}

fn find_max_x(points: &[slicer_ir::Point3WithWidth]) -> f32 {
    points.iter().map(|p| p.x).fold(f32::MIN, f32::max)
}
