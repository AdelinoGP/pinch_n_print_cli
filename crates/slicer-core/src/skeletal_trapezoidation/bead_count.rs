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
//! with OrcaSlicer. OrcaSlicer's `updateBeadCount` reads a single scalar
//! `distance_to_boundary` per graph *node*
//! (`node.bead_count = getOptimalBeadCount(node.distance_to_boundary * 2)`);
//! this crate's graph instead stores `r_min`/`r_max` per **edge** (see
//! [`super::graph::STHalfEdge`]'s doc comment on why this simplified
//! topology has no single-scalar-per-node equivalent). `r_avg = (r_min +
//! r_max) / 2.0` below is a from-first-principles adaptation of the same
//! formula to that shape, not a literal port. `crates/slicer-core/tests/
//! bead_count.rs` locks in this implementation's own output as a
//! self-captured regression baseline — it is not, and must not be
//! described as, independently-derived OrcaSlicer ground truth.
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

/// Assigns [`STHalfEdge::bead_count`](super::graph::STHalfEdge::bead_count)
/// to every edge of `graph`, in place.
///
/// For each edge with `central == true`: `r_avg = (r_min + r_max) / 2.0`,
/// then `bead_count = Some(strategy.optimal_bead_count(2.0 * r_avg) as u32)`
/// — mirroring OrcaSlicer's `getOptimalBeadCount(node.distance_to_boundary *
/// 2)` call site (see this module's doc comment on the `r_avg` adaptation).
/// Every non-central edge's `bead_count` is (re)set to `None`, which makes
/// this function idempotent regardless of what was already on the graph.
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

    for edge in graph.edges.iter_mut() {
        if !edge.central {
            edge.bead_count = None;
            continue;
        }
        let r_avg = (edge.r_min + edge.r_max) / 2.0;
        let n = strategy.optimal_bead_count(2.0 * r_avg);
        edge.bead_count = Some(n as u32);
    }

    Ok(())
}
