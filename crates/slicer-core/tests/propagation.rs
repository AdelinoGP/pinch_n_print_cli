//! Bead-count propagation + transition-marking tests for
//! `propagate_beadings_upward`/`propagate_beadings_downward` (T-222, packet
//! 112 Step 3 of the M2 Arachne port).
//!
//! # Self-captured regression baselines — NOT OrcaSlicer goldens
//!
//! Packet 112 has no OrcaSlicer oracle for this step (see
//! `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`'s
//! module-level doc comment for why marking transitions inside
//! `propagate_beadings_upward`/`_downward` at all is an intentional
//! from-first-principles adaptation — upstream places `TransitionMiddle`/
//! `TransitionEnd` in a wholly separate pass this crate does not have). The
//! three fixture files under `tests/fixtures/arachne/propagation_*.json` are
//! **self-captured regression baselines**: on first run, they write this
//! implementation's own output to disk; on every subsequent run, they
//! compare against the committed file and fail on any drift. This locks in
//! *this* implementation's behavior for regression purposes — it is not,
//! and must never be described as, independently-derived OrcaSlicer ground
//! truth. The real correctness signal is the invariant assertions
//! (uniform ⇒ zero transitions, a transition marker only exists next to a
//! genuinely differing bead count, determinism) documented per-fixture
//! below.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, propagate_beadings_downward, propagate_beadings_upward,
    CentralityParams, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
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

/// Uniform fixture: the exact same tapered-wedge geometry and parameters as
/// `tests/bead_count.rs`'s `bead_count_tapered_wedge` — a needle-like
/// isoceles triangle (acute apex at the origin, blunt end at x = 10000).
/// Under those exact params, the committed self-captured baseline
/// `tests/fixtures/arachne/bead_count_tapered_wedge.json` already shows the
/// six central edges all landing on `bead_count = 5` — genuinely uniform,
/// not contrived — making this the cheapest falsifying check for the
/// zero-transitions invariant (AC-3.1).
fn tapered_wedge_fixture() -> ExPolygon {
    expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)])
}

/// Same `BeadingFactoryParams` as `tests/bead_count.rs`'s `factory_params()`
/// — reused verbatim (not re-derived) so the uniform fixture's known-uniform
/// `bead_count = 5` result carries over unchanged.
fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        max_bead_count: 9,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
    }
}

/// Same tightened `CentralityParams` as `tests/centrality.rs`/
/// `tests/bead_count.rs` (a nonzero `min_central_distance` floor so both the
/// wedge and multi-feature fixtures get a genuine central/non-central mix).
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

/// Multi-feature fixture: the identical L-shaped polygon (one reflex corner)
/// as `tests/centrality.rs`'s `multi_feature_fixture` — a structurally
/// richer medial axis than either the uniform wedge or the hand-built
/// varying graph, used here as a general "does the full pipeline run
/// end-to-end without contrivance" baseline (AC-3.3).
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

/// Builds a fresh graph for `poly`, runs `filter_central` then
/// `assign_bead_counts` with a freshly-built strategy instance from `params`.
/// Does *not* run propagation yet — callers apply
/// `propagate_beadings_upward`/`_downward` themselves so tests can inspect
/// intermediate state.
fn build_filtered_and_assigned_with(
    poly: &ExPolygon,
    params: &BeadingFactoryParams,
) -> SkeletalTrapezoidationGraph {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    filter_central(&mut graph, &centrality_params());

    let strategy = BeadingStrategyFactory::create_stack(params);
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

    graph
}

/// [`build_filtered_and_assigned_with`] using `factory_params()` (the
/// tapered-wedge-tuned parameters shared with `tests/bead_count.rs`).
fn build_filtered_and_assigned(poly: &ExPolygon) -> SkeletalTrapezoidationGraph {
    build_filtered_and_assigned_with(poly, &factory_params())
}

/// Per-edge `(is_transition_middle, is_transition_end)` marker vectors, in
/// `graph.edges` order.
fn markers(graph: &SkeletalTrapezoidationGraph) -> (Vec<bool>, Vec<bool>) {
    let middle = graph.edges.iter().map(|e| e.is_transition_middle).collect();
    let end = graph.edges.iter().map(|e| e.is_transition_end).collect();
    (middle, end)
}

