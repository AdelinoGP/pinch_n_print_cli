#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

use slicer_core::algos::bridge_over_infill::{
    qualify_internal_bridge_surface, unsupported_span_areas,
};
use slicer_core::polygon_ops::{intersection, offset, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

fn side_slabs(surface_size: f32, gap_min: f32, gap_max: f32) -> Vec<ExPolygon> {
    vec![
        square(0.0, 0.0, gap_min, surface_size),
        square(gap_max, 0.0, surface_size, surface_size),
    ]
}

#[test]
fn fully_supported_surface_yields_no_unsupported_area() {
    let surface = square(0.0, 0.0, 6.4, 6.4);
    assert!(unsupported_span_areas(&[surface], &[], 0.4, 3.0).is_empty());
}

#[test]
fn fully_supported_surface_qualifies_nothing() {
    let surface = square(0.0, 0.0, 6.4, 6.4);
    let unsupported = unsupported_span_areas(&[surface.clone()], &[], 0.4, 3.0);
    // An empty unsupported intersection is not a bridge, even when nofilter is set.
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, false).is_none());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, true).is_none());
}

#[test]
fn unsupported_span_qualifies_and_clip_expand_matches_canonical() {
    let spacing = 0.4;
    let surface = square(0.0, 0.0, 6.4, 6.4);
    let lower_fills = side_slabs(6.4, 0.8, 5.6);
    let unsupported = unsupported_span_areas(&lower_fills, &[], spacing, 3.0);
    assert!(!unsupported.is_empty());

    let expected = intersection(
        std::slice::from_ref(&surface),
        &offset(&unsupported, 4.0 * spacing, OffsetJoinType::Miter, 0.0),
    );
    let worth = qualify_internal_bridge_surface(&surface, &unsupported, spacing, false)
        .expect("large unsupported span qualifies");
    assert_eq!(worth, expected);
}

#[test]
fn partial_support_area_gate_and_nofilter_bypass() {
    let surface = square(0.0, 0.0, 4.0, 4.0);
    let lower_fills = side_slabs(4.0, 0.5, 3.5);
    let unsupported = unsupported_span_areas(&lower_fills, &[], 0.4, 3.0);
    assert!(!unsupported.is_empty());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, false).is_none());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, true).is_some());
}
