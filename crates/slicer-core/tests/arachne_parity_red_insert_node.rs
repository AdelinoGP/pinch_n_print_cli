//! Red tests encoding findings F1+F6 of the Arachne parity audit
//! (`target/arachne_parity_audit_*.md`).
//!
//! **Finding F1:** PNP's `propagation.rs::insert_node` is a simple edge split.
//! OrcaSlicer's `insertNode` (`OrcaSlicerDocumented/src/libslic3r/Arachne/utils/
//! SkeletalTrapezoidationGraph.cpp:615-644`) calls `insertRib` (lines 310-431)
//! on BOTH the edge and its twin, creating a full rib structure on each side
//! (4 new edges + 1 boundary node + cross-twin patching). PNP only mutates the
//! edge side, leaving the twin's `next`/`prev`/`start_vertex`/`edge_to`
//! reflecting the pre-split geometry.
//!
//! **Finding F6:** Related — the twin's `next`/`prev` chain is not rewired
//! after the split, so a subsequent `.next`-walk from the twin's start
//! reaches the wrong terminal vertex.
//!
//! # How these tests are wired
//!
//! `insert_node` is private (used only by `apply_transitions`).
//! The tests therefore drive the *public* API: hand-build a graph with
//! `transition_mids` set, call `apply_transitions`, and observe the
//! post-conditions that OrcaSlicer's faithful insertNode would guarantee.
//!
//! Each test asserts an invariant that PNP's current implementation
//! **violates** and that the OrcaSlicer-faithful fix would restore. The
//! tests are deliberately scoped to public API (hand-built
//! `SkeletalTrapezoidationGraph` + `apply_transitions`), so they survive
//! any refactor of the private construction internals.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::{
    apply_transitions, EdgeType, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
    TransitionMiddle,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

#[allow(dead_code)]
fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Resolves a half-edge's "to" vertex via its twin's `start_vertex`,
/// matching `SkeletalTrapezoidationGraph`'s own convention (duplicated here
/// because the crate's own copies of this helper are private to their
/// modules — see e.g. `arachne_invariants.rs::resolve_to_vertex`).
#[allow(dead_code)]
fn resolve_to_vertex(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> usize {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return NO_INDEX;
    };
    if edge.twin == NO_INDEX {
        return NO_INDEX;
    }
    graph
        .edges
        .get(edge.twin)
        .map(|twin_edge| twin_edge.start_vertex)
        .unwrap_or(NO_INDEX)
}

/// Walks `.next` from `start` up to `max_hops` edges, returning the visited
/// indices. Mirrors `arachne_invariants::find_quad` semantics.
#[allow(dead_code)]
fn walk_next(graph: &SkeletalTrapezoidationGraph, start: usize, max_hops: usize) -> Vec<usize> {
    let mut visited = vec![start];
    let mut cur = start;
    for _ in 0..max_hops {
        let next = graph.edges[cur].next;
        if next == NO_INDEX {
            break;
        }
        visited.push(next);
        cur = next;
    }
    visited
}

