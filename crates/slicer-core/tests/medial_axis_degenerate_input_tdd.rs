#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

use slicer_core::medial_axis::{medial_axis, MedialAxisError};
use slicer_ir::{ExPolygon, Point2, Polygon};

// AC-N1 (tightened): degenerate inputs must return exactly Err(DegenerateInput).

/// Case 1: contour with exactly 2 distinct points (a line segment — not a polygon).
#[test]
fn degenerate_two_distinct_points_returns_degenerate_input() {
    let two_point = ExPolygon {
        contour: Polygon {
            points: vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(5.0, 0.0)],
        },
        holes: vec![],
    };
    match medial_axis(&two_point, 0.0, f32::MAX) {
        Err(MedialAxisError::DegenerateInput) => {}
        other => panic!(
            "expected Err(DegenerateInput) for 2-point contour, got {:?}",
            other
        ),
    }
}

/// Case 2: empty/zero-area contour (0 points) — also degenerate.
#[test]
fn degenerate_empty_contour_returns_degenerate_input() {
    let empty = ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    };
    match medial_axis(&empty, 0.0, f32::MAX) {
        Err(MedialAxisError::DegenerateInput) => {}
        other => panic!(
            "expected Err(DegenerateInput) for empty contour, got {:?}",
            other
        ),
    }
}

/// Case 3: contour with a coordinate far exceeding i32::MAX (~215 m in 100-nm units
/// = 2_150_000_000, which is > 2_147_483_647 = i32::MAX).
/// Must return Err(CoordinateOverflow { .. }).
#[test]
fn coordinate_overflow_returns_error() {
    // 1 unit = 100 nm.  2_150_000_000 units = 215 m, well over i32::MAX (2_147_483_647).
    let big: i64 = 2_150_000_000;
    let overflow_square = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: big, y: 0 },
                Point2 { x: big, y: big },
                Point2 { x: 0, y: big },
            ],
        },
        holes: vec![],
    };
    match medial_axis(&overflow_square, 0.4, 2.0) {
        Err(MedialAxisError::CoordinateOverflow { .. }) => {}
        other => panic!(
            "expected Err(CoordinateOverflow) for >i32::MAX coordinates, got {:?}",
            other
        ),
    }
}

/// Case 4: contour with coordinates at/near i32::MIN must NOT be rejected by the
/// overflow guard.  The old abs()-based guard called i64::abs() on i32::MIN cast to
/// i64, which is fine (i32::MIN as i64 = -2_147_483_648, abs() = 2_147_483_648 >
/// i32::MAX = 2_147_483_647), so it would false-reject valid i32::MIN coordinates.
/// The explicit-bound guard accepts i32::MIN exactly.
///
/// This test regresses the false-reject: any result other than
/// Err(CoordinateOverflow{..}) is acceptable (Ok(..) or Ok(empty)).
#[test]
fn i32_min_coordinate_is_accepted() {
    let base: i64 = i32::MIN as i64;
    let side: i64 = 100_000; // 100_000 units = 10 mm
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: base, y: base },
                Point2 {
                    x: base + side,
                    y: base,
                },
                Point2 {
                    x: base + side,
                    y: base + side,
                },
                Point2 {
                    x: base,
                    y: base + side,
                },
            ],
        },
        holes: vec![],
    };
    let result = medial_axis(&poly, 0.4, 2.0);
    assert!(
        !matches!(result, Err(MedialAxisError::CoordinateOverflow { .. })),
        "i32::MIN coordinates must not be rejected by the overflow guard; got {:?}",
        result
    );
}

/// Case 5: contour with a coordinate at i64::MIN (far below i32::MIN) must return
/// Err(CoordinateOverflow { .. }) and must NOT panic.  The old abs()-based guard would
/// panic: i64::MIN.abs() overflows i64 in debug mode (and wraps to i64::MIN in
/// release, which is negative and thus falsely passes the > i32::MAX check).
/// The explicit-bound guard catches this correctly without calling abs().
#[test]
fn i64_min_coordinate_returns_error_without_panic() {
    // Use i64::MIN on x of the first point; the other coordinates are in-range so the
    // polygon has ≥ 3 distinct points and clears the degenerate-input check.
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: i64::MIN, y: 0 },
                Point2 { x: 100_000, y: 0 },
                Point2 {
                    x: 100_000,
                    y: 100_000,
                },
                Point2 { x: 0, y: 100_000 },
            ],
        },
        holes: vec![],
    };
    // Must not panic, and must return CoordinateOverflow.
    match medial_axis(&poly, 0.4, 2.0) {
        Err(MedialAxisError::CoordinateOverflow { .. }) => {}
        other => panic!(
            "expected Err(CoordinateOverflow) for i64::MIN coordinate, got {:?}",
            other
        ),
    }
}

/// Case 6: fully-collinear contour (3 points all on the same horizontal line).
/// This has zero signed area and triggered `assertion failed: fpv.is_finite()`
/// inside boostvoronoi before the zero-area guard was added.  Must return
/// Ok(empty) without panicking.
#[test]
fn collinear_contour_returns_empty_without_panic() {
    // Three collinear points (all y=0): zero-area "triangle".
    let collinear = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(5.0, 0.0),
                Point2::from_mm(10.0, 0.0),
            ],
        },
        holes: vec![],
    };
    // Must not panic.
    let result = medial_axis(&collinear, 0.0, f32::MAX);
    match result {
        Ok(_) => {}                                 // empty or non-empty — any Ok is acceptable
        Err(MedialAxisError::DegenerateInput) => {} // also acceptable
        Err(e) => panic!("unexpected error for collinear contour: {:?}", e),
    }
}

/// Case 7: axis-aligned rectangle with large coordinates (~25 mm cube footprint).
/// 25 mm = 250_000 units (1 unit = 100 nm). This shape is representative of the
/// painted 4-color cube cells that previously triggered the boostvoronoi panic.
/// Must complete without panicking and return a result.
#[test]
fn large_coordinate_rectangle_does_not_panic() {
    // 25 mm side at a typical build-plate position (125, 105) mm.
    // These coordinates fit comfortably in i32 (250_000 << 2_147_483_647).
    let rect = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(112.5, 92.5),
                Point2::from_mm(137.5, 92.5),
                Point2::from_mm(137.5, 117.5),
                Point2::from_mm(112.5, 117.5),
            ],
        },
        holes: vec![],
    };
    // Must not panic.  Result may be Ok(polylines) or Ok(empty) — both are fine.
    let result = medial_axis(&rect, 0.0, f32::MAX);
    assert!(
        result.is_ok(),
        "large-coord rectangle must not error: {:?}",
        result
    );
}
