// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Arachne/
// SkeletalTrapezoidationGraph.cpp:452 (`makeRib` / `transferEdge` rib insertion).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Synthetic quad/rib topology pass for the skeletal trapezoidation graph.       
//!                                                                               
//! OrcaSlicer's `SkeletalTrapezoidationGraph::makeRib()` inserts a pair of        
//! `EXTRA_VD` half-edges (a "rib") from a skeleton node to a new boundary node    
//! at the foot-of-perpendicular on a source polygon segment. The rib, the       
//! source segment, and the adjacent spine edges form a 4-vertex quadrilateral     
//! cell (a trapezoid). This pass records those cells and marks the rib edges as   
//! `EXTRA_VD` / non-central so later centrality, bead-count, transition, and    
//! junction-connecting passes can read the topology.                              
//!                                                                               
//! For Step 1 of packet 113b the implementation is intentionally minimal: we      
//! classify the existing Voronoi half-edges around polygon corners into a spine  
//! pair and a rib pair, and build a `QuadCell` of four edges / four vertices. No   
//! new vertices or edges are inserted into the graph; the synthetic `QuadCell`     
//! stores the makeRib-style grouping. Ribs are only created at reflex/sharp       
//! polygon corners where the Voronoi vertex sits exactly on the boundary         
//! (`distance_to_boundary == 0`). A square has no such corners, so it produces    
//! zero ribs (AC-N1).                                                            

use std::collections::BTreeMap;

use crate::skeletal_trapezoidation::graph::SkeletalTrapezoidationGraph;
use crate::voronoi::NO_INDEX;

/// Identifier for a quadrilateral (trapezoid) cell produced by `makeRib`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuadCellId(pub u32);

/// A single quad/trapezoid cell: four half-edges and four vertices.
///
/// The canonical OrcaSlicer makeRib cell is a trapezoid with:
/// - one spine edge on the skeleton side;
/// - a rib pair (`EXTRA_VD`) connecting the skeleton to the boundary;
/// - the source polygon segment as the fourth side.
///
/// Order is spine edge → rib forth → source segment → rib back, but callers
/// should not rely on the geometric meaning of each slot; the cell simply
/// records the four edges and vertices incident to the rib grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuadCell {
    /// The four half-edge indices forming the cell cycle.
    pub edges: [usize; 4],
    /// The four vertex indices at the cell corners.
    pub vertices: [usize; 4],
}

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
    /// A transition-region terminal edge. Reserved for Step 4; declared now so
    /// the `EdgeType` enum shape is stable for downstream passes.
    TRANSITION_END,
}

/// Rib-topology data owned by [`SkeletalTrapezoidationGraph`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RibData {
    /// All quad cells built by [`build_quad_rib_topology`], in allocation order.
    pub quad_cells: Vec<QuadCell>,
    /// Monotonic allocator for the next [`QuadCellId`].
    pub next_quad_cell_id: u32,
}

/// Errors from [`build_quad_rib_topology`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RibError {
    /// The underlying Voronoi diagram has not been built yet.
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

