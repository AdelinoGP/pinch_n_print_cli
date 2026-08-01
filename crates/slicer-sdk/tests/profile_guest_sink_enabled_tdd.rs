//! End-to-end proof that the guest profiling sink turns `slicer-core`'s inert
//! scope marks into host calls (ADR-0055).
//!
//! # Why this is its own test binary
//!
//! `slicer_core::profile::install` is once-per-process, so a binary can hold
//! exactly one sink for its whole life. The enabled and disabled cases are
//! therefore two binaries, not two `#[test]`s:
//! `profile_guest_sink_disabled_tdd.rs` is the other half.
//!
//! # What "the right scope identity" means here
//!
//! The fake host mirrors `slicer_wasm_host::profiling::register_scope`: a name
//! that matches a `slicer-core` scope resolves to that scope's *reserved* id
//! rather than minting a new one. So a mark for `polygon_ops::offset` must
//! arrive as `ScopeId::OFFSET.raw()` — and it must arrive because the bridge
//! registered the name and used the answer, which
//! `crates/slicer-sdk/src/profile.rs`'s unit tests prove separately with a host
//! that deliberately answers with different ids.

use std::cell::RefCell;

use slicer_core::polygon_ops::{difference_ex, offset, OffsetJoinType};
use slicer_core::profile::ScopeId;
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::profile::{BridgeSink, MarkEdge, ProfilingHost};

thread_local! {
    static REGISTERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static MARKS: RefCell<Vec<(u32, MarkEdge)>> = const { RefCell::new(Vec::new()) };
}

/// Stands in for the real host's `profiling` implementation.
struct FakeHost;

impl ProfilingHost for FakeHost {
    fn profile_enabled(&self) -> bool {
        true
    }

    fn profile_register(&self, name: &str) -> u32 {
        REGISTERED.with(|reg| reg.borrow_mut().push(name.to_string()));
        // Same rule as `slicer_wasm_host::profiling::register_scope`.
        for core in ScopeId::CORE_SCOPES {
            if core.name() == Some(name) {
                return core.raw();
            }
        }
        ScopeId::USER_BASE
    }

    fn profile_mark(&self, scope: u32, edge: MarkEdge) {
        MARKS.with(|marks| marks.borrow_mut().push((scope, edge)));
    }
}

static SINK: BridgeSink<FakeHost> = BridgeSink::new(FakeHost);

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

fn take_marks() -> Vec<(u32, MarkEdge)> {
    MARKS.with(|marks| marks.borrow_mut().drain(..).collect())
}

#[test]
fn installed_guest_sink_forwards_polygon_ops_marks_to_the_host() {
    let subject = vec![square(0.0, 10.0)];
    let clip = vec![square(4.0, 12.0)];

    // ---- Before install: the marks exist but reach nothing -----------------
    assert!(
        slicer_core::profile::sink().is_none(),
        "this test must own the process-global sink slot"
    );
    let _ = offset(&subject, 0.5, OffsetJoinType::Miter, 0.0);
    assert!(
        take_marks().is_empty(),
        "an uninstalled sink must be a true no-op"
    );

    // ---- Install ------------------------------------------------------------
    assert!(SINK.prime(), "the fake host reports profiling on");
    assert!(SINK.is_enabled());
    assert!(
        slicer_core::profile::install(&SINK),
        "this test must win the install race"
    );

    // Priming registered every core scope by name, up front — never on the hot
    // path.
    let expected_names: Vec<String> = ScopeId::CORE_SCOPES
        .iter()
        .map(|s| s.name().expect("core scope must be named").to_string())
        .collect();
    assert_eq!(REGISTERED.with(|reg| reg.borrow().clone()), expected_names);

    // ---- A marked primitive produces exactly one enter/exit pair -----------
    let _ = offset(&subject, 0.5, OffsetJoinType::Miter, 0.0);
    assert_eq!(
        take_marks(),
        vec![
            (ScopeId::OFFSET.raw(), MarkEdge::Enter),
            (ScopeId::OFFSET.raw(), MarkEdge::Exit),
        ],
        "`polygon_ops::offset` must mark under the OFFSET scope"
    );

    // ---- A delegating wrapper reports its primitive, not itself ------------
    // `difference_ex` -> `difference` -> `clip_polygons`.
    let _ = difference_ex(&subject, &clip);
    assert_eq!(
        take_marks(),
        vec![
            (ScopeId::CLIP_POLYGONS.raw(), MarkEdge::Enter),
            (ScopeId::CLIP_POLYGONS.raw(), MarkEdge::Exit),
        ]
    );

    // ---- Nothing was registered on the hot path ----------------------------
    assert_eq!(
        REGISTERED.with(|reg| reg.borrow().clone()),
        expected_names,
        "`profile-register` must not be called while marking"
    );
}
