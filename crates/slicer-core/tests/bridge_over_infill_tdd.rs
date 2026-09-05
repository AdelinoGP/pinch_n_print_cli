#![allow(missing_docs)]

use slicer_core::algos::bridge_over_infill::{
    construct_anchored_polygon, depth_window_start, determine_bridging_angle, filled_window_start,
    gather_areas_w_depth, remove_filled_polygons_on_lower_layers, BridgeCandidateLayer,
    BridgeDepthLayer,
};
use slicer_core::flow::canonical_bridging_flow;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn square(width: f32, height: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(width, 0.0),
                Point2::from_mm(width, height),
                Point2::from_mm(0.0, height),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn bridging_angle_unequal_anchor_lengths_are_length_weighted() {
    let anchors = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)],
        vec![
            Point2::from_mm(0.0, 10.0),
            Point2::from_mm(3.0, 10.0 + 3.0 * 10.0_f32.to_radians().tan()),
        ],
    ];
    let area = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        vec![Point2::from_mm(0.0, 10.0), Point2::from_mm(3.0, 10.0)],
    ];
    // Samples are 1 + 2: (90 + 100 + 100) / 3 = 96.6666667 degrees.
    let angle = determine_bridging_angle(&anchors, &area, 0.0);
    // Point2 stores integer native units, so the 10 degree line quantizes here.
    assert!((angle - 96.66691).abs() <= 1e-6, "angle={angle}");
}

#[test]
fn bridging_angle_histogram_wraps_modulo_180_seam() {
    let anchors = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)],
        vec![
            Point2::from_mm(0.0, 10.0),
            Point2::from_mm(-10.0, 10.0 - 10.0 * 1.0_f32.to_radians().tan()),
        ],
    ];
    let area = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        vec![Point2::from_mm(0.0, 10.0), Point2::from_mm(1.0, 10.0)],
    ];
    let angle = determine_bridging_angle(&anchors, &area, 0.0);
    assert!((angle - 90.500145).abs() <= 1e-6, "angle={angle}");
}

#[test]
fn bridging_angle_is_deterministic() {
    let anchors = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)]];
    let area = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(4.0, 0.0)]];
    assert_eq!(
        determine_bridging_angle(&anchors, &area, 0.0),
        determine_bridging_angle(&anchors, &area, 0.0)
    );
}

#[test]
fn bridging_angle_override_is_exactly_45_degrees() {
    let anchors = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 10.0)]];
    assert_eq!(determine_bridging_angle(&anchors, &[], 45.0), 45.0);
}

#[test]
fn anchored_polygon_line_count_tracks_round_span_over_spacing() {
    let anchors = vec![
        vec![Point2::from_mm(-1.0, 0.0), Point2::from_mm(11.0, 0.0)],
        vec![Point2::from_mm(-1.0, 4.0), Point2::from_mm(11.0, 4.0)],
    ];
    let (_polygons, lines) =
        construct_anchored_polygon(&anchors, &[square(10.0, 4.0)], 90.0, 1.0, 0.4);
    assert!(
        (lines.len() as i32 - 10).abs() <= 1,
        "line count: {}",
        lines.len()
    );
}

#[test]
fn bridging_flow_uses_configured_bridge_line_width() {
    let spec = canonical_bridging_flow(0.6, 1.0, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.6);
    assert!((spec.spacing_mm - 0.65).abs() <= 1e-6);
}

#[test]
fn bridging_flow_uses_nozzle_when_bridge_line_width_is_unset() {
    let spec = canonical_bridging_flow(0.0, 1.0, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.4);
    assert!((spec.spacing_mm - 0.45).abs() <= 1e-6);
}

#[test]
fn bridging_flow_scales_thread_diameter_by_ratio() {
    let spec = canonical_bridging_flow(0.0, 0.25, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.2);
    assert_eq!(spec.spacing_mm, 0.25);
}

#[test]
fn depth_window_preserves_full_gather_at_cutoffs_and_filtered_layer_gaps() {
    let height = 0.2_f32;
    let factor = 2.0_f32;
    let current_z = 1.0_f32;
    let cutoff = current_z - height * factor - 0.0001_f32;
    let below = f32::from_bits(cutoff.to_bits() - 1);
    let above = f32::from_bits(cutoff.to_bits() + 1);
    // Gaps model the already-matched region timeline, not the unfiltered stack.
    let cases = [
        (vec![current_z], 0),
        (vec![0.1, current_z], 0),
        (vec![0.1, 0.2, current_z], 1), // retain the far-below predecessor
        (vec![0.1, below, cutoff, above, 0.9, current_z], 2),
        (vec![0.1, 0.7, current_z], 1),
    ];
    for (zs, expected_start) in cases {
        let index = zs.len() - 1;
        let start = depth_window_start(&zs, index, height, factor);
        assert_eq!(start, expected_start, "Z timeline: {zs:?}");
        let layers: Vec<_> = zs
            .iter()
            .enumerate()
            .map(|(i, &print_z)| BridgeDepthLayer {
                print_z,
                sparse_infill: vec![square(10.0 + i as f32, 10.0)],
                not_sparse_infill: vec![square(1.0 + i as f32, 1.0)],
            })
            .collect();
        let full = gather_areas_w_depth(&layers, index, height, factor);
        let trimmed = gather_areas_w_depth(&layers[start..], index - start, height, factor);
        assert_eq!(full, trimmed, "Z timeline: {zs:?}");
        assert_eq!(full.is_empty(), index == 0, "non-vacuous gather: {zs:?}");
    }
    assert_eq!(depth_window_start(&[], 0, height, factor), 0);
    assert!(gather_areas_w_depth(&[], 0, height, factor).is_empty());
}

#[test]
fn filled_window_preserves_removal_at_inclusive_cutoff_and_empty_windows() {
    let height = 0.2_f32;
    let current_z = 1.0_f32;
    let cutoff = current_z - height - 0.0001_f32;
    let below = f32::from_bits(cutoff.to_bits() - 1);
    let above = f32::from_bits(cutoff.to_bits() + 1);
    for (zs, expected_start) in [
        (vec![], 0),
        (vec![0.1, below], 2),
        (vec![cutoff, above], 0),
        (vec![0.1, below, cutoff, above], 2),
    ] {
        let start = filled_window_start(&zs, current_z, height);
        assert_eq!(start, expected_start, "Z timeline: {zs:?}");
        let layers: Vec<_> = zs
            .iter()
            .enumerate()
            .map(|(i, &print_z)| BridgeCandidateLayer {
                print_z,
                new_polys: vec![square(1.0 + i as f32, 1.0)],
            })
            .collect();
        let sparse = vec![square(10.0, 10.0)];
        let full = remove_filled_polygons_on_lower_layers(&sparse, &layers, current_z, height);
        let trimmed =
            remove_filled_polygons_on_lower_layers(&sparse, &layers[start..], current_z, height);
        assert_eq!(full, trimmed, "Z timeline: {zs:?}");
        assert_eq!(
            full == sparse,
            start == zs.len(),
            "non-vacuous removal: {zs:?}"
        );
    }
}
