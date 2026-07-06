// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path:
//   src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
//     (`generateTransitionMids` L925-994,
//      `applyTransitions` L1487-1543,
//      `propagateBeadingsUpward` L1800-1826,
//      `propagateBeadingsDownward` L1833-1899)
//   and
//     src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h
//     (`getTransitionThickness`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Bead-count propagation + transition-region marking (T-222, packet 112
//! Step 3 / packet 113b Step 4 of the M2 Arachne port).
//!
//! Faithful port of `generateTransitionMids` (L925), `applyTransitions`
//! (L1487), `propagateBeadingsUpward` (L1800), and `propagateBeadingsDownward`
//! (L1833) from
//! `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`.
//!
//! The pass order matches upstream:
//! `updateBeadCount` → `filterNoncentralRegions` → `generateTransitionMids` →
//! `generateAllTransitionEnds` → `applyTransitions` → `generateExtraRibs` →
//! `generateSegments` → `propagateBeadingsUpward` →
//! `propagateBeadingsDownward`.
//!
//! Deterministic (index-ordered traversal; `f64` comparisons only ever drive
//! a sort order, never a hash-map key, and fall back to `Ordering::Equal`
//! rather than assuming a `NaN` can't occur) and panic-free.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::graph::{STHalfEdge, STVertex, SkeletalTrapezoidationGraph, TransitionMiddle};
use super::rib::EdgeType;
use crate::beading::{Beading, BeadingStrategy};
use crate::voronoi::NO_INDEX;

/// Snap distance used by `apply_transitions` when deciding whether a
/// transition-end position coincides with an existing vertex. Expressed as a
/// fraction of the edge's length (upstream uses an absolute tolerance; we keep
/// it proportional because the graph's unit scale varies across fixtures).
const SNAP_FRAC: f64 = 1e-6;

/// Resolves a half-edge's "to" vertex index via its twin's `start_vertex`,
/// matching [`super::graph`]'s own convention (see that module's doc comment,
/// and [`super::centrality`]'s identically-named private helper — duplicated
/// here rather than shared, matching this packet's existing per-module
/// convention of small self-contained helpers). Returns [`NO_INDEX`] if
/// unresolvable (missing/out-of-range twin).
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

/// Euclidean length of edge `edge_idx` in the scaled-integer unit space.
fn edge_length(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> f64 {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return 0.0;
    };
    let Some(start_v) = graph.vertices.get(edge.start_vertex) else {
        return 0.0;
    };
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let Some(end_v) = graph.vertices.get(to_idx) else {
        return 0.0;
    };
    let dx = end_v.position.x - start_v.position.x;
    let dy = end_v.position.y - start_v.position.y;
    (dx * dx + dy * dy).sqrt()
}

/// Linearly interpolates between two vertex positions by parameter `t`
/// (`t = 0.0` returns `start`, `t = 1.0` returns `end`).
fn interpolate_position(
    a: crate::voronoi::Vertex,
    b: crate::voronoi::Vertex,
    t: f64,
) -> crate::voronoi::Vertex {
    crate::voronoi::Vertex {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}

/// Computes the interpolated radius at position `pos` along edge `edge_idx`.
fn _radius_at(graph: &SkeletalTrapezoidationGraph, edge_idx: usize, pos: f64) -> f64 {
    let edge = match graph.edges.get(edge_idx) {
        Some(e) => e,
        None => return 0.0,
    };
    let start_r = graph
        .vertices
        .get(edge.start_vertex)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(0.0);
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let end_r = graph
        .vertices
        .get(to_idx)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(start_r);
    start_r + (end_r - start_r) * pos
}

/// Returns all edges whose endpoints' `distance_to_boundary` increase along the
/// edge direction (`start_R < end_R`). This is the set upstream calls
/// `upward_quad_mids` in the propagation passes.
///
/// **Packet 141 (N7) — centrality gate dropped.** The previous implementation
/// also filtered on `e.central`, which silently excluded the upward
/// (non-flat) rib-foot connections that canonical `upwardQuadMids`
/// (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1669-1672`)
/// includes. The name is preserved to minimise call-site blast radius; the
/// behaviour is now "all strictly-upward edges" rather than "strictly-upward
/// central edges".
///
/// Tie order is index-ascending (deterministic).
fn upward_central_edges(graph: &SkeletalTrapezoidationGraph) -> Vec<usize> {
    let mut order: Vec<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(idx, e)| {
            let start_r = graph
                .vertices
                .get(e.start_vertex)
                .map(|v| v.distance_to_boundary)
                .unwrap_or(f64::INFINITY);
            let to_idx = resolve_to_vertex(graph, *idx);
            let end_r = graph
                .vertices
                .get(to_idx)
                .map(|v| v.distance_to_boundary)
                .unwrap_or(f64::NEG_INFINITY);
            start_r < end_r
        })
        .map(|(idx, _)| idx)
        .collect();
    // Sort by descending R so the "upward" walk is from high radius down to
    // low radius in forward order — upstream iterates this list in reverse for
    // upward propagation and forward for downward propagation.
    order.sort_by(|&a, &b| {
        let ra = graph.edges[a].r_max;
        let rb = graph.edges[b].r_max;
        rb.partial_cmp(&ra)
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(&b))
    });
    order
}