/// Independently verifies "a transition edge only exists between differing
/// bead counts" by re-enumerating central neighbors from scratch here in the
/// test (deliberately *not* calling back into `propagation.rs`'s private
/// helpers), so this check cannot pass merely by tautology with the code
/// under test.
fn assert_transitions_imply_differing_neighbor(graph: &SkeletalTrapezoidationGraph, label: &str) {
    for (idx, edge) in graph.edges.iter().enumerate() {
        if !edge.is_transition_middle && !edge.is_transition_end {
            continue;
        }
        assert!(
            edge.central,
            "{label}: edge {idx} marked as a transition but is not central"
        );
        let Some(bc) = edge.bead_count else {
            panic!("{label}: edge {idx} marked as a transition but has no bead_count");
        };

        let to_v = if edge.twin == NO_INDEX {
            NO_INDEX
        } else {
            graph.edges[edge.twin].start_vertex
        };

        let has_differing_neighbor = graph.edges.iter().enumerate().any(|(n_idx, n_edge)| {
            n_idx != idx
                && n_idx != edge.twin
                && n_edge.central
                && (n_edge.start_vertex == edge.start_vertex || n_edge.start_vertex == to_v)
                && matches!(n_edge.bead_count, Some(nb) if nb != bc)
        });

        assert!(
            has_differing_neighbor,
            "{label}: edge {idx} marked transition (middle={}, end={}) but no central neighbor \
             has a differing bead_count",
            edge.is_transition_middle, edge.is_transition_end
        );
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PropagationFixture {
    /// Explicit disclosure: this is a self-captured regression baseline
    /// (this implementation's own output), not an OrcaSlicer golden — see
    /// this file's module-level doc comment.
    provenance: String,
    fixture: String,
    edge_count: usize,
    is_transition_middle: Vec<bool>,
    is_transition_end: Vec<bool>,
}

const PROVENANCE: &str = "Self-captured regression baseline: serialized from this crate's own \
     propagate_beadings_upward/propagate_beadings_downward implementation (packet 112 Step 3 / \
     T-222). NOT derived from, and not a substitute for, OrcaSlicer ground truth — no OrcaSlicer \
     oracle exists for this step, and the transition-marking behavior itself is an intentional \
     from-first-principles adaptation (see propagation.rs's module-level doc comment). Locks in \
     current behavior for regression purposes only.";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/arachne")
        .join(format!("propagation_{name}.json"))
}

/// Writes `fixture` to disk if absent (first run seeds the baseline);
/// otherwise reads the committed baseline and asserts it matches `fixture`
/// exactly (regression lock).
fn write_or_compare_baseline(fixture: &PropagationFixture) {
    let path = fixture_path(&fixture.fixture);
    match fs::read_to_string(&path) {
        Ok(existing) => {
            let baseline: PropagationFixture =
                serde_json::from_str(&existing).unwrap_or_else(|e| {
                    panic!(
                        "{}: failed to parse committed baseline: {e}",
                        path.display()
                    )
                });
            assert_eq!(
                &baseline,
                fixture,
                "{}: transition markers drifted from the committed self-captured baseline. If \
                 this drift is an intentional behavior change, delete the file and rerun to \
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
                .expect("PropagationFixture serialization is infallible");
            fs::write(&path, json).unwrap_or_else(|e| {
                panic!("{}: failed to write new baseline: {e}", path.display())
            });
        }
        Err(e) => panic!("{}: failed to read baseline: {e}", path.display()),
    }
}

/// Hand-built varying-bead-count graph: a 5-vertex path (v0-v1-v2-v3-v4) of
/// four undirected central edges A(bead_count=3)-B(4)-C(5)-D(5), each edge
/// represented as its usual pair of twin half-edges.
///
/// # Why hand-built (documented per this packet's brief)
///
/// The real `tapered_wedge_fixture` + `factory_params()` combination (the
/// same one this file's uniform fixture reuses) happens to land all six
/// central edges on the *same* `bead_count = 5` (see
/// `tests/fixtures/arachne/bead_count_tapered_wedge.json`) — useful as the
/// uniform fixture, but there is no available geometry/param combination in
/// this packet's existing fixture set with a *known, pre-verified* bead-count
/// spread across central edges, and tuning one blind (without an oracle to
/// check against) would risk an untestable, un-reviewable magic-number hunt.
/// A small hand-built graph literal — the same pattern
/// `tests/centrality.rs`'s `centrality_stage_two_whisker_dissolve_is_exercised`
/// already uses for its own from-scratch topology test — gives full,
/// reviewable control over exactly which edges the algorithm should mark.
/// The expected per-edge markers are derived by hand (not just asserted
/// blindly) in the comment block directly above the `assert_eq!(end_a, ...)`
/// calls in `propagation_three_fixtures`.
fn varying_hand_built_graph() -> SkeletalTrapezoidationGraph {
    fn vertex(x: f64, y: f64, distance_to_boundary: f64) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary,
        }
    }

    fn edge(start_vertex: usize, twin: usize, r_min: f64, r_max: f64, bc: u32) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min,
            r_max,
            central: true,
            is_curved: false,
            bead_count: Some(bc),
            is_transition_middle: false,
            is_transition_end: false,
        }
    }

    // Vertices 0..4 along a straight chain; distance_to_boundary values are
    // not read by propagation.rs (only used by centrality.rs's local-maximum
    // predicate), so any monotonic placeholder is fine here.
    let vertices = vec![
        vertex(0.0, 0.0, 3.0),
        vertex(10.0, 0.0, 4.0),
        vertex(20.0, 0.0, 5.0),
        vertex(30.0, 0.0, 5.0),
        vertex(40.0, 0.0, 5.0),
    ];

    // Edge A: v0<->v1, bead_count=3 (indices 0,1).
    // Edge B: v1<->v2, bead_count=4 (indices 2,3).
    // Edge C: v2<->v3, bead_count=5 (indices 4,5).
    // Edge D: v3<->v4, bead_count=5 (indices 6,7).
    let edges = vec![
        edge(0, 1, 0.0, 5.0, 3),   // 0: A v0->v1
        edge(1, 0, 0.0, 5.0, 3),   // 1: A v1->v0
        edge(1, 3, 5.0, 10.0, 4),  // 2: B v1->v2
        edge(2, 2, 5.0, 10.0, 4),  // 3: B v2->v1
        edge(2, 5, 10.0, 15.0, 5), // 4: C v2->v3
        edge(3, 4, 10.0, 15.0, 5), // 5: C v3->v2
        edge(3, 7, 15.0, 20.0, 5), // 6: D v3->v4
        edge(4, 6, 15.0, 20.0, 5), // 7: D v4->v3
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
    }
}

