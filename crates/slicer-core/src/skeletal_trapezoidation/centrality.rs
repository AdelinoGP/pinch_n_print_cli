// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`updateIsCentral`, `filterCentral` entry + recursive overloads,
// `isEndOfCentral`, `STHalfEdgeNode::isLocalMaximum`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Centrality filtering (T-220, packet 112 Step 1 of the M2 Arachne port).
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! This module does **not** claim numeric parity with OrcaSlicer. OrcaSlicer's
//! `SkeletalTrapezoidationGraph` is built with a substantially richer topology
//! than ours (explicit "rib" edges from `graph.makeRib()`, synthetic
//! `EXTRA_VD` edges, point-vs-segment cell bookkeeping) — see
//! [`crate::skeletal_trapezoidation::graph`]'s own design notes: our graph is
//! a direct 1:1 wrap of the raw (unclipped) `boostvoronoi` segment diagram,
//! including exterior half-edges (rays to infinity) and degenerate
//! zero-length primary edges at input-segment endpoints that OrcaSlicer's
//! richer construction never produces in the first place. A literal
//! byte-for-byte port of `updateIsCentral`'s `dR < dD * sin(angle/2)` test
//! onto *this* topology is not well-defined (our single-hop corner→spine
//! edges play a different structural role than OrcaSlicer's chain of quad
//! ribs). What follows is a **from-first-principles predicate**, inspired by
//! OrcaSlicer's two real (documented) mechanisms, adapted to fit the fields
//! this graph actually has:
//!
//! 1. **Depth floor** (mirrors `updateIsCentral`'s
//!    `max(from.R, to.R) < outer_edge_filter_length → non-central` rule): an
//!    edge whose deeper endpoint never reaches `min_central_distance` from
//!    the boundary is never central — it's a shallow rib, not skeleton.
//! 2. **Whisker dissolve** (the *fixed*, actually-invoked form of
//!    `filterCentral`'s recursive overload — see below): a chain of central
//!    edges that dead-ends within `transition_filter_dist` of accumulated
//!    length, and whose terminal vertex is not a local distance-to-boundary
//!    maximum, gets dissolved back to non-central. This mirrors the
//!    recursive `filterCentral(edge_t*, traveled_dist, max_length)` overload
//!    verbatim; only the outer seeding condition changes.
//!
//! **Research flag inherited from the OrcaSlicer source**: the public
//! `filterCentral(coord_t max_length)` entry overload in OrcaSlicer guards
//! the recursive call with `edge.to->isLocalMaximum() &&
//! !edge.to->isLocalMaximum()` — always false, so upstream's recursive
//! whisker-dissolve pass is dead code and never runs in practice (upstream
//! ships relying solely on `filterNoncentralRegions` for the equivalent
//! cleanup). Per this packet's brief, `filter_central` here implements the
//! *intended* behavior (seed from genuine whisker tips via [`is_end_of_central`],
//! recurse for real) rather than porting the dead guard.
//!
//! The three regression fixtures in `tests/centrality.rs` are **self-captured
//! baselines**: they lock in this implementation's own output for regression
//! purposes. They are not derived from, and must not be described as,
//! OrcaSlicer ground truth.

use std::collections::HashSet;

use super::graph::SkeletalTrapezoidationGraph;
use crate::voronoi::NO_INDEX;

/// Tolerance for floating-point boundary-distance / length comparisons.
/// Vertex positions and `distance_to_boundary` are `f64`s derived from
/// `boostvoronoi`'s numerics (see [`crate::voronoi::Vertex`]'s doc comment),
/// so exact equality is not meaningful; this keeps comparisons robust to
/// last-bit noise without being loose enough to matter at slicer-unit scale
/// (1 unit = 100 nm).
const EPS: f64 = 1e-6;

/// Parameters governing [`filter_central`]'s two-stage centrality predicate.
///
/// Both fields are in the same scaled-integer unit space as the rest of the
/// slicer (1 unit = 100 nm = 10⁻⁴ mm, see `docs/08_coordinate_system.md`),
/// even though they're stored as `f64` to match [`super::graph::STVertex`]'s
/// `distance_to_boundary` representation.
///
/// These are placeholder defaults for this packet; a later packet is
/// expected to derive both from `BeadingFactoryParams` (analogous to
/// OrcaSlicer's `beading_strategy.getTransitionThickness(0)` and
/// `central_filter_dist()` respectively) rather than using fixed constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentralityParams {
    /// Maximum accumulated whisker length (see [`filter_central`]'s
    /// module-level doc) a chain of central edges may span and still be
    /// dissolved to non-central, provided none of its terminal vertices is a
    /// local distance-to-boundary maximum. Mirrors OrcaSlicer's
    /// `central_filter_dist()` (hardcoded `scaled<coord_t>(0.02)` = 0.02 mm
    /// there; 0.02 mm = 200 units here).
    pub transition_filter_dist: f64,
    /// Minimum `distance_to_boundary` an edge's deeper endpoint must reach
    /// to ever be considered central. Mirrors OrcaSlicer's
    /// `outer_edge_filter_length = beading_strategy.getTransitionThickness(0) / 2`.
    /// Defaults to `0.0` (no floor) until a later packet wires this to
    /// `BeadingFactoryParams`.
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
/// See the module-level doc comment for the two-stage predicate this
/// implements and why it is a from-first-principles adaptation rather than
/// a literal OrcaSlicer port.
pub fn filter_central(graph: &mut SkeletalTrapezoidationGraph, params: &CentralityParams) {
    // --- Stage 1: depth floor (mirrors `updateIsCentral`'s R-threshold rule) ---
    // `r_max` is the deeper of an edge's two endpoints (see
    // `STHalfEdge::r_max`'s doc comment); an edge whose deepest point never
    // reaches `min_central_distance` is never central. Twin symmetry holds
    // automatically because `r_min`/`r_max` are computed order-independently
    // per edge/twin pair in `graph.rs`'s `edge_radius_bounds`.
    for edge in graph.edges.iter_mut() {
        edge.central = edge.r_max >= params.min_central_distance;
    }

    // --- Stage 2: whisker dissolve (the real, non-dead `filterCentral`) ---
    // Seed from every genuine whisker tip (`is_end_of_central`) and recurse
    // backward into the chain, exactly mirroring OrcaSlicer's
    // `filterCentral(edge_t*, traveled_dist, max_length)` recursive overload.
    let edge_count = graph.edges.len();
    for edge_idx in 0..edge_count {
        if !is_end_of_central(graph, edge_idx) {
            continue;
        }
        let Some(twin) = graph.edges.get(edge_idx).map(|e| e.twin) else {
            continue;
        };
        if twin == NO_INDEX {
            debug_assert!(false, "central edge {edge_idx} has no twin");
            continue;
        }
        let mut visited = HashSet::new();
        try_dissolve(
            graph,
            twin,
            0.0,
            params.transition_filter_dist,
            &mut visited,
        );
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

/// Euclidean length of a half-edge, or `f64::INFINITY` if either endpoint is
/// unresolvable (an unbounded ray has no finite length). Mirrors OrcaSlicer's
/// `(from->p - to->p).cast<int64_t>().norm()`, but computed directly on the
/// `f64` Voronoi vertex positions this graph stores (see
/// [`crate::voronoi::Vertex`]) rather than rounding through integer
/// coordinates.
fn edge_length(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> f64 {
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
fn is_local_maximum(graph: &SkeletalTrapezoidationGraph, vertex_idx: usize) -> bool {
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

    let Some(edge) = graph.edges.get(edge_idx).copied() else {
        return false;
    };
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
