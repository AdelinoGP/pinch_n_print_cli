#![allow(missing_docs)]
//! AC-1: offset2_ex round-trip identity on a convex square.
//!
//! Geometric reasoning:
//!   offset2_ex(square, delta1=-1, delta2=+1) = erode by 1 mm → (1,1)..(9,9),
//!   then dilate by 1 mm → back to original (0,0)..(10,10).
//!
//! The AC packet text mentions "(1,1)..(9,9)" as an intermediate; the correct
//! post-round-trip result on a convex miter-joined square is (0,0)..(10,10).
//! This test asserts the geometrically-correct round-trip identity AND adds a
//! secondary assertion on the intermediate erode-only result.
//!
//! AC text discrepancy note: packet AC-1 states contour AABB "(1.0,1.0)..(9.0,9.0)"
//! but that describes the intermediate erode result, not the round-trip.  The
//! documented semantics ("shrink-then-expand by the same delta returns the
//! original shape modulo join tolerance") require asserting (0,0)..(10,10).
//! We follow the documented semantics and note the AC text is imprecise.

use slicer_core::polygon_ops::{offset, offset2_ex, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Build a square ExPolygon from (min_mm, min_mm) to (max_mm, max_mm).
fn square_mm(lo: f32, hi: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(lo, lo),
                Point2::from_mm(hi, lo),
                Point2::from_mm(hi, hi),
                Point2::from_mm(lo, hi),
            ],
        },
        holes: Vec::new(),
    }
}

fn contour_aabb(polys: &[ExPolygon]) -> (f64, f64, f64, f64) {
    let units_per_mm = 10_000.0_f64;
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for ep in polys {
        for p in &ep.contour.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    (
        min_x as f64 / units_per_mm,
        min_y as f64 / units_per_mm,
        max_x as f64 / units_per_mm,
        max_y as f64 / units_per_mm,
    )
}

#[test]
fn offset2_ex_round_trip_identity_on_convex_square() {
    // 10mm × 10mm square from (0,0) to (10,10)
    let square = square_mm(0.0, 10.0);

    // Round-trip: erode 1 mm then dilate 1 mm
    let result = offset2_ex(
        &[square],
        -1.0,
        1.0,
        OffsetJoinType::Miter,
        // miter_limit: OrcaSlicer default is 125% of scale = 2.0 per Clipper2 docs
        2.0,
    );

    assert!(
        !result.is_empty(),
        "offset2_ex round-trip must not collapse the square"
    );

    let (min_x, min_y, max_x, max_y) = contour_aabb(&result);
    let tol = 0.01; // 0.01 mm tolerance for miter join
    assert!(
        (min_x - 0.0).abs() < tol,
        "round-trip min_x expected ≈0.0, got {min_x}"
    );
    assert!(
        (min_y - 0.0).abs() < tol,
        "round-trip min_y expected ≈0.0, got {min_y}"
    );
    assert!(
        (max_x - 10.0).abs() < tol,
        "round-trip max_x expected ≈10.0, got {max_x}"
    );
    assert!(
        (max_y - 10.0).abs() < tol,
        "round-trip max_y expected ≈10.0, got {max_y}"
    );
}

#[test]
fn offset2_ex_intermediate_erode_gives_inner_square() {
    // Secondary assertion: erode-only gives the (1,1)..(9,9) intermediate
    // that the AC text references.
    let square = square_mm(0.0, 10.0);
    let eroded = offset(&[square], -1.0_f32, OffsetJoinType::Miter, 0.0);

    assert!(
        !eroded.is_empty(),
        "erode by 1 mm must not collapse 10mm square"
    );

    let (min_x, min_y, max_x, max_y) = contour_aabb(&eroded);
    let tol = 0.01;
    assert!(
        (min_x - 1.0).abs() < tol,
        "erode min_x expected ≈1.0, got {min_x}"
    );
    assert!(
        (min_y - 1.0).abs() < tol,
        "erode min_y expected ≈1.0, got {min_y}"
    );
    assert!(
        (max_x - 9.0).abs() < tol,
        "erode max_x expected ≈9.0, got {max_x}"
    );
    assert!(
        (max_y - 9.0).abs() < tol,
        "erode max_y expected ≈9.0, got {max_y}"
    );
}
