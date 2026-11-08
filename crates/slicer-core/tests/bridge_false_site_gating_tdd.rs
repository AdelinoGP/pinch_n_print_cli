#![allow(missing_docs)]

//! Whole-set bridge classification: a bottom area is a bridge only where the
//! layer below does not support it.
//!
//! These claims used to live in `gate_bridge_areas_by_unsupported_span`, which
//! subtracted the committed lower layer *after* the facet path assembled
//! candidates. Ticket 123 moved the difference into the classifier itself
//! (canonical `PrintObject::detect_surfaces_type`), so the same claims are
//! re-derived here against `classify_unsupported_bridge_areas`.

use slicer_core::algos::prepass_slice::classify_unsupported_bridge_areas;
use slicer_core::polygon_ops::intersection;
use slicer_ir::{ExPolygon, Point2, Polygon, SlicedRegion};

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

fn region_with_infill(infill: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: "object".to_string(),
        infill_areas: vec![infill],
        ..Default::default()
    }
}

fn area_mm2(region: &SlicedRegion) -> f64 {
    fn ring_area(ring: &Polygon) -> f64 {
        let points = &ring.points;
        if points.len() < 3 {
            return 0.0;
        }
        let twice: f64 = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let next = &points[(index + 1) % points.len()];
                point.x as f64 * next.y as f64 - next.x as f64 * point.y as f64
            })
            .sum();
        twice.abs() / 2.0
    }
    let units2: f64 = region
        .bridge_areas
        .iter()
        .map(|expolygon| {
            ring_area(&expolygon.contour) - expolygon.holes.iter().map(ring_area).sum::<f64>()
        })
        .sum();
    units2 / 1.0e8
}

/// A span the layer below fully covers is not a bridge: the safety-offset
/// difference is empty.
#[test]
fn solid_underneath_span_produces_no_bridge_area() {
    let body = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_infill(body.clone());
    classify_unsupported_bridge_areas(&mut region, &[body.clone()], Some(&[body]), 0.4);
    assert!(region.bridge_areas.is_empty());
    assert!(!region.is_bridge);
}

/// The unsupported part is retained in full and never overlaps the anchor.
#[test]
fn unsupported_span_retains_bridge_area() {
    let current = square(0.0, 0.0, 10.0, 10.0);
    let anchor = square(0.0, 0.0, 4.0, 10.0);
    let mut region = region_with_infill(current.clone());
    classify_unsupported_bridge_areas(&mut region, &[current], Some(&[anchor.clone()]), 0.4);

    assert!(region.is_bridge);
    assert!(
        intersection(&region.bridge_areas, std::slice::from_ref(&anchor)).is_empty(),
        "the anchor span must not be reported as bridge area"
    );
    // 6 x 10 mm minus the one-unit safety offset at the anchor boundary.
    let area = area_mm2(&region);
    assert!(
        (area - 60.0).abs() < 0.3,
        "retained area must be the 60 mm2 unsupported remainder, got {area:.2} mm2"
    );
}

/// Canonical's `lower_layer == nullptr` guard: with no previous cross-section,
/// a bottom is `stBottom`, never a bridge. Pre-existing candidates are cleared.
#[test]
fn no_lower_layer_clears_bridge_areas() {
    let body = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_infill(body.clone());
    region.bridge_areas = vec![body.clone()];
    region.is_bridge = true;
    classify_unsupported_bridge_areas(&mut region, &[body], None, 0.4);
    assert!(region.bridge_areas.is_empty());
    assert!(!region.is_bridge);
}

/// An existing-but-empty lower layer subtracts nothing, so the whole span is
/// unsupported (the old gate's AC-2 distinction between missing and empty).
#[test]
fn existing_empty_lower_layer_classifies_the_whole_span() {
    let body = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_infill(body.clone());
    classify_unsupported_bridge_areas(&mut region, &[body], Some(&[]), 0.4);
    assert!(region.is_bridge);
    let area = area_mm2(&region);
    assert!(
        (area - 100.0).abs() < 0.5,
        "an empty lower layer must leave the whole 100 mm2 span unsupported, got {area:.2} mm2"
    );
}
