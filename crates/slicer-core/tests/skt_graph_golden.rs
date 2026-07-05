//! Golden/invariant tests for `SkeletalTrapezoidationGraph::from_polygons`
//! (T-202, M2 Arachne port foundations, Step 3 of packet 110).
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features — it must not break
//! `cargo check --workspace --all-targets` without `--features host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::SkeletalTrapezoidationGraph;
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Every edge's `r_min <= r_max`, both non-negative, and never `NaN`.
fn assert_radius_bounds_valid(graph: &SkeletalTrapezoidationGraph) {
    for (i, e) in graph.edges.iter().enumerate() {
        assert!(
            e.r_min.is_finite() && e.r_max.is_finite(),
            "edge {i}: r_min/r_max must be finite, got r_min={}, r_max={}",
            e.r_min,
            e.r_max
        );
        assert!(
            e.r_min >= 0.0,
            "edge {i}: r_min must be non-negative, got {}",
            e.r_min
        );
        assert!(
            e.r_max >= 0.0,
            "edge {i}: r_max must be non-negative, got {}",
            e.r_max
        );
        assert!(
            e.r_min <= e.r_max,
            "edge {i}: r_min ({}) must be <= r_max ({})",
            e.r_min,
            e.r_max
        );
    }
}

/// `twin` wiring is involutive: `edges[edges[e].twin].twin == e` for every
/// edge with a resolvable (non-sentinel) twin.
fn assert_twin_involutive(graph: &SkeletalTrapezoidationGraph) {
    for (i, e) in graph.edges.iter().enumerate() {
        if e.twin == NO_INDEX {
            continue;
        }
        let twin_edge = &graph.edges[e.twin];
        assert_eq!(
            twin_edge.twin, i,
            "edge {i}: twin ({}) does not point back (twin.twin = {})",
            e.twin, twin_edge.twin
        );
    }
}

/// `next`/`prev` wiring is consistent: `edges[edges[e].next].prev == e` for
/// every edge with a resolvable (non-sentinel) `next`.
fn assert_next_prev_consistent(graph: &SkeletalTrapezoidationGraph) {
    for (i, e) in graph.edges.iter().enumerate() {
        if e.next == NO_INDEX {
            continue;
        }
        let next_edge = &graph.edges[e.next];
        assert_eq!(
            next_edge.prev, i,
            "edge {i}: next ({}) does not point back via prev (next.prev = {})",
            e.next, next_edge.prev
        );
    }
}

/// Every edge's `central` field defaults to `false` (P112 fills it later).
fn assert_all_central_false(graph: &SkeletalTrapezoidationGraph) {
    for (i, e) in graph.edges.iter().enumerate() {
        assert!(!e.central, "edge {i}: central must default to false");
    }
}

#[test]
fn skt_graph_square_and_wedge() {
    square_fixture_wiring_and_radius_invariants();
    wedge_fixture_wiring_invariants_and_radius_variation();
}

/// Square fixture: reuses Step 2's square (corners (0,0),(1000,0),
/// (1000,1000),(0,1000)) as a single closed polygon.
fn square_fixture_wiring_and_radius_invariants() {
    let square = expoly(vec![p(0, 0), p(1000, 0), p(1000, 1000), p(0, 1000)]);

    let graph = SkeletalTrapezoidationGraph::from_polygons(&[square])
        .expect("square fixture should build a valid SKT graph");

    assert_radius_bounds_valid(&graph);
    assert_twin_involutive(&graph);
    assert_next_prev_consistent(&graph);
    assert_all_central_false(&graph);

    // Golden counts, observed empirically from this constructor under the
    // rib-interleaved topology (packet 113c): a rib foot is now inserted
    // after every transferred edge, not just at reflex corners, so for this
    // convex square there are 9 vertices — the 4 input corners, 1 interior
    // center vertex equidistant from all 4 sides, and 4 rib feet (the
    // midpoints of each side) — and 16 half-edges connecting them (this
    // supersedes the pre-113c raw-DCEL-shaped golden of 5 vertices / 24
    // edges, which mirrored Step 2's `voronoi_from_segments` output
    // directly and predates rib interleaving).
    assert_eq!(
        graph.vertices.len(),
        9,
        "square fixture: unexpected vertex count (update this golden if the \
         construction intentionally changes, but verify against a fresh \
         empirical run first)"
    );
    assert_eq!(
        graph.edges.len(),
        16,
        "square fixture: unexpected edge count (update this golden if the \
         construction intentionally changes, but verify against a fresh \
         empirical run first)"
    );

    // The square's center (500, 500) is the unique interior Voronoi vertex
    // equidistant from all 4 sides — 500 scaled units from each edge.
    let center = graph
        .vertices
        .iter()
        .find(|v| (v.position.x - 500.0).abs() < 1e-6 && (v.position.y - 500.0).abs() < 1e-6)
        .expect("square fixture should produce a center vertex at (500, 500)");
    assert!(
        (center.distance_to_boundary - 500.0).abs() < 1e-6,
        "square center distance_to_boundary should be 500.0, got {}",
        center.distance_to_boundary
    );
}

