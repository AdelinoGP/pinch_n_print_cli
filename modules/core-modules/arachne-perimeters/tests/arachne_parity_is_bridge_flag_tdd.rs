//! TDD test for arachne per-vertex parity packet 148, AC-4.
//!
//! `region.bridge_areas()` marks polygons that are bridge spans. Every
//! Outer/Inner `WallLoop` vertex whose path point lies inside one of those
//! areas must get `feature_flags[j].is_bridge == true`; vertices outside get
//! `false`. `ThinWall`/`GapFill` walls must never set `is_bridge`, even if
//! their vertices happen to fall inside a bridge area (mirrors
//! `classic-perimeters`' own is_bridge shape, per-vertex not per-line).

use arachne_perimeters::ArachnePerimeters;
use slicer_core::perimeter_utils::point_in_any_polygon;
use slicer_ir::{mm_to_units, ConfigView, LoopType, Point2};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        .build()
}

/// 10mm square region (centered at origin, per `square_polygon`'s own
/// center-based convention) with a 4mm x 4mm bridge area at the same center.
fn make_region(side_mm: f32, bridge_side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .bridge_areas(vec![square_polygon(0.0, 0.0, bridge_side_mm)])
        .build()
}

#[test]
fn is_bridge_set_per_vertex_inside_bridge_area_outer_inner_only() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 4.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let bridge_areas = vec![square_polygon(0.0, 0.0, 4.0)];
    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let mut checked_any_outer_inner = false;
    for wall in output.wall_loops() {
        for (j, flag) in wall.feature_flags.iter().enumerate() {
            let pt = &wall.path.points[j];
            let units_pt = Point2 {
                x: mm_to_units(pt.x),
                y: mm_to_units(pt.y),
            };
            let inside = point_in_any_polygon(&units_pt, &bridge_areas);

            match wall.loop_type {
                LoopType::Outer | LoopType::Inner => {
                    checked_any_outer_inner = true;
                    assert_eq!(
                        flag.is_bridge,
                        inside,
                        "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                         expected is_bridge == {} (point-in-bridge-area == {}), got {}",
                        wall.loop_type,
                        wall.perimeter_index,
                        j,
                        pt.x,
                        pt.y,
                        inside,
                        inside,
                        flag.is_bridge
                    );
                }
                _ => {
                    assert!(
                        !flag.is_bridge,
                        "ThinWall/GapFill/NonPlanarShell walls must never set is_bridge \
                         (loop_type={:?}, vertex {})",
                        wall.loop_type, j
                    );
                }
            }
        }
    }

    assert!(
        checked_any_outer_inner,
        "expected at least one Outer or Inner wall loop to verify is_bridge against"
    );
}
