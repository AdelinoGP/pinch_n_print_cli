// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`updateBeadCount`, `BeadingStrategy::getOptimalBeadCount` call site).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Bead-count assignment (T-221, packet 112 Step 2 of the M2 Arachne port).
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! Like [`super::centrality`], this module does not claim numeric parity
//! with OrcaSlicer. OrcaSlicer's `updateBeadCount`
//! (SkeletalTrapezoidation.cpp:777) reads a single scalar
//! `distance_to_boundary` per graph *node*
//! (`node.bead_count = getOptimalBeadCount(node.distance_to_boundary * 2)`),
//! then recomputes it at locally maximal `distance_to_boundary` nodes.
//! This crate mirrors that directly: bead counts are stored on
//! [`super::graph::STVertex::bead_count`], assigned from each central edge's
//! `to` vertex, and re-derived at local-radius-maximum vertices. This is a
//! faithful topology adaptation of the upstream algorithm, not a new
//! from-first-principles formula. `crates/slicer-core/tests/bead_count.rs`
//! locks in this implementation's own output as a self-captured regression
//! baseline — it is not, and must not be described as,
//! independently-derived OrcaSlicer ground truth.
//!
//! # AC-N1: distinguishing "no central edges" from "centrality never ran"
//!
//! A freshly built [`SkeletalTrapezoidationGraph`] has every edge's
//! [`central`](super::graph::STHalfEdge::central) field default to `false`
//! — indistinguishable, by inspecting `central` alone, from a graph that
//! genuinely has no central edges after
//! [`filter_central`](super::centrality::filter_central) ran.
//! [`SkeletalTrapezoidationGraph::centrality_filtered`] disambiguates:
//! `filter_central` sets it `true` on completion, and [`assign_bead_counts`]
//! refuses to run (returning [`BeadCountError::CentralityNotRun`]) until it
//! is `true`.

use std::error::Error;
use std::fmt;

use super::graph::SkeletalTrapezoidationGraph;
use crate::beading::BeadingStrategy;
use crate::voronoi::NO_INDEX;

/// Errors from [`assign_bead_counts`].
#[derive(Debug, Clone, PartialEq)]
pub enum BeadCountError {
    /// [`filter_central`](super::centrality::filter_central) has not been
    /// run on this graph yet — see
    /// [`SkeletalTrapezoidationGraph::centrality_filtered`] and this
    /// module's AC-N1 doc section.
    CentralityNotRun,
    /// The graph's topology could not be resolved well enough to assign bead
    /// counts. Reserved for future stricter validation; [`assign_bead_counts`]
    /// does not currently return this variant (its per-edge walk is
    /// panic-free and total over any [`SkeletalTrapezoidationGraph`]
    /// produced by [`SkeletalTrapezoidationGraph::from_polygons`]).
    InvalidGraph(String),
}

impl fmt::Display for BeadCountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BeadCountError::CentralityNotRun => write!(
                f,
                "assign_bead_counts: filter_central has not been run on this graph yet \
                 (SkeletalTrapezoidationGraph::centrality_filtered is false)"
            ),
            BeadCountError::InvalidGraph(msg) => {
                write!(f, "assign_bead_counts: invalid graph: {msg}")
            }
        }
    }
}

impl Error for BeadCountError {}

