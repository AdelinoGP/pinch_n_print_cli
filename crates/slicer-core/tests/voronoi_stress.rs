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
