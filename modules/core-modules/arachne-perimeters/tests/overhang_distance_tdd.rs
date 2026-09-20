//! TDD red test for packet 193, AC-6 (test half):
//! `arachne_stamps_distance_for_regions_with_no_overhang_bands`.
//!
//! A region WITH a previous-layer slice boundary but NO quartile bands
//! (nothing overhangs) must still receive `Some(overhang_distance_mm)` on
//! every stamped wall vertex — the distance carrier and the quartile bands
//! have different availability, and this population is what packet 190 must
//! interpolate a fast speed for.
//!
//! References symbols that do not exist yet
//! (`SliceRegionViewBuilder::previous_layer_boundary`,
//! `Point3WithWidth::overhang_distance_mm`) — this binary MUST fail to
//! compile until the production half of the packet lands.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::ConfigView;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
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
        .build()
}

/// 10mm square region (centered at origin) whose previous layer was a 12mm
/// square — fully supported, so NO quartile bands are attached, but the
/// previous-layer boundary carrier IS present.
fn make_region() -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.4)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .previous_layer_boundary(vec![square_polygon(0.0, 0.0, 12.0)])
        .build()
}

#[test]
fn arachne_stamps_distance_for_regions_with_no_overhang_bands() {
    let config = make_config();
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let regions = vec![make_region()];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let mut checked_any_point = false;
    for wall in output.wall_loops() {
        for pt in &wall.path.points {
            checked_any_point = true;
            assert!(
                pt.overhang_distance_mm.is_some(),
                "vertex at ({}, {}) mm: region has a previous-layer boundary but no \
                 quartile bands — overhang_distance_mm must still be Some; got {:?}",
                pt.x,
                pt.y,
                pt.overhang_distance_mm
            );
        }
    }

    assert!(
        checked_any_point,
        "expected at least one path point to verify"
    );
}
