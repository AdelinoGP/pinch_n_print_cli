//! Centrality-filtering tests for `filter_central` (T-220, packet 112 Step 1
//! of the M2 Arachne port).
//!
//! # Self-captured regression baselines — NOT OrcaSlicer goldens
//!
//! Packet 112 has no OrcaSlicer oracle for this step (see
//! `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`'s
//! module-level doc comment for why a literal byte-for-byte port of
//! `updateIsCentral`/`filterCentral` isn't well-defined on this crate's
//! simplified graph topology). The three fixture files under
//! `tests/fixtures/arachne/centrality_*.json` are **self-captured
//! regression baselines**: on first run, `centrality_three_fixtures` writes
//! this implementation's own output to disk; on every subsequent run, it
//! compares against the committed file and fails on any drift. This locks
//! in *this* implementation's behavior for regression purposes — it is not,
//! and must never be described as, independently-derived OrcaSlicer ground
//! truth. The real correctness signal is the invariant assertions
//! (determinism, the depth-floor predicate actually discriminating,
//! symmetric geometry never spuriously rejected) documented per-fixture
//! below.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use slicer_core::skeletal_trapezoidation::{
    filter_central, CentralityParams, EdgeType, RibData, SkeletalTrapezoidationGraph,
};
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

/// Square fixture: same corners as `skt_graph_golden.rs`'s square
/// ((0,0)/(1000,0)/(1000,1000)/(0,1000)) — fully symmetric, no reflex
/// features, so there is no geometric "variation" for the centrality
/// predicate to reject anything on.
fn square_fixture() -> ExPolygon {
    expoly(vec![p(0, 0), p(1000, 0), p(1000, 1000), p(0, 1000)])
}

/// Wedge fixture: a needle-like isoceles triangle, acute apex at the origin,
/// wide (blunt) end at x = 10000. The apex→incenter medial-axis edge has a
/// large depth swing (r=0 to r=99, deep); the two blunt-end corner→incenter
/// edges are the shallow "cap" of the wedge (r_max caps at 99 too, but their
/// *own* boundary-adjacent ray/degenerate edges never reach any real depth)
/// — exercising [`CentralityParams::min_central_distance`]'s floor.
fn wedge_fixture() -> ExPolygon {
    expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)])
}

/// Multi-feature fixture: an L-shaped polygon (one reflex corner), so the
/// medial axis has multiple branches meeting at an interior junction rather
/// than the square's single symmetric hub — a structurally richer case than
/// either the square or the wedge.
fn multi_feature_fixture() -> ExPolygon {
    expoly(vec![
        p(0, 0),
        p(2000, 0),
        p(2000, 800),
        p(800, 800),
        p(800, 2000),
        p(0, 2000),
    ])
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct CentralityFixture {
    /// Explicit disclosure per-fixture: this is a self-captured regression
    /// baseline (this implementation's own output), not an OrcaSlicer
    /// golden — see this file's module-level doc comment.
    provenance: String,
    fixture: String,
    transition_filter_dist: f64,
    min_central_distance: f64,
    edge_count: usize,
    /// `central` marker per edge index, in `graph.edges` order.
    central: Vec<bool>,
}

const PROVENANCE: &str = "Self-captured regression baseline: serialized from this crate's own \
     filter_central implementation (packet 112 Step 1 / T-220). NOT derived from, and not a \
     substitute for, OrcaSlicer ground truth — no OrcaSlicer oracle exists for this step (see \
     centrality.rs's module-level doc comment). Locks in current behavior for regression \
     purposes only.";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/arachne")
        .join(format!("centrality_{name}.json"))
}

