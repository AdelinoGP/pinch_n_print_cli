#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

//! T-201 acceptance fixtures for `slicer_core::voronoi`.
//!
//! Covers AC-2 (square fixture: vertex/edge counts), AC-3's voronoi-stress
//! portion (collinear / T-junction / duplicate-vertex degeneracy classes
//! from `docs/adr/0023-arachne-port-strategy.md`), and AC-N1 (empty input
//! never touches `boostvoronoi`).
//!
//! All non-empty-input counts below are recorded from `boostvoronoi 0.12.1`
//! output observed by actually running these tests (see design.md Risks) —
//! they are not fabricated or ported from any OrcaSlicer reference.

use slicer_core::voronoi::{voronoi_from_segments, Segment, VoronoiError};
use slicer_ir::Point2;

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn seg(a: Point2, b: Point2) -> Segment {
    Segment { a, b }
}

/// AC-2: a unit square's four segments (corners at (0,0),(1000,0),(1000,1000),(0,1000))
/// produce the expected vertex/edge counts.
#[test]
fn voronoi_square_four_segments() {
    let segments = [
        seg(p(0, 0), p(1000, 0)),
        seg(p(1000, 0), p(1000, 1000)),
        seg(p(1000, 1000), p(0, 1000)),
        seg(p(0, 1000), p(0, 0)),
    ];

    let graph = match voronoi_from_segments(&segments) {
        Ok(graph) => graph,
        Err(err) => panic!("square fixture should build, got {err}"),
    };

    // Recorded from boostvoronoi 0.12.1 output for this exact fixture.
    assert_eq!(
        graph.vertices.len(),
        5,
        "expected 4 corner vertices + 1 centroid"
    );
    assert_eq!(
        graph.edges.len(),
        24,
        "recorded from boostvoronoi 0.12.1 output"
    );
}

/// AC-3 (collinear stress): a straight edge split into two collinear
/// segments sharing an endpoint must build without panicking. Boost-VD
/// handles collinear input via its own built-in degeneracy handling — no
/// pre-snap is exercised here (ADR-0023, "Collinear input points" row).
#[test]
fn voronoi_stress_collinear() {
    let segments = [seg(p(0, 0), p(500, 0)), seg(p(500, 0), p(1000, 0))];

    let graph = match voronoi_from_segments(&segments) {
        Ok(graph) => graph,
        Err(err) => panic!("collinear fixture should build, got {err}"),
    };

    // Recorded from boostvoronoi 0.12.1 output for this exact fixture.
    assert_eq!(
        graph.edges.len(),
        8,
        "recorded from boostvoronoi 0.12.1 output"
    );
}

/// AC-3 (T-junction stress): three segments meeting at a shared point — a
/// "+"-missing-one-arm shape, pre-resolved so the contact is a shared
/// endpoint rather than an interior touch (ADR-0023, "T-junctions" row: the
/// unresolved case is the caller's — T-204's — responsibility, not this
/// wrapper's).
#[test]
fn voronoi_stress_t_junction() {
    let hub = p(500, 500);
    let segments = [
        seg(p(0, 500), hub),
        seg(hub, p(1000, 500)),
        seg(hub, p(500, 1000)),
    ];

    let graph = match voronoi_from_segments(&segments) {
        Ok(graph) => graph,
        Err(err) => panic!("T-junction fixture should build, got {err}"),
    };

    // Recorded from boostvoronoi 0.12.1 output for this exact fixture.
    assert_eq!(
        graph.edges.len(),
        18,
        "recorded from boostvoronoi 0.12.1 output"
    );
}

/// AC-3 (duplicate-vertex stress): four segments radiating from a single
/// hub point, so that one coordinate value appears as an endpoint four
/// times in the flat input list (ADR-0023, "Duplicate vertices" row —
/// distinct from the 3-way T-junction case above).
#[test]
fn voronoi_stress_duplicate_vertex() {
    let hub = p(500, 500);
    let segments = [
        seg(hub, p(500, 1000)),
        seg(hub, p(1000, 500)),
        seg(hub, p(500, 0)),
        seg(hub, p(0, 500)),
    ];

    let graph = match voronoi_from_segments(&segments) {
        Ok(graph) => graph,
        Err(err) => panic!("duplicate-vertex fixture should build, got {err}"),
    };

    // Recorded from boostvoronoi 0.12.1 output for this exact fixture.
    assert_eq!(
        graph.edges.len(),
        24,
        "recorded from boostvoronoi 0.12.1 output"
    );
}

