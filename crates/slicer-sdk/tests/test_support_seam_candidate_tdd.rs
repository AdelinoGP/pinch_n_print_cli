//! TDD tests for the `seam_candidate` freestanding fixture helper.

use slicer_ir::{Point3WithWidth, SeamReason};
use slicer_sdk::test_prelude::*;

fn pt(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

#[test]
fn seam_candidate_round_trip_preserves_inputs() {
    let pos = pt(1.5, 2.5);
    let sc = seam_candidate(pos, 0.75, SeamReason::Sharp);
    assert_eq!(sc.position, pos);
    assert!((sc.score - 0.75).abs() < f32::EPSILON);
    assert_eq!(sc.reason, SeamReason::Sharp);
}

#[test]
fn seam_candidate_supports_all_seam_reasons() {
    let reasons = [
        SeamReason::Concave,
        SeamReason::Aligned,
        SeamReason::UserForced,
        SeamReason::Sharp,
    ];
    for reason in reasons {
        let sc = seam_candidate(pt(0.0, 0.0), 0.1, reason);
        assert_eq!(sc.reason, reason);
    }
}

#[test]
fn seam_candidate_score_can_be_zero_or_negative_sentinel() {
    // The fixture helper does not clamp.
    let sc_zero = seam_candidate(pt(0.0, 0.0), 0.0, SeamReason::Aligned);
    assert!((sc_zero.score - 0.0).abs() < f32::EPSILON);
    let sc_neg = seam_candidate(pt(0.0, 0.0), -1.0, SeamReason::Aligned);
    assert!((sc_neg.score + 1.0).abs() < f32::EPSILON);
}
