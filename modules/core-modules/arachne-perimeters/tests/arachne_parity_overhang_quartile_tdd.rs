//! TDD test for arachne per-vertex parity packet 148, AC-5.
//!
//! `region.overhang_quartile_polygons()` carries `QuartileBand`s (quartile +
//! polygons). Every wall-loop path point whose (x, y) mm location falls
//! inside a band's polygon must get `path.points[j].overhang_quartile ==
//! Some(band.quartile)`; points outside every band get `None`. Lookup mirrors
//! `slicer_core::perimeter_utils::expolygon_to_path3d` (perimeter_utils.rs
//! ~316-331): filter bands whose polygons contain the point, take the max
//! quartile.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{mm_to_units, point_in_polygon_winding, ConfigView};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("optimal_width", mm_to_units(line_width_mm) as f64)
        .float(
            "preferred_bead_width_outer",
            mm_to_units(line_width_mm) as f64,
        )
        .build()
}

/// 10mm square region (centered at origin) with a single quartile-3 band
/// covering a 4mm x 4mm area at the same center.
fn make_region(side_mm: f32, band_side_mm: f32, quartile: u8, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .overhang_quartile_polygons(vec![QuartileBand {
            quartile,
            polygons: vec![square_polygon(0.0, 0.0, band_side_mm)],
        }])
        .build()
}

#[test]
fn overhang_quartile_set_per_vertex_inside_band() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 4.0, 3, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let band_polygon = square_polygon(0.0, 0.0, 4.0);
    let mut checked_any_point = false;

    for wall in output.wall_loops() {
        for pt in &wall.path.points {
            checked_any_point = true;
            let inside = point_in_polygon_winding(&band_polygon, pt.x as f64, pt.y as f64, 0.0);
            let expected = if inside { Some(3u8) } else { None };
            assert_eq!(
                pt.overhang_quartile, expected,
                "vertex at ({}, {}) mm: expected overhang_quartile == {:?} \
                 (point-in-band == {}), got {:?}",
                pt.x, pt.y, expected, inside, pt.overhang_quartile
            );
        }
    }

    assert!(
        checked_any_point,
        "expected at least one path point to verify"
    );
}