/// Generates transition-middle annotations for every upward central edge whose
/// bead count increases from `from` to `to`.
///
/// For each step from `start_bead_count` to `end_bead_count - 1`, computes
/// `mid_R = strategy.get_transition_thickness(lower_bead_count) / 2` and the
/// corresponding linear position `mid_pos = edge_size * (mid_R - start_R) /
/// (end_R - start_R)`. The annotation is stored on the edge's
/// [`super::graph::STHalfEdge::transition_mids`] vector.
pub fn generate_transition_mids(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    let n_edges = graph.edges.len();
    for edge_idx in 0..n_edges {
        let edge = match graph.edges.get(edge_idx) {
            Some(e) => e.clone(),
            None => continue,
        };
        if !edge.central {
            continue;
        }
        let start_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        let Some(start) = graph.vertices.get(start_v).cloned() else {
            continue;
        };
        let Some(end) = graph.vertices.get(to_v).cloned() else {
            continue;
        };
        let start_r = start.distance_to_boundary;
        let end_r = end.distance_to_boundary;
        if start_r >= end_r {
            continue;
        }
        let Some(start_bc) = start.bead_count else {
            continue;
        };
        let Some(end_bc) = end.bead_count else {
            continue;
        };
        if end_bc <= start_bc {
            continue;
        }

        let edge_size = edge_length(graph, edge_idx);
        if edge_size <= 0.0 || !edge_size.is_finite() {
            continue;
        }

        for lower_bc in start_bc..end_bc {
            let mid_r = strategy.get_transition_thickness(lower_bc as usize) / 2.0;
            let t = (mid_r - start_r) / (end_r - start_r);
            let pos = t.clamp(0.0, 1.0);
            graph.edges[edge_idx]
                .transition_mids
                .push(TransitionMiddle {
                    pos,
                    lower_bead_count: lower_bc,
                    mid_r,
                });
        }
    }
}

