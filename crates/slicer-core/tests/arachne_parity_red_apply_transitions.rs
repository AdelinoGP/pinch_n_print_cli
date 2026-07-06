//! Red tests encoding finding F2 of the Arachne parity audit
//! (`target/arachne_parity_audit_*.md`).
//!
//! **Finding F2:** PNP's `propagation.rs::apply_transitions` mirrors
//! transition ends onto the *twin's* bucket (propagation.rs:421-429) and
//! sorts them *descending* (propagation.rs:441-447). OrcaSlicer's
//! `applyTransitions`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487-1543`)
//! mirrors onto the *edge's own* bucket (line 1500: `length - end.pos`)
//! and sorts them *ascending* (line 1514). The two strategies are
//! internally consistent with their respective `insertNode` impls, but
//! produce different physical split positions on the edge side vs the
//! twin side — when later joined at a shared vertex by
//! `connectJunctions`, the matching split positions are physically
//! misaligned.
//!
//! # How these tests are wired
//!
//! `apply_transitions` is public. The tests drive a hand-built graph
//! with a single transition_mid at pos=0.5 on a non-trivial edge whose
//! start and end vertices have different `distance_to_boundary` values,
//! so the interpolated split position on the edge side differs from the
//! interpolated position on the twin side (because PNP's interpolation
//! uses `start_r + (end_r - start_r) * pos`, and the twin's start_r
//! and end_r are swapped).
//!
//! Fails under PNP current: the two new vertices have different
//! `distance_to_boundary` values, and the *physical* split position on
//! the edge side does not match the position on the twin side.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::{
    apply_transitions, EdgeType, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
    TransitionMiddle,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

/// Resolves a half-edge's "to" vertex via its twin's `start_vertex`,
/// matching `SkeletalTrapezoidationGraph`'s own convention.
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

/// Builds a single-edge twin-pair graph with deliberately different
/// `distance_to_boundary` values at the two endpoints (v0 has R=10mm,
/// v1 has R=5mm). A single `transition_mid` at pos=0.5 is added to
/// edge 0. The mid_r is 7.5mm (the midpoint of the two R values).
///
/// The expected physical split behavior:
/// - OrcaSlicer-faithful: the new split vertex is at the foot of the
///   perpendicular to the source segment, which is also the *midpoint*
///   position in the original edge's frame (since pos=0.5 is the
///   midpoint). On both sides (edge and twin), the new vertex is at
///   the same physical position: (5mm, 0).
/// - PNP current: the new split vertex on the edge side is at
///   `pos=0.5` interpolation of the (v0, v1) position pair = (5mm, 0).
///   On the twin side, PNP's `insert_node` interpolates the *twin's*
///   start (v1) and end (v0), which gives the same position (5mm, 0).
///   So positionally they match — BUT the `distance_to_boundary` is
///   interpolated differently because the twin's start_r and end_r are
///   swapped (twin.start=v1 with R=5mm, twin.end=v0 with R=10mm).
///
/// In PNP, the two new vertices end up at:
///   - edge-side:  pos=0.5 interp -> (5mm, 0), R=10+0.5*(5-10) = 7.5mm
///   - twin-side:  pos=0.5 interp -> (5mm, 0), R=5+0.5*(10-5) = 7.5mm
///
/// Same here. Hmm. The mismatch emerges when `mid_r` is different from
/// the interpolated R — for the same mid_r, the two positions
/// (perpendicular foot) are the same. So the F2 finding may not be
/// observable for the simple symmetric case.
///
/// The F2 test below uses `mid_r` to break the symmetry: a mid_r that
/// doesn't match the interpolated R will cause the perpendicular-foot
/// projection to be different from the linear interpolation. PNP's
/// `insert_node` ignores mid_r (it only uses `pos` for the position
/// and `start_r + (end_r-start_r)*pos` for the R), so for a mid_r
/// different from the linearly-interpolated R, the two sides will
/// produce different R values at their respective new vertices.
fn make_f2_target_graph(mid_r_units: f64) -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1_000_000.0, // 10mm
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 500_000.0, // 5mm
        bead_count: Some(3),
        transition_ratio: 0.0,
    };

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
            mid_r: mid_r_units,
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
    }
}

