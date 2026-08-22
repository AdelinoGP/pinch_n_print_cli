#![allow(missing_docs)]
//! TDD for `detect_support_overhangs`, the support-generation sibling of
//! `annotate_overhangs` (packet 224, RC-0).
//!
//! These tests pin the two properties that the previous support implementation
//! got wrong, and that made the decisive SupportTest fixture undetectable:
//!
//! 1. A contact is produced **once, at the overhang's own Z** — not re-derived
//!    at every layer, and not absent because the source facets happen to be
//!    coplanar. Contact detection is 2D over slices, so facet coplanarity is
//!    irrelevant to it.
//! 2. A mesh with no overhang produces **no contacts at all**. Any change that
//!    makes support appear under non-overhanging geometry is a regression.
//!
//! Mirrors canonical `detect_overhangs` (`SupportMaterial.cpp`), which grows the
//! lower layer by an angle-derived offset before differencing.

use slicer_core::algos::overhang_annotation::detect_support_overhangs;
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Axis-aligned rectangle in mm.
fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    let p = |x: f32, y: f32| Point2::from_mm(x, y);
    ExPolygon {
        contour: Polygon {
            points: vec![
                p(min_x, min_y),
                p(max_x, min_y),
                p(max_x, max_y),
                p(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

/// Total area of a polygon set, in mm^2, via the shoelace formula on contours.
fn area_mm2(polys: &[ExPolygon]) -> f32 {
    polys
        .iter()
        .map(|poly| {
            let pts = &poly.contour.points;
            let mut acc = 0.0_f64;
            for i in 0..pts.len() {
                let a = pts[i];
                let b = pts[(i + 1) % pts.len()];
                let (ax, ay) = (slicer_ir::units_to_mm(a.x) as f64, slicer_ir::units_to_mm(a.y) as f64);
                let (bx, by) = (slicer_ir::units_to_mm(b.x) as f64, slicer_ir::units_to_mm(b.y) as f64);
                acc += ax * by - bx * ay;
            }
            (acc / 2.0).abs() as f32
        })
        .sum()
}

/// A pillar that abruptly widens into a cap, reproducing the decisive
/// SupportTest fixture's shape: narrow column below, wide plate above. The
/// widening is a step, so in the mesh its downward facets are coplanar — which
/// is exactly what defeated facet-based detection.
fn pillar_then_cap() -> Vec<(u32, f32, Vec<ExPolygon>)> {
    let pillar = vec![rect(0.0, 0.0, 4.0, 4.0)];
    let cap = vec![rect(-8.0, 0.0, 12.0, 4.0)];
    vec![
        (0, 0.2, pillar.clone()),
        (1, 0.2, pillar.clone()),
        (2, 0.2, pillar.clone()),
        // The step: layer 3 is much wider than layer 2.
        (3, 0.2, cap.clone()),
        (4, 0.2, cap.clone()),
        (5, 0.2, cap),
    ]
}

#[test]
fn overhang_is_detected_once_at_the_step_layer() {
    let contacts = detect_support_overhangs(&pillar_then_cap(), 45.0);

    assert_eq!(
        contacts.keys().copied().collect::<Vec<_>>(),
        vec![3_u32],
        "contact must be produced exactly once, at the step layer, got keys {:?}",
        contacts.keys().collect::<Vec<_>>()
    );

    // The cap overhangs the pillar on both sides: 8mm left + 8mm right, 4mm
    // deep. The lower layer is grown by 0.2/tan(45 deg) = 0.2mm before the
    // difference, so each 8mm-wide wing loses 0.2mm of width.
    let expected = 2.0 * (8.0 - 0.2) * 4.0;
    let got = area_mm2(&contacts[&3]);
    assert!(
        (got - expected).abs() < 0.1,
        "contact area {got:.3}mm^2 should match the angle-thresholded overhang {expected:.3}mm^2"
    );
}

#[test]
fn coplanar_step_does_not_hide_the_contact() {
    // Regression pin for RC-1: facet-based detection filtered facets by
    // `max_z >= slab_bottom && min_z <= layer.z`, so a step whose downward
    // facets are coplanar matched at most one layer slab and typically none.
    // Slice-based detection cannot have that failure mode.
    let contacts = detect_support_overhangs(&pillar_then_cap(), 45.0);
    assert!(
        !contacts.is_empty(),
        "a coplanar step must still register a support contact"
    );
}

#[test]
fn straight_column_produces_no_contacts() {
    // The invariant the previous session's fallback destroyed: no overhang
    // must mean no support, regardless of the mesh being non-empty.
    let column = vec![rect(0.0, 0.0, 4.0, 4.0)];
    let layers: Vec<_> = (0..8).map(|i| (i as u32, 0.2_f32, column.clone())).collect();

    let contacts = detect_support_overhangs(&layers, 45.0);

    assert!(
        contacts.is_empty(),
        "a straight column has no overhang and must produce no contacts, got {contacts:?}"
    );
}

#[test]
fn shallower_threshold_yields_no_more_contact_area_than_a_plain_difference() {
    // Growing the lower layer before differencing can only shrink the result,
    // so an angle threshold is always a subset of the unsupported area.
    let layers = pillar_then_cap();
    let plain = detect_support_overhangs(&layers, 0.0);
    let thresholded = detect_support_overhangs(&layers, 45.0);

    let plain_area = area_mm2(&plain[&3]);
    let thresholded_area = area_mm2(&thresholded[&3]);
    assert!(
        thresholded_area <= plain_area + 1e-3,
        "thresholded contact ({thresholded_area:.3}) must not exceed the plain difference ({plain_area:.3})"
    );
    assert!(
        thresholded_area < plain_area,
        "a 45-degree threshold must actually trim the contact; got {thresholded_area:.3} vs {plain_area:.3}"
    );
}

#[test]
fn zero_angle_degenerates_to_a_plain_difference_without_dividing_by_zero() {
    let contacts = detect_support_overhangs(&pillar_then_cap(), 0.0);
    let expected = 2.0 * 8.0 * 4.0;
    let got = area_mm2(&contacts[&3]);
    assert!(
        (got - expected).abs() < 0.1,
        "zero threshold must be a plain difference: got {got:.3}mm^2, expected {expected:.3}mm^2"
    );
}

#[test]
fn degenerate_inputs_do_not_panic() {
    assert!(detect_support_overhangs(&[], 45.0).is_empty());
    let single = vec![(0_u32, 0.2_f32, vec![rect(0.0, 0.0, 4.0, 4.0)])];
    assert!(detect_support_overhangs(&single, 45.0).is_empty());
}
