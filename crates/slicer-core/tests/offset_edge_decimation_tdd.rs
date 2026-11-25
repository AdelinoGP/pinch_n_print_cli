#![allow(missing_docs)]
//! t169: canonical `append_offset_path` edge decimation in `polygon_ops`.
//!
//! Canonical `v2.5.0` `ClipperUtils.cpp::append_offset_path` drops vertices
//! closer than `0.005 * |delta|` to the last kept vertex before every
//! `ClipperOffset` call, and drops a closed ring left with fewer than three
//! vertices. The helper's own rules are pinned by the unit tests in
//! `crates/slicer-core/src/polygon_ops.rs`; this file pins the three behaviours
//! visible through the public API:
//!
//! 1. a sub-threshold pinhole is deleted instead of being inflated into a
//!    crater by an erosion (the `IB_DEEP_NEG` class t167 measured),
//! 2. a hole above the threshold is preserved and offset normally,
//! 3. `offset2_ex` still reconstructs hole nesting after pass 1's flat paths,
//!    which now pass through a second decimation at `delta2`'s threshold.

use slicer_core::polygon_ops::{offset, offset2_ex, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn square_mm(lo: f32, hi: f32) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(lo, lo),
            Point2::from_mm(hi, lo),
            Point2::from_mm(hi, hi),
            Point2::from_mm(lo, hi),
        ],
    }
}

fn span_mm(polygon: &Polygon) -> f64 {
    let (mut lo, mut hi) = (i64::MAX, i64::MIN);
    for p in &polygon.points {
        lo = lo.min(p.x);
        hi = hi.max(p.x);
    }
    (hi - lo) as f64 / 10_000.0
}

/// A 20 mm solid with a centred square hole of `hole_mm` side.
fn plate_with_hole_mm(hole_mm: f32) -> ExPolygon {
    let c = 10.0;
    let r = hole_mm / 2.0;
    ExPolygon {
        contour: square_mm(0.0, 20.0),
        holes: vec![Polygon {
            // Holes are CW by `ExPolygon`'s invariant.
            points: vec![
                Point2::from_mm(c - r, c - r),
                Point2::from_mm(c - r, c + r),
                Point2::from_mm(c + r, c + r),
                Point2::from_mm(c + r, c - r),
            ],
        }],
    }
}

#[test]
fn sub_threshold_pinhole_is_deleted_instead_of_inflated_by_erosion() {
    // 0.004 mm hole under a 1 mm erosion: the decimation threshold is
    // 0.005 * 1 mm = 0.005 mm, so all four hole vertices collapse and the ring
    // is dropped. Without decimation Clipper2 grows the hole with the erosion,
    // turning a 4 µm pinhole into a ~2 mm crater.
    let plate = plate_with_hole_mm(0.004);
    let eroded = offset(&[plate], -1.0_f32, OffsetJoinType::Miter, 0.0);

    assert_eq!(
        eroded.len(),
        1,
        "eroding a 20 mm plate by 1 mm leaves one region"
    );
    assert!(
        eroded[0].holes.is_empty(),
        "the 4 µm pinhole must be dropped by decimation, not inflated into a crater; got {} hole(s)",
        eroded[0].holes.len()
    );
    let span = span_mm(&eroded[0].contour);
    assert!(
        (span - 18.0).abs() < 0.01,
        "the plate itself erodes to 18 mm, got {span}"
    );
}

#[test]
fn hole_above_the_threshold_is_kept_and_grows_with_the_erosion() {
    // 0.02 mm hole: its 200-unit edges are four times the 50-unit threshold, so
    // the ring survives and grows by 1 mm per side with the erosion.
    let plate = plate_with_hole_mm(0.02);
    let eroded = offset(&[plate], -1.0_f32, OffsetJoinType::Miter, 0.0);

    assert_eq!(eroded.len(), 1);
    assert_eq!(
        eroded[0].holes.len(),
        1,
        "a 0.02 mm hole is above the 0.005 mm threshold and must survive"
    );
    let hole_span = span_mm(&eroded[0].holes[0]);
    assert!(
        (hole_span - 2.02).abs() < 0.01,
        "the hole grows to 0.02 + 2 * 1.0 mm, got {hole_span}"
    );
}

#[test]
fn offset2_ex_preserves_hole_nesting_across_both_decimated_passes() {
    // 20 mm plate, 6 mm hole, erode then dilate by 1 mm. Pass 1 emits flat
    // paths and pass 2 re-decimates at `delta2`'s threshold; nesting must still
    // be reconstructed from the PolyTree.
    let plate = plate_with_hole_mm(6.0);
    let round_tripped = offset2_ex(&[plate], -1.0, 1.0, OffsetJoinType::Miter, 2.0);

    assert_eq!(round_tripped.len(), 1);
    assert_eq!(
        round_tripped[0].holes.len(),
        1,
        "the round trip must not fill the 6 mm hole or report it as a second region"
    );
    assert!(
        (span_mm(&round_tripped[0].contour) - 20.0).abs() < 0.01,
        "the contour round-trips to 20 mm"
    );
    assert!(
        (span_mm(&round_tripped[0].holes[0]) - 6.0).abs() < 0.01,
        "the hole round-trips to 6 mm"
    );
}
