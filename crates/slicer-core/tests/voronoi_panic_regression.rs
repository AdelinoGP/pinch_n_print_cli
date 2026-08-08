#![cfg(feature = "host-algos")]
#![allow(missing_docs)]
#![doc = "Regression test for boostvoronoi's guarded predicate panic."]

//! Explicitly enabled regression for boostvoronoi's production panic guards.
//!
//! This target is intentionally separate from `voronoi_stress`: enabling
//! `boostvoronoi/console_debug` for this one test must never become part of the
//! workspace's normal test feature set.

use slicer_core::voronoi::{voronoi_from_segments, Segment, VoronoiError};
use slicer_ir::Point2;

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn seg(a: Point2, b: Point2) -> Segment {
    Segment {
        a,
        b,
        ..segment_base()
    }
}

fn segment_base() -> Segment {
    // exhaustive: file-local base; no Default impl for Segment (packet 196)
    Segment {
        a: Point2::default(),
        b: Point2::default(),
    }
}

/// The synthetic input must trip boostvoronoi's guarded assertion. If the
/// `catch_unwind` arm is removed, this test fails by unwinding the test
/// process instead of accepting an `Err(VoronoiError::PredicatePanic)`.
#[test]
fn voronoi_from_segments_predicate_panic_fires_on_synthetic_input() {
    let segments = [
        seg(p(i64::MAX, 0), p(i64::MAX, 1_000_000_000)),
        seg(p(i64::MAX, 1_000_000_000), p(i64::MAX - 1, 1_000_000_000)),
        seg(p(i64::MAX - 1, 1_000_000_000), p(i64::MAX - 1, 0)),
    ];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        voronoi_from_segments(&segments)
    }));
    let inner = result.unwrap_or_else(|payload| {
        panic!(
            "voronoi_from_segments panicked instead of returning a Result: {:?}",
            payload
        )
    });

    match inner {
        Err(VoronoiError::PredicatePanic {
            segment_count,
            min_x,
            min_y,
            max_x,
            max_y,
            has_duplicate_endpoint,
            has_zero_length_segment,
            has_near_collinear_pair,
        }) => {
            assert_eq!(segment_count, 3);
            assert!(has_duplicate_endpoint);
            assert!(!has_zero_length_segment);
            assert!(!has_near_collinear_pair);
            assert_eq!(min_x, i64::MAX - 1);
            assert_eq!(max_x, i64::MAX);
            assert_eq!(min_y, 0);
            assert_eq!(max_y, 1_000_000_000);
        }
        Ok(_) => panic!("synthetic input did not fire boostvoronoi's guarded predicate panic"),
        Err(other) => panic!(
            "expected PredicatePanic on synthetic stress input, got {:?}",
            other
        ),
    }
}
