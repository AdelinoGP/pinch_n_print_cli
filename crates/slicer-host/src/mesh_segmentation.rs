//! Mesh segmentation execution contract.

use std::sync::Arc;

use slicer_ir::MeshIR;

/// Deterministic reasons a projected paint stroke cannot be normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegenerateStrokeReason {
    /// The projected stroke has zero area and cannot split a facet.
    ZeroAreaStrokeTriangle,
    /// The projected stroke only grazes an edge, so the split would be ambiguous.
    TangentToFacetEdge,
    /// The projected stroke only touches a triangle vertex, so ownership is ambiguous.
    TouchesFacetVertex,
}

/// Structured mesh-segmentation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshSegmentationError {
    /// A stroke could not be normalized deterministically.
    DegenerateStroke {
        /// Object carrying the invalid paint stroke.
        object_id: String,
        /// Paint layer index within `FacetPaintData.layers`.
        layer_index: usize,
        /// Stroke index within `PaintLayer.strokes`.
        stroke_index: usize,
        /// Stable reason for rejection.
        reason: DegenerateStrokeReason,
    },
}

/// Normalize sub-facet paint strokes into whole-triangle assignments.
pub fn execute_mesh_segmentation(
    mesh_ir: Arc<MeshIR>,
) -> Result<Arc<MeshIR>, MeshSegmentationError> {
    let _ = mesh_ir;
    todo!("TASK-028: implement MeshSegmentation stage executor")
}