/// Hand-built graph with one genuine `bead_count == None` gap, to prove
/// `fill_gaps` (exercised indirectly by both `propagate_beadings_upward` and
/// `propagate_beadings_downward`) is not vestigial: in the packet's own
/// pipeline, `assign_bead_counts` already assigns every central edge, so
/// gap-filling is a no-op there (see `propagation.rs`'s module doc comment).
/// A 3-vertex path v0-v1-v2: edge A (v0<->v1) has `bead_count = Some(4)`;
/// edge B (v1<->v2) starts with `bead_count = None` on both its
/// half-edges. `propagate_beadings_upward` must fill both of B's directions
/// from A (the only central neighbor reachable at v1) with `Some(4)`.
fn gapped_hand_built_graph() -> SkeletalTrapezoidationGraph {
    fn vertex(x: f64, y: f64) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary: 5.0,
        }
    }

    fn edge(
        start_vertex: usize,
        twin: usize,
        r_min: f64,
        r_max: f64,
        bc: Option<u32>,
    ) -> STHalfEdge {
        STHalfEdge {
            start_vertex,
            twin,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min,
            r_max,
            central: true,
            is_curved: false,
            bead_count: bc,
            is_transition_middle: false,
            is_transition_end: false,
        }
    }

    let vertices = vec![vertex(0.0, 0.0), vertex(10.0, 0.0), vertex(20.0, 0.0)];

    let edges = vec![
        edge(0, 1, 0.0, 5.0, Some(4)), // 0: A v0->v1
        edge(1, 0, 0.0, 5.0, Some(4)), // 1: A v1->v0
        edge(1, 3, 5.0, 10.0, None),   // 2: B v1->v2 (gap)
        edge(2, 2, 5.0, 10.0, None),   // 3: B v2->v1 (gap)
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
    }
}

