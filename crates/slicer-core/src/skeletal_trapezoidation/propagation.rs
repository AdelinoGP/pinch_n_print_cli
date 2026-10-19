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

use super::graph::{
    STHalfEdge, STVertex, SkeletalTrapezoidationGraph, TransitionEnd, TransitionMiddle,
};
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
/// The boundary foot of a rib inserted at `p` on `edge_idx`'s chain, and its
/// distance from `p`: canonical `SkeletalTrapezoidationGraph::insertRib`'s
/// `getSource(edge).distance_to_squared(p, &px)`.
///
/// `getSource` walks `prev` to the chain's first edge and `next` to its last;
/// the source "segment" runs from the first edge's `from` (a boundary point)
/// to the last edge's `to` (a boundary point) — the stretch of outline the
/// quad was built from. `px` is the closest point of that segment to `p`.
///
/// Returns `None` when the chain cannot be resolved; callers keep their
/// previous fallback in that case.
fn rib_source_foot(
    graph: &SkeletalTrapezoidationGraph,
    edge_idx: usize,
    p: crate::voronoi::Vertex,
) -> Option<(crate::voronoi::Vertex, f64)> {
    let guard = graph.edges.len();
    let mut from_edge = edge_idx;
    for _ in 0..guard {
        let prev = graph.edges.get(from_edge)?.prev;
        if prev == NO_INDEX {
            break;
        }
        from_edge = prev;
    }
    let mut to_edge = edge_idx;
    for _ in 0..guard {
        let next = graph.edges.get(to_edge)?.next;
        if next == NO_INDEX {
            break;
        }
        to_edge = next;
    }
    let a = graph
        .vertices
        .get(graph.edges.get(from_edge)?.start_vertex)?
        .position;
    let b = graph
        .vertices
        .get(resolve_to_vertex(graph, to_edge))?
        .position;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let foot = crate::voronoi::Vertex {
        x: a.x + t * dx,
        y: a.y + t * dy,
    };
    let dist = ((p.x - foot.x).powi(2) + (p.y - foot.y).powi(2)).sqrt();
    Some((foot, dist))
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
/// Each side's rib ends on that side's outline: the boundary node sits at
/// the mid node's closest point on the side's source segment (canonical
/// `getSource` + `insertRib`, see [`rib_source_foot`]), and the mid node's
/// `distance_to_boundary` is that projection distance, as canonical sets it.
/// `mid_r` (the caller's transition radius) is only the fallback for a chain
/// whose source cannot be resolved.
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

    // --- Boundary (rib-foot) nodes, one per side --------------------------
    // Canonical `insertRib` projects the mid node onto each side's source
    // segment (`getSource`: the chain's first `from` to its last `to`), puts
    // the rib's boundary node at that foot, and sets the mid node's
    // `distance_to_boundary` to the projection distance (the twin side's
    // call runs last, so its distance is the one that sticks). Both feet are
    // resolved here, before any topology below is rewired.
    //
    // The feet used to sit AT the mid node, making every transition rib a
    // zero-length edge from an R=0 "boundary" node to an R>0 node on top of
    // it. `generate_junctions` emits nothing on a zero-length edge, so every
    // quad bounded by such a rib lost its walls once `connect_junctions`
    // stopped bridging quads (benchy layer 51 hull sides: all three walls
    // gapped over the 4 -> 5 bead transition).
    let (foot_in_pos, _) = rib_source_foot(graph, edge_idx, mid_pos).unwrap_or((mid_pos, mid_r));
    let (foot_twin_pos, twin_dist) =
        rib_source_foot(graph, twin_idx, mid_pos).unwrap_or((mid_pos, mid_r));
    let mid_r = if twin_dist > 0.0 { twin_dist } else { mid_r };

    // --- Shared mid node (spine split vertex) -----------------------------
    let mid_node = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: mid_pos,
        distance_to_boundary: mid_r,
        bead_count: Some(bead_count),
        transition_ratio: 0.0,
    });

    let foot_in = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: foot_in_pos,
        distance_to_boundary: 0.0,
        bead_count: None,
        transition_ratio: 0.0,
    });
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
    // Canonical `insertRib` foot + distance; see `insert_node`.
    let (foot_pos, dist) = rib_source_foot(graph, edge_idx, mid_pos).unwrap_or((mid_pos, mid_r));
    let mid_r = if dist > 0.0 { dist } else { mid_r };

    let mid_node = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: mid_pos,
        distance_to_boundary: mid_r,
        bead_count: Some(bead_count),
        transition_ratio: 0.0,
    });
    let foot = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: foot_pos,
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

/// Canonical `transition_filter_dist` (`WallToolPaths::generate`,
/// `WallToolPaths.cpp`): `scaled<coord_t>(100.f)`, i.e. 100 mm, in this
/// crate's 100 nm units. It bounds how far `dissolve_nearby_transitions`
/// walks the central skeleton looking for a partner transition; in practice
/// the walk stops much earlier, at the `allowed_filter_deviation` gate or at
/// the end of the central region.
pub const TRANSITION_FILTER_DIST_UNITS: f64 = 100.0 * slicer_ir::UNITS_PER_MM;

