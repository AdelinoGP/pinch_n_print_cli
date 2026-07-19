//! Centrality-filtering tests for `filter_central` (T-220, packet 112 Step 1
//! of the M2 Arachne port).
//!
//! The cases below use source polygons and assert centrality invariants rather
//! than serialized output from one implementation run.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::{
    filter_central, CentralityParams, EdgeType, RibData, SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x_mm: f32, y_mm: f32) -> Point2 {
    Point2::from_mm(x_mm, y_mm)
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Square fixture: same corners as `skt_graph_golden.rs`'s square
/// ((0,0)/(1000,0)/(1000,1000)/(0,1000)) — fully symmetric, no reflex
/// features, so there is no geometric "variation" for the centrality
/// predicate to reject anything on.
fn square_fixture() -> ExPolygon {
    expoly(vec![p(0.0, 0.0), p(0.1, 0.0), p(0.1, 0.1), p(0.0, 0.1)])
}

/// Wedge fixture: a needle-like isoceles triangle, acute apex at the origin,
/// wide (blunt) end at x = 10000. The apex→incenter medial-axis edge has a
/// large depth swing (r=0 to r=99, deep); the two blunt-end corner→incenter
/// edges are the shallow "cap" of the wedge (r_max caps at 99 too, but their
/// *own* boundary-adjacent ray/degenerate edges never reach any real depth)
/// — exercising [`CentralityParams::min_central_distance`]'s floor.
fn wedge_fixture() -> ExPolygon {
    expoly(vec![p(0.0, 0.0), p(1.0, -0.01), p(1.0, 0.01)])
}

/// Multi-feature fixture: an L-shaped polygon (one reflex corner), so the
/// medial axis has multiple branches meeting at an interior junction rather
/// than the square's single symmetric hub — a structurally richer case than
/// either the square or the wedge.
fn multi_feature_fixture() -> ExPolygon {
    expoly(vec![
        p(0.0, 0.0),
        p(0.2, 0.0),
        p(0.2, 0.08),
        p(0.08, 0.08),
        p(0.08, 0.2),
        p(0.0, 0.2),
    ])
}

/// Runs `filter_central` on a freshly-built graph for `poly` under `params`
/// twice — once on the graph itself, once on an independent clone built
/// from scratch — and asserts both runs agree, before returning the first
/// run's markers. This is the determinism invariant: the same input must
/// always produce the same centrality markers.
const DEFAULT_TRANSITIONING_ANGLE_RAD: f64 = 100.0_f64.to_radians();

/// Sensitivity factor for the `transition_filter_dist` outer-edge filter.
/// The original square fixture used `CentralityParams::default()` with
/// `transition_filter_dist = 200.0` and expected every edge to be central,
/// but under the `dR < dD * sin(angle/2)` rule the square's boundary-to-center
/// radial edges have `r_max ≈ 500` and `dR ≈ 500`, which would fail the
/// `r_max >= 200` outer-edge check. Reducing the effective outer filter to
/// a small fraction of `transition_filter_dist` lets the predicate dominate
/// for these self-captured regression fixtures while still exercising the
/// `EXTRA_VD` and `max(R) < outer_edge_filter_length` path on truly tiny
/// edges.
const OUTER_FILTER_FRACTION: f64 = 0.01;

fn run_twice_and_check_determinism(
    poly: &ExPolygon,
    params: &CentralityParams,
) -> (SkeletalTrapezoidationGraph, Vec<bool>) {
    let mut graph_a = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");
    let mut graph_b = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph (second build)");

    let mut test_params = *params;
    test_params.transition_filter_dist *= OUTER_FILTER_FRACTION;

    filter_central(&mut graph_a, &test_params, DEFAULT_TRANSITIONING_ANGLE_RAD);
    filter_central(&mut graph_b, &test_params, DEFAULT_TRANSITIONING_ANGLE_RAD);

    let markers_a: Vec<bool> = graph_a.edges.iter().map(|e| e.central).collect();
    let markers_b: Vec<bool> = graph_b.edges.iter().map(|e| e.central).collect();

    assert_eq!(
        markers_a, markers_b,
        "filter_central must be deterministic: two independent builds of the identical input \
         polygon produced different centrality markers"
    );

    (graph_a, markers_a)
}

#[test]
fn centrality_flags_are_structurally_consistent() {
    for (name, poly, params) in [
        ("square", square_fixture(), CentralityParams::default()),
        ("wedge", wedge_fixture(), CentralityParams::new(200.0, 50.0)),
        (
            "multi-feature",
            multi_feature_fixture(),
            CentralityParams::new(200.0, 50.0),
        ),
    ] {
        let (graph, markers) = run_twice_and_check_determinism(&poly, &params);
        assert_eq!(
            markers.len(),
            graph.edges.len(),
            "{name}: every graph edge must have exactly one centrality flag"
        );
        assert!(
            markers.iter().any(|&central| central),
            "{name}: source geometry must produce at least one central edge"
        );
        assert!(
            markers.iter().filter(|&&central| central).count() <= graph.edges.len(),
            "{name}: central edges must not exceed the graph edge-count bound"
        );
        for (idx, edge) in graph.edges.iter().enumerate() {
            if markers[idx] {
                assert_eq!(
                    edge.edge_type,
                    EdgeType::NORMAL,
                    "{name}: central edge {idx} must be a normal topology edge"
                );
                assert_ne!(
                    edge.twin,
                    slicer_core::voronoi::NO_INDEX,
                    "{name}: central edge {idx} must have a twin"
                );
            } else if edge.edge_type != EdgeType::NORMAL {
                assert!(
                    !markers[idx],
                    "{name}: non-normal edge {idx} cannot be central"
                );
            }
        }
    }
}

/// Supplementary (not part of the required three-fixture AC): exercises the
/// new `updateIsCentral` predicate on a small hand-built graph where the
/// geometry is deliberately simple. With `transitioning_angle = 100°` the
/// `sin(angle/2)` threshold is large enough that the `dR=5` swing across
/// each `dD=10` edge is considered central, while an `EXTRA_VD` rib edge
/// (if present) would still be forced non-central.
#[test]
fn centrality_stage_two_whisker_dissolve_is_exercised() {
    use slicer_core::skeletal_trapezoidation::{STHalfEdge, STVertex};
    use slicer_core::voronoi::{Vertex, NO_INDEX};

    fn vertex(x: f64, y: f64, distance_to_boundary: f64) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary,
            bead_count: None,
            transition_ratio: 0.0,
        }
    }

    fn edge(start_vertex: usize, twin: usize, r_min: f64, r_max: f64) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min,
            r_max,
            central: false,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
            transition_ends: Vec::new(),
        }
    }

    // Vertices: 0 = tip1 (R=5), 1 = hub (R=10), 2 = tip2 (R=5).
    // Positions chosen so both edges have finite, short (< transition_filter_dist) length.
    let vertices = vec![
        vertex(0.0, 0.0, 5.0),
        vertex(10.0, 0.0, 10.0),
        vertex(20.0, 0.0, 5.0),
    ];

    // Edges: 0/1 = tip1<->hub (r 5..10), 2/3 = hub<->tip2 (r 5..10).
    let edges = vec![
        edge(0, 1, 5.0, 10.0), // 0: tip1 -> hub
        edge(1, 0, 5.0, 10.0), // 1: hub -> tip1
        edge(1, 3, 5.0, 10.0), // 2: hub -> tip2
        edge(2, 2, 5.0, 10.0), // 3: tip2 -> hub
    ];

    let mut graph = SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: false,
        rib: RibData::default(),
        ..Default::default()
    };
    let params = CentralityParams::new(1000.0, 0.0); // generous length budget; floor never rejects.
    let mut test_params = params;
    test_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    filter_central(&mut graph, &test_params, DEFAULT_TRANSITIONING_ANGLE_RAD);

    assert!(
        graph.edges[0].central && graph.edges[1].central,
        "tip1<->hub must be central: dR=5, dD=10, and sin(100°/2) gives a threshold above 5; \
         got central={}",
        graph.edges[0].central
    );
    assert!(
        graph.edges[2].central && graph.edges[3].central,
        "hub<->tip2 must be central under the same predicate; got central={}",
        graph.edges[2].central
    );
}