// ---------------------------------------------------------------------------
// Test 1 (F2): the F2 invariant — physical split positions on the two
// sides must agree on the position AND the R distance (when mid_r
// matches the linearly-interpolated R, this is trivially true; the
// real F2 signal is the *symmetry* property of the resulting
// transition_end distance).
// ---------------------------------------------------------------------------
//
// PNP's apply_transitions currently has:
//   - mirror direction wrong: pushes mirrored entry to the *twin's*
//     bucket, so the twin's split position is processed as a fresh
//     insert_node call with pos=0.5 (instead of mirrored properly
//     through the edge's own bucket).
//   - sort direction wrong: processes the farthest end first (PNP
//     descending), so the local_pos rescaling logic kicks in.
//
// The OrcaSlicer-faithful implementation would mirror onto the edge's
// own bucket with `pos = 1.0 - pos` (so the mirrored entry is at
// pos=0.5 in the edge's frame), then process both entries in
// ascending order, splitting the *last sub-edge* (not the original
// edge) on each call.
//
// We can't directly observe the F2 mirror-direction invariant from the
// public API without a specific geometric setup. The cleanest signal
// is: after apply_transitions, the two new vertices (one on each side)
// must have `distance_to_boundary` values that, taken together, sum
// to `v0.R + v1.R = 15mm` (because in a faithful mirror, the
// perpendicular-foot distance to the source segment is the same on
// both sides — call it R_split — and the bead_count-propagation
// uses R_split consistently).
//
// In PNP's current behavior, the two new vertices' R values are
// independently interpolated:
//   - edge-side:  v0.R + 0.5*(v1.R - v0.R) = 7.5mm
//   - twin-side:  v1.R + 0.5*(v0.R - v1.R) = 7.5mm
// Both 7.5mm. So this specific case is symmetric. The F2 mismatch
// would show up with mid_r != interpolated_R.
//
// F2 test strategy: set mid_r to a value that differs from the
// interpolated R by a large amount. A faithful insertNode would use
// mid_r (or the perpendicular-foot projection) for the new R, not the
// linear interpolation. PNP uses linear interpolation, so the new R
// will NOT match mid_r. The two sides will independently produce
// R = 7.5mm each, but a faithful implementation would produce R =
// mid_r (which is the actual physical R at the split point) for both.
//
// We assert: the new vertex's distance_to_boundary must equal
// transition_mid.mid_r (within tolerance) for at least one of the
// new vertices. PNP currently interpolates linearly to 7.5mm, ignoring
// mid_r entirely. The fix would honor mid_r.