/// Removes bead-count transitions that would only create a short, marginal
/// bead-count region, mirroring canonical
/// `SkeletalTrapezoidation::filterTransitionMids` (`SkeletalTrapezoidation.cpp`).
///
/// For each upward central edge carrying mids, the back (highest-position)
/// mid is paired with the same-`lower_bead_count` mids reachable going up,
/// and the front mid with those reachable going down
/// (`TransitionFilter::dissolve_nearby_transitions`). When the walk
/// succeeds, the region enclosed by the pair is relabelled to the bead count
/// outside it (`dissolve_bead_count_region`) and the paired mids are removed.
/// A mid whose transition would run past the end of the central region is
/// removed too (`filter_end_of_central_transition`).
///
/// `transition_filter_dist` is canonical's constant
/// ([`TRANSITION_FILTER_DIST_UNITS`]); `allowed_filter_deviation` is the
/// configured `wall_transition_filter_deviation`: the line-width deviation
/// that dissolving a region may introduce. Both are in slicer units.
///
/// Mid positions are stored as fractions of the edge length here, where
/// canonical stores absolute distances from `edge.from`; every distance below
/// is converted back to units before it is compared.
pub fn filter_transition_mids(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
    transition_filter_dist: f64,
    allowed_filter_deviation: f64,
) {
    let filter = TransitionFilter {
        max_dist: transition_filter_dist,
        allowed_filter_deviation,
    };
    for edge_idx in 0..graph.edges.len() {
        if graph.edges[edge_idx].transition_mids.is_empty() {
            continue;
        }
        let twin_idx = graph.edges[edge_idx].twin;
        let ab_size = edge_length(graph, edge_idx);

        // Back: walk up from the highest mid.
        let back = graph.edges[edge_idx]
            .transition_mids
            .last()
            .cloned()
            .expect("checked non-empty");
        let back_dist = (1.0 - back.pos) * ab_size;
        let back_refs = filter.dissolve_nearby_transitions(graph, edge_idx, &back, back_dist, true);
        let mut should_dissolve_back = !back_refs.is_empty();
        if should_dissolve_back {
            dissolve_bead_count_region(
                graph,
                edge_idx,
                back.lower_bead_count + 1,
                back.lower_bead_count,
            );
            erase_mids(graph, &back_refs);
        }
        {
            let bc = back.lower_bead_count as usize;
            let upper_half_length = (1.0 - strategy.get_transition_anchor_pos(bc))
                * strategy.get_transitioning_length(bc);
            should_dissolve_back |= filter_end_of_central_transition(
                graph,
                edge_idx,
                back_dist,
                upper_half_length,
                back.lower_bead_count,
                0,
            );
        }
        if should_dissolve_back {
            graph.edges[edge_idx].transition_mids.pop();
        }
        if graph.edges[edge_idx].transition_mids.is_empty() || twin_idx == NO_INDEX {
            continue;
        }

        // Front: walk down (along the twin) from the lowest mid.
        let front = graph.edges[edge_idx].transition_mids[0];
        let front_dist = front.pos * ab_size;
        let front_refs =
            filter.dissolve_nearby_transitions(graph, twin_idx, &front, front_dist, false);
        let mut should_dissolve_front = !front_refs.is_empty();
        if should_dissolve_front {
            dissolve_bead_count_region(
                graph,
                twin_idx,
                front.lower_bead_count,
                front.lower_bead_count + 1,
            );
            erase_mids(graph, &front_refs);
        }
        {
            let bc = front.lower_bead_count as usize;
            let lower_half_length =
                strategy.get_transition_anchor_pos(bc) * strategy.get_transitioning_length(bc);
            should_dissolve_front |= filter_end_of_central_transition(
                graph,
                twin_idx,
                front_dist,
                lower_half_length,
                front.lower_bead_count + 1,
                0,
            );
        }
        if should_dissolve_front && !graph.edges[edge_idx].transition_mids.is_empty() {
            graph.edges[edge_idx].transition_mids.remove(0);
        }
    }
}

/// `(edge, mid index)` of a mid that `TransitionFilter::dissolve_nearby_transitions`
/// paired with the origin mid: canonical's `TransitionMidRef`.
type MidRef = (usize, usize);

/// Removes the referenced mids, highest index first so the remaining indices
/// stay valid (canonical erases each list iterator in turn).
fn erase_mids(graph: &mut SkeletalTrapezoidationGraph, refs: &[MidRef]) {
    let mut refs = refs.to_vec();
    refs.sort_unstable();
    refs.dedup();
    for &(edge_idx, mid_idx) in refs.iter().rev() {
        if let Some(edge) = graph.edges.get_mut(edge_idx) {
            if mid_idx < edge.transition_mids.len() {
                edge.transition_mids.remove(mid_idx);
            }
        }
    }
}

/// The half-edges leaving `edge_idx`'s `to` node, other than its own twin:
/// canonical's `for (edge = edge_to_start->next; edge && edge !=
/// edge_to_start->twin; edge = edge->twin->next)`. Capped at the edge count
/// so a malformed rotation cannot loop forever.
fn outgoing_edges(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let Some(start) = graph.edges.get(edge_idx) else {
        return out;
    };
    let stop = start.twin;
    let mut cursor = start.next;
    while cursor != NO_INDEX && cursor != stop && out.len() <= graph.edges.len() {
        out.push(cursor);
        cursor = graph
            .edges
            .get(cursor)
            .and_then(|e| graph.edges.get(e.twin))
            .map(|t| t.next)
            .unwrap_or(NO_INDEX);
    }
    out
}

fn vertex_r(graph: &SkeletalTrapezoidationGraph, vertex_idx: usize) -> f64 {
    graph
        .vertices
        .get(vertex_idx)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(0.0)
}

/// The two thresholds canonical `SkeletalTrapezoidation` keeps as members for
/// `dissolveNearbyTransitions`.
struct TransitionFilter {
    max_dist: f64,
    allowed_filter_deviation: f64,
}

/// One pending call of canonical's recursive `dissolveNearbyTransitions`.
struct DissolveFrame {
    outgoing: Vec<usize>,
    next: usize,
    traveled: f64,
    edge_to_start: usize,
    /// Whether this call has collected a mid or spawned a successful branch.
    produced: bool,
}