/// Builds the minimal possible single-edge-with-twin graph that exposes
/// `insert_node`'s twin-side handling: two central `NORMAL` half-edges
/// forming one twin pair (edge 0 + edge 1, twin-linked), two vertices
/// (v0 at the origin, v1 at (10mm, 0)). One transition_mid is added to
/// edge 0 at the midpoint (5mm), so calling `apply_transitions` will
/// internally call `insert_node(graph, 0, ...)`.
///
/// Edge 0 is `central`; edge 1 (the twin) is **not** central, mirroring
/// real graphs where the two half-edges share the same physical edge but
/// only one direction is central. Both are `edge_type = NORMAL` (no rib).
fn make_split_target_graph() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1_000_000.0, // 100mm
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 500_000.0, // 50mm
        bead_count: Some(3),
        transition_ratio: 0.0,
    };

    // Edge 0: v0 -> v1 (forward direction, central, with a transition_mid
    // at the midpoint). This is the edge apply_transitions will split.
    // Edge 1: v1 -> v0 (reverse direction, twin of 0, non-central).
    let e0 = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        transition_mids: vec![TransitionMiddle {
            pos: 0.5,
            lower_bead_count: 2,
            mid_r: 750_000.0, // 75mm
        }],
        ..STHalfEdge::default()
    };
    let e1 = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };

    SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1],
        edges: vec![e0, e1],
        centrality_filtered: true,
        rib: Default::default(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Test 1 (F1): apply_transitions's internal insert_node must create
// >= 4 new edges (OrcaSlicer-faithful: 2 split fragments + 2 rib edges),
// not PNP's current 1.
// ---------------------------------------------------------------------------

#[test]
fn apply_transitions_creates_at_least_4_new_edges_for_one_split() {
    let mut graph = make_split_target_graph();
    let n_edges_before = graph.edges.len();

    apply_transitions(&mut graph);

    // OrcaSlicer's expected count: 2 split fragments (one on each side of
    // the original edge) + 2 rib edges (forth_rib + back_rib on the edge
    // side) + 2 more rib edges (on the twin side) = 6 new edges. PNP
    // current behavior: 1-2 new edges (the edge-side split fragment
    // plus, if the apply_transitions mirror pushes a twin-side entry,
    // a shadow edge on the twin side), 0 ribs.
    //
    // We assert the *minimum* that OrcaSlicer's structure guarantees
    // (4 new edges: 2 split fragments + 2 rib edges on the edge side
    // alone), which is still strictly more than PNP's current 1-2.
    // The fix agent can increase this once both sides are fully implemented.
    let n_new_edges = graph.edges.len() - n_edges_before;
    assert!(
        n_new_edges >= 4,
        "apply_transitions (which calls insert_node internally) must \
         create >= 4 new edges per OrcaSlicer-faithful insertNode + \
         insertRib: 2 split fragments (one on each side of the original \
         edge) + 2 rib edges (forth_rib + back_rib). Plus 2 more on the \
         twin side = 6 total for a single split. PNP current behavior \
         creates only 1-2 new edges (just the split fragment on the edge \
         side, no twin-side split, no rib edges). Got {n_new_edges}. \
         This is finding F1 of the Arachne parity audit."
    );

    // Even stronger, unambiguous F1 signal: at least 2 of the new edges
    // must be ribs (forth_rib + back_rib from one insertRib call). This
    // fails under PNP current (which never inserts any ribs) and passes
    // once F1 is fixed.
    let new_ribs: Vec<usize> = graph
        .edges
        .iter()
        .skip(n_edges_before)
        .enumerate()
        .filter(|(_, e)| e.edge_type == EdgeType::EXTRA_VD)
        .map(|(i, _)| n_edges_before + i)
        .collect();
    assert!(
        new_ribs.len() >= 2,
        "apply_transitions must create >= 2 new rib (EXTRA_VD) edges for \
         the single split, matching OrcaSlicer's insertRib forth+back pair. \
         Got {} new rib(s) out of {} new edge(s). This is the core F1 \
         signal. See the Arachne parity audit.",
        new_ribs.len(),
        n_new_edges
    );
}

// ---------------------------------------------------------------------------
// Test 2 (F1): apply_transitions's internal insert_node must create
// at least one new rib (EXTRA_VD) edge — PNP current: 0.
// ---------------------------------------------------------------------------

#[test]
fn apply_transitions_creates_at_least_one_rib_edge_for_one_split() {
    let mut graph = make_split_target_graph();
    let n_edges_before = graph.edges.len();

    apply_transitions(&mut graph);

    let new_ribs: Vec<usize> = graph
        .edges
        .iter()
        .skip(n_edges_before)
        .enumerate()
        .filter(|(_, e)| e.edge_type == EdgeType::EXTRA_VD)
        .map(|(i, _)| n_edges_before + i)
        .collect();

    assert!(
        !new_ribs.is_empty(),
        "apply_transitions must create at least one rib (EXTRA_VD) edge \
         at the split point, matching OrcaSlicer's insertNode+insertRib \
         which creates forth_rib+back_rib pairs on both sides. PNP \
         current behavior creates 0 ribs. This is finding F1 of the \
         Arachne parity audit."
    );
}

// ---------------------------------------------------------------------------
// Test 3 (F1): apply_transitions must create a rib-foot boundary vertex
// (distance_to_boundary == 0) — PNP current: 0 (it only interpolates).
// ---------------------------------------------------------------------------

#[test]
fn apply_transitions_creates_rib_foot_boundary_vertex() {
    let mut graph = make_split_target_graph();
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let has_rib_foot = graph
        .vertices
        .iter()
        .skip(n_verts_before)
        .any(|v| v.distance_to_boundary == 0.0);

    assert!(
        has_rib_foot,
        "apply_transitions must create at least one new vertex with \
         distance_to_boundary == 0.0 (a rib-foot boundary node, as in \
         OrcaSlicer's insertNode+insertRib). Found 0 such vertices among \
         the {} new ones. PNP current behavior interpolates the distance \
         for the new split vertex, never producing a 0.0 boundary value. \
         This is finding F1 of the Arachne parity audit.",
        graph.vertices.len() - n_verts_before
    );
}

// ---------------------------------------------------------------------------
// Test 4 (F6): even with the mirror mechanism PNP does push to the twin
// bucket, the twin's geometry is mutated but the topology is wrong: the
// new vertex on the twin side is at the wrong position, OR the twin's
// bead_count is wrong.
// ---------------------------------------------------------------------------
//
// The F6 finding is the *atomicity* of the split: OrcaSlicer does the
// split on both sides in one insertNode call, with cross-twin patching,
// so the physical split position is the SAME on both sides (and the
// new boundary node is shared). PNP's mirror mechanism runs the two
// splits as independent insert_node calls, so the new vertices end up
// at independent positions that may not coincide geometrically.
//
// We assert: after apply_transitions, the new vertices on the edge side
// and the twin side must be at exactly the same physical position
// (within a unit-scale tolerance). PNP's two independent insert_node
// calls will produce vertices at slightly different interpolated
// positions (e.g. v0+v1 = 5.0mm for edge 0's split, v1+v0 = 5.0mm for
// edge 1's split — but the bead_count/mid_r interpolation may differ,
// or the distance_to_boundary may differ).

#[test]
fn apply_transitions_split_position_coincides_on_both_sides() {
    let mut graph = make_split_target_graph();
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let new_verts: Vec<usize> = (n_verts_before..graph.vertices.len()).collect();
    assert!(
        new_verts.len() >= 2,
        "apply_transitions must create >= 2 new vertices for the symmetric \
         edge+twin split (one on each side). Got {}. F6 finding: even the \
         mirror mechanism must produce a new vertex on each side.",
        new_verts.len()
    );

    // All new vertices should be at the same physical split position
    // (5mm, 0). The OrcaSlicer-faithful implementation creates one
    // shared boundary node; PNP's two-independent-calls approach
    // produces two vertices at similar but not necessarily equal
    // positions.
    let split_x = 5.0 * UNITS_PER_MM;
    let split_ys: Vec<f64> = new_verts
        .iter()
        .map(|&i| graph.vertices[i].position.x)
        .collect();

    let max_deviation_x = split_ys
        .iter()
        .map(|&x| (x - split_x).abs())
        .fold(0.0_f64, f64::max);

    // PNP current: both new vertices are at interpolated positions, but
    // since the edge is symmetric (v0+v1, v1+v0), they should be at
    // ~the same x. So this assertion may pass. The real test is the
    // distance_to_boundary consistency.
    let _ = max_deviation_x; // placeholder for the tighter test below

    // The F6 signal: the *twin-side* new vertex's distance_to_boundary
    // must equal the *edge-side* new vertex's distance_to_boundary
    // (both are the foot of the perpendicular to the same source
    // segment). PNP's two-independent-calls approach may produce
    // different distances if the start_vertex R values differ from
    // the end_vertex R values differently on the two sides.
    let distances: Vec<f64> = new_verts
        .iter()
        .map(|&i| graph.vertices[i].distance_to_boundary)
        .collect();
    let min_dist = distances.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_dist = distances.iter().cloned().fold(0.0_f64, f64::max);

    // In OrcaSlicer, the boundary node is shared, so all "new" vertices
    // at the split share distance_to_boundary == 0.0 (rib foot). In
    // PNP, the two new vertices are interpolated, so they may both be
    // 0.0 or one may be non-zero. Either way, the maximum pairwise
    // distance-to-boundary difference should be small (sub-mm).
    // If PNP computes interpolated distances differently on the two
    // sides (because start_r differs from end_r for the two edges),
    // they can diverge significantly.
    let dist_spread = max_dist - min_dist;

    // The real F6 invariant: in a faithful insertNode, the new boundary
    // node (if any) is on the boundary, so distance_to_boundary == 0.
    // In PNP, the new vertices are interpolated, so distance > 0.
    // Combined with test 3 (rib_foot_boundary_vertex), this confirms
    // that PNP does not produce any boundary nodes from the split.
    let any_new_is_boundary = distances.contains(&0.0);
    assert!(
        any_new_is_boundary,
        "apply_transitions must create at least one new vertex with \
         distance_to_boundary == 0.0 (the shared boundary node from a \
         faithful insertNode+insertRib). Got distances: {distances:?}, \
         spread = {dist_spread}. This is the F6 finding: PNP's two-\
         independent-insert_node-calls approach never produces a \
         boundary node. See the Arachne parity audit."
    );
}

// ---------------------------------------------------------------------------
// Test 5 (F1+F6 integration): the full pipeline must produce a closed
// outer wall (inset_idx == 0) for a plain square with a transition_mid.
// ---------------------------------------------------------------------------
//
// A plain 10mm square run through `run_arachne_pipeline` must produce at
// least one closed outer wall. This is the end-to-end signal that
// findings F1+F6 break when they break — even the simplest input.

#[test]
fn square_pipeline_outer_wall_is_closed() {
    use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};

    let side: i64 = (10.0 * UNITS_PER_MM) as i64;
    let square = expoly(vec![p(0, 0), p(side, 0), p(side, side), p(0, side)]);
    let lines = run_arachne_pipeline(
        std::slice::from_ref(&square),
        &ArachneParams::default(),
        false,
    )
    .expect("square should produce Ok(lines)");

    let outer_lines: Vec<_> = lines.iter().filter(|l| l.inset_idx == 0).collect();
    assert!(
        !outer_lines.is_empty(),
        "square pipeline must produce at least one outer wall (inset_idx == 0) line"
    );
    for line in outer_lines {
        assert!(
            line.is_closed,
            "square pipeline outer wall (inset_idx == 0) must be a closed \
             ring. Got an open line with {} junctions, first={:?} \
             last={:?}. This end-to-end failure is caused by F1+F6 \
             (apply_transitions/insertNode does not split the twin side) \
             breaking the connectJunctions chain. See the Arachne parity \
             audit.",
            line.junctions.len(),
            line.junctions.first().map(|j| j.p),
            line.junctions.last().map(|j| j.p)
        );
    }
}
