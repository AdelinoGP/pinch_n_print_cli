// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path:
// OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672
// (`updateIsCentral`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Centrality filtering (packet 113b Step 2 — Arachne topology faithfulness).
//!
//! Implements the `updateIsCentral` predicate from
//! `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672`:
//!
//! > `bool is_central = dR < dD * sin(transitioning_angle / 2);`
//!
//! where `dR = |to.R - from.R|`, `dD = (to.p - from.p).cast<int64_t>().norm()`,
//! and `transitioning_angle` is the configured wall-transition angle in
//! radians (`beading_strategy.getTransitioningAngle()` upstream). Edges with
//! `max(from.R, to.R) < outer_edge_filter_length` are forced non-central, and
//! `EXTRA_VD` (synthetic rib) edges are always non-central. Twin edges share the
//! same centrality marker.
//!
//! # Note on topology differences
//!
//! This crate's `SkeletalTrapezoidationGraph` is a direct 1:1 wrap of the raw
//! `boostvoronoi` segment diagram (see [`crate::skeletal_trapezoidation::graph`]
//! for the design rationale), so it includes exterior half-edges (rays to
//! infinity) and degenerate zero-length primary edges that OrcaSlicer's richer
//! graph construction never produces. The predicate below applies the documented
//! OrcaSlicer rule to every half-edge that has resolvable endpoints; unbounded
//! edges and `EXTRA_VD` ribs are handled as non-central by construction.
//!
//! # Fixtures
//!
//! The three regression fixtures in `tests/centrality.rs` are **self-captured
//! baselines** that lock in this implementation's own output for regression
//! purposes. They are not derived from, and must not be described as,
//! OrcaSlicer ground truth.

use std::collections::HashSet;

use super::graph::SkeletalTrapezoidationGraph;
use super::rib::EdgeType;
use crate::voronoi::NO_INDEX;

/// Tolerance for floating-point boundary-distance / length comparisons.
/// Vertex positions and `distance_to_boundary` are `f64`s derived from
/// `boostvoronoi`'s numerics (see [`crate::voronoi::Vertex`]'s doc comment),
/// so exact equality is not meaningful; this keeps comparisons robust to
/// last-bit noise without being loose enough to matter at slicer-unit scale
/// (1 unit = 100 nm).
const EPS: f64 = 1e-6;

/// Parameters governing [`filter_central`]'s centrality predicate.
///
/// All length fields are in the same scaled-integer unit space as the rest of
/// the slicer (1 unit = 100 nm = 10⁻⁴ mm, see `docs/08_coordinate_system.md`),
/// even though they're stored as `f64` to match [`super::graph::STVertex`]'s
/// `distance_to_boundary` representation. `transitioning_angle_rad` is in
/// radians and feeds the `sin(angle/2)` term directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentralityParams {
    /// `outer_edge_filter_length` proxy: an edge whose deepest endpoint has
    /// `distance_to_boundary` below this threshold is forced non-central.
    /// Mirrors OrcaSlicer's
    /// `outer_edge_filter_length = beading_strategy.getTransitionThickness(0) / 2`.
    /// Upstream this is a derived beading-strategy value; here it is an
    /// explicit parameter so `filter_central` stays strategy-agnostic.
    pub transition_filter_dist: f64,
    /// Minimum `distance_to_boundary` an edge's deeper endpoint must reach
    /// to ever be considered central. Kept for backward compatibility with
    /// existing tests; in the upstream predicate this is folded into the
    /// `outer_edge_filter_length` rule above.
    pub min_central_distance: f64,
}

impl Default for CentralityParams {
    fn default() -> Self {
        Self {
            transition_filter_dist: 200.0,
            min_central_distance: 0.0,
        }
    }
}

impl CentralityParams {
    /// Constructs explicit params, bypassing [`Default`].
    pub fn new(transition_filter_dist: f64, min_central_distance: f64) -> Self {
        Self {
            transition_filter_dist,
            min_central_distance,
        }
    }
}