impl TransitionFilter {
    /// Canonical `dissolveNearbyTransitions`: walks the central skeleton from
    /// `edge_to_start`'s `to` node, away from where it came from, collecting
    /// every mid with `origin`'s `lower_bead_count` reached within
    /// `max_dist`. Any branch that dead-ends, overruns `max_dist`, or reaches a
    /// node whose radius deviates from `origin.mid_r` by more than
    /// `allowed_filter_deviation` (in line-width terms) empties the whole
    /// result: the region is then too long or too different to dissolve.
    ///
    /// Canonical recurses once per edge walked; this walks an explicit stack
    /// of frames instead, because a 100 mm budget over a finely split
    /// skeleton can nest deeper than a worker thread's stack allows. A branch
    /// that re-enters a half-edge already on the current path fails. That is
    /// where canonical's recursion ends up once the loop has used up
    /// `max_dist`, because a second lap cannot meet a mid the first lap missed.
    fn dissolve_nearby_transitions(
        &self,
        graph: &SkeletalTrapezoidationGraph,
        edge_to_start: usize,
        origin: &TransitionMiddle,
        traveled: f64,
        going_up: bool,
    ) -> Vec<MidRef> {
        if traveled > self.max_dist {
            return Vec::new();
        }
        let dissolve_result_is_odd = (origin.lower_bead_count % 2 == 1) == going_up;
        let mut found: Vec<MidRef> = Vec::new();
        let mut on_path: BTreeSet<usize> = BTreeSet::new();
        on_path.insert(edge_to_start);
        let mut stack = vec![DissolveFrame {
            outgoing: outgoing_edges(graph, edge_to_start),
            next: 0,
            traveled,
            edge_to_start,
            produced: false,
        }];

        while let Some(frame) = stack.last_mut() {
            if frame.next >= frame.outgoing.len() {
                // Canonical: an empty result from any call aborts the whole
                // dissolve (`to_be_dissolved_here.empty()` -> clear, return).
                if !frame.produced {
                    return Vec::new();
                }
                on_path.remove(&frame.edge_to_start);
                stack.pop();
                continue;
            }
            let edge_idx = frame.outgoing[frame.next];
            frame.next += 1;
            let Some(edge) = graph.edges.get(edge_idx) else {
                continue;
            };
            if !edge.central {
                continue;
            }

            let from_r = vertex_r(graph, edge.start_vertex);
            let to_r = vertex_r(graph, resolve_to_vertex(graph, edge_idx));
            let width_deviation = (origin.mid_r - from_r).abs() * 2.0;
            let line_width_deviation = if dissolve_result_is_odd {
                width_deviation
            } else {
                width_deviation / 2.0
            };
            if line_width_deviation > self.allowed_filter_deviation {
                return Vec::new();
            }

            let ab_size = edge_length(graph, edge_idx);
            // Mids live on the upward half-edge only (see
            // `generate_transition_mids`); an equidistant edge carries none.
            let is_aligned = from_r < to_r;
            let aligned_idx = if is_aligned { edge_idx } else { edge.twin };
            let mut seen_transition_on_this_edge = false;
            if let Some(aligned) = graph.edges.get(aligned_idx) {
                for (mid_idx, mid) in aligned.transition_mids.iter().enumerate() {
                    let pos = if is_aligned { mid.pos } else { 1.0 - mid.pos } * ab_size;
                    if frame.traveled + pos < self.max_dist
                        && mid.lower_bead_count == origin.lower_bead_count
                    {
                        found.push((aligned_idx, mid_idx));
                        seen_transition_on_this_edge = true;
                    }
                }
            }
            if seen_transition_on_this_edge {
                frame.produced = true;
                continue;
            }

            let next_traveled = frame.traveled + ab_size;
            if next_traveled > self.max_dist || on_path.contains(&edge_idx) {
                return Vec::new();
            }
            // A failing child aborts everything, so a pushed child that
            // returns counts as this frame's contribution.
            frame.produced = true;
            on_path.insert(edge_idx);
            stack.push(DissolveFrame {
                outgoing: outgoing_edges(graph, edge_idx),
                next: 0,
                traveled: next_traveled,
                edge_to_start: edge_idx,
                produced: false,
            });
        }
        found
    }
}

/// Canonical `dissolveBeadCountRegion`: relabels the central region reached
/// from `edge_to_start`'s `to` node from `from_bead_count` to
/// `to_bead_count`, stopping at nodes with any other count. Iterative flood
/// fill (canonical recurses); each node is relabelled at most once, so the
/// result does not depend on the visiting order.
fn dissolve_bead_count_region(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_to_start: usize,
    from_bead_count: u32,
    to_bead_count: u32,
) {
    let mut stack = vec![edge_to_start];
    while let Some(edge_idx) = stack.pop() {
        let to_v = resolve_to_vertex(graph, edge_idx);
        match graph.vertices.get_mut(to_v) {
            Some(v) if v.bead_count == Some(from_bead_count) => {
                v.bead_count = Some(to_bead_count);
            }
            _ => continue,
        }
        for next in outgoing_edges(graph, edge_idx) {
            if graph.edges.get(next).is_some_and(|e| e.central) {
                stack.push(next);
            }
        }
    }
}

/// Depth cap for [`filter_end_of_central_transition`]. Its walk is bounded by
/// half a transition length, so the cap only guards degenerate zero-length
/// cycles, which canonical would walk forever.
const END_OF_CENTRAL_MAX_DEPTH: usize = 4096;

/// Canonical `filterEndOfCentralTransition`: when the central region ends
/// within `max_dist` of the mid (half the transition length on that side),
/// the transition cannot fit. The nodes on the way get
/// `replacing_bead_count`, and `true` tells the caller to drop the mid.
fn filter_end_of_central_transition(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_to_start: usize,
    traveled: f64,
    max_dist: f64,
    replacing_bead_count: u32,
    depth: usize,
) -> bool {
    if traveled > max_dist || depth > END_OF_CENTRAL_MAX_DEPTH {
        return false;
    }
    let mut is_end_of_central = true;
    let mut should_dissolve = false;
    for next in outgoing_edges(graph, edge_to_start) {
        if graph.edges.get(next).is_some_and(|e| e.central) {
            let length = edge_length(graph, next);
            should_dissolve |= filter_end_of_central_transition(
                graph,
                next,
                traveled + length,
                max_dist,
                replacing_bead_count,
                depth + 1,
            );
            is_end_of_central = false;
        }
    }
    if is_end_of_central && traveled < max_dist {
        should_dissolve = true;
    }
    if should_dissolve {
        let to_v = resolve_to_vertex(graph, edge_to_start);
        if let Some(v) = graph.vertices.get_mut(to_v) {
            v.bead_count = Some(replacing_bead_count);
        }
    }
    should_dissolve
}