/// Assigns [`STVertex::bead_count`](super::graph::STVertex::bead_count)
/// to every vertex of `graph` reached by a central edge, in place.
///
/// For each central edge: the `to` vertex receives
/// `bead_count = Some(strategy.optimal_bead_count(2.0 * to.distance_to_boundary) as u32)`
/// — directly mirroring OrcaSlicer's `updateBeadCount`
/// (`SkeletalTrapezoidation.cpp:777`). After the edge pass, every vertex
/// that is a local maximum in `distance_to_boundary` (all incident central
/// edges have a strictly smaller `r_max`) has its `bead_count` recomputed
/// from its own `distance_to_boundary * 2`, exactly as upstream re-derives it
/// for `node.isLocalMaximum()` nodes.
///
/// Every non-central-adjacent vertex's `bead_count` is left as `None`,
/// while every central edge's `to` vertex is guaranteed to carry a
/// `Some(_)`. This makes the function idempotent regardless of what was
/// already on the graph.
///
/// Returns [`BeadCountError::CentralityNotRun`] if
/// [`SkeletalTrapezoidationGraph::centrality_filtered`] is `false` (AC-N1) —
/// see this module's doc comment. Deterministic (same graph + same strategy
/// state ⇒ same bead counts) and panic-free.
pub fn assign_bead_counts(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> Result<(), BeadCountError> {
    if !graph.centrality_filtered {
        return Err(BeadCountError::CentralityNotRun);
    }

    // Reset all vertex bead counts so the pass is idempotent.
    for vertex in graph.vertices.iter_mut() {
        vertex.bead_count = None;
    }

    // Primary pass (OrcaSlicer SkeletalTrapezoidation.cpp:777-786): for each
    // central edge, assign the bead count at the edge's `to` vertex from
    // that vertex's own distance_to_boundary.
    for edge in graph.edges.iter() {
        if !edge.central {
            continue;
        }
        let to_idx = if edge.twin == NO_INDEX {
            NO_INDEX
        } else {
            graph
                .edges
                .get(edge.twin)
                .map(|twin| twin.start_vertex)
                .unwrap_or(NO_INDEX)
        };
        if to_idx == NO_INDEX {
            continue;
        }
        if let Some(to_vertex) = graph.vertices.get_mut(to_idx) {
            let n = strategy.optimal_bead_count(2.0 * to_vertex.distance_to_boundary);
            to_vertex.bead_count = Some(n as u32);
        }
    }

    // Recompute at local-maximum radius vertices (OrcaSlicer
    // SkeletalTrapezoidation.cpp:786-802): a vertex is a local maximum when it
    // is incident to at least one central edge and every central edge incident
    // to it has a strictly smaller radius at the *other* endpoint.
    for v_idx in 0..graph.vertices.len() {
        let Some(vertex) = graph.vertices.get(v_idx) else {
            continue;
        };
        let mut touches_central = false;
        let is_local_max = graph.edges.iter().all(|e| {
            if !e.central {
                return true;
            }
            let touches = e.start_vertex == v_idx || edge_ends_at(graph, e, v_idx);
            if !touches {
                return true;
            }
            touches_central = true;
            let other_r = other_endpoint_distance(graph, e, v_idx);
            other_r < vertex.distance_to_boundary
        });
        if !touches_central || !is_local_max {
            continue;
        }
        if let Some(vertex) = graph.vertices.get_mut(v_idx) {
            let n = strategy.optimal_bead_count(2.0 * vertex.distance_to_boundary);
            vertex.bead_count = Some(n as u32);
        }
    }

    Ok(())
}

/// Does edge `e` end at vertex `v_idx`?
fn edge_ends_at(
    graph: &SkeletalTrapezoidationGraph,
    e: &super::graph::STHalfEdge,
    v_idx: usize,
) -> bool {
    if e.twin == NO_INDEX {
        return false;
    }
    graph
        .edges
        .get(e.twin)
        .map(|twin| twin.start_vertex == v_idx)
        .unwrap_or(false)
}

/// Returns the `distance_to_boundary` at the endpoint of `e` that is *not*
/// `v_idx`, or `0.0` if it cannot be resolved.
fn other_endpoint_distance(
    graph: &SkeletalTrapezoidationGraph,
    e: &super::graph::STHalfEdge,
    v_idx: usize,
) -> f64 {
    let start_r = graph
        .vertices
        .get(e.start_vertex)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(0.0);
    let to_r = if e.twin == NO_INDEX {
        0.0
    } else {
        graph
            .edges
            .get(e.twin)
            .and_then(|twin| graph.vertices.get(twin.start_vertex))
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0)
    };
    if e.start_vertex == v_idx {
        to_r
    } else {
        start_r
    }
}
