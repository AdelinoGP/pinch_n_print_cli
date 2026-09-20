//! Committed-wall evidence for all Arachne wall-sequence modes.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn config(mode: &str) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 3)
        .string("wall_sequence", mode)
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
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .int("extra_perimeters", 0)
        .bool("precise_outer_wall", false)
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

fn region() -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build()
}

fn region_at(id: u32, x: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(u64::from(id))
        .z(0.2)
        .add_polygon(square_polygon(x, 0.0, 10.0))
        .build()
}

fn committed_indices(mode: &str, layer: u32) -> Vec<u32> {
    let config = config(mode);
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let regions = vec![region()];
    let paint = PaintRegionLayerView::new(layer);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer, &regions, &paint, &mut output, &config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .map(|wall| wall.perimeter_index)
        .collect()
}

fn committed_markers(mode: &str, layer: u32) -> Vec<(f32, u32, LoopType)> {
    let config = config(mode);
    let module = ArachnePerimeters::from_config(&config).unwrap();
    let regions = vec![region_at(1, 0.0), region_at(2, 20.0)];
    let paint = PaintRegionLayerView::new(layer);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer, &regions, &paint, &mut output, &config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .map(|wall| {
            (
                wall.path
                    .points
                    .iter()
                    .map(|point| point.x)
                    .fold(f32::INFINITY, f32::min),
                wall.perimeter_index,
                wall.loop_type,
            )
        })
        .collect()
}

#[test]
fn commits_plain_wall_sequences_in_configured_order() {
    assert_eq!(committed_indices("InnerOuter", 0), vec![0, 1, 2]);
    assert_eq!(committed_indices("OuterInner", 0), vec![2, 1, 0]);
}

#[test]
fn commits_sandwich_wall_sequence_by_layer() {
    assert_eq!(committed_indices("InnerOuterInner", 0), vec![0, 1, 2]);
    assert_eq!(committed_indices("InnerOuterInner", 1), vec![1, 0, 2]);
}

#[test]
fn preserves_cross_region_core_path_precedence_over_perimeter_index() {
    let markers = committed_markers("InnerOuter", 0);
    assert!(
        markers.len() >= 3,
        "expected two region wall batches: {markers:?}"
    );

    let first_region_end = markers
        .iter()
        .position(|(x, _, _)| *x >= 10.0)
        .expect("second-region marker");
    assert!(
        first_region_end > 0,
        "first region must commit before second region"
    );
    assert!(markers[..first_region_end]
        .iter()
        .all(|(x, _, _)| *x < 10.0));
    assert!(markers[first_region_end..]
        .iter()
        .all(|(x, _, _)| *x >= 10.0));

    let first_region = &markers[..first_region_end];
    assert_eq!(first_region[0].2, LoopType::Outer);
    let inner_a = first_region
        .iter()
        .find(|(_, _, loop_type)| *loop_type != LoopType::Outer)
        .expect("first region must retain an inner path marker");
    let outer_b = markers[first_region_end..]
        .iter()
        .find(|(_, _, loop_type)| *loop_type == LoopType::Outer)
        .expect("second region must retain an outer path marker");
    assert!(
        inner_a.1 > outer_b.1,
        "fixture must distinguish path order from index order"
    );
}
