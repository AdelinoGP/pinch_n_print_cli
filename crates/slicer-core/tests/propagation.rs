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
//! # `transition_flags`/`STHalfEdge::transition_mids` (packet 113b onward)
//!
//! This file originally checked two now-removed `STHalfEdge` booleans
//! (`is_transition_middle`/`is_transition_end`) that `propagate_beadings_*`
//! stopped setting once packet 113b introduced the real transition mechanism
//! (`transition_mids`, populated by `generate_transition_mids` — a separate
//! pass this file's fixtures do not call). [`transition_flags`] reads the
//! real field, but none of the three fixtures below currently populate it, so
//! [`assert_transitions_imply_differing_neighbor`] remains dormant (its body
//! never executes past the empty-check) — same as it always has been in this
//! file. Wiring `generate_transition_mids` into these fixtures to make that
//! check non-trivial is left to a follow-on: it surfaces real geometric edge
//! cases (e.g. a leaf/tip vertex with no other central-edge neighbor to
//! cross-check against) that this helper's current design does not yet
//! account for.
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
    CentralityParams, EdgeType, RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
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
///
/// The `transition_filter_dist` is multiplied by a small fraction before
/// passing to `filter_central` so the `dR < dD * sin(angle/2)` predicate
/// dominates for these existing fixtures; otherwise the outer-edge filter
/// would reject the entire tapered wedge (its deepest point is below the
/// unscaled `200.0` threshold).
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

