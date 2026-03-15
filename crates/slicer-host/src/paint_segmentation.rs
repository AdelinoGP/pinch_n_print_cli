//! Paint segmentation execution contract.

use std::sync::Arc;

use slicer_ir::{LayerPlanIR, MeshIR, PaintRegionIR, PaintSemantic, SurfaceClassificationIR};

/// Structured paint-segmentation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintSegmentationError {
    /// Surface classification data is missing for a mesh object.
    MissingSurfaceObject {
        /// Object that could not be matched in `SurfaceClassificationIR`.
        object_id: String,
    },
    /// Layer planning data is missing for a mesh object.
    MissingLayerParticipation {
        /// Object that could not be matched in `LayerPlanIR.object_participation`.
        object_id: String,
    },
    /// One paint layer does not have one facet value per mesh triangle.
    MalformedFacetValues {
        /// Object carrying the malformed paint layer.
        object_id: String,
        /// Paint layer index within `FacetPaintData.layers`.
        layer_index: usize,
        /// Expected triangle count from `mesh.indices.len() / 3`.
        expected_facets: usize,
        /// Actual number of facet values present in the paint layer.
        actual_facet_values: usize,
    },
    /// Overlapping custom paint produced a deterministic equal-precedence conflict.
    DeterministicConflict {
        /// Global layer where the conflict occurs.
        global_layer_index: u32,
        /// Object owning the conflicting regions.
        object_id: String,
        /// Semantic family carrying the conflict.
        semantic: PaintSemantic,
        /// Equal precedence that caused the fatal ambiguity.
        paint_order: u64,
    },
}

/// Convert segmented whole-triangle paint assignments into immutable per-layer paint regions.
pub fn execute_paint_segmentation(
    mesh_ir: Arc<MeshIR>,
    surface_classification_ir: Arc<SurfaceClassificationIR>,
    layer_plan_ir: Arc<LayerPlanIR>,
) -> Result<Arc<PaintRegionIR>, PaintSegmentationError> {
    let _ = (mesh_ir, surface_classification_ir, layer_plan_ir);
    todo!("TASK-029 PaintSegmentation executor not implemented")
}
