#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

use slicer_core::algos::bridge_over_infill::{
    expand_candidate_area, qualify_internal_bridge_surface, unsupported_span_areas,
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
fn fills_are_the_initial_unsupported_carrier() {
    // RC-A regression: canonical initializes the unsupported carrier to the
    // lower fills themselves (closing(SCALED_EPSILON)), NOT the bbox complement
    // of the fills. Two 0.8mm side slabs shrink by mult*spacing = 1.2mm and
    // annihilate -> empty. The old complement semantics returned the non-empty
    // gap between the slabs.
    let lower_fills = side_slabs(6.4, 0.8, 5.6);
    let unsupported = unsupported_span_areas(&lower_fills, &[], 0.4, 3.0);
    assert!(unsupported.is_empty());
}

#[test]
fn fully_filled_lower_layer_is_the_initial_unsupported_carrier() {
    let surface = square(0.0, 0.0, 6.4, 6.4);
    // Canonical: the fills themselves are the carrier, shrunk by mult*spacing.
    // The old complement semantics returned empty here.
    let unsupported = unsupported_span_areas(&[surface], &[], 0.4, 3.0);
    assert!(!unsupported.is_empty());
}

#[test]
fn fully_filled_lower_layer_qualifies() {
    let surface = square(0.0, 0.0, 6.4, 6.4);
    let unsupported = unsupported_span_areas(&[surface.clone()], &[], 0.4, 3.0);
    // A non-empty unsupported carrier qualifies as a bridge, with or without
    // the nofilter bypass.
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, false).is_some());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, true).is_some());
}

#[test]
fn unsupported_span_qualifies_and_clip_expand_matches_canonical() {
    let spacing = 0.4;
    let surface = square(0.0, 0.0, 6.4, 6.4);
    // Fully-filled lower layer: the shrunk fills are the unsupported carrier.
    let lower_fills = vec![square(0.0, 0.0, 6.4, 6.4)];
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
    // A small fill shrinks to a tiny unsupported carrier that trips the
    // partial-support area gate (unsupported_area <= 9*spacing^2).
    let lower_fills = vec![square(0.0, 0.0, 2.6, 2.6)];
    let unsupported = unsupported_span_areas(&lower_fills, &[], 0.4, 3.0);
    assert!(!unsupported.is_empty());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, false).is_none());
    assert!(qualify_internal_bridge_surface(&surface, &unsupported, 0.4, true).is_some());
}

#[test]
fn expansion_source_grows_candidate_by_one_spacing_before_clipping() {
    let candidate = square(2.0, 2.0, 3.0, 3.0);
    let source = square(3.0, 2.0, 4.0, 3.0);
    let deep = square(0.0, 0.0, 6.0, 6.0);
    let expanded = expand_candidate_area(
        std::slice::from_ref(&candidate),
        std::slice::from_ref(&source),
        std::slice::from_ref(&deep),
        std::slice::from_ref(&candidate),
        0.4,
    );
    let min_x = expanded
        .iter()
        .flat_map(|p| p.contour.points.iter())
        .map(|p| p.x)
        .min()
        .expect("candidate remains after expansion");
    assert!(min_x <= Point2::from_mm(1.6, 0.0).x);
}

#[test]
fn expansion_empty_candidate_short_circuits() {
    assert!(expand_candidate_area(&[], &[], &[], &[], 0.4).is_empty());
}

#[test]
fn expansion_uses_canonical_opening_and_closing_radii() {
    let candidate = square(1.0, 1.0, 5.0, 5.0);
    let deep = square(0.0, 0.0, 6.0, 6.0);
    let expanded = expand_candidate_area(
        std::slice::from_ref(&candidate),
        &[],
        std::slice::from_ref(&deep),
        std::slice::from_ref(&candidate),
        0.4,
    );
    assert!(!expanded.is_empty());
    let min_x = expanded
        .iter()
        .flat_map(|p| p.contour.points.iter())
        .map(|p| p.x)
        .min()
        .unwrap();
    assert!(min_x < Point2::from_mm(1.0, 0.0).x);
}
