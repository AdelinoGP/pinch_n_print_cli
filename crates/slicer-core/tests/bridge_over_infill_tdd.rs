#![allow(missing_docs)]

use slicer_core::algos::bridge_over_infill::{
    construct_anchored_polygon, determine_bridging_angle,
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