/// Canonical `generateAllTransitionEnds` port: converts each `TransitionMiddle`
/// into a lower end and an upper end straddling the mid by the configured
/// transition length, recursing onto successor edges when an end spills past
/// the current edge.
///
/// The lower end walks backward from the mid toward the edge's start vertex;
/// the upper end walks forward toward the edge's end vertex. When an end
/// spills past an edge boundary, the traversed vertex receives a fractional
/// `transition_ratio` and the remaining length is carried forward (or backward)
/// onto successor edges.
pub fn generate_all_transition_ends(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    clear_transition_ends(graph);
    let n_edges = graph.edges.len();
    for edge_idx in 0..n_edges {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        if !edge.central || edge.transition_mids.is_empty() {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        if edge_len <= 0.0 || !edge_len.is_finite() {
            continue;
        }
        let to_v = resolve_to_vertex(graph, edge_idx);
        let start_v = edge.start_vertex;
        for mid in edge.transition_mids.clone() {
            let lower_bc = mid.lower_bead_count as usize;
            let transition_len = strategy.get_transitioning_length(lower_bc);
            let anchor = strategy.get_transition_anchor_pos(lower_bc);
            let lower_len = anchor * transition_len;
            let upper_len = (1.0 - anchor) * transition_len;

            // PNP adaptation: skip transitions with lower_bead_count == 0
            // because creating a bead_count=0 lower-end TransitionEnd would
            // corrupt downstream propagation (PNP's propagation/toolpath
            // code cannot handle bead_count=0 nodes the way canonical can).
            // Canonical OrcaSlicer does NOT skip (BeadingStrategy.cpp:49-57
            // returns a tiny 10µm length to avoid division-by-zero), but
            // PNP's graph model diverges enough that the skip is needed here.
            // This remains a deliberate transition-end adaptation.
            if mid.lower_bead_count == 0 {
                continue;
            }

            // --- Lower end (backward from mid toward start vertex) ---
            let lower_end_fraction = mid.pos - lower_len / edge_len;
            if lower_end_fraction >= 0.0 {
                graph.edges[edge_idx].transition_ends.push(TransitionEnd {
                    pos: lower_end_fraction,
                    lower_bead_count: mid.lower_bead_count,
                    mid_r: mid.mid_r,
                    is_lower_end: true,
                });
            } else if let Some(v) = graph.vertices.get_mut(start_v) {
                v.bead_count = Some(mid.lower_bead_count);
            }

            // --- Upper end (forward from mid toward end vertex) ---
            let upper_end_fraction = mid.pos + upper_len / edge_len;
            if upper_end_fraction <= 1.0 {
                graph.edges[edge_idx].transition_ends.push(TransitionEnd {
                    pos: upper_end_fraction,
                    lower_bead_count: mid.lower_bead_count,
                    mid_r: mid.mid_r,
                    is_lower_end: false,
                });
            } else {
                // Upper end spills past this edge's end vertex.
                let remaining_units = (upper_end_fraction - 1.0) * edge_len;
                if remaining_units > 0.0 {
                    let ratio = (remaining_units / upper_len).min(1.0);
                    if let Some(v) = graph.vertices.get_mut(to_v) {
                        v.transition_ratio = ratio;
                    }
                    push_upper_end_on_successors(
                        graph,
                        edge_idx,
                        to_v,
                        remaining_units,
                        mid.lower_bead_count,
                        mid.mid_r,
                    );
                }
            }
        }
    }
}

/// Pushes an upper-end `TransitionEnd` on all central successor edges,
/// carrying `remaining_units` forward from the vertex at `from_v`.
fn push_upper_end_on_successors(
    graph: &mut SkeletalTrapezoidationGraph,
    current_edge: usize,
    _from_v: usize,
    remaining_units: f64,
    lower_bead_count: u32,
    mid_r: f64,
) {
    // Follow the quad chain: current_edge.next, then outgoing edges via
    // twin->next (matching canonical outgoing central successor walk).
    let mut visited = BTreeSet::new();
    visited.insert(current_edge);

    let mut queue: Vec<(usize, f64)> = Vec::new();
    if let Some(edge) = graph.edges.get(current_edge) {
        let next = edge.next;
        if next != NO_INDEX && visited.insert(next) {
            queue.push((next, remaining_units));
        }
    }

    while let Some((edge_idx, rem)) = queue.pop() {
        let edge = match graph.edges.get(edge_idx) {
            Some(e) => e.clone(),
            None => continue,
        };
        if !edge.central {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        if edge_len <= 0.0 || !edge_len.is_finite() {
            continue;
        }
        let pos = (rem / edge_len).min(1.0);
        graph.edges[edge_idx].transition_ends.push(TransitionEnd {
            pos,
            lower_bead_count,
            mid_r,
            is_lower_end: false,
        });

        // Also follow outgoing edges from the twin side for multi-way junctions.
        if edge.next != NO_INDEX && visited.insert(edge.next) {
            queue.push((edge.next, 0.0));
        }
        let twin = edge.twin;
        if twin != NO_INDEX && twin != edge_idx {
            if let Some(twin_edge) = graph.edges.get(twin) {
                let twin_next = twin_edge.next;
                if twin_next != NO_INDEX && visited.insert(twin_next) {
                    queue.push((twin_next, 0.0));
                }
            }
        }
    }
}

/// Clears all `transition_ends` from all edges (idempotent).
fn clear_transition_ends(graph: &mut SkeletalTrapezoidationGraph) {
    for edge in graph.edges.iter_mut() {
        edge.transition_ends.clear();
    }
}

/// Applies transition-end splits to the graph, inserting new spine vertices
/// at positions marked by `TransitionEnd` annotations.
///
/// With explicit ends (from `generate_all_transition_ends`), `is_lower_end`
/// determines bead count. Falling back to `transition_mids` (legacy callers),
/// bead count is always `lower_bead_count`.
///
/// For each edge carrying [`super::graph::STHalfEdge::transition_mids`],
/// generates corresponding transition ends on the edge's **own** bucket,
/// sorts them **ascending** by position, then inserts new vertices via
/// [`insert_node`] — one atomic call per end that splits BOTH the edge and
/// its twin at the same physical position, producing a single shared
/// boundary (rib-foot) node.
///
/// Mirrors OrcaSlicer's `applyTransitions` (`SkeletalTrapezoidation.cpp`): the mirrored ends go onto the
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
pub fn apply_transitions(graph: &mut SkeletalTrapezoidationGraph) {
    let mut per_edge_ends: BTreeMap<usize, Vec<TransitionEnd>> = BTreeMap::new();
    let mut has_explicit_ends = false;
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if !edge.transition_ends.is_empty() {
            has_explicit_ends = true;
            let bucket = per_edge_ends.entry(edge_idx).or_default();
            bucket.extend(edge.transition_ends.iter().cloned());
        }
    }

    if !has_explicit_ends {
        for (edge_idx, edge) in graph.edges.iter().enumerate() {
            if edge.transition_mids.is_empty() {
                continue;
            }
            let bucket = per_edge_ends.entry(edge_idx).or_default();
            for tm in &edge.transition_mids {
                bucket.push(TransitionEnd {
                    pos: tm.pos,
                    lower_bead_count: tm.lower_bead_count,
                    mid_r: tm.mid_r,
                    is_lower_end: true,
                });
            }
        }
    }

    // When reading from explicit ends (generate_all_transition_ends), bead count
    // is determined by is_lower_end. When falling back to transition_mids (legacy
    // callers that never ran generate_all_transition_ends), bead count is always
    // lower_bead_count (the old single-mid-split behavior).
    let compute_bead_count = if has_explicit_ends {
        |end: &TransitionEnd| -> u32 {
            if end.is_lower_end {
                end.lower_bead_count
            } else {
                end.lower_bead_count + 1
            }
        }
    } else {
        |end: &TransitionEnd| -> u32 { end.lower_bead_count }
    };

    for ends in per_edge_ends.values_mut() {
        ends.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(Ordering::Equal)
                .then(a.lower_bead_count.cmp(&b.lower_bead_count))
                .then(a.mid_r.partial_cmp(&b.mid_r).unwrap_or(Ordering::Equal))
        });
        ends.dedup_by(|a, b| {
            (a.pos - b.pos).abs() < SNAP_FRAC
                && a.lower_bead_count == b.lower_bead_count
                && a.is_lower_end == b.is_lower_end
        });
    }

    for (edge_idx, ends) in per_edge_ends {
        let mut working_edge = edge_idx;
        let mut consumed = 0.0_f64;
        for end in ends {
            let bead_count = compute_bead_count(&end);
            let remaining = (1.0 - consumed).max(f64::EPSILON);
            let local_pos = ((end.pos - consumed) / remaining).clamp(0.0, 1.0);
            let edge = match graph.edges.get(working_edge) {
                Some(e) => e.clone(),
                None => continue,
            };
            if local_pos < SNAP_FRAC {
                if let Some(v) = graph.vertices.get_mut(edge.start_vertex) {
                    v.bead_count = Some(bead_count);
                }
                continue;
            }
            if local_pos > 1.0 - SNAP_FRAC {
                let to_v = resolve_to_vertex(graph, working_edge);
                if let Some(v) = graph.vertices.get_mut(to_v) {
                    v.bead_count = Some(bead_count);
                }
                continue;
            }

            let returned = insert_node(graph, working_edge, local_pos, bead_count, end.mid_r);
            if returned == NO_INDEX {
                continue;
            }
            consumed = end.pos;
            working_edge = returned;
        }
    }
}

