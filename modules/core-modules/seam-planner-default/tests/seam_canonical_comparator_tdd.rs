//! Canonical seam comparator ordering tests.

#![allow(missing_docs)]

#[path = "../src/comparator.rs"]
mod comparator;

use comparator::{EnforcedBlockedSeamPoint, SeamCandidate, SeamComparator, SeamSetup};

fn candidate() -> SeamCandidate {
    SeamCandidate {
        position: [0.0, 0.0, 0.0],
        visibility: 0.0,
        overhang: 0.0,
        unsupported_dist: 0.0,
        embedded_distance: 0.0,
        local_ccw_angle: 0.0,
        layer_angle: 0.0,
        central_enforcer: false,
        point_type: EnforcedBlockedSeamPoint::Neutral,
        flow_width: 0.4,
    }
}

#[test]
fn painted_seam_priority_before_chaining() {
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let mut enforced = candidate();
    enforced.point_type = EnforcedBlockedSeamPoint::Enforced;
    enforced.central_enforcer = true;

    let mut neutral = candidate();
    neutral.local_ccw_angle = -2.0;

    assert!(comparator.is_first_better(&enforced, &neutral, None));
    assert!(!comparator.is_first_better(&neutral, &enforced, None));
}

#[test]
fn painted_seam_blocked_is_excluded() {
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let mut blocked = candidate();
    blocked.point_type = EnforcedBlockedSeamPoint::Blocked;
    blocked.local_ccw_angle = -2.0;

    let neutral = candidate();

    assert!(comparator.is_first_better(&neutral, &blocked, None));
    assert!(!comparator.is_first_better(&blocked, &neutral, None));
}

#[test]
fn layer_angle_field_is_present() {
    let mut seam = candidate();
    seam.layer_angle = 1.25;
    assert_eq!(seam.layer_angle, 1.25);
}