/// AC-N1: empty input returns `Err(VoronoiError::EmptyInput)`, never
/// touching `boostvoronoi` (no panic, no allocation past the error path).
#[test]
fn voronoi_empty_input_returns_err() {
    let result = voronoi_from_segments(&[]);
    assert_eq!(result, Err(VoronoiError::EmptyInput));
}

#[test]
fn voronoi_from_segments_degenerate_input_returns_result_not_panic() {
    let segments = [
        seg(p(0, 0), p(1000, 0)),
        seg(p(0, 0), p(500, 500)),
        seg(p(1000, 1000), p(1000, 1000)),
        seg(p(500, 500), p(1000, 500)),
        seg(p(500, 500), p(1000, 501)),
    ];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        voronoi_from_segments(&segments)
    }));

    assert!(
        result.is_ok(),
        "voronoi_from_segments panicked on degenerate input: {:?}",
        result.err()
    );
    match result.unwrap() {
        Ok(_) => {}
        Err(VoronoiError::PredicatePanic { .. }) => {}
        Err(other) => panic!("expected Ok or PredicatePanic, got {:?}", other),
    }
}

/// Direct unit test of the `VoronoiError::PredicatePanic { .. }` variant
/// itself — independent of boostvoronoi's behavior. Constructs the variant
/// directly with known field values and asserts (a) the fields populate as
/// constructed, (b) `Debug` derives a non-empty representation, and (c) the
/// `Display` impl includes the segment count and the duplicate / zero /
/// collinear flags. Together with
/// `voronoi_from_segments_predicate_panic_fires_on_synthetic_input`, this
/// proves the variant is well-formed and the catch arm produces the right
/// diagnostic string.
#[test]
fn voronoi_error_predicate_panic_field_population() {
    let err = VoronoiError::PredicatePanic {
        segment_count: 7,
        min_x: -42,
        min_y: -7,
        max_x: 1_000_000,
        max_y: 2_000_000,
        has_duplicate_endpoint: true,
        has_zero_length_segment: false,
        has_near_collinear_pair: true,
    };

    // Display contract: includes the segment count and all three flags so
    // a triager can see which degeneracy classes were present without
    // re-running the slice.
    let rendered = format!("{}", err);
    assert!(
        rendered.contains("7 segments"),
        "Display must include segment_count, got: {rendered}"
    );
    assert!(
        rendered.contains("duplicate=true"),
        "Display must include duplicate flag, got: {rendered}"
    );
    assert!(
        rendered.contains("zero_length=false"),
        "Display must include zero_length flag, got: {rendered}"
    );
    assert!(
        rendered.contains("near_collinear=true"),
        "Display must include near_collinear flag, got: {rendered}"
    );
    assert!(
        rendered.contains("x=[-42, 1000000]") && rendered.contains("y=[-7, 2000000]"),
        "Display must include coordinate bounds in internal units, got: {rendered}"
    );

    // Debug contract: non-empty.
    let debug = format!("{:?}", err);
    assert!(!debug.is_empty(), "Debug must be non-empty");

    // Equality contract: structurally equal instances compare equal; differing
    // fields do not.
    let same = VoronoiError::PredicatePanic {
        segment_count: 7,
        min_x: -42,
        min_y: -7,
        max_x: 1_000_000,
        max_y: 2_000_000,
        has_duplicate_endpoint: true,
        has_zero_length_segment: false,
        has_near_collinear_pair: true,
    };
    let different = VoronoiError::PredicatePanic {
        segment_count: 8,
        min_x: -42,
        min_y: -7,
        max_x: 1_000_000,
        max_y: 2_000_000,
        has_duplicate_endpoint: true,
        has_zero_length_segment: false,
        has_near_collinear_pair: true,
    };
    assert_eq!(err, same);
    assert_ne!(err, different);
    assert_ne!(err, VoronoiError::EmptyInput);
}