/// Generates extra rib nodes along upward central edges longer than
/// `DISCRETIZATION_STEP_UNITS` by inserting a vertex at every nonlinear
/// thickness radius returned by `strategy.get_nonlinear_thicknesses()`.
///
/// Port of OrcaSlicer's `generateExtraRibs`
/// (`SkeletalTrapezoidation.cpp:1579-1633`).
pub fn generate_extra_ribs(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    use super::graph::DISCRETIZATION_STEP_UNITS;

    let n_edges = graph.edges.len();
    for edge_idx in 0..n_edges {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        // Qualifying edges: central, upward (from.R < to.R), and long enough
        if !edge.central {
            continue;
        }
        let from_v_idx = edge.start_vertex;
        let to_v_idx = resolve_to_vertex(graph, edge_idx);
        let from_r = graph
            .vertices
            .get(from_v_idx)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0);
        let to_r = graph
            .vertices
            .get(to_v_idx)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0);
        if from_r >= to_r {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        if edge_len <= DISCRETIZATION_STEP_UNITS {
            continue;
        }

        let from_bead_count = graph
            .vertices
            .get(from_v_idx)
            .and_then(|v| v.bead_count)
            .unwrap_or(0);
        let to_bead_count = graph
            .vertices
            .get(to_v_idx)
            .and_then(|v| v.bead_count)
            .unwrap_or(0);

        let rib_thicknesses = strategy.get_nonlinear_thicknesses(from_bead_count as usize);
        if rib_thicknesses.is_empty() {
            continue;
        }

        let mut last_edge: usize = edge_idx;
        // Upstream snap_dist() returns a small absolute tolerance in slicer units
        let snap_dist = 10.0; // ~0.001mm, canonical snap_dist()
        for rib_thickness in &rib_thicknesses {
            let half_thickness = rib_thickness / 2.0;
            if half_thickness <= from_r {
                continue;
            }
            if half_thickness >= to_r {
                break;
            }

            let new_bead_count = std::cmp::min(from_bead_count, to_bead_count);
            // Position along the edge: pos = (half_thickness - from_r) / (to_r - from_r)
            let t = (half_thickness - from_r) / (to_r - from_r);
            let pos = t.clamp(0.0, 1.0);

            // Snap to close vertex if within snap_dist
            let close_bead_count = if pos < 0.5 {
                from_bead_count
            } else {
                to_bead_count
            };
            let abs_pos = pos * edge_len;
            if (abs_pos < snap_dist || abs_pos > edge_len - snap_dist)
                && close_bead_count == new_bead_count
            {
                if pos < snap_dist {
                    // Close to from node — set transition_ratio
                    if let Some(v) = graph.vertices.get_mut(from_v_idx) {
                        v.transition_ratio = 0.0;
                    }
                } else {
                    if let Some(v) = graph.vertices.get_mut(to_v_idx) {
                        v.transition_ratio = 0.0;
                    }
                }
                continue;
            }

            let mid_r = half_thickness;
            // Split at pos, using the current last_edge
            last_edge = insert_node(graph, last_edge, pos, new_bead_count, mid_r);
            if last_edge == NO_INDEX {
                break;
            }
        }
    }
}