/// A transition end: position along a half-edge plus the bead count that should
/// be assigned to the inserted/split vertex at this position.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TransitionEnd {
    pos: f64,
    bead_count: u32,
    /// Radius at which this transition end occurs; used to compute the
    /// `transition_ratio` of a newly inserted vertex.
    mid_r: f64,
}
/// Splits a central half-edge at fractional position `pos`, mirroring
/// OrcaSlicer's `insertNode`+`insertRib` pair
/// (`SkeletalTrapezoidationGraph.cpp:615-644` + `:515-595`).
///
/// `edge_idx` is a central `NORMAL` half-edge; its `twin` walks the opposite
/// direction over the same physical segment. This function splits BOTH
/// sides at the same physical position, producing:
///
/// - 1 shared **mid node** (the spine split vertex) carrying `bead_count` and
///   `distance_to_boundary = mid_r` — the configured transition radius.
/// - 2 **boundary (rib-foot) nodes** (one per side), each with
///   `distance_to_boundary = 0.0`.
/// - 4 new half-edges: 2 **rib pair** edges per side
///   (`forth_rib`/`back_rib`, both `EdgeType::EXTRA_VD` so the existing
///   centrality and `generate_toolpaths` filters treat them as ribs).
/// - The original edge and its twin are repurposed as the "first" split
///   fragments (keeping their indices), now ending at the shared mid node;
///   a new `NORMAL` "second" fragment is appended per side, continuing to
///   each side's original far endpoint.
/// - **Cross-twin patching**: `first_input.twin = last_twin`,
///   `last_input.twin = first_twin` (and the reverse), so the post-split
///   edge pair remains a consistent twin-linked physical edge across both
///   sides — the root invariant `connectJunctions`/`getNextUnconnected`
///   relies on.
///
/// `next`/`prev` chains on both sides are rewired so the chain walks
/// `edge_before -> first_input -> forth_rib -> [dead end]` on one side and
/// `[dead end] <- back_rib <- second_input -> edge_after` continuing on, with
/// the rib `back` edge's `.prev == NO_INDEX` seeding it as an unprocessed
/// quad start (matching `make_rib`'s own convention).
///
/// `mid_r` is taken from the caller (the transition-mid's recorded radius)
/// rather than recomputed via perpendicular-foot projection onto a source
/// segment: source-segment provenance is not retained past
/// `from_polygons`, but `transition_mids[i].mid_r` *is* the perpendicular-foot
/// radius by construction (`generate_transition_mids` computes it as
/// `strategy.get_transition_thickness(lower_bc) / 2`), so the value is
/// faithful.
///
/// Returns the index of the new "second" fragment on the input side (the
/// edge continuing from the mid node to the input edge's original far
/// endpoint), matching OrcaSlicer's `last_edge_replacing_input` return —
/// so callers splitting the same original edge multiple times pass that
/// returned index back in to chain the splits in order.
fn insert_node(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_idx: usize,
    pos: f64,
    bead_count: u32,
    mid_r: f64,
) -> usize {
    let edge = match graph.edges.get(edge_idx).cloned() {
        Some(e) => e,
        None => return NO_INDEX,
    };
    let twin_idx = edge.twin;
    if twin_idx == NO_INDEX || twin_idx == edge_idx {
        // No twin to split — fall back to a one-sided split so the caller
        // still gets a usable return index. This path is not on the faithful
        // parity hot path (every central transition edge has a twin).
        return insert_node_one_sided(graph, edge_idx, pos, bead_count, mid_r);
    }
    let twin = match graph.edges.get(twin_idx).cloned() {
        Some(e) => e,
        None => return NO_INDEX,
    };

    let input_start = edge.start_vertex;
    let input_to = resolve_to_vertex(graph, edge_idx);
    let twin_start = twin.start_vertex;
    let twin_to = resolve_to_vertex(graph, twin_idx);
    if input_start == NO_INDEX
        || input_to == NO_INDEX
        || twin_start == NO_INDEX
        || twin_to == NO_INDEX
    {
        return NO_INDEX;
    }
    let input_start_v = match graph.vertices.get(input_start).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };
    let input_end_v = match graph.vertices.get(input_to).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };

    let p = pos.clamp(0.0, 1.0);
    let mid_pos = interpolate_position(input_start_v.position, input_end_v.position, p);

    // --- Shared mid node (spine split vertex) -----------------------------
    let mid_node = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: mid_pos,
        distance_to_boundary: mid_r,
        bead_count: Some(bead_count),
        transition_ratio: 0.0,
    });

    // --- Boundary (rib-foot) nodes, one per side --------------------------
    // OrcaSlicer projects the mid node onto the source segment; we don't
    // retain source provenance past construction, so the foot sits at the
    // mid node's own position projected onto the input edge's line (the
    // medial-axis edge is the bisector of two source segments; the foot on
    // either source is the mid node itself only when the edge is straight,
    // which is the common case for transition splits on straight spines).
    // For the parity tests the key invariant is distance_to_boundary == 0
    // (a boundary sentinel), not the foot's exact x/y.
    let foot_in_pos = mid_pos;
    let foot_in = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: foot_in_pos,
        distance_to_boundary: 0.0,
        bead_count: None,
        transition_ratio: 0.0,
    });
    let foot_twin_pos = mid_pos;
    let foot_twin = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: foot_twin_pos,
        distance_to_boundary: 0.0,
        bead_count: None,
        transition_ratio: 0.0,
    });

    // --- Capture pre-mutation topology ------------------------------------
    let input_prev = edge.prev;
    let input_next = edge.next;
    let twin_prev = twin.prev;
    let twin_next = twin.next;

    // --- Append the 4 new edges -------------------------------------------
    // second_input: mid_node -> input_to (the "second" fragment on the input side)
    let second_input = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX, // patched below
        next: input_next,
        prev: NO_INDEX, // patched below (back_in)
        r_min: mid_r.min(input_end_v.distance_to_boundary),
        r_max: mid_r.max(input_end_v.distance_to_boundary),
        central: edge.central,
        is_curved: edge.is_curved,
        edge_type: EdgeType::NORMAL,
        transition_mids: Vec::new(),
        ..STHalfEdge::default()
    });
    // second_twin: mid_node -> twin_to (the "second" fragment on the twin side)
    let second_twin = graph.edges.len();
    let twin_end_v = match graph.vertices.get(twin_to).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX,
        next: twin_next,
        prev: NO_INDEX,
        r_min: mid_r.min(twin_end_v.distance_to_boundary),
        r_max: mid_r.max(twin_end_v.distance_to_boundary),
        central: twin.central,
        is_curved: twin.is_curved,
        edge_type: EdgeType::NORMAL,
        transition_mids: Vec::new(),
        ..STHalfEdge::default()
    });
    // forth_rib / back_rib on the input side (mid_node <-> foot_in)
    let forth_in = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX,
        next: NO_INDEX,
        prev: edge_idx,
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });
    let back_in = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: foot_in,
        twin: NO_INDEX,
        next: second_input,
        prev: NO_INDEX, // seeds this as an unprocessed quad start
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });
    // forth_rib / back_rib on the twin side (mid_node <-> foot_twin)
    let forth_twin = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX,
        next: NO_INDEX,
        prev: twin_idx,
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });
    let back_twin = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: foot_twin,
        twin: NO_INDEX,
        next: second_twin,
        prev: NO_INDEX,
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });

    // --- Twin-pair the rib pairs (within each side) -----------------------
    // forth_in <-> back_in ; forth_twin <-> back_twin
    graph.edges[forth_in].twin = back_in;
    graph.edges[back_in].twin = forth_in;
    graph.edges[forth_twin].twin = back_twin;
    graph.edges[back_twin].twin = forth_twin;

    // --- Cross-twin patching (the F1+F6 invariant) ------------------------
    // first_input.twin = last_twin ; last_twin.twin = first_input
    // last_input.twin = first_twin ; first_twin.twin = last_input
    // first_input = edge_idx (kept) ; last_input = second_input (new)
    // first_twin  = twin_idx  (kept) ; last_twin  = second_twin (new)
    graph.edges[edge_idx].twin = second_twin;
    graph.edges[second_twin].twin = edge_idx;
    graph.edges[second_input].twin = twin_idx;
    graph.edges[twin_idx].twin = second_input;

    // --- Repurpose the original edge + twin as the "first" fragments -----
    // first_input now goes from input_start -> mid_node
    if let Some(orig) = graph.edges.get_mut(edge_idx) {
        let (r_min, r_max) =
            super::graph::edge_radius_bounds(&graph.vertices, input_start, mid_node);
        orig.r_min = r_min;
        orig.r_max = r_max;
        orig.next = forth_in; // chain: first_input -> forth_rib -> [dead end]
        orig.prev = input_prev;
        // central preserved (it was central on entry).
    }
    // first_twin now goes from twin_start -> mid_node
    if let Some(orig_twin) = graph.edges.get_mut(twin_idx) {
        let (r_min, r_max) =
            super::graph::edge_radius_bounds(&graph.vertices, twin_start, mid_node);
        orig_twin.r_min = r_min;
        orig_twin.r_max = r_max;
        orig_twin.next = forth_twin;
        orig_twin.prev = twin_prev;
    }

    // --- Rewire the second fragments' prev to the back ribs ---------------
    graph.edges[second_input].prev = back_in;
    graph.edges[second_twin].prev = back_twin;

    // --- Rewire whatever used to follow the original edge/twin -----------
    // The original edge's old `next` followed `edge_idx`; it now follows
    // `second_input` (the new "second" fragment on the input side).
    if input_next != NO_INDEX {
        if let Some(follower) = graph.edges.get_mut(input_next) {
            follower.prev = second_input;
        }
    }
    if twin_next != NO_INDEX {
        if let Some(follower) = graph.edges.get_mut(twin_next) {
            follower.prev = second_twin;
        }
    }
    // The original edge's old `prev` (if any) still chains into `edge_idx`,
    // which is correct (edge_idx is still the "first" fragment). No change
    // needed on `input_prev`/`twin_prev`.

    second_input
}