/// Sets [`super::graph::STHalfEdge::central`] on every edge of `graph`,
/// in place. Deterministic (same graph in ⇒ same markers out) and
/// panic-free: malformed topology (an out-of-range index that should not
/// occur given [`SkeletalTrapezoidationGraph::from_polygons`]'s own
/// invariants) degrades to "not resolvable" rather than indexing out of
/// bounds.
///
/// Implements the `updateIsCentral` predicate from
/// `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672`:
/// `central = dR < dD * sin(transitioning_angle/2)`, subject to the
/// `EXTRA_VD` / `outer_edge_filter_length` overrides described in the
/// module-level doc.
pub fn filter_central(
    graph: &mut SkeletalTrapezoidationGraph,
    params: &CentralityParams,
    transitioning_angle_rad: f64,
) {
    // `updateIsCentral` computes one value per edge pair: if a twin exists,
    // the second half-edge mirrors the first half-edge's result. We process
    // every half-edge but only resolve endpoints once per edge.
    let cap = (transitioning_angle_rad / 2.0).sin();

    for edge_idx in 0..graph.edges.len() {
        let Some(edge) = graph.edges.get(edge_idx) else {
            continue;
        };

        // EXTRA_VD ribs are always non-central.
        if edge.edge_type == EdgeType::EXTRA_VD {
            graph.edges[edge_idx].central = false;
            continue;
        }

        let Some(from_v) = graph.vertices.get(edge.start_vertex) else {
            graph.edges[edge_idx].central = false;
            continue;
        };
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let to_d = if to_idx == NO_INDEX {
            0.0
        } else {
            graph
                .vertices
                .get(to_idx)
                .map(|v| v.distance_to_boundary)
                .unwrap_or(0.0)
        };
        let from_d = from_v.distance_to_boundary;

        let d_r = (to_d - from_d).abs();
        let d_d = edge_length(graph, edge_idx);

        // Edges with at least one endpoint on the boundary and max R below the
        // outer-edge filter are forced non-central. For unbounded rays (to_idx
        // == NO_INDEX) we use from_d as the only resolvable radius.
        let r_max = if to_idx == NO_INDEX {
            from_d
        } else {
            from_d.max(to_d)
        };
        let outer_filter = params.transition_filter_dist;
        let passes_outer_filter = r_max >= outer_filter;

        // A finite edge is central when the radius swing across it is smaller
        // than the geometric length times sin(angle/2). Infinite-length or
        // unresolvable edges default to non-central.
        let passes_predicate =
            d_d.is_finite() && d_d > EPS && passes_outer_filter && d_r < d_d * cap;

        // Also honor the legacy min_central_distance floor if it is stricter
        // than the outer-filter proxy.
        let passes_floor = r_max >= params.min_central_distance;

        graph.edges[edge_idx].central = passes_predicate && passes_floor;
    }

    // Twin symmetry: the second half-edge of a pair mirrors the first. Orca
    // does this implicitly by sharing the edge-pair data structure; here we
    // copy the value from the lower-indexed half-edge to its twin.
    for edge_idx in 0..graph.edges.len() {
        let Some(edge) = graph.edges.get(edge_idx) else {
            continue;
        };
        let edge_twin = edge.twin;
        if edge_twin == NO_INDEX || edge_twin <= edge_idx {
            continue;
        }
        let twin_value = graph.edges[edge_idx].central;
        if let Some(twin) = graph.edges.get_mut(edge_twin) {
            twin.central = twin_value;
        }
    }

    // Marks that centrality has been computed on this graph, so
    // `assign_bead_counts` (P112 Step 2) can tell "genuinely no central
    // edges" apart from "centrality was never run" (AC-N1) — see
    // `SkeletalTrapezoidationGraph::centrality_filtered`'s doc comment.
    graph.centrality_filtered = true;
}