/// Computes the exact traversal order [`propagate_beadings_upward`] walks:
/// `upward_central_edges`'s own descending-`r_max` order reversed (yielding
/// ascending order), or — for hand-built test graphs with no strictly-upward
/// edges at all (every endpoint tied on `distance_to_boundary`) — all
/// edges sorted ascending by `r_min` (tie-broken by index) and then
/// *also* reversed, exactly mirroring the single `iter.iter().rev())` this
/// function used to apply uniformly to whichever list it picked.
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
/// along central edges, copying the `from` node's **beading** into an unset
/// `to` node's side-table slot.
///
/// Faithful port of `propagateBeadingsUpward`
/// (`SkeletalTrapezoidation.cpp:1561-1588`), whose body is:
///
/// ```text
/// if (upward_edge->to->data.bead_count >= 0)   continue; // Don't override local beading
/// if (!upward_edge->from->data.hasBeading())   continue; // Only propagate if we have something
/// if (upward_edge->to->data.hasBeading())      continue; // Only propagate where there is place
/// BeadingPropagation upper_beading = lower_beading;      // COPY the beading
/// upward_edge->to->data.setBeading(...);                 // attach it; bead_count stays -1
/// ```
///
/// Two properties are load-bearing and were previously violated:
///
/// 1. **Copy the beading, don't propagate the scalar bead count.** Canonical
///    carries the *lower* node's whole `Beading` (its thickness and its actual
///    per-bead widths) upward verbatim. The previous implementation instead wrote
///    `to.bead_count = from.bead_count` and let `populate_beading_propagation`
///    later recompute `compute(2 * to.distance_to_boundary, bead_count)` — i.e.
///    the THIN node's bead count against the THICK node's thickness. On a benchy
///    hull's medial spine that yielded `compute(19.7mm, 3)` = `[0.4, 18.9, 0.4]`:
///    `DistributedBeadingStrategy` parks the whole surplus in the middle bead, so
///    a ~19mm-wide "wall" (43x the nozzle) got extruded — the D4 inner-wall
///    over-extrusion. Canonical's own assert documents the intent:
///    `upper_beading.beading.total_thickness <= to->distance_to_boundary * 2` —
///    a propagated beading is EXPECTED to be thinner than its destination; the
///    surplus is infill, not extrudate.
/// 2. **Never write `bead_count`/`transition_ratio` on the destination joint.**
///    Canonical leaves a purely-propagated node at `bead_count == -1` and only
///    attaches the beading. `propagate_beadings_downward` already documents this
///    same rule (writing `bead_count` flattens the gradient and trips
///    `generate_junctions`'s same-bead-count skip gate).
///
/// Iterates `upward_quad_mids` in reverse order (see [`upward_propagation_order`]).
/// Requires [`populate_beading_propagation`] to have run first (canonical order,
/// `SkeletalTrapezoidation.cpp:1488-1508`) so there are beadings to propagate.
pub fn propagate_beadings_upward(graph: &mut SkeletalTrapezoidationGraph) {
    if graph.beading_propagation.len() != graph.vertices.len() {
        graph.beading_propagation.resize(graph.vertices.len(), None);
    }
    resize_dist_to_bottom_source(graph);
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
        // "Don't override local beading" (`:1567-1570`): a node with its own
        // bead count already had its beading computed by
        // `populate_beading_propagation`.
        if graph
            .vertices
            .get(to_v)
            .and_then(|v| v.bead_count)
            .is_some()
        {
            continue;
        }
        // "Only propagate to places where there is place" (`:1577-1580`).
        if graph
            .beading_propagation
            .get(to_v)
            .and_then(|slot| slot.as_ref())
            .is_some()
        {
            continue;
        }
        // "Only propagate if we have something to propagate" (`:1571-1574`).
        let Some(from_beading) = graph
            .beading_propagation
            .get(from_v)
            .and_then(|slot| slot.as_ref())
            .cloned()
        else {
            continue;
        };
        if let Some(slot) = graph.beading_propagation.get_mut(to_v) {
            *slot = Some(from_beading);
        }
        // `upper_beading.dist_to_bottom_source += length` (canonical
        // `propagateBeadingsUpward`): the copy remembers how far it has
        // travelled from the node whose own bead count produced it, which is
        // what lets `propagate_beadings_downward` overwrite it once it is
        // farther than the transition distance from that source.
        let from_dist = graph.beading_dist_to_bottom_source[from_v];
        graph.beading_dist_to_bottom_source[to_v] = from_dist + edge_length(graph, edge_idx);
    }
}