const CENTRALITY_TRANSITIONING_ANGLE_RAD: f64 = 0.17453292519943295; // 10°
const OUTER_FILTER_FRACTION: f64 = 0.01;

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
/// Does *not* run `generate_transition_mids`/`apply_transitions` or
/// propagation yet — callers apply
/// `propagate_beadings_upward`/`_downward` themselves so tests can inspect
/// intermediate state.
fn build_filtered_and_assigned_with(
    poly: &ExPolygon,
    params: &BeadingFactoryParams,
) -> SkeletalTrapezoidationGraph {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    filter_central(
        &mut graph,
        &centrality_params,
        CENTRALITY_TRANSITIONING_ANGLE_RAD,
    );

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

/// Per-edge "does this edge carry at least one transition split point"
/// vector, in `graph.edges` order. Reflects the real transition-marking
/// mechanism (`STHalfEdge::transition_mids`, populated by
/// `generate_transition_mids` and consumed by `apply_transitions`) — the
/// coarser `is_transition_middle`/`is_transition_end` booleans this helper
/// used before packet 113b's topology-faithfulness rework were removed once
/// `propagate_beadings_*` stopped setting them (see this file's module doc
/// comment).
fn transition_flags(graph: &SkeletalTrapezoidationGraph) -> Vec<bool> {
    graph
        .edges
        .iter()
        .map(|e| !e.transition_mids.is_empty())
        .collect()
}

/// Independently verifies "a transition edge only exists between differing
/// bead counts" by re-enumerating central neighbors from scratch here in the
/// test (deliberately *not* calling back into `propagation.rs`'s private
/// helpers), so this check cannot pass merely by tautology with the code
/// under test.
fn assert_transitions_imply_differing_neighbor(graph: &SkeletalTrapezoidationGraph, label: &str) {
    for (idx, edge) in graph.edges.iter().enumerate() {
        if edge.transition_mids.is_empty() {
            continue;
        }
        assert!(
            edge.central,
            "{label}: edge {idx} marked as a transition but is not central"
        );
        let start_v = edge.start_vertex;
        let to_v = if edge.twin == NO_INDEX {
            NO_INDEX
        } else {
            graph.edges[edge.twin].start_vertex
        };
        let start_bc = graph.vertices.get(start_v).and_then(|v| v.bead_count);
        let to_bc = graph.vertices.get(to_v).and_then(|v| v.bead_count);
        let (Some(start_bc), Some(to_bc)) = (start_bc, to_bc) else {
            panic!("{label}: edge {idx} marked as a transition but an endpoint has no bead_count");
        };

        let start_has_differing = graph.edges.iter().enumerate().any(|(n_idx, n_edge)| {
            n_idx != idx
                && n_idx != edge.twin
                && n_edge.central
                && n_edge.start_vertex == start_v
                && vertex_bead_count_at_end(graph, n_idx).map_or(false, |nb| nb != start_bc)
        });
        let to_has_differing = graph.edges.iter().enumerate().any(|(n_idx, n_edge)| {
            n_idx != idx
                && n_idx != edge.twin
                && n_edge.central
                && n_edge.start_vertex == to_v
                && vertex_bead_count_at_end(graph, n_idx).map_or(false, |nb| nb != to_bc)
        });

        assert!(
            start_has_differing || to_has_differing,
            "{label}: edge {idx} has {} transition_mids but no central neighbor has a \
             differing bead_count",
            edge.transition_mids.len()
        );
    }
}

/// Bead count on the `to` vertex of edge `edge_idx`.
fn vertex_bead_count_at_end(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> Option<u32> {
    let edge = graph.edges.get(edge_idx)?;
    let to_v = if edge.twin == NO_INDEX {
        NO_INDEX
    } else {
        graph.edges.get(edge.twin)?.start_vertex
    };
    graph.vertices.get(to_v).and_then(|v| v.bead_count)
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PropagationFixture {
    /// Explicit disclosure: this is a self-captured regression baseline
    /// (this implementation's own output), not an OrcaSlicer golden — see
    /// this file's module-level doc comment.
    provenance: String,
    fixture: String,
    edge_count: usize,
    /// Per-edge "carries at least one transition split point"
    /// (`!edge.transition_mids.is_empty()`), in `graph.edges` order — see
    /// [`transition_flags`].
    has_transition: Vec<bool>,
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
/// four undirected central edges with per-vertex bead counts
/// v0(3)-v1(3)-v2(4)-v3(5)-v4(5), each edge represented as its usual pair of
/// twin half-edges.
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
    fn vertex(x: f64, y: f64, distance_to_boundary: f64, bc: u32) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary,
            bead_count: Some(bc),
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
            central: true,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
        }
    }

    // Vertices 0..4 along a straight chain; distance_to_boundary values are
    // not read by `propagate_beadings_upward`/`_downward` themselves (only by
    // centrality.rs's local-maximum predicate and, upstream of this test,
    // `generate_transition_mids` — not called against this hand-built graph;
    // see `propagation_three_fixtures`'s fixture-2 block), so any monotonic
    // placeholder is fine here.
    let vertices = vec![
        vertex(0.0, 0.0, 3.0, 3),
        vertex(10.0, 0.0, 4.0, 3),
        vertex(20.0, 0.0, 5.0, 4),
        vertex(30.0, 0.0, 5.0, 5),
        vertex(40.0, 0.0, 5.0, 5),
    ];

    // Edge A: v0<->v1, bead_count=3 (indices 0,1).
    // Edge B: v1<->v2, bead_count=4 (indices 2,3).
    // Edge C: v2<->v3, bead_count=5 (indices 4,5).
    // Edge D: v3<->v4, bead_count=5 (indices 6,7).
    let edges = vec![
        edge(0, 1, 0.0, 5.0),   // 0: A v0->v1
        edge(1, 0, 0.0, 5.0),   // 1: A v1->v0
        edge(1, 3, 5.0, 10.0),  // 2: B v1->v2
        edge(2, 2, 5.0, 10.0),  // 3: B v2->v1
        edge(2, 5, 10.0, 15.0), // 4: C v2->v3
        edge(3, 4, 10.0, 15.0), // 5: C v3->v2
        edge(3, 7, 15.0, 20.0), // 6: D v3->v4
        edge(4, 6, 15.0, 20.0), // 7: D v4->v3
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        rib: RibData::default(),
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
    fn vertex(x: f64, y: f64, bc: Option<u32>) -> STVertex {
        STVertex {
            position: Vertex { x, y },
            distance_to_boundary: 5.0,
            bead_count: bc,
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
            central: true,
            is_curved: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
        }
    }

    let vertices = vec![
        vertex(0.0, 0.0, Some(4)),
        vertex(10.0, 0.0, Some(4)),
        vertex(20.0, 0.0, None),
    ];

    let edges = vec![
        edge(0, 1, 0.0, 5.0),  // 0: A v0->v1
        edge(1, 0, 0.0, 5.0),  // 1: A v1->v0
        edge(1, 3, 5.0, 10.0), // 2: B v1->v2 (gap)
        edge(2, 2, 5.0, 10.0), // 3: B v2->v1 (gap)
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        rib: RibData::default(),
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

        let flags_a = transition_flags(&graph_a);
        let flags_b = transition_flags(&graph_b);

        assert_eq!(
            flags_a, flags_b,
            "uniform fixture: propagation must be deterministic across independent builds of \
             the same input"
        );

        assert!(
            flags_a.iter().all(|&f| !f),
            "uniform fixture: after propagation the central edges must carry a single effective \
             bead count and therefore have zero transition markers; got has_transition={flags_a:?}"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "uniform");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "uniform".to_string(),
            edge_count: graph_a.edges.len(),
            has_transition: flags_a,
        });
    }

    // --- Fixture 2: varying (AC-3.2) — hand-built graph literal. ---
    // See `varying_hand_built_graph`'s doc comment for why this is
    // hand-built rather than tuned from polygon geometry. This graph is built
    // directly (bypassing `build_filtered_and_assigned*`, and so bypassing
    // `generate_transition_mids` too), so `transition_mids` — and therefore
    // `transition_flags` — is trivially empty here; the fixture instead
    // exercises `propagate_beadings_upward`/`_downward`'s own
    // order-independence and determinism directly against known per-vertex
    // bead counts.
    {
        let mut graph_a = varying_hand_built_graph();
        propagate_beadings_upward(&mut graph_a);
        let flags_upward_only = transition_flags(&graph_a);
        propagate_beadings_downward(&mut graph_a);
        let flags_a = transition_flags(&graph_a);

        assert_eq!(
            flags_upward_only, flags_a,
            "varying fixture: transition marking must be order-independent — running \
             propagate_beadings_downward after propagate_beadings_upward must not change \
             markers already settled by the final bead_count state (see propagation.rs's \
             module doc comment)"
        );

        let mut graph_b = varying_hand_built_graph();
        propagate_beadings_upward(&mut graph_b);
        propagate_beadings_downward(&mut graph_b);
        let flags_b = transition_flags(&graph_b);

        assert_eq!(
            flags_a, flags_b,
            "varying fixture: propagation must be deterministic across independent builds of \
             the same input"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "varying");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "varying".to_string(),
            edge_count: graph_a.edges.len(),
            has_transition: flags_a,
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

        let flags_a = transition_flags(&graph_a);
        let flags_b = transition_flags(&graph_b);

        assert_eq!(
            flags_a, flags_b,
            "multi-feature fixture: propagation must be deterministic across independent builds \
             of the same input"
        );

        assert_transitions_imply_differing_neighbor(&graph_a, "multi_feature");

        write_or_compare_baseline(&PropagationFixture {
            provenance: PROVENANCE.to_string(),
            fixture: "multi_feature".to_string(),
            edge_count: graph_a.edges.len(),
            has_transition: flags_a,
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
        graph.vertices[2].bead_count, None,
        "precondition: vertex 2 (v2) must start as a genuine gap"
    );

    propagate_beadings_upward(&mut graph);

    assert_eq!(
        graph.vertices[2].bead_count,
        Some(4),
        "vertex 2 (v2): gap must be filled from its only central neighbor at v1 (edge 0/1, \
         bead_count=4)"
    );

    // Filling a gap so both sides now agree (4 == 4) must not spuriously
    // populate transition_mids — `propagate_beadings_upward` only ever
    // touches `bead_count`/`transition_ratio` on vertices, never
    // `transition_mids` on edges (that's `generate_transition_mids`'s job,
    // not called in this gap-filling-only test).
    assert!(
        graph.edges[0].transition_mids.is_empty() && graph.edges[2].transition_mids.is_empty(),
        "gap-filled graph must have zero transition_mids entries (propagate_beadings_upward \
         does not populate them); got edge0={:?}, edge2={:?}",
        graph.edges[0].transition_mids,
        graph.edges[2].transition_mids
    );
}