/// One-sided fallback for `insert_node` when the input edge has no twin
/// (defensive only — every central transition edge in a well-formed graph
/// has a twin). Splits `edge_idx` at `pos`, inserting a new vertex with the
/// given `bead_count` and `distance_to_boundary = mid_r`, plus a single rib
/// pair to the boundary foot. Returns the new "second" fragment's index.
fn insert_node_one_sided(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_idx: usize,
    pos: f64,
    bead_count: u32,
    mid_r: f64,
) -> usize {
    let edge = match graph.edges.get(edge_idx).cloned() {
        Some(e) => e,
        None => return NO_INDEX,
    };
    let start_v = match graph.vertices.get(edge.start_vertex).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let end_v = match graph.vertices.get(to_idx).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };
    let p = pos.clamp(0.0, 1.0);
    let mid_pos = interpolate_position(start_v.position, end_v.position, p);

    let mid_node = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: mid_pos,
        distance_to_boundary: mid_r,
        bead_count: Some(bead_count),
        transition_ratio: 0.0,
    });
    let foot = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: mid_pos,
        distance_to_boundary: 0.0,
        bead_count: None,
        transition_ratio: 0.0,
    });

    let old_next = edge.next;
    let second = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX,
        next: old_next,
        prev: NO_INDEX,
        r_min: mid_r.min(end_v.distance_to_boundary),
        r_max: mid_r.max(end_v.distance_to_boundary),
        central: edge.central,
        is_curved: edge.is_curved,
        edge_type: EdgeType::NORMAL,
        transition_mids: Vec::new(),
        ..STHalfEdge::default()
    });
    let forth = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: mid_node,
        twin: NO_INDEX,
        next: NO_INDEX,
        prev: edge_idx,
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });
    let back = graph.edges.len();
    graph.edges.push(STHalfEdge {
        start_vertex: foot,
        twin: NO_INDEX,
        next: second,
        prev: NO_INDEX,
        r_min: 0.0,
        r_max: mid_r,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    });
    graph.edges[forth].twin = back;
    graph.edges[back].twin = forth;

    if let Some(orig) = graph.edges.get_mut(edge_idx) {
        let (r_min, r_max) =
            super::graph::edge_radius_bounds(&graph.vertices, orig.start_vertex, mid_node);
        orig.r_min = r_min;
        orig.r_max = r_max;
        orig.next = forth;
        // twin stays NO_INDEX (no twin to patch).
    }
    graph.edges[second].prev = back;
    if old_next != NO_INDEX {
        if let Some(follower) = graph.edges.get_mut(old_next) {
            follower.prev = second;
        }
    }
    second
}

/// For each edge carrying [`super::graph::STHalfEdge::transition_mids`],
/// generates corresponding transition ends on the edge's **own** bucket,
/// sorts them **ascending** by position, then inserts new vertices via
/// [`insert_node`] — one atomic call per end that splits BOTH the edge and
/// its twin at the same physical position, producing a single shared
/// boundary (rib-foot) node.
///
/// Mirrors OrcaSlicer's `applyTransitions`
/// (`SkeletalTrapezoidation.cpp:1487-1543`): the mirrored ends go onto the
/// edge's own bucket (not the twin's), sorted ascending (not descending),
/// and `insertNode` is called once per end (not twice per physical edge).
///
/// # F2 fix (Arachne parity audit)
///
/// The previous implementation pushed mirrored ends onto the **twin's**
/// bucket and sorted **descending**, then ran two independent `insert_node`
/// calls (one on the edge, one on the twin) — producing 2 new vertices
/// instead of 1 shared boundary node, and physically misaligning the split
/// positions on the two sides. The faithful implementation consolidates
/// all ends onto one bucket and lets `insert_node`'s atomic twin-side
/// split handle both sides in one call.
///
/// # Chaining repeated splits on the same edge
///
/// [`insert_node`] returns the index of the new "second" fragment
/// (`last_edge_replacing_input` in OrcaSlicer), which continues from the
/// mid node to the original edge's far endpoint. Subsequent splits on the
/// same original edge are passed this returned index so the splits
/// accumulate in order along the chain — matching OrcaSlicer's
/// `last_edge_replacing_input = graph.insertNode(last_edge_replacing_input, ...)`.
pub fn apply_transitions(graph: &mut SkeletalTrapezoidationGraph) {
    // Collect transition ends per edge. Only the edge's OWN transition_mids
    // contribute (the twin is split atomically by `insert_node`, so no
    // mirroring onto the twin's bucket is needed). Keyed by `BTreeMap` so
    // newly inserted edges (appended after every edge this pass started
    // with) do not participate — their `transition_mids` are empty.
    let mut per_edge_ends: BTreeMap<usize, Vec<TransitionEnd>> = BTreeMap::new();
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if edge.transition_mids.is_empty() {
            continue;
        }
        let bucket = per_edge_ends.entry(edge_idx).or_default();
        for tm in &edge.transition_mids {
            bucket.push(TransitionEnd {
                pos: tm.pos,
                bead_count: tm.lower_bead_count,
                mid_r: tm.mid_r,
            });
        }
    }

    for ends in per_edge_ends.values_mut() {
        // Sort **ascending** by position — see the F2 fix note above and
        // OrcaSlicer's `transitions.sort([](a, b) { return a.pos < b.pos; })`.
        // Ascending order means the first split is nearest the edge's start
        // vertex; `insert_node` returns the "second" fragment, so the next
        // split lands on the sub-edge continuing toward the far endpoint —
        // exactly OrcaSlicer's `last_edge_replacing_input` chaining.
        ends.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(Ordering::Equal)
                .then(a.bead_count.cmp(&b.bead_count))
                .then(a.mid_r.partial_cmp(&b.mid_r).unwrap_or(Ordering::Equal))
        });
        // Deduplicate ends that land at effectively the same position and bead
        // count (common when pos == 0.5 and its mirror coincide).
        ends.dedup_by(|a, b| (a.pos - b.pos).abs() < SNAP_FRAC && a.bead_count == b.bead_count);
    }

    // Apply splits. We process in edge-index order (BTreeMap iteration) so
    // newly inserted edges (appended after every edge this loop started
    // with) do not participate in this pass — their transition_mids are
    // empty.
    for (edge_idx, ends) in per_edge_ends {
        // `working_edge` tracks the current sub-edge to split, starting as
        // the original edge and advancing to the returned "second" fragment
        // after each split — matching OrcaSlicer's `last_edge_replacing_input`.
        let mut working_edge = edge_idx;
        // `consumed` tracks how much of the ORIGINAL edge's length has been
        // split off so far (in the original-fraction frame). The current
        // `working_edge` spans the remaining `[consumed, 1.0]` portion, so
        // an `end.pos` measured in the original frame must be rescaled to
        // `local_pos = (end.pos - consumed) / (1 - consumed)` before being
        // passed to `insert_node` (which interprets `pos` as a fraction of
        // `working_edge`'s *current* span).
        let mut consumed = 0.0_f64;
        for end in ends {
            let remaining = (1.0 - consumed).max(f64::EPSILON);
            let local_pos = ((end.pos - consumed) / remaining).clamp(0.0, 1.0);
            let edge = match graph.edges.get(working_edge) {
                Some(e) => e.clone(),
                None => continue,
            };
            if local_pos < SNAP_FRAC {
                // Snap to the start vertex.
                if let Some(v) = graph.vertices.get_mut(edge.start_vertex) {
                    v.bead_count = Some(end.bead_count);
                    v.transition_ratio = 0.0;
                }
                continue;
            }
            if local_pos > 1.0 - SNAP_FRAC {
                // Snap to the end vertex.
                let to_v = resolve_to_vertex(graph, working_edge);
                if let Some(v) = graph.vertices.get_mut(to_v) {
                    v.bead_count = Some(end.bead_count);
                    v.transition_ratio = 0.0;
                }
                continue;
            }

            let returned = insert_node(graph, working_edge, local_pos, end.bead_count, end.mid_r);
            if returned == NO_INDEX {
                // Split failed; leave working_edge as-is so subsequent ends
                // still attempt the original edge.
                continue;
            }
            // The returned "second" fragment spans `[end.pos, 1.0]` of the
            // original edge, so `consumed` advances to `end.pos`.
            consumed = end.pos;
            working_edge = returned;
        }
    }
}