/// Builds the synthetic quad/rib topology on a freshly constructed skeletal
/// trapezoidation graph.
///
/// Runs after [`SkeletalTrapezoidationGraph::from_polygons`] and before
/// `filter_central` (the pipeline wiring is Step 5's responsibility; this
/// function is exported so Step 5 can call it).
///
/// # Determinism
///
/// Iterates vertices and edges in index order and uses only `BTreeMap` for any
/// keyed bookkeeping, so two runs with the same input produce identical rib
/// edges and quad cells.
pub fn build_quad_rib_topology(graph: &mut SkeletalTrapezoidationGraph) -> Result<(), RibError> {
    if graph.vertices.is_empty() || graph.edges.is_empty() {
        return Err(RibError::VoronoiNotBuilt);
    }

    // Step 1a: classify boundary-touching edges as ribs (EXTRA_VD) or spine.
    //
    // An edge is a rib candidate if at least one endpoint has
    // distance_to_boundary == 0. Rib pairs are emitted around polygon corners
    // where the Voronoi vertex itself lies on the boundary; the two edges
    // incident to that vertex that lead away from the boundary form the spine
    // pair, and the source segment (via its twin source-vertex association)
    // completes the quad cell.
    //
    // For Step 1 we keep the implementation minimal and deterministic: walk
    // vertices in index order, and for each boundary vertex build at most one
    // quad cell from its incident edge cycle.
    let mut quad_cells: Vec<QuadCell> = Vec::new();
    let mut used_for_quad = vec![false; graph.edges.len()];

    // Collect, per boundary vertex, the cycle of non-degenerate half-edges
    // starting at that vertex. Degenerate zero-length edges (where a half-edge
    // and its twin share the same start vertex) are filtered out: they are
    // boostvoronoi artifacts at segment endpoints and do not represent real
    // polygon corners that need a rib.
    let mut outgoing_by_vertex: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (edge_id, edge) in graph.edges.iter().enumerate() {
        if edge.start_vertex == NO_INDEX {
            continue;
        }
        if edge.twin != NO_INDEX {
            let twin_start = graph.edges[edge.twin].start_vertex;
            if twin_start == edge.start_vertex {
                // Zero-length degenerate edge; skip.
                continue;
            }
        }
        outgoing_by_vertex
            .entry(edge.start_vertex)
            .or_default()
            .push(edge_id);
    }

    for (vertex_id, _) in graph.vertices.iter().enumerate() {
        if graph.vertices[vertex_id].distance_to_boundary != 0.0 {
            continue;
        }

        let outgoing = match outgoing_by_vertex.get(&vertex_id) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };

        // A square has no sharp/reflex corners where the Voronoi vertex sits
        // exactly on the boundary, so for a square this loop never creates a
        // quad cell. (AC-N1)
        //
        // A rib corner needs at least two distinct geometric edges that leave
        // the boundary and reach skeleton vertices (distance_to_boundary > 0).
        // Collect those skeleton-ward edges; if there are fewer than two, this
        // boundary vertex is a smooth convex corner and produces no rib.
        let skeleton_ward: Vec<usize> = outgoing
            .iter()
            .copied()
            .filter(|&eid| {
                let twin = graph.edges[eid].twin;
                twin != NO_INDEX
                    && graph.edges[twin].start_vertex != NO_INDEX
                    && graph.vertices[graph.edges[twin].start_vertex].distance_to_boundary > 0.0
            })
            .collect();

        if skeleton_ward.len() < 2 {
            continue;
        }

        let spine_a = skeleton_ward[0];
        let spine_b = skeleton_ward[1];
        let twin_a = graph.edges[spine_a].twin;
        let twin_b = graph.edges[spine_b].twin;

        if twin_a == NO_INDEX
            || twin_b == NO_INDEX
            || used_for_quad[spine_a]
            || used_for_quad[spine_b]
        {
            continue;
        }

        // Mark the spine pair used and build a four-edge trapezoid cell:
        // spine_a, rib_a (=spine_a.twin), source_segment (=spine_b.twin), rib_b (=spine_b).
        used_for_quad[spine_a] = true;
        used_for_quad[spine_b] = true;

        let v0 = graph.edges[spine_a].start_vertex;
        let v1 = graph.edges[twin_a].start_vertex;
        let v2 = graph.edges[twin_b].start_vertex;
        let v3 = graph.edges[spine_b].start_vertex;

        let cell = QuadCell {
            edges: [spine_a, twin_a, twin_b, spine_b],
            vertices: [v0, v1, v2, v3],
        };
        quad_cells.push(cell);

        // Mark edge type and per-edge rib bookkeeping.
        graph.edges[spine_a].edge_type = EdgeType::NORMAL;
        graph.edges[spine_b].edge_type = EdgeType::NORMAL;

        // The twin edges represent the rib pair for this minimal Step 1 model.
        graph.edges[twin_a].edge_type = EdgeType::EXTRA_VD;
        graph.edges[twin_a].rib_twin = Some(spine_a);
        graph.edges[twin_a].central = false;

        graph.edges[twin_b].edge_type = EdgeType::EXTRA_VD;
        graph.edges[twin_b].rib_twin = Some(spine_b);
        graph.edges[twin_b].central = false;

        let cell_id = QuadCellId(graph.rib.next_quad_cell_id);
        graph.rib.next_quad_cell_id += 1;

        for &edge_id in &cell.edges {
            graph.edges[edge_id].quad_cell = Some(cell_id.0);
        }
    }

    graph.rib.quad_cells = quad_cells;
    Ok(())
}
