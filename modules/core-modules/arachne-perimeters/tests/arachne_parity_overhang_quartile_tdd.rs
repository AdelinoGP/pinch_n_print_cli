//! TDD test for arachne per-vertex parity packet 148, AC-5.
//!
//! Ticket 142 replaced the band-polygon lookup with a distance-derived
//! quartile: every wall-loop path point that lies outside
//! `region.prev_layer_boundary()` must get
//! `path.points[j].overhang_quartile == Some(bucket)` where the bucket is the
//! signed distance bucketed at `region.overhang_band_width_mm() x {0.5, 1, 1.5}`;
//! points inside the boundary get `None`. The oracle mirrors
//! `slicer_core::perimeter_utils::overhang_quartile_from_distance` exactly.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::ConfigView;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .float("resolution", 0.01)
        .int("wall_count", wall_count as i64)
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        // Required-read baseline (packet 06 5c', item 11): the classified
        // reads in run_perimeters/arachne_params_from_config are now
        // require_*; the view holds every key those paths read, at
        // manifest-default values. line_width holds its post-expansion
        // default (1.125 x nozzle_diameter): the raw 0 is the auto sentinel
        // expanded at Phase B and cannot survive the D-162 spacing gate.
        .float("layer_height", 0.2)
        .float("nozzle_diameter", 0.4)
        .float("line_width", 0.45)
        .float("bridge_line_width", 0.0)
        .float("initial_layer_line_width", 0.0)
        .int("extra_perimeters", 0)
        .bool("precise_outer_wall", false)
        .string("wall_sequence", "InnerOuter")
        .int("support_raft_layers", 0)
        .bool("only_one_wall_top", false)
        .string("wall_direction", "counter_clockwise")
        .bool("alternate_extra_wall", false)
        .bool("spiral_vase", false)
        .float("sparse_infill_density", 20.0)
        .bool("only_one_wall_first_layer", false)
        .bool("detect_overhang_wall", true)
        .bool("overhang_reverse", false)
        .bool("overhang_reverse_internal_only", false)
        .float("overhang_reverse_threshold", 0.0)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("seam_candidate_angle_threshold_deg", 30.0)
        // Classifier liveness predicate: the per-vertex stamps are only
        // produced when a consumer reads them.
        .bool("enable_overhang_speed", true)
        .float("overhang_1_4_speed", 0.0)
        .float("overhang_2_4_speed", 0.0)
        .float("overhang_3_4_speed", 30.0)
        .float("overhang_4_4_speed", 0.0)
        .build()
}

/// 10mm square region (centered at origin) over a smaller previous-layer
/// boundary. Ticket 142: the quartile comes from the signed distance to
/// `previous_layer_boundary`, bucketed at `overhang_band_width_mm`.
fn make_region(side_mm: f32, boundary_side_mm: f32, band_width_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .previous_layer_boundary(vec![square_polygon(0.0, 0.0, boundary_side_mm)])
        .overhang_band_width_mm(band_width_mm)
        .build()
}

#[test]
fn overhang_quartile_set_per_vertex_inside_band() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let band_width_mm = 0.4_f32;
    let regions = vec![make_region(10.0, 6.0, band_width_mm, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let boundary = vec![square_polygon(0.0, 0.0, 6.0)];
    let mut checked_any_point = false;
    let mut checked_some_overhang = false;

    for wall in output.wall_loops() {
        for pt in &wall.path.points {
            checked_any_point = true;
            let raw =
                slicer_core::perimeter_utils::signed_distance_to_boundary(pt.x, pt.y, &boundary);
            let expected =
                slicer_core::perimeter_utils::overhang_quartile_from_distance(raw, band_width_mm);
            checked_some_overhang |= expected.is_some();
            assert_eq!(
                pt.overhang_quartile, expected,
                "vertex at ({}, {}) mm: expected overhang_quartile == {:?} \
                 (signed distance {raw} mm), got {:?}",
                pt.x, pt.y, expected, pt.overhang_quartile
            );
        }
    }

    assert!(
        checked_any_point,
        "expected at least one path point to verify"
    );
    assert!(
        checked_some_overhang,
        "fixture must exercise at least one overhanging vertex"
    );
}
