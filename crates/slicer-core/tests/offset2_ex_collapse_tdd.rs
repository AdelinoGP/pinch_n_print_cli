#![allow(missing_docs)]
//! AC-N2: offset2_ex with a delta1 that fully removes the input returns
//! `Vec::new()` (empty, no panic).

use slicer_core::polygon_ops::{offset2_ex, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn tiny_square_mm() -> ExPolygon {
    // 1mm × 1mm square
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(1.0, 0.0),
                Point2::from_mm(1.0, 1.0),
                Point2::from_mm(0.0, 1.0),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn offset2_ex_collapse_returns_empty_no_panic() {
    // Erode by 100 mm — completely destroys a 1 mm square.
    // The second pass (dilate) must not panic on empty input and must
    // return an empty Vec.
    let result = offset2_ex(&[tiny_square_mm()], -100.0, 1.0, OffsetJoinType::Miter, 2.0);
    assert!(
        result.is_empty(),
        "fully-eroded input must produce empty output, got {} polygon(s)",
        result.len()
    );
}

#[test]
fn offset2_ex_empty_input_returns_empty() {
    let result = offset2_ex(&[], -1.0, 1.0, OffsetJoinType::Miter, 2.0);
    assert!(result.is_empty(), "empty input must produce empty output");
}