#[test]
fn propagation_three_fixtures() {
    // --- Fixture 1: uniform (AC-3.1) — cheapest falsifying check. ---
    // The tapered wedge under `factory_params()` is a real, previously
    // committed self-captured baseline
    // (`tests/fixtures/arachne/bead_count_tapered_wedge.json`) with all six
    // central edges landing on the identical `bead_count = 5` — genuine
    // uniformity, not contrivance. Zero edges must ever be marked a
    // transition.
    {
        let wedge = tapered_wedge_fixture();

        let mut graph_a = build_filtered_and_assigned(&wedge);
        propagate_beadings_upward(&mut graph_a);
        propagate_beadings_downward(&mut graph_a);

        let mut graph_b = build_filtered_and_assigned(&wedge);
        propagate_beadings_upward(&mut graph_b);
        propagate_beadings_downward(&mut graph_b);

        let (middle_a, end_a) = markers(&graph_a);
        let (middle_b, end_b) = markers(&graph_b);

        assert_eq!(
            (&middle_a, &end_a),
            (&middle_b, &end_b),
            "uniform fixture: propagation must be deterministic across independent builds of \
             the same input"
        );

        // Sanity: confirm the fixture is genuinely uniform among central
        // edges (not just central-vs-noncentral), otherwise the
        // zero-transitions assertion below would be checking a vacuous case.
        let central_bead_counts: Vec<u32> = graph_a
            .edges
            .iter()
            .filter(|e| e.central)
            .filter_map(|e| e.bead_count)
            .collect();
        assert!(
            !central_bead_counts.is_empty(),
            "uniform fixture: expected at least one central edge with a bead_count, found none"
        );
        let first = central_bead_counts[0];
        assert!(
            central_bead_counts.iter().all(|&bc| bc == first),
            "uniform fixture: expected every central edge to share one bead_count, got {central_bead_counts:?}"
        );

        assert!(
            middle_a.iter().all(|&m| !m) && end_a.iter().all(|&e| !e),
            "uniform fixture: a graph with a single bead_count across all central edges must \
             have zero transition markers; got is_transition_middle={middle_a:?}, \
             is_transition_end={end_a:?}"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "uniform");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "uniform".to_string(),
            edge_count: graph_a.edges.len(),
            is_transition_middle: middle_a,
            is_transition_end: end_a,
        });
    }

    // --- Fixture 2: varying (AC-3.2) — hand-built graph literal. ---
    // See `varying_hand_built_graph`'s doc comment for why this is
    // hand-built rather than tuned from polygon geometry. The exact expected
    // per-edge markers are worked out and asserted further down (see the
    // comment block above the `assert_eq!(end_a, ...)` calls below).
    {
        let mut graph_a = varying_hand_built_graph();
        propagate_beadings_upward(&mut graph_a);
        let (middle_upward_only, end_upward_only) = markers(&graph_a);
        propagate_beadings_downward(&mut graph_a);
        let (middle_a, end_a) = markers(&graph_a);

        assert_eq!(
            (&middle_upward_only, &end_upward_only),
            (&middle_a, &end_a),
            "varying fixture: transition marking must be order-independent — running \
             propagate_beadings_downward after propagate_beadings_upward must not change \
             markers already settled by the final bead_count state (see propagation.rs's \
             module doc comment)"
        );

        let mut graph_b = varying_hand_built_graph();
        propagate_beadings_upward(&mut graph_b);
        propagate_beadings_downward(&mut graph_b);
        let (middle_b, end_b) = markers(&graph_b);

        assert_eq!(
            (&middle_a, &end_a),
            (&middle_b, &end_b),
            "varying fixture: propagation must be deterministic across independent builds of \
             the same input"
        );

        assert!(
            end_a.iter().any(|&e| e),
            "varying fixture: expected at least one edge marked is_transition_end, got none: \
             {end_a:?}"
        );

        // Exact hand-derived markers (per-edge index, matching
        // `varying_hand_built_graph`'s edge indices 0..8):
        // 0 (A v0->v1, bc=3): only neighbor is edge 2 (B, bc=4) at v1 -> differs on one side only -> end.
        // 1 (A v1->v0, bc=3): only neighbor is edge 2 (B, bc=4) at v1 -> end.
        // 2 (B v1->v2, bc=4): neighbor edge 1 (A, bc=3) at v1 differs, neighbor edge 4 (C, bc=5) at v2 differs -> both sides -> middle.
        // 3 (B v2->v1, bc=4): neighbor edge 4 (C, bc=5) at v2 differs, neighbor edge 1 (A, bc=3) at v1 differs -> middle.
        // 4 (C v2->v3, bc=5): neighbor edge 3 (B, bc=4) at v2 differs, neighbor edge 6 (D, bc=5) at v3 does NOT differ -> one side -> end.
        // 5 (C v3->v2, bc=5): neighbor edge 6 (D, bc=5) at v3 does not differ, neighbor edge 3 (B, bc=4) at v2 differs -> end.
        // 6 (D v3->v4, bc=5): neighbor edge 5 (C, bc=5) at v3 does not differ; no neighbor at v4 -> not marked.
        // 7 (D v4->v3, bc=5): no neighbor at v4; neighbor edge 5 (C, bc=5) at v3 does not differ -> not marked.
        assert_eq!(
            end_a,
            vec![true, true, false, false, true, true, false, false],
            "varying fixture: is_transition_end markers do not match the hand-derived expectation"
        );
        assert_eq!(
            middle_a,
            vec![false, false, true, true, false, false, false, false],
            "varying fixture: is_transition_middle markers do not match the hand-derived expectation"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "varying");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "varying".to_string(),
            edge_count: graph_a.edges.len(),
            is_transition_middle: middle_a,
            is_transition_end: end_a,
        });
    }

    // --- Fixture 3: multi-feature (AC-3.3) — general run, self-captured
    // baseline. Reuses `tests/centrality.rs`'s L-shaped multi-feature
    // polygon (a structurally richer medial axis than either the wedge or
    // the hand-built path graph). Uses *default* `BeadingFactoryParams`
    // (rather than `factory_params()`, the tapered-wedge-tuned values also
    // used by the uniform fixture above): this polygon's coordinates run up
    // to 2000 units, and under the wedge-tuned `optimal_width = 20.0` every
    // central edge here saturates at `LimitedBeadingStrategy`'s
    // `max_bead_count = 9` ceiling — genuinely uniform, but a degenerate,
    // uninteresting "general run" that duplicates the uniform fixture's
    // shape of result. Default params (`optimal_width = 4000.0`, matching
    // this shape's actual coordinate scale) were confirmed (empirically, via
    // a throwaway debug print during development) to produce real bead-count
    // variety among this fixture's central edges instead. No uniform/varying
    // requirement applies to this fixture either way — only determinism and
    // the differing-neighbor invariant — but a non-degenerate result is a
    // more useful regression baseline. ---
    {
        let multi = multi_feature_fixture();
        let multi_params = BeadingFactoryParams::default();

        let mut graph_a = build_filtered_and_assigned_with(&multi, &multi_params);
        propagate_beadings_upward(&mut graph_a);
        propagate_beadings_downward(&mut graph_a);

        let mut graph_b = build_filtered_and_assigned_with(&multi, &multi_params);
        propagate_beadings_upward(&mut graph_b);
        propagate_beadings_downward(&mut graph_b);

        let (middle_a, end_a) = markers(&graph_a);
        let (middle_b, end_b) = markers(&graph_b);

        assert_eq!(
            (&middle_a, &end_a),
            (&middle_b, &end_b),
            "multi-feature fixture: propagation must be deterministic across independent builds \
             of the same input"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "multi_feature");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "multi_feature".to_string(),
            edge_count: graph_a.edges.len(),
            is_transition_middle: middle_a,
            is_transition_end: end_a,
        });
    }
}