/// Resolves a half-edge's "to" vertex index via its twin's `start_vertex`,
/// matching [`super::graph`]'s own convention (see that module's doc comment
/// on why the "to" vertex isn't stored directly). Returns [`NO_INDEX`] if
/// unresolvable (missing/out-of-range twin).
pub(super) fn resolve_to_vertex(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> usize {
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

/// Euclidean length of a half-edge, or `f64::INFINITY` if either endpoint is
/// unresolvable (an unbounded ray has no finite length). Mirrors OrcaSlicer's
/// `(from->p - to->p).cast<int64_t>().norm()`, but computed directly on the
/// `f64` Voronoi vertex positions this graph stores (see
/// [`crate::voronoi::Vertex`]) rather than rounding through integer
/// coordinates.
pub(super) fn edge_length(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> f64 {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return f64::INFINITY;
    };
    if edge.start_vertex == NO_INDEX {
        return f64::INFINITY;
    }
    let to_v = resolve_to_vertex(graph, edge_idx);
    if to_v == NO_INDEX {
        return f64::INFINITY;
    }
    let (Some(from), Some(to)) = (
        graph.vertices.get(edge.start_vertex),
        graph.vertices.get(to_v),
    ) else {
        return f64::INFINITY;
    };
    let dx = to.position.x - from.position.x;
    let dy = to.position.y - from.position.y;
    (dx * dx + dy * dy).sqrt()
}

/// Whether `vertex_idx` is a local maximum of `distance_to_boundary`: no
/// edge starting there reaches a strictly higher-R neighbor. Mirrors
/// `STHalfEdgeNode::isLocalMaximum` verbatim, including its fast path
/// (a vertex sitting exactly on the boundary, `distance_to_boundary == 0`,
/// is never a local maximum).
///
/// Enumerates "edges starting at `vertex_idx`" via a linear scan rather than
/// OrcaSlicer's `edge->twin->next` cell-walk: for a graph satisfying the
/// twin-involution and next/prev-consistency invariants this crate's own
/// golden tests already assert (`skt_graph_golden.rs`), both enumerate the
/// identical *set* of outgoing half-edges — order doesn't matter here since
/// every outgoing edge is inspected regardless of order.
// The recursive whisker-dissolve helpers below are no longer used by the
// current `updateIsCentral` implementation, but they are retained so a future
// packet that wires in the real `filterCentral`/`filterOuterCentral` passes
// can reuse the existing helpers without re-porting them.
//
// `is_local_maximum` is an exception: it is the canonical-faithful all-edges
// local-maximum predicate (no centrality gate) shared with
// [`super::bead_count::assign_bead_counts`]'s second loop, matching OrcaSlicer's
// `STHalfEdgeNode::isLocalMaximum` (`SkeletalTrapezoidationGraph.cpp:254-274`)
// which `updateBeadCount` (`SkeletalTrapezoidation.cpp:786-801`) calls without
// any `isCentral()` check.
pub(super) fn is_local_maximum(graph: &SkeletalTrapezoidationGraph, vertex_idx: usize) -> bool {
    if vertex_idx == NO_INDEX {
        return false;
    }
    let Some(vertex) = graph.vertices.get(vertex_idx) else {
        return false;
    };
    if vertex.distance_to_boundary <= EPS {
        return false;
    }
    for (idx, edge) in graph.edges.iter().enumerate() {
        if edge.start_vertex != vertex_idx {
            continue;
        }
        let to_v = resolve_to_vertex(graph, idx);
        if to_v == NO_INDEX {
            continue; // Unbounded ray: no finite R to compare against.
        }
        let Some(to_vertex) = graph.vertices.get(to_v) else {
            continue;
        };
        if to_vertex.distance_to_boundary > vertex.distance_to_boundary + EPS {
            return false;
        }
    }
    true
}

/// Whether `edge_idx` is central and terminates a whisker: its "to" vertex
/// has no other central edge starting there (besides `edge_idx`'s own
/// twin). Mirrors `SkeletalTrapezoidation::isEndOfCentral` verbatim,
/// including the "no next" boundary case, which here falls out naturally
/// (an unresolvable "to" vertex vacuously has no other outgoing edges to
/// find, so it's trivially an end).
#[allow(dead_code)]
fn is_end_of_central(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> bool {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return false;
    };
    if !edge.central {
        return false;
    }
    let to_v = resolve_to_vertex(graph, edge_idx);
    if to_v == NO_INDEX {
        return true;
    }
    for (idx, other) in graph.edges.iter().enumerate() {
        if idx == edge.twin {
            continue;
        }
        if other.start_vertex == to_v && other.central {
            return false;
        }
    }
    true
}

/// Recursive whisker-dissolve step, mirroring OrcaSlicer's recursive
/// `filterCentral(edge_t* starting_edge, coord_t traveled_dist, coord_t
/// max_length)` overload verbatim. Returns whether `edge_idx` (and
/// everything reachable from it within budget) was dissolved to
/// non-central.
///
/// `visited` is a cycle guard not present in the OrcaSlicer source (that
/// code relies on `traveled_dist` strictly increasing along a tree-shaped
/// central subgraph to terminate); it's added defensively here so a
/// pathological zero-length-edge cycle degrades to "don't dissolve" rather
/// than an unbounded recursion.
#[allow(dead_code)]
fn try_dissolve(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_idx: usize,
    traveled_dist: f64,
    max_length: f64,
    visited: &mut HashSet<usize>,
) -> bool {
    if !visited.insert(edge_idx) {
        return false;
    }

    let length = edge_length(graph, edge_idx);
    if !length.is_finite() || length <= EPS || traveled_dist + length > max_length {
        return false;
    }

    let Some(edge_ref) = graph.edges.get(edge_idx) else {
        return false;
    };
    let edge = edge_ref.clone();
    let to_v = resolve_to_vertex(graph, edge_idx);

    let mut should_dissolve = true;
    if to_v == NO_INDEX {
        // An unbounded ray can't be part of a dissolvable whisker (its own
        // length is already `INFINITY`, handled above); reachable only if
        // topology is malformed, so keep the edge as-is defensively.
        should_dissolve = false;
    } else {
        for idx in 0..graph.edges.len() {
            if idx == edge.twin {
                continue;
            }
            let is_candidate =
                graph.edges.get(idx).map(|e| (e.start_vertex, e.central)) == Some((to_v, true));
            if is_candidate {
                let ok = try_dissolve(graph, idx, traveled_dist + length, max_length, visited);
                should_dissolve &= ok;
            }
        }
        if is_local_maximum(graph, to_v) {
            should_dissolve = false;
        }
    }

    if should_dissolve {
        if let Some(e) = graph.edges.get_mut(edge_idx) {
            e.central = false;
        }
        if edge.twin != NO_INDEX {
            if let Some(t) = graph.edges.get_mut(edge.twin) {
                t.central = false;
            }
        }
    }
    should_dissolve
}

const NONCENTRAL_REGION_MAX_DIST: f64 = 4000.0;

fn dissolve_noncentral_gap(
    graph: &mut SkeletalTrapezoidationGraph,
    source_v: usize,
    edge_idx: usize,
    total_dist: f64,
) -> bool {
    if total_dist > NONCENTRAL_REGION_MAX_DIST {
        return false;
    }
    let to_v = resolve_to_vertex(graph, edge_idx);
    if to_v == NO_INDEX || to_v == source_v {
        return false;
    }
    let (src_bc, dst_bc) = {
        let src = graph.vertices[source_v].bead_count;
        let dst = graph.vertices[to_v].bead_count;
        (src, dst)
    };
    let src_bc = match src_bc {
        Some(bc) => bc,
        None => return false,
    };
    let should_dissolve = match dst_bc {
        Some(dst) if dst == src_bc => true,
        None => {
            graph.vertices[to_v].bead_count = Some(src_bc);
            true
        }
        Some(dst) if src_bc.abs_diff(dst) <= 1 => true,
        _ => false,
    };
    if !should_dissolve {
        return false;
    }
    if let Some(e) = graph.edges.get_mut(edge_idx) {
        e.central = true;
    }
    let twin = graph.edges[edge_idx].twin;
    if twin != NO_INDEX {
        if let Some(t) = graph.edges.get_mut(twin) {
            t.central = true;
        }
    }
    for next_idx in 0..graph.edges.len() {
        if next_idx == twin {
            continue;
        }
        if graph.edges[next_idx].start_vertex != to_v || graph.edges[next_idx].central {
            continue;
        }
        let next_twin = graph.edges[next_idx].twin;
        let next_to = if next_twin != NO_INDEX {
            graph.edges[next_twin].start_vertex
        } else {
            NO_INDEX
        };
        if next_to == source_v {
            continue;
        }
        let edge_len = graph.edges[next_idx].r_max - graph.edges[next_idx].r_min;
        if edge_len > 0.0 {
            dissolve_noncentral_gap(graph, to_v, next_idx, total_dist + edge_len);
        }
    }
    true
}

/// Promotes non-central gaps between same/±1-bead-count central regions back
/// to central, mirroring `SkeletalTrapezoidation::filterNoncentralRegions`
/// (`SkeletalTrapezoidation.cpp:811-862`). Called unconditionally after
/// `assign_bead_counts` in the pipeline, before transition machinery.
pub fn filter_noncentral_regions(graph: &mut SkeletalTrapezoidationGraph) {
    for edge_idx in 0..graph.edges.len() {
        if !is_end_of_central(graph, edge_idx) {
            continue;
        }
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v == NO_INDEX {
            continue;
        }
        let edge_len = graph.edges[edge_idx].r_max - graph.edges[edge_idx].r_min;
        dissolve_noncentral_gap(graph, to_v, edge_idx, edge_len);
    }
}