/// Sizes [`SkeletalTrapezoidationGraph::beading_dist_to_bottom_source`] to
/// the vertex count; missing entries read as `0.0`, the value of a freshly
/// constructed canonical `BeadingPropagation`.
fn resize_dist_to_bottom_source(graph: &mut SkeletalTrapezoidationGraph) {
    let n = graph.vertices.len();
    if graph.beading_dist_to_bottom_source.len() != n {
        graph.beading_dist_to_bottom_source.resize(n, 0.0);
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
/// Element-wise interpolation between two [`Beading`]s, matching canonical
/// `SkeletalTrapezoidation::interpolate` (`SkeletalTrapezoidation.cpp:1976-1995`).
///
/// `ratio_left` is the weight for `left`; `right` gets `1.0 - ratio_left`.
/// Beads beyond `min(left.size, right.size)` are taken from the larger beading
/// (whichever had more `total_thickness`). Zero-width wall markers stay zero.
fn interpolate_beading(left: &Beading, ratio_left: f64, right: &Beading) -> Beading {
    let ratio_right = 1.0 - ratio_left;
    // Start from the larger beading (canonical `:1981`).
    let mut ret = if left.total_thickness > right.total_thickness {
        left.clone()
    } else {
        right.clone()
    };
    let n = left.bead_widths.len().min(right.bead_widths.len());
    for i in 0..n {
        if left.bead_widths[i] == 0.0 || right.bead_widths[i] == 0.0 {
            ret.bead_widths[i] = 0.0;
        } else {
            ret.bead_widths[i] =
                ratio_left * left.bead_widths[i] + ratio_right * right.bead_widths[i];
        }
        ret.toolpath_locations[i] =
            ratio_left * left.toolpath_locations[i] + ratio_right * right.toolpath_locations[i];
    }
    ret
}

/// Canonical four-argument `SkeletalTrapezoidation::interpolate(left,
/// ratio_left_to_whole, right, switching_radius)`: the element-wise blend of
/// [`interpolate_beading`] (which keeps the thicker beading's bead list), then,
/// when a bead of `left` that lies inside `switching_radius` has been pushed
/// outside it by the blend ("one inset disappeared"), a re-blend with the
/// ratio that puts that bead back on `switching_radius`, plus 0.1.
fn interpolate_beading_at_switching_radius(
    left: &Beading,
    ratio_left_to_whole: f64,
    right: &Beading,
    switching_radius: f64,
) -> Beading {
    let ret = interpolate_beading(left, ratio_left_to_whole, right);
    let Some(next_inset_idx) = left
        .toolpath_locations
        .iter()
        .rposition(|&location| switching_radius > location)
    else {
        // There is no next inset, because there is only one.
        return ret;
    };
    if next_inset_idx + 1 == left.toolpath_locations.len() {
        // Canonical: "We cant adjust to fit the next edge because there is
        // no previous one?!"
        return ret;
    }
    // `ret` follows the thicker of left/right and can hold fewer insets than
    // `left`; canonical skips the adjustment then. `right` must hold the
    // inset too, or canonical would read past its end.
    if next_inset_idx >= ret.toolpath_locations.len()
        || next_inset_idx >= right.toolpath_locations.len()
    {
        return ret;
    }
    if ret.toolpath_locations[next_inset_idx] > switching_radius {
        // One inset disappeared between left and the merged one; solve
        // f * l + (1 - f) * r = s for f.
        let l = left.toolpath_locations[next_inset_idx] as f32;
        let r = right.toolpath_locations[next_inset_idx] as f32;
        let new_ratio = (switching_radius as f32 - r) / (l - r);
        let new_ratio = (f64::from(new_ratio) + 0.1).min(1.0);
        return interpolate_beading(left, new_ratio, right);
    }
    ret
}

/// Populates the [`SkeletalTrapezoidationGraph`] `beading_propagation` side
/// table for every vertex that carries a `bead_count`, mirroring canonical
/// `SkeletalTrapezoidation.cpp:1700-1725`.
///
/// Finding #6: when `transition_ratio != 0.0`, canonical interpolates between
/// the beading for `bc` and `bc + 1` via `interpolate(low, 1.0 - tr, high)`
/// (`:1704-1715`). When `transition_ratio == 0.0`, the beading is computed
/// directly (current behavior).
pub fn populate_beading_propagation(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    if graph.beading_propagation.len() != graph.vertices.len() {
        graph.beading_propagation.resize(graph.vertices.len(), None);
    }
    let mut own_beadings = Vec::new();
    for (v_idx, v) in graph.vertices.iter().enumerate() {
        let Some(bc) = v.bead_count else {
            continue;
        };
        if bc == 0 {
            continue;
        }
        let thickness = 2.0 * v.distance_to_boundary;
        // Finding #6: branch on transition_ratio, matching canonical
        // `SkeletalTrapezoidation.cpp:1704-1715`.
        let beading = if v.transition_ratio == 0.0 {
            strategy.compute(thickness, bc as usize)
        } else {
            let low = strategy.compute(thickness, bc as usize);
            let high = strategy.compute(thickness, bc as usize + 1);
            interpolate_beading(&low, 1.0 - v.transition_ratio, &high)
        };
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
        own_beadings.push(v_idx);
    }
    // A beading computed from the node's own bead count is a fresh canonical
    // `BeadingPropagation`: `dist_to_bottom_source == 0`.
    resize_dist_to_bottom_source(graph);
    for v_idx in own_beadings {
        graph.beading_dist_to_bottom_source[v_idx] = 0.0;
    }
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
/// radius) along NON-central upward edges, blending via canonical
/// `interpolate()` when the lower node already carries a beading, using
/// `transition_dist` as the beading-propagation transition distance
/// (upstream's `beading_propagation_transition_dist`, in this crate's units).
///
/// Mirrors canonical `propagateBeadingsDownward` (`SkeletalTrapezoidation.cpp`).
/// Iterates the upward edges in forward order (descending R), skips central
/// ones as canonical does, and routes single-edge propagation from the peak
/// (`edge_to_peak->to`) down to the bottom (`edge_to_peak->from`).
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
/// `dist_to_bottom_source` is read from
/// [`SkeletalTrapezoidationGraph::beading_dist_to_bottom_source`], which
/// [`populate_beading_propagation`] (0 for own-bead-count beadings) and
/// [`propagate_beadings_upward`] (`+= length` per copy) fill exactly as
/// canonical fills the field on each `BeadingPropagation`. The two copy
/// branches here also copy the top's value, and the merge branch resets it,
/// as canonical's whole-object assignments do.
///
/// It used to be *recomputed* here by replaying the upward walk from a
/// "primary source" set defined as every vertex that is the `to` of some
/// edge. After the packet 141 centrality-gate removal that set held nearly
/// every vertex, so the replay recorded no distances, `ratio_of_top` was
/// always 0, and an upward-propagated thin beading was never overwritten by
/// the wider beading from above. On the benchy hull (layer 29, 3 walls) the
/// 3-bead beading of the 1.5 mm gap between the stern ring hole and the
/// transom climbed the whole stern spine: the third wall vanished there and
/// its ring closed across the hull as a straight chord.
pub fn propagate_beadings_downward_with_transition_dist(
    graph: &mut SkeletalTrapezoidationGraph,
    transition_dist: f64,
) {
    let transition_dist = if transition_dist.is_finite() && transition_dist > 0.0 {
        transition_dist
    } else {
        default_beading_propagation_transition_dist()
    };

    // Canonical keeps both distances on each node's `BeadingPropagation`;
    // this pass works on local copies and writes `dist_to_bottom_source`
    // back at the end. Before this pass every beading's
    // `dist_from_top_source` is 0 (populate and the upward copy never touch
    // it), so that map starts empty.
    resize_dist_to_bottom_source(graph);
    let mut dist_to_bottom_source: Vec<f64> = graph.beading_dist_to_bottom_source.clone();

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
        // Canonical `propagateBeadingsDownward(upward_quad_mids, ...)` only
        // transfers beadings down NON-central edges: both ends of a central
        // edge carry their own bead count (`updateBeadCount`), so nothing
        // flows along it.
        if edge.central {
            continue;
        }
        // For a single central edge, the peak is the `to` vertex (higher R),
        // the bottom is the `from` vertex (lower R).
        let peak_v = resolve_to_vertex(graph, edge_idx);
        let bottom_v = edge.start_vertex;
        if peak_v == NO_INDEX || bottom_v == NO_INDEX {
            continue;
        }
        // Canonical gates this pass on `hasBeading()`, never on `bead_count`
        // (`SkeletalTrapezoidation.cpp:1614-1624`: it takes
        // `getOrCreateBeading(edge_to_peak->to)` unconditionally). Gating on
        // `bead_count` here would skip every peak that only carries a
        // *propagated* beading — which, since `propagate_beadings_upward` now
        // correctly leaves such nodes at `bead_count == None` (canonical `-1`),
        // is exactly the set canonical still propagates through.
        if graph
            .beading_propagation
            .get(peak_v)
            .and_then(|slot| slot.as_ref())
            .is_none()
        {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        if !edge_len.is_finite() || edge_len <= 0.0 {
            continue;
        }

        let top_dist_from_source = dist_from_top_source.get(&peak_v).copied().unwrap_or(0.0);
        // Canonical's branch predicate is `!edge_to_peak->from->data.hasBeading()`
        // (`SkeletalTrapezoidation.cpp:1624`) — the side table, not `bead_count`.
        let bottom_has_beading = graph
            .beading_propagation
            .get(bottom_v)
            .and_then(|slot| slot.as_ref())
            .is_some();

        if !bottom_has_beading {
            // Fresh downward propagation (upstream's `!hasBeading()` branch at
            // `SkeletalTrapezoidation.cpp:1872-1876`): copy the top vertex's
            // `BeadingPropagation` into the bottom vertex's side-table slot and
            // extend the bookkeeping — but do NOT write `bead_count` or
            // `transition_ratio` on the joint. Canonical leaves the joint
            // `bead_count` at its default (-1 / `None` here) and only attaches
            // the beading weak_ptr via `setBeading`. Writing `bead_count` here
            // would flatten the gradient (boundary vertices would inherit the
            // peak's count) and trip `generate_junctions`'s same-bead-count
            // skip gate (`SkeletalTrapezoidation.cpp:2024-2027`), producing
            // empty output for shapes whose medial axis has no central edges
            // (e.g. a square at `wall_transition_angle=10°`). See
            // See the historical centrality-threshold correction in the deviation log.
            //
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
            // `propagated_beading = top_beading` copies BOTH distances;
            // only `dist_from_top_source` then grows by the edge length.
            dist_to_bottom_source[bottom_v] = dist_to_bottom_source[peak_v];
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
            continue;
        }

        // (`bottom_has_beading` is true here — canonical's `else` branch at
        // `SkeletalTrapezoidation.cpp:1636`, which reads the bottom's existing
        // beading from the side table rather than requiring a `bead_count`.)
        let bottom_dist_to_source = dist_to_bottom_source[bottom_v];
        let total_dist = top_dist_from_source + edge_len + bottom_dist_to_source;
        let denom = total_dist.min(transition_dist).max(f64::EPSILON);
        // Floor at 0 only -- NOT a symmetric clamp(0,1); the explicit
        // `>= 1.0` branch below handles the ceiling (see doc comment).
        let ratio_of_top = (bottom_dist_to_source / denom).max(0.0);

        if ratio_of_top >= 1.0 {
            // Full overwrite (upstream: `bottom_beading = top_beading;
            // bottom_beading.dist_from_top_source += length;` at lines
            // 1887-1889). Canonical mutates the `BeadingPropagation` side
            // table in-place via reference and does NOT write the joint's
            // `bead_count` or `transition_ratio` — see the `!hasBeading()`
            // branch comment above for why writing `bead_count` here would
            // flatten the gradient and break emission.
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
            dist_to_bottom_source[bottom_v] = dist_to_bottom_source[peak_v];
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
        } else {
            // Side-table write (the audit's "width/location blend" path):
            // when both endpoints already carry a stored beading, blend
            // them elementwise into the bottom vertex's slot. This is the
            // canonical OrcaSlicer `BeadingPropagation` merge shape
            // (`SkeletalTrapezoidation.cpp:1890-1894`); canonical does NOT
            // write the joint's `bead_count` here either — the merge lives
            // entirely in the side table. See the `!hasBeading()` branch
            // comment above for why writing `bead_count` would break
            // emission for non-central-medial-axis shapes.
            let bottom_r_for_merge = graph.vertices[bottom_v].distance_to_boundary;
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
                // Canonical `interpolate(top_beading.beading, ratio_of_top,
                // bottom_beading.beading, edge_to_peak->from->R)`: keeps the
                // thicker beading's bead list (the previous truncating blend
                // dropped every bead the thinner side lacked).
                *slot = Some(interpolate_beading_at_switching_radius(
                    top_beading,
                    ratio_of_top,
                    bottom_beading,
                    bottom_r_for_merge,
                ));
            }
            // Upstream: a merged beading is a fresh `BeadingPropagation`, so
            // BOTH of its distances reset to 0 here.
            dist_to_bottom_source[bottom_v] = 0.0;
            dist_from_top_source.remove(&bottom_v);
        }
    }
    graph.beading_dist_to_bottom_source = dist_to_bottom_source;
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