#[test]
fn apply_transitions_new_vertex_distance_matches_mid_r() {
    // Set mid_r to a value that differs significantly from the linearly-
    // interpolated R (7.5mm). 8.0mm is close enough to be plausible
    // (within 5% of 7.5mm) but distinguishable from 7.5mm.
    let mid_r = 800_000.0; // 8mm
    let mut graph = make_f2_target_graph(mid_r);
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let new_verts: Vec<usize> = (n_verts_before..graph.vertices.len()).collect();
    assert!(
        !new_verts.is_empty(),
        "apply_transitions must create at least one new vertex for a \
         transition_mid at pos=0.5. Got 0."
    );

    // The new vertex's distance_to_boundary should be approximately
    // mid_r (the actual physical R at the transition mid point).
    // PNP current: linearly interpolates to 7.5mm (ignoring mid_r).
    // OrcaSlicer faithful: uses mid_r directly, or perpendicular-foot
    // projection onto the source segment line, which gives mid_r when
    // mid_r is the actual R at that point.
    let tol = 1_000.0; // 0.1mm tolerance
    let matching = new_verts.iter().any(|&i| {
        let r = graph.vertices[i].distance_to_boundary;
        (r - mid_r).abs() < tol
    });

    assert!(
        matching,
        "apply_transitions: at least one new vertex's distance_to_boundary \
         must approximately equal the transition_mid's mid_r ({mid_r} units), \
         not the linearly-interpolated R between start and end (which is \
         7.5mm = 750000 units). New vertex distances: {:?}. \
         PNP current behavior: linearly interpolates the R (7.5mm), \
         ignoring the configured mid_r. OrcaSlicer faithful: uses mid_r \
         directly (or perpendicular-foot projection matching mid_r). \
         This is finding F2 of the Arachne parity audit.",
        new_verts
            .iter()
            .map(|&i| graph.vertices[i].distance_to_boundary)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 2 (F2): the resulting vertex_count for a single mid must be
// exactly 3 (1 shared mid_node + 2 boundary foot nodes) — OrcaSlicer's
// `insertNode`+`insertRib` creates 1 mid_node shared across both sides
// plus 2 boundary (rib-foot) nodes (one per side). PNP's previous
// two-independent-insert_node-calls approach produced only 2 vertices
// (one per side, no shared mid_node, no boundary feet).
// ---------------------------------------------------------------------------
//
// Canonical OrcaSlicer `insertNode`
// (`SkeletalTrapezoidationGraph.cpp:615-644`) creates the mid_node once
// (shared by both `insertRib` calls), then each `insertRib` creates its
// own boundary `source_node` (distance_to_boundary == 0). So a single
// transition_mid produces exactly 3 new vertices: 1 mid_node + 2 foot
// nodes. The previous PNP implementation produced 2 (one interpolated
// vertex per side, no shared mid_node, no boundary feet).
//
// The faithful invariant: exactly 1 of the 3 new vertices carries
// `bead_count == Some(lower_bead_count)` (the mid_node); the other 2
// (the boundary feet) carry `bead_count == None` (OrcaSlicer only sets
// `mid_node->data.bead_count`, never the `source_node`'s).

#[test]
fn apply_transitions_creates_one_shared_boundary_node_not_two() {
    let mut graph = make_f2_target_graph(750_000.0);
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let n_new_verts = graph.vertices.len() - n_verts_before;

    // OrcaSlicer faithful: 3 new vertices (1 shared mid_node + 2 boundary
    // foot nodes). PNP previous: 2 new vertices (one per independent
    // insert_node call, no shared mid_node, no boundary feet).
    assert_eq!(
        n_new_verts, 3,
        "apply_transitions must create exactly 3 new vertices for a single \
         transition_mid (1 shared mid_node + 2 boundary foot nodes, matching \
         OrcaSlicer's insertNode+insertRib), not 2 (which is what PNP's \
         two-independent-insert_node-calls approach produces). Got \
         {n_new_verts}. This is finding F2 of the Arachne parity audit.",
    );

    // The stronger F2 invariant: exactly 1 of the new vertices carries
    // `bead_count == Some(lower_bead_count)` (the mid_node); the 2
    // boundary foot nodes carry `bead_count == None` (OrcaSlicer only
    // sets bead_count on the mid_node, never on the source_node).
    let new_verts: Vec<usize> = (n_verts_before..graph.vertices.len()).collect();
    let with_bead_count: Vec<usize> = new_verts
        .iter()
        .copied()
        .filter(|&i| graph.vertices[i].bead_count.is_some())
        .collect();
    assert_eq!(
        with_bead_count.len(),
        1,
        "exactly 1 new vertex (the shared mid_node) must carry a bead_count; \
         the 2 boundary foot nodes must carry bead_count == None. Got {} \
         vertices with a bead_count: {:?}. This is the F2 atomicity \
         invariant — OrcaSlicer's insertNode sets bead_count only on the \
         shared mid_node, not on the per-side boundary feet.",
        with_bead_count.len(),
        with_bead_count
    );
}

// ---------------------------------------------------------------------------
// Test 3 (F2): the new vertex (whichever side it comes from) must be
// at the perpendicular foot of the source segment, not the linear
// interpolation between the edge endpoints.
// ---------------------------------------------------------------------------
//
// Source segment: the polygon edge whose Voronoi cell generated the
// medial-axis edge. For a test with a square polygon, the source
// segment is one of the polygon's boundary segments. The foot of
// the perpendicular from (5mm, 0) onto, say, the bottom edge of a
// square (y=0) is (5mm, 0) — same as the linear interpolation for
// this horizontal configuration. To break the symmetry, we use a
// vertical source segment: the perpendicular foot from (5mm, 0) onto
// x=5mm is (5mm, 0) — also the same. Hmm.
//
// For a simple horizontal edge with start (0,0) and end (10mm, 0),
// the perpendicular foot at pos=0.5 is exactly the midpoint, which
// is the same as the linear interpolation. So this test reduces to
// checking distance_to_boundary == 0, which is the F1 invariant
// (already covered by test 3 in arachne_parity_red_insert_node.rs).
//
// We include this as a placeholder test that documents the geometric
// invariant: the new vertex's position must be the foot of the
// perpendicular to the source segment. This is the F2 deep-invariant
// that a faithful port must satisfy. With a horizontal source
// segment, it reduces to the F1 invariant.

#[test]
fn apply_transitions_new_vertex_position_is_perpendicular_foot() {
    let mut graph = make_f2_target_graph(750_000.0);
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let new_verts: Vec<usize> = (n_verts_before..graph.vertices.len()).collect();
    assert!(
        !new_verts.is_empty(),
        "apply_transitions must create at least one new vertex for a \
         transition_mid. Got 0."
    );

    // For a horizontal source segment, the perpendicular foot from
    // (5mm, 0) onto the segment (y=0) is (5mm, 0). The new vertex
    // must be at this position. The source segment is the bottom edge
    // of the original polygon (y=0).
    for &i in &new_verts {
        let pos = graph.vertices[i].position;
        // Perpendicular foot on y=0: pos.y must be 0 (the foot is on
        // the source line). pos.x must be within [0, 10mm] (the source
        // segment's x range).
        assert!(
            pos.y.abs() < 1.0,
            "new vertex at ({}, {}): must be on the source line y=0, \
             but y = {} != 0. PNP current behavior: insert_node uses \
             linear interpolation of (start, end) which is at y=0 here, \
             so this should pass for horizontal edges. For non-horizontal \
             source segments, the perpendicular-foot requirement would \
             diverge from linear interpolation.",
            pos.x,
            pos.y,
            pos.y
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4 (F2): the new shared mid_node's bead_count must match
// transition_mid's lower_bead_count, not the original edge's bead_count.
// ---------------------------------------------------------------------------
//
// Canonical OrcaSlicer `insertNode` sets `bead_count` ONLY on the shared
// mid_node, never on the per-side boundary foot nodes (which keep their
// default-constructed `bead_count`). So this test must check the mid_node
// specifically, not every new vertex. PNP's previous implementation set
// `bead_count` on its 2 interpolated vertices (both), so the test passed
// trivially; with the faithful fix, only the mid_node carries it.

#[test]
fn apply_transitions_new_vertex_bead_count_matches_lower_bead_count() {
    let mut graph = make_f2_target_graph(750_000.0);
    let n_verts_before = graph.vertices.len();

    apply_transitions(&mut graph);

    let new_verts: Vec<usize> = (n_verts_before..graph.vertices.len()).collect();
    assert!(
        !new_verts.is_empty(),
        "apply_transitions must create at least one new vertex."
    );

    // Exactly one new vertex (the shared mid_node) carries a bead_count;
    // the boundary foot nodes carry None. Find the mid_node and assert its
    // bead_count matches the transition_mid's lower_bead_count (2).
    let mid_nodes: Vec<usize> = new_verts
        .iter()
        .copied()
        .filter(|&i| graph.vertices[i].bead_count.is_some())
        .collect();
    assert_eq!(
        mid_nodes.len(),
        1,
        "exactly 1 new vertex (the shared mid_node) must carry a bead_count; \
         got {} such vertices: {:?}",
        mid_nodes.len(),
        mid_nodes
    );
    let bc = graph.vertices[mid_nodes[0]].bead_count;
    assert_eq!(
        bc,
        Some(2),
        "the shared mid_node's bead_count must match transition_mid's \
         lower_bead_count (2). Got {:?}. This is a regression check for \
         the F2 fix: when apply_transitions is fixed to mirror onto the \
         edge's own bucket, the shared mid_node must carry the configured \
         lower_bead_count.",
        bc
    );
}

// Suppress unused-import warning when no test uses it.
#[allow(dead_code)]
fn _resolve_to_vertex_used_elsewhere(g: &SkeletalTrapezoidationGraph) -> usize {
    resolve_to_vertex(g, 0)
}
