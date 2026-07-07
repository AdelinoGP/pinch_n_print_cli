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
/// (`SkeletalTrapezoidation.cpp:777-783`). After the edge pass, every vertex
/// that is a geometric local maximum in `distance_to_boundary` (no outgoing
/// edge reaches a strictly higher-R neighbor) has its `bead_count` recomputed
/// from its own `distance_to_boundary * 2`, exactly as upstream re-derives it
/// for `node.isLocalMaximum()` nodes
/// (`SkeletalTrapezoidation.cpp:786-801`). Canonical's local-maximum pass has
/// NO centrality gate — a local maximum with zero incident central edges
/// (e.g. the center of a square whose Voronoi edges all fail the centrality
/// predicate) still receives a bead count.
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
    // `SkeletalTrapezoidation.cpp:786-802`). Canonical's second loop iterates
    // every node and assigns a bead count whenever `node.isLocalMaximum()` is
    // true — with NO centrality check (`isCentral` is never called in lines
    // 786-801). `isLocalMaximum` (`SkeletalTrapezoidationGraph.cpp:254-274`)
    // is purely geometric: a node is a local maximum when no outgoing edge
    // leads to a neighbor with strictly higher `distance_to_boundary`, and it
    // returns false for boundary nodes (`distance_to_boundary == 0`) and for
    // nodes whose `twin->next` is null (boundary-adjacent). Reusing
    // [`super::centrality::is_local_maximum`] keeps this port faithful: it
    // inspects every edge starting at the vertex (no centrality filter) and
    // rejects vertices whose neighbor has a strictly higher R.
    //
    // Prior to this fix the loop required `touches_central` (at least one
    // incident central edge) and inspected only central edges — a
    // non-canonical gate. For a square at `wall_transition_angle=10°` every
    // Voronoi edge is a radial spoke (`dR/dD ≈ 0.707..1.0`, all above
    // `sin(5°) ≈ 0.087`), so `filter_central` marks nothing central and the
    // square's center vertex (a true geometric local maximum at
    // `distance_to_boundary = 5mm`) was skipped — producing empty output.
    // The π hack masked this by making the diagonals central. See
    // `docs/DEVIATION_LOG.md` `D-144-ANGLE-FUDGE-NONCENTRAL`.
    for v_idx in 0..graph.vertices.len() {
        if !super::centrality::is_local_maximum(graph, v_idx) {
            continue;
        }
        if let Some(vertex) = graph.vertices.get_mut(v_idx) {
            let n = strategy.optimal_bead_count(2.0 * vertex.distance_to_boundary);
            vertex.bead_count = Some(n as u32);
        }
    }

    Ok(())
}