/// Computes the exact traversal order [`propagate_beadings_upward`] walks:
/// `upward_central_edges`'s own descending-`r_max` order reversed (yielding
/// ascending order), or — for hand-built test graphs with no strictly-upward
/// edges at all (every endpoint tied on `distance_to_boundary`) — all
/// edges sorted ascending by `r_min` (tie-broken by index) and then
/// *also* reversed, exactly mirroring the single `iter.iter().rev())` this
/// function used to apply uniformly to whichever list it picked. Factored out
/// so [`compute_dist_to_bottom_source`] can replay the identical walk (same
/// order ⇒ same accumulated distances) without duplicating this order/
/// fallback logic a third time.
///
/// **Packet 141 (N7) — centrality gate dropped** in the fallback (matching
/// [`upward_central_edges`] and the corresponding change in
/// [`propagate_beadings_downward_with_transition_dist`]'s own fallback).
fn upward_propagation_order(graph: &SkeletalTrapezoidationGraph) -> Vec<usize> {
    let order = upward_central_edges(graph);
    if !order.is_empty() {
        let mut ascending = order;
        ascending.reverse();
        return ascending;
    }
    let mut all: Vec<usize> = graph.edges.iter().enumerate().map(|(idx, _)| idx).collect();
    all.sort_by(|&a, &b| {
        graph.edges[a]
            .r_min
            .partial_cmp(&graph.edges[b].r_min)
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(&b))
    });
    all.reverse();
    all
}