/// Writes `fixture` to disk if absent (first run seeds the baseline);
/// otherwise reads the committed baseline and asserts it matches `fixture`
/// exactly (regression lock). Returns nothing — panics via `assert_eq!` on
/// mismatch, matching this suite's other `assert*` failure style.
fn write_or_compare_baseline(fixture: &CentralityFixture) {
    let path = fixture_path(&fixture.fixture);
    match fs::read_to_string(&path) {
        Ok(existing) => {
            let baseline: CentralityFixture = serde_json::from_str(&existing).unwrap_or_else(|e| {
                panic!(
                    "{}: failed to parse committed baseline: {e}",
                    path.display()
                )
            });
            assert_eq!(
                &baseline,
                fixture,
                "{}: centrality markers drifted from the committed self-captured baseline. \
                 If this drift is an intentional predicate change, delete the file and rerun to \
                 re-seed it (after confirming the new invariants still hold).",
                path.display()
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("{}: failed to create fixtures dir: {e}", parent.display())
                });
            }
            let json = serde_json::to_string_pretty(fixture)
                .expect("CentralityFixture serialization is infallible");
            fs::write(&path, json).unwrap_or_else(|e| {
                panic!("{}: failed to write new baseline: {e}", path.display())
            });
        }
        Err(e) => panic!("{}: failed to read baseline: {e}", path.display()),
    }
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

fn run_twice_and_check_determinism(poly: &ExPolygon, params: &CentralityParams) -> Vec<bool> {
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

    markers_a
}

#[test]
fn centrality_three_fixtures() {
    // --- Square: fully symmetric, no reflex features. Under default
    // params (no depth floor), the predicate must never spuriously reject
    // anything — there is no "boundary-distance variation" for it to act
    // on. ---
    let square = square_fixture();
    let square_params = CentralityParams::default();
    let square_markers = run_twice_and_check_determinism(&square, &square_params);
    assert!(
        square_markers.iter().any(|&c| c),
        "square fixture: the symmetric medial-axis skeleton must remain central somewhere — \
         expected at least one central edge, got all non-central: {square_markers:?}"
    );
    write_or_compare_baseline(&CentralityFixture {
        provenance: PROVENANCE.to_string(),
        fixture: "square".to_string(),
        transition_filter_dist: square_params.transition_filter_dist,
        min_central_distance: square_params.min_central_distance,
        edge_count: square_markers.len(),
        central: square_markers,
    });

    // --- Wedge: needle apex vs. blunt end. A nonzero `min_central_distance`
    // floor (deliberately tighter than the square's default) must actually
    // discriminate: the wedge's shallow boundary-adjacent structure gets
    // rejected while its one genuine deep medial-axis hub stays central. ---
    let wedge = wedge_fixture();
    let wedge_params = CentralityParams::new(200.0, 50.0);
    let wedge_markers = run_twice_and_check_determinism(&wedge, &wedge_params);
    assert!(
        wedge_markers.iter().any(|&c| !c),
        "wedge fixture: the depth-floor predicate must actually discriminate — expected at \
         least one non-central edge, got all central: {wedge_markers:?}"
    );
    assert!(
        wedge_markers.iter().any(|&c| c),
        "wedge fixture: the genuine medial-axis hub must remain central — expected at least \
         one central edge, got all non-central: {wedge_markers:?}"
    );
    write_or_compare_baseline(&CentralityFixture {
        provenance: PROVENANCE.to_string(),
        fixture: "wedge".to_string(),
        transition_filter_dist: wedge_params.transition_filter_dist,
        min_central_distance: wedge_params.min_central_distance,
        edge_count: wedge_markers.len(),
        central: wedge_markers,
    });

    // --- Multi-feature: an L-shaped polygon (reflex corner), a structurally
    // richer case than either the square or the wedge. Same tightened
    // params as the wedge must again discriminate. ---
    let multi = multi_feature_fixture();
    let multi_params = CentralityParams::new(200.0, 50.0);
    let multi_markers = run_twice_and_check_determinism(&multi, &multi_params);
    assert!(
        multi_markers.iter().any(|&c| !c),
        "multi-feature fixture: the depth-floor predicate must actually discriminate — expected \
         at least one non-central edge, got all central: {multi_markers:?}"
    );
    assert!(
        multi_markers.iter().any(|&c| c),
        "multi-feature fixture: the medial-axis skeleton must remain central somewhere — \
         expected at least one central edge, got all non-central: {multi_markers:?}"
    );
    write_or_compare_baseline(&CentralityFixture {
        provenance: PROVENANCE.to_string(),
        fixture: "multi_feature".to_string(),
        transition_filter_dist: multi_params.transition_filter_dist,
        min_central_distance: multi_params.min_central_distance,
        edge_count: multi_markers.len(),
        central: multi_markers,
    });
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
