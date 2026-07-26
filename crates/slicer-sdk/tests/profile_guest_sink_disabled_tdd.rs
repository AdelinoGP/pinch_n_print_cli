//! The other half of `profile_guest_sink_enabled_tdd.rs`: with the host
//! reporting profiling **off**, an installed guest sink must never call the host
//! again (ADR-0050, "Always compiled in, host-gated").
//!
//! Marks ship in every guest, so this is the property that makes that
//! affordable: after the one cached `profile-enabled` answer, a marked
//! `polygon_ops` call costs a branch on a cached bool and nothing crosses the
//! WIT boundary.
//!
//! Separate binary because `slicer_core::profile::install` is once-per-process.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

use slicer_core::polygon_ops::{
    clip_polygons, closing_ex, difference_ex, offset, offset2_ex, opening, ClipOperation,
    OffsetJoinType,
};
use slicer_core::profile::ScopeId;
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::profile::{BridgeSink, MarkEdge, ProfilingHost};

/// Counts every crossing of the (simulated) WIT boundary.
static ENABLED_QUERIES: AtomicU32 = AtomicU32::new(0);
static REGISTER_CALLS: AtomicU32 = AtomicU32::new(0);
static MARK_CALLS: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static MARKED_SCOPES: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

struct DisabledHost;

impl ProfilingHost for DisabledHost {
    fn profile_enabled(&self) -> bool {
        ENABLED_QUERIES.fetch_add(1, Ordering::Relaxed);
        false
    }

    fn profile_register(&self, _name: &str) -> u32 {
        REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
        ScopeId::USER_BASE
    }

    fn profile_mark(&self, scope: u32, _edge: MarkEdge) {
        MARK_CALLS.fetch_add(1, Ordering::Relaxed);
        MARKED_SCOPES.with(|scopes| scopes.borrow_mut().push(scope));
    }
}

static SINK: BridgeSink<DisabledHost> = BridgeSink::new(DisabledHost);

fn square(min: f32, max: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min, min),
                Point2::from_mm(max, min),
                Point2::from_mm(max, max),
                Point2::from_mm(min, max),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn a_disabled_host_is_asked_once_and_never_called_again() {
    let subject = vec![square(0.0, 10.0)];
    let clip = vec![square(4.0, 12.0)];

    assert!(!SINK.prime(), "the fake host reports profiling off");
    assert!(!SINK.is_enabled());
    assert!(
        slicer_core::profile::install(&SINK),
        "this test must win the install race"
    );
    assert!(
        slicer_core::profile::sink().is_some(),
        "the sink really is installed — this test would be vacuous otherwise"
    );

    // Every marked primitive, plus the wrappers that delegate into them.
    let _ = clip_polygons(&subject, &clip, ClipOperation::Difference);
    let _ = offset(&subject, 0.5, OffsetJoinType::Miter, 0.0);
    let _ = offset2_ex(&subject, -0.5, 0.5, OffsetJoinType::Miter, 3.0);
    let _ = opening(&subject, 0.5, OffsetJoinType::Miter);
    let _ = closing_ex(&subject, 0.5, OffsetJoinType::Miter);
    let _ = difference_ex(&subject, &clip);

    assert_eq!(
        MARK_CALLS.load(Ordering::Relaxed),
        0,
        "no mark may cross the boundary with profiling off, saw {:?}",
        MARKED_SCOPES.with(|scopes| scopes.borrow().clone())
    );
    assert_eq!(
        REGISTER_CALLS.load(Ordering::Relaxed),
        0,
        "scope names must not be registered when nothing will be marked"
    );
    assert_eq!(
        ENABLED_QUERIES.load(Ordering::Relaxed),
        1,
        "`profile-enabled` is cached once per guest instance"
    );

    // Repeat priming (as every WIT export body does, via `install_guest_sink`)
    // must not re-ask.
    for _ in 0..8 {
        assert!(!SINK.prime());
    }
    assert_eq!(ENABLED_QUERIES.load(Ordering::Relaxed), 1);
}