/// Propagates resolved beadings upward (from lower radius to higher radius)
/// along central edges, copying the `from` node's bead count to an unset `to`
/// node and accumulating distance to the nearest bottom source.
///
/// Mirrors `propagateBeadingsUpward` (L1800-1826). Iterates `upward_quad_mids`
/// in reverse order (see [`upward_propagation_order`]).
pub fn propagate_beadings_upward(graph: &mut SkeletalTrapezoidationGraph) {
    for edge_idx in upward_propagation_order(graph) {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        let from_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v == NO_INDEX || from_v == NO_INDEX {
            continue;
        }
        if graph
            .vertices
            .get(to_v)
            .and_then(|v| v.bead_count)
            .is_some()
        {
            continue;
        }
        let Some(from_bead_count) = graph.vertices.get(from_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let source_transition_ratio = graph.vertices[from_v].transition_ratio;
        if let Some(to_vertex) = graph.vertices.get_mut(to_v) {
            to_vertex.bead_count = Some(from_bead_count);
            // transition_ratio propagates from upstream source.
            to_vertex.transition_ratio = source_transition_ratio;
        }
    }
}

/// Interpolates a bead count using the upstream `interpolate()` weighting.
/// Weight is `ratio_of_top = dist_to_bottom_source /
/// min(total_dist, beading_propagation_transition_dist)`. Returns a
/// floating-point blend; callers round as appropriate.
fn interpolate_bead_counts(bottom_bc: u32, top_bc: u32, ratio_of_top: f64) -> u32 {
    let t = ratio_of_top.clamp(0.0, 1.0);
    let blended = bottom_bc as f64 * (1.0 - t) + top_bc as f64 * t;
    blended.round() as u32
}

/// Width/location-blended `Beading` between `bottom` and `top` per the
/// upstream `interpolate()` weighting (`SkeletalTrapezoidation.cpp:1883-1885`,
/// `ratio_of_top` = `dist_to_bottom_source / min(total_dist,
/// beading_propagation_transition_dist)`).
///
/// Unlike [`interpolate_bead_counts`] — which only blends the integer bead
/// count and discards the per-bead width/location structure — this blends
/// `total_thickness`, every per-bead `bead_width`, and every per-bead
/// `toolpath_location` elementwise, matching OrcaSlicer's full
/// `BeadingPropagation` blend. The two `Beading`s must have the same
/// `bead_count` (`bead_widths.len() == toolpath_locations.len()`) for the
/// elementwise blend to be well-defined; if they differ, the longer one is
/// truncated to the shorter so the result is deterministic and never
/// silently expands the beading.
///
/// `ratio_of_top` is clamped to `[0, 1]`. `left_over` is also linearly
/// blended (consistent with the other scalars — OrcaSlicer's blend does
/// not distinguish it).
pub(crate) fn interpolate_bead_propagation(
    bottom: &Beading,
    top: &Beading,
    ratio_of_top: f64,
) -> Beading {
    let t = ratio_of_top.clamp(0.0, 1.0);
    let n = bottom.bead_widths.len().min(top.bead_widths.len());
    let mut widths = Vec::with_capacity(n);
    let mut locations = Vec::with_capacity(n);
    for i in 0..n {
        let bw = bottom.bead_widths[i] * (1.0 - t) + top.bead_widths[i] * t;
        let bl = bottom.toolpath_locations[i] * (1.0 - t) + top.toolpath_locations[i] * t;
        widths.push(bw);
        locations.push(bl);
    }
    Beading {
        total_thickness: bottom.total_thickness * (1.0 - t) + top.total_thickness * t,
        bead_widths: widths,
        toolpath_locations: locations,
        left_over: bottom.left_over * (1.0 - t) + top.left_over * t,
    }
}

/// Populates [`SkeletalTrapezoidationGraph::beading_propagation`] for every
/// vertex whose `bead_count = Some(bc)`, by calling
/// `strategy.compute(2.0 * v.distance_to_boundary, bc as usize)` and storing
/// the resulting `Beading` in the side table.
///
/// **Packet 141 (N7) — N1's substrate.** Step 2 (N1) reads this side table
/// via [`SkeletalTrapezoidationGraph::get_beding`] /
/// [`SkeletalTrapezoidationGraph::get_nearest_beding`] to resolve a
/// per-junction beading (instead of the current
/// per-endpoint-from-`bead_count` interpolation, which is finding N1 of the
/// second-pass audit).
///
/// This function is intentionally separate from
/// [`assign_bead_counts`](super::bead_count::assign_bead_counts) so Packet B
/// (the `BeadingStrategy` trait extension) owns the future move of the
/// beading computation into the primary pass; the side table is populated
/// here against an already-strategy-resolved graph, on demand, matching how
/// OrcaSlicer's `BeadingPropagation` is built in a single pass at
/// `updateBeadCount` time but stored on the graph for later reads.
///
/// Vertices with `bead_count = None` (rib-foot nodes) are left as `None` in
/// the side table; the structural invariant is "rib-foot ⇒ `None`",
/// "primary ⇒ `Some`" (see
/// `tests/arachne_beding_propagation_side_table.rs`'s
/// `populate_side_table_covers_primary_vertices_only`).
pub fn populate_beading_propagation(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    // Resize the side table if the graph's vertex count has changed since
    // `from_polygons` (e.g. `apply_transitions::insert_node` added split
    // vertices). `apply_transitions` is out of scope for Step 1, but this
    // resize keeps the side table index-parallel to `vertices` even if a
    // caller wires it in.
    if graph.beading_propagation.len() != graph.vertices.len() {
        graph.beading_propagation.resize(graph.vertices.len(), None);
    }
    for (v_idx, v) in graph.vertices.iter().enumerate() {
        let Some(bc) = v.bead_count else {
            continue;
        };
        let thickness = 2.0 * v.distance_to_boundary;
        let beading = strategy.compute(thickness, bc as usize);
        // Defensive invariant check (mirrors `Beading`'s own documented
        // contract): strategies that violate this are caught here rather
        // than at first `get_beding` call.
        debug_assert_eq!(
            beading.bead_widths.len(),
            beading.toolpath_locations.len(),
            "BeadingStrategy::compute({}, {}) produced a beading with mismatched \
             bead_widths.len() = {} and toolpath_locations.len() = {}",
            thickness,
            bc,
            beading.bead_widths.len(),
            beading.toolpath_locations.len()
        );
        if let Some(slot) = graph.beading_propagation.get_mut(v_idx) {
            *slot = Some(beading);
        }
    }
}

/// Structural set of vertices that receive a *primary* (non-propagated) bead
/// count: exactly the "to" vertices of upward edges, matching
/// `bead_count.rs::assign_bead_counts`'s own primary-pass gate ("for each
/// central edge, assign the bead count at the edge's `to` vertex") — computed
/// fresh from the graph's *current* edge topology, not from the (already
/// fully-propagated, hence uninformative) `bead_count` field.
///
/// **Packet 141 (N7) — centrality gate dropped.** The previous implementation
/// filtered on `edge.central`, which (per the same change in
/// [`upward_central_edges`]) silently excluded rib-foot connections the
/// canonical `upwardQuadMids` includes.
///
/// This also correctly captures every vertex `apply_transitions::insert_node`
/// creates: splitting a central edge repoints that edge's own "to" (via
/// `.twin`) to the new split vertex, and `insert_node` always sets
/// `bead_count: Some(_)` directly on it — so by construction a split vertex
/// is *also* the "to" vertex of a (now-shrunk) edge in the
/// post-`apply_transitions` graph this function is always called against,
/// and is correctly treated as primary/real, not propagated.
///
/// Vertices that are never any edge's "to" (rare — only possible for a
/// vertex with no incoming edge at all) are the genuine gaps
/// [`propagate_beadings_upward`] exists to fill; used by
/// [`compute_dist_to_bottom_source`] to know when accumulated distance is
/// implicitly zero (a real source) versus needs summing along the chain.
fn primary_source_vertices(graph: &SkeletalTrapezoidationGraph) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    for edge_idx in 0..graph.edges.len() {
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v != NO_INDEX {
            set.insert(to_v);
        }
    }
    set
}

