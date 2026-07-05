// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Arachne/
// SkeletalTrapezoidationGraph.cpp:452 (`makeRib`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Rib edge classification types for the skeletal trapezoidation graph.
//!
//! # Packet 113c supersession
//!
//! Earlier (packet 113b) this module owned `build_quad_rib_topology`, a
//! *separate* pass that inserted rib (`EXTRA_VD`) edges **only at reflex/sharp
//! polygon corners** where a Voronoi vertex sat exactly on the boundary. That
//! assumption was wrong: OrcaSlicer's real `makeRib` runs **unconditionally
//! after every transferred edge** during graph construction, interleaving a
//! rib pair between every spine sub-segment. A square (no sharp corners)
//! therefore has *many* ribs, not zero — the previous corner-only pass left
//! almost every spine edge un-ribbed, breaking `getNextUnconnected`-style
//! domain traversal at every junction (`D-112-MMU-TOPOLOGY`,
//! `D-113B-CONNECTJUNCTIONS`).
//!
//! Packet 113c moves faithful rib insertion **into**
//! [`super::graph::SkeletalTrapezoidationGraph::from_polygons`] (the per-cell
//! `transferEdge`/`makeRib` port), so this module no longer performs a separate
//! rib pass. What remains here are the small shared type shapes still consumed
//! across the crate:
//!
//! - [`EdgeType`] — edge classification (`NORMAL` / `EXTRA_VD` / `TRANSITION_END`),
//!   set directly by `from_polygons` (ribs → `EXTRA_VD`) and read by
//!   centrality / bead-count / toolpath passes.
//! - [`RibData`] — the (now empty) rib-topology payload field on the graph,
//!   retained so existing graph constructors and hand-built test fixtures keep
//!   compiling; ribs now live directly in `graph.edges`.
//! - [`build_quad_rib_topology`] — retained as a **vestigial no-op** so its
//!   existing call sites (`arachne::pipeline`) keep compiling until Step 4
//!   removes them; ribs are already present after `from_polygons`, so there is
//!   nothing left for it to do.

use crate::skeletal_trapezoidation::graph::SkeletalTrapezoidationGraph;

/// Edge classification used by Arachne's later passes.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationEdge::type` enum. Variant
/// names intentionally preserve upstream spelling (`EXTRA_VD`,
/// `TRANSITION_END`) for cross-reference clarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum EdgeType {
    /// A normal skeleton edge (Voronoi bisector).
    #[default]
    NORMAL,
    /// A synthetic rib edge inserted by `makeRib` (`EXTRA_VD` upstream).
    EXTRA_VD,
    /// A transition-region terminal edge. Declared so the `EdgeType` enum shape
    /// is stable for downstream passes.
    TRANSITION_END,
}

/// Rib-topology data owned by [`SkeletalTrapezoidationGraph`].
///
/// Empty since packet 113c: rib (`EXTRA_VD`) edges are now built directly into
/// [`SkeletalTrapezoidationGraph::edges`] by
/// [`super::graph::SkeletalTrapezoidationGraph::from_polygons`], so there is no
/// separate quad-cell side table to carry. Retained as a graph field (and
/// re-exported) purely so existing constructors and hand-built test fixtures
/// that write `rib: RibData::default()` keep compiling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RibData {}

/// Errors from [`build_quad_rib_topology`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RibError {
    /// The underlying Voronoi diagram has not been built yet (empty graph).
    VoronoiNotBuilt,
}

impl std::fmt::Display for RibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RibError::VoronoiNotBuilt => {
                write!(f, "build_quad_rib_topology: Voronoi diagram not built")
            }
        }
    }
}

impl std::error::Error for RibError {}

/// Vestigial no-op retained for source compatibility (packet 113c).
///
/// Rib (`EXTRA_VD`) edges are now inserted by
/// [`super::graph::SkeletalTrapezoidationGraph::from_polygons`] itself (the
/// faithful per-cell `transferEdge`/`makeRib` port), so there is nothing for a
/// separate pass to do. This function only validates that the graph is
/// non-empty and returns `Ok(())`; it makes no topology changes. Its remaining
/// call sites (`arachne::pipeline::run_arachne_pipeline`) are slated for removal
/// in Step 4 of packet 113c; the shim exists so the crate keeps compiling in
/// the meantime.
pub fn build_quad_rib_topology(graph: &mut SkeletalTrapezoidationGraph) -> Result<(), RibError> {
    if graph.vertices.is_empty() || graph.edges.is_empty() {
        return Err(RibError::VoronoiNotBuilt);
    }
    Ok(())
}
