//! Edge-case coverage for classic-perimeters: a concave (L-shaped) region must
//! still emit walls with an Outer wall at index 0 and not panic.

#![allow(missing_docs)]

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ExPolygon, LoopType, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{ConfigViewBuilder, SliceRegionViewBuilder};
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};

/// L-shape: 10×10 square with the upper-right 5×5 quadrant removed.
fn l_shape() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(5.0, -5.0),
                Point2::from_mm(5.0, 0.0),
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(0.0, 5.0),
                Point2::from_mm(-5.0, 5.0),
            ],
        },
        holes: vec![],
    }
}

#[test]
fn concave_region_emits_outer_wall_without_panic() {
    let cfg = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("line_width", 0.4)
        .build();
    let module = ClassicPerimeters::on_print_start(&cfg).unwrap();

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(l_shape())
        .build();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &[region], &paint, &mut output, &cfg)
        .expect("run_perimeters must not panic on a concave region");

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "concave region must still emit walls");
    assert_eq!(
        walls[0].loop_type,
        LoopType::Outer,
        "first wall must be the outer loop"
    );
    assert_eq!(walls[0].perimeter_index, 0, "outer wall is index 0");
}