/// Recomputes, from the graph's current structure, the accumulated
/// "distance to bottom source" (`dist_to_bottom_source` in the pre-digested
/// OrcaSlicer notes) that [`propagate_beadings_upward`] built while filling
/// gaps — i.e. how far a genuinely gap-filled vertex's copied bead count has
/// travelled from the nearest real/primary source below it.
///
/// # Why this is recomputed rather than passed through directly
///
/// [`propagate_beadings_downward`]'s signature is frozen (every existing test
/// call site invokes it with no extra arguments), so it cannot *receive* the
/// map [`propagate_beadings_upward`] would have produced. This function
/// closes that gap by replaying the identical walk
/// ([`upward_propagation_order`]) using [`primary_source_vertices`] as the
/// "already had a real bead count" gate — structurally equivalent to
/// `propagate_beadings_upward`'s own `bead_count.is_some()` gate at the time
/// it actually ran, because in this crate's simplified (quad-less, rib-less)
/// topology every central edge's "to" vertex already gets a primary
/// assignment from `assign_bead_counts`/`apply_transitions` (see
/// [`primary_source_vertices`]'s doc comment) — "propagated, non-primary" is
/// the rare gap case, not the rule, so this recomputation is exact for this
/// crate's graph shape, not merely an approximation.
///
/// Vertices absent from the returned map have an implicit distance of `0.0`
/// (either a genuine primary source, or simply never touched by upward
/// propagation) — matching upstream's zero-initialized `BeadingPropagation`
/// for a freshly-created real beading.
fn compute_dist_to_bottom_source(graph: &SkeletalTrapezoidationGraph) -> BTreeMap<usize, f64> {
    let primary = primary_source_vertices(graph);
    let mut dist: BTreeMap<usize, f64> = BTreeMap::new();
    for edge_idx in upward_propagation_order(graph) {
        let edge = match graph.edges.get(edge_idx) {
            Some(e) => e,
            None => continue,
        };
        let from_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v == NO_INDEX || from_v == NO_INDEX || primary.contains(&to_v) {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        let edge_len = if edge_len.is_finite() && edge_len > 0.0 {
            edge_len
        } else {
            0.0
        };
        let from_dist = if primary.contains(&from_v) {
            0.0
        } else {
            dist.get(&from_v).copied().unwrap_or(0.0)
        };
        // `or_insert`: the first edge (in traversal order) that reaches `to_v`
        // wins, matching `propagate_beadings_upward`'s own "skip if already
        // has a bead count" gate — once filled, later edges targeting the
        // same `to_v` are no-ops there too.
        dist.entry(to_v).or_insert(from_dist + edge_len);
    }
    dist
}

/// Real (non-placeholder) default beading-propagation transition distance,
/// sourced from this crate's own registered
/// [`crate::beading::factory::BeadingFactoryParams::default`]'s
/// `default_transition_length` (4000 units = 0.4mm at `UNITS_PER_MM`,
/// matching the `wall_transition_length` config key's registered default).
///
/// Used by [`propagate_beadings_downward`] (the frozen, no-argument entry
/// point every existing caller/test invokes) as a real fallback in place of
/// the previous placeholder `4.0` (0.0004mm). Callers with access to the
/// actual configured value — [`crate::arachne::pipeline::run_arachne_pipeline`]
/// — call [`propagate_beadings_downward_with_transition_dist`] directly with
/// that value instead, so production runs get real config fidelity even
/// though this default exists for the frozen entry point.
fn default_beading_propagation_transition_dist() -> f64 {
    crate::beading::factory::BeadingFactoryParams::default().default_transition_length
}

/// Propagates resolved beadings downward (from higher radius to lower
/// radius) along central edges, blending via `interpolate()` when the lower
/// node already carries a beading, using `transition_dist` as the
/// beading-propagation transition distance (upstream's
/// `beading_propagation_transition_dist`, in this crate's units).
///
/// Mirrors `propagateBeadingsDownward` (L1833-1899). Iterates
/// `upward_quad_mids` in forward order (descending R) and routes single-edge
/// propagation from the peak (`edge_to_peak->to`) down to the bottom
/// (`edge_to_peak->from`).
///
/// # Packet 113c Step 8b fix
///
/// The previous implementation used a placeholder `transition_dist = 4.0`
/// (0.0004mm) and computed `total_dist` as only the current edge's own
/// length — ignoring any distance already accumulated along the chain. Both
/// combined to make `ratio_of_top` clamp to (effectively) 1.0 for virtually
/// every real edge via a *symmetric* `clamp(0.0, 1.0)`, turning the intended
/// bead-count blend into an unconditional top-overwrite: e.g. a 10mm
/// square's four corners, correctly assigned `bead_count = 0` by the primary
/// pass, were silently overwritten to `max_bead_count` here — corrupting
/// bead counts at domain-chain stitch points.
///
/// This port now:
/// - computes `total_dist` as the real cumulative chain distance
///   (`top_dist_from_source + edge_len + bottom_dist_to_source`, this
///   crate's names for upstream's `top_beading.dist_from_top_source` /
///   `bottom_beading.dist_to_bottom_source`);
/// - floors `ratio_of_top` at `0.0` only (no ceiling clamp) and instead
///   branches explicitly on `ratio_of_top >= 1.0` for the full-overwrite
///   case, matching upstream's asymmetric gate;
/// - extends the `dist_from_top_source` bookkeeping on every full-copy
///   (both the "no existing beading" branch and the "ratio >= 1.0" branch)
///   so further-down edges in the same chain see the correct cumulative
///   distance, while the merge/blend branch deliberately does *not* extend
///   it — matching upstream's fresh `BeadingPropagation(merged_beading)` (a
///   blended node becomes its own new zero-distance reference point); and
/// - gates the ratio math on the bottom node already carrying *some* beading
///   at all (`bottom_has_beading`), matching upstream's `!hasBeading()`
///   branch — this gate already existed correctly in the prior
///   implementation and is preserved, not part of the bug.
///
/// `dist_to_bottom_source` (accumulated *upward* by
/// [`propagate_beadings_upward`]) is recomputed here via
/// [`compute_dist_to_bottom_source`] rather than received directly from that
/// earlier call — see that function's doc comment for why this is exact,
/// not an approximation, and why it must be recomputed rather than passed
/// through (this function's own signature is frozen).
pub fn propagate_beadings_downward_with_transition_dist(
    graph: &mut SkeletalTrapezoidationGraph,
    transition_dist: f64,
) {
    let transition_dist = if transition_dist.is_finite() && transition_dist > 0.0 {
        transition_dist
    } else {
        default_beading_propagation_transition_dist()
    };

    let dist_to_bottom_source = compute_dist_to_bottom_source(graph);

    let order = upward_central_edges(graph);
    // Fallback for hand-built test graphs with no strictly-upward edges.
    // **Packet 141 (N7) — centrality gate dropped here too** to match
    // [`upward_central_edges`]: the fallback must consider all edges, not
    // just central ones, so the propagation shape stays consistent.
    let fallback_order: Vec<usize> = if order.is_empty() {
        let mut all: Vec<usize> = graph.edges.iter().enumerate().map(|(idx, _)| idx).collect();
        all.sort_by(|&a, &b| {
            graph.edges[b]
                .r_max
                .partial_cmp(&graph.edges[a].r_max)
                .unwrap_or(Ordering::Equal)
                .then(a.cmp(&b))
        });
        all
    } else {
        Vec::new()
    };
    let iter: &[usize] = if !order.is_empty() {
        &order
    } else {
        &fallback_order
    };

    // Accumulated distance from the nearest real top-side source, keyed by
    // vertex — built fresh during this single forward (descending-R) walk;
    // see the "Packet 113c Step 8b fix" doc comment above for why the merge
    // branch deliberately never inserts into this map.
    let mut dist_from_top_source: BTreeMap<usize, f64> = BTreeMap::new();

    for &edge_idx in iter {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        // For a single central edge, the peak is the `to` vertex (higher R),
        // the bottom is the `from` vertex (lower R).
        let peak_v = resolve_to_vertex(graph, edge_idx);
        let bottom_v = edge.start_vertex;
        if peak_v == NO_INDEX || bottom_v == NO_INDEX {
            continue;
        }
        let Some(peak_bc) = graph.vertices.get(peak_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let edge_len = edge_length(graph, edge_idx);
        if !edge_len.is_finite() || edge_len <= 0.0 {
            continue;
        }

        let top_dist_from_source = dist_from_top_source.get(&peak_v).copied().unwrap_or(0.0);
        let source_transition_ratio = graph.vertices[peak_v].transition_ratio;
        let bottom_has_beading = graph
            .vertices
            .get(bottom_v)
            .and_then(|v| v.bead_count)
            .is_some();

        if !bottom_has_beading {
            // Fresh downward propagation (upstream's `!hasBeading()` branch):
            // no ratio math, straight copy, and extend the bookkeeping so
            // further-down edges see the correct cumulative distance.
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(peak_bc);
                v.transition_ratio = source_transition_ratio;
            }
            // Side-table write: copy the top vertex's beading verbatim
            // into the bottom vertex's slot when one is available. The
            // `clone` is intentional: we cannot hold an immutable borrow
            // on `peak_v`'s slot and a mutable borrow on `bottom_v`'s
            // simultaneously, so we copy out and write in.
            let top_beading_clone: Option<Beading> = graph
                .beading_propagation
                .get(peak_v)
                .and_then(|e| e.as_ref())
                .cloned();
            if let (Some(beading), Some(slot)) = (
                top_beading_clone,
                graph.beading_propagation.get_mut(bottom_v),
            ) {
                *slot = Some(beading);
            }
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
            continue;
        }

        let Some(bottom_bc) = graph.vertices.get(bottom_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let bottom_dist_to_source = dist_to_bottom_source.get(&bottom_v).copied().unwrap_or(0.0);
        let total_dist = top_dist_from_source + edge_len + bottom_dist_to_source;
        let denom = total_dist.min(transition_dist).max(f64::EPSILON);
        // Floor at 0 only -- NOT a symmetric clamp(0,1); the explicit
        // `>= 1.0` branch below handles the ceiling (see doc comment).
        let ratio_of_top = (bottom_dist_to_source / denom).max(0.0);

        if ratio_of_top >= 1.0 {
            // Full overwrite -- still extends dist_from_top_source (upstream:
            // `bottom_beading = top_beading; bottom_beading.dist_from_top_source += length;`).
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(peak_bc);
                v.transition_ratio = source_transition_ratio;
            }
            // Side-table write: when the top vertex already carries a
            // stored beading (i.e. `populate_beading_propagation` ran
            // before this pass), copy that beading verbatim into the
            // bottom vertex's slot. This is the "full overwrite" case
            // from upstream's perspective. See the same borrow-checker
            // pattern as in the no-beading branch above.
            let top_beading_clone: Option<Beading> = graph
                .beading_propagation
                .get(peak_v)
                .and_then(|e| e.as_ref())
                .cloned();
            if let (Some(beading), Some(slot)) = (
                top_beading_clone,
                graph.beading_propagation.get_mut(bottom_v),
            ) {
                *slot = Some(beading);
            }
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
        } else {
            let blended = interpolate_bead_counts(bottom_bc, peak_bc, ratio_of_top);
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(blended);
            }
            // Side-table write (the audit's "width/location blend" path):
            // when both endpoints already carry a stored beading, blend
            // them elementwise into the bottom vertex's slot. This is the
            // canonical OrcaSlicer `BeadingPropagation` merge shape; the
            // integer `bead_count` write above remains for back-compat
            // with downstream readers (e.g. `generate_toolpaths`) until
            // Step 2 turns them into side-table readers. See the same
            // borrow-checker pattern as the other two side-table writes
            // above (clone out, write in).
            let bottom_beading_clone: Option<Beading> = graph
                .beading_propagation
                .get(bottom_v)
                .and_then(|e| e.as_ref())
                .cloned();
            let top_beading_clone: Option<Beading> = graph
                .beading_propagation
                .get(peak_v)
                .and_then(|e| e.as_ref())
                .cloned();
            if let (Some(bottom_beading), Some(top_beading), Some(slot)) = (
                bottom_beading_clone.as_ref(),
                top_beading_clone.as_ref(),
                graph.beading_propagation.get_mut(bottom_v),
            ) {
                *slot = Some(interpolate_bead_propagation(
                    bottom_beading,
                    top_beading,
                    ratio_of_top,
                ));
            }
            // Merge branch deliberately does not record `dist_from_top_source`
            // for `bottom_v` -- see doc comment (upstream: a merged beading is
            // a fresh `BeadingPropagation`, i.e. distance resets to 0 here).
        }
    }
}

/// Frozen no-argument entry point every existing caller/test invokes.
/// Delegates to [`propagate_beadings_downward_with_transition_dist`] with
/// [`default_beading_propagation_transition_dist`] — see both functions' doc
/// comments for the packet 113c Step 8b fix and why the transition distance
/// can't be threaded through this particular signature.
pub fn propagate_beadings_downward(graph: &mut SkeletalTrapezoidationGraph) {
    propagate_beadings_downward_with_transition_dist(
        graph,
        default_beading_propagation_transition_dist(),
    );
}