/// Wedge fixture: a needle-like isoceles triangle with an acute apex at the
/// origin and a wide end at x = 10000 — the classic test for a single
/// medial-axis edge whose radius grows away from the acute apex.
///
/// This is a boostvoronoi-output-derived golden, not an OrcaSlicer
/// reference; OrcaSlicer numerical parity lands in P112/T-231.
fn wedge_fixture_wiring_invariants_and_radius_variation() {
    let wedge = expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)]);

    let graph = SkeletalTrapezoidationGraph::from_polygons(&[wedge])
        .expect("wedge fixture should build a valid SKT graph");

    assert_radius_bounds_valid(&graph);
    assert_twin_involutive(&graph);
    assert_next_prev_consistent(&graph);
    assert_all_central_false(&graph);

    // Golden counts, observed empirically from this constructor under the
    // rib-interleaved topology (packet 113c): this convex 3-sided wedge has
    // 7 vertices — the 3 input corners, 1 interior branch vertex (the
    // incenter-like point equidistant from all 3 sides), and 3 rib feet
    // (one per side) — and 12 half-edges connecting them (this supersedes
    // the pre-113c golden of 4 vertices / 18 edges, from before rib
    // interleaving existed).
    assert_eq!(
        graph.vertices.len(),
        7,
        "wedge fixture: unexpected vertex count (update this golden if the \
         construction intentionally changes, but verify against a fresh \
         empirical run first)"
    );
    assert_eq!(
        graph.edges.len(),
        12,
        "wedge fixture: unexpected edge count (update this golden if the \
         construction intentionally changes, but verify against a fresh \
         empirical run first)"
    );

    // Qualitative property: distance-to-boundary must grow as we move away
    // from the acute apex at (0, 0) toward the wide end at x = 10000.
    //
    // Under the rib-interleaved topology (packet 113c), a rib foot is
    // inserted after every transferred edge, not just at reflex corners.
    // For this convex 3-sided wedge that means every polygon corner AND
    // every rib foot is now a graph vertex sitting exactly on the input
    // boundary, and per `STVertex::distance_to_boundary`'s contract such
    // boundary-anchored vertices always carry the sentinel `0.0` — only the
    // wedge's single interior branch vertex (the incenter-like point
    // equidistant from all 3 sides) carries a positive radius. A plain
    // "nearest vertex in Euclidean space" search is no longer a reliable
    // proxy for "the medial-axis radius near this boundary location": a rib
    // foot now sits exactly at the wide-end query point `(10_000, 0)` (on
    // the boundary segment between the two wide-end corners) and would
    // trivially win that search with `distance_to_boundary == 0.0`, even
    // though the wide end plainly has *larger* local medial radius than the
    // needle-thin apex. The apex-side search is unaffected by this: the
    // nearest vertex to `(0, 0)` is, both before and after this topology
    // change, the acute apex corner itself (also always `0.0`). So only the
    // wide-end search needs to explicitly restrict its candidates to
    // interior (non-boundary-anchored) vertices to keep measuring the
    // intended quantity.
    let apex = (0.0_f64, 0.0_f64);
    let wide_end = (10_000.0_f64, 0.0_f64);

    let nearest_to_apex = graph
        .vertices
        .iter()
        .min_by(|a, b| {
            dist_sq(a.position.x, a.position.y, apex)
                .partial_cmp(&dist_sq(b.position.x, b.position.y, apex))
                .expect("distances are always finite, non-NaN")
        })
        .expect("wedge fixture must produce at least one vertex");

    let nearest_to_wide_end = graph
        .vertices
        .iter()
        .filter(|v| v.distance_to_boundary > 0.0)
        .min_by(|a, b| {
            dist_sq(a.position.x, a.position.y, wide_end)
                .partial_cmp(&dist_sq(b.position.x, b.position.y, wide_end))
                .expect("distances are always finite, non-NaN")
        })
        .expect(
            "wedge fixture must produce at least one interior (non-boundary) \
             vertex to represent the medial-axis radius near the wide end",
        );

    assert!(
        nearest_to_apex.distance_to_boundary < nearest_to_wide_end.distance_to_boundary,
        "medial-axis radius should grow away from the acute apex: \
         apex-side distance_to_boundary = {}, wide-end-side = {}",
        nearest_to_apex.distance_to_boundary,
        nearest_to_wide_end.distance_to_boundary
    );
}

fn dist_sq(x: f64, y: f64, target: (f64, f64)) -> f64 {
    let dx = x - target.0;
    let dy = y - target.1;
    dx * dx + dy * dy
}

#[test]
fn empty_polygon_slice_returns_err() {
    let result = SkeletalTrapezoidationGraph::from_polygons(&[]);
    assert!(matches!(
        result,
        Err(slicer_core::skeletal_trapezoidation::SktError::EmptyInput)
    ));
}

#[test]
fn degenerate_ring_returns_err_not_panic() {
    let degenerate = expoly(vec![p(0, 0), p(100, 0)]); // only 2 points
    let result = SkeletalTrapezoidationGraph::from_polygons(&[degenerate]);
    assert!(matches!(
        result,
        Err(slicer_core::skeletal_trapezoidation::SktError::DegeneratePolygon(_))
    ));
}