/// Supplementary (not part of the required three-fixture AC): directly
/// exercises `fill_gaps` (indirectly, via `propagate_beadings_upward`) on a
/// hand-built graph with a genuine `bead_count == None` gap, proving
/// gap-filling is not vestigial dead code — see `gapped_hand_built_graph`'s
/// doc comment for why the packet's own real pipeline never exercises this
/// path (every central edge already has a `bead_count` by the time
/// propagation runs).
#[test]
fn propagation_fills_gap_from_central_neighbor() {
    let mut graph = gapped_hand_built_graph();

    assert_eq!(
        graph.edges[2].bead_count, None,
        "precondition: edge 2 (B v1->v2) must start as a genuine gap"
    );
    assert_eq!(
        graph.edges[3].bead_count, None,
        "precondition: edge 3 (B v2->v1) must start as a genuine gap"
    );

    propagate_beadings_upward(&mut graph);

    assert_eq!(
        graph.edges[2].bead_count,
        Some(4),
        "edge 2 (B v1->v2): gap must be filled from its only central neighbor at v1 (edge 0/1, \
         bead_count=4)"
    );
    assert_eq!(
        graph.edges[3].bead_count,
        Some(4),
        "edge 3 (B v2->v1): gap must be filled from its only central neighbor at v1 (edge 0/1, \
         bead_count=4)"
    );

    // Filling a gap so both sides now agree (4 == 4) must not spuriously
    // mark a transition.
    assert!(
        !graph.edges[0].is_transition_middle
            && !graph.edges[0].is_transition_end
            && !graph.edges[2].is_transition_middle
            && !graph.edges[2].is_transition_end,
        "gap-filled graph with a single resulting bead_count must have zero transition markers; \
         got edge0=(middle={}, end={}), edge2=(middle={}, end={})",
        graph.edges[0].is_transition_middle,
        graph.edges[0].is_transition_end,
        graph.edges[2].is_transition_middle,
        graph.edges[2].is_transition_end
    );
}
