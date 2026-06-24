//! AC-1: outer/inner width and spacing contract (T-051/T-052, packet 105).
//!
//! Given an ExPolygon square of side 10 mm with outer_wall_line_width=0.5 mm,
//! inner_wall_line_width=0.4 mm, wall_count=3:
//! - Outer wall (index 0) has every vertex width=0.5 mm
//! - Inner walls (indices 1,2) have every vertex width=0.4 mm
//! - Radial gap between outer and first-inner = ext_perimeter_spacing2 = 0.45 mm
//! - Radial gap between walls 1 and 2 = perimeter_spacing = 0.4 mm

use classic_perimeters::ClassicPerimeters;
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

    let module = ClassicPerimeters::on_print_start(&config).unwrap();
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
    let expected_outer_right = half_side - outer_w / 2.0;
    let expected_first_inner_right = half_side - outer_w / 2.0 - (outer_w + inner_w) / 2.0;
    let expected_second_inner_right =
        half_side - outer_w / 2.0 - (outer_w + inner_w) / 2.0 - inner_w;

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
    let expected_gap_outer_to_first = (outer_w + inner_w) / 2.0;
    assert!(
        (gap_outer_to_first - expected_gap_outer_to_first).abs() < 0.005,
        "Gap outer→first inner {} != ext_perimeter_spacing2 {}",
        gap_outer_to_first,
        expected_gap_outer_to_first
    );

    let gap_first_to_second = first_inner_x - second_inner_x;
    assert!(
        (gap_first_to_second - inner_w).abs() < 0.005,
        "Gap first→second inner {} != perimeter_spacing {}",
        gap_first_to_second,
        inner_w
    );
}

fn find_max_x(points: &[slicer_ir::Point3WithWidth]) -> f32 {
    points.iter().map(|p| p.x).fold(f32::MIN, f32::max)
}
