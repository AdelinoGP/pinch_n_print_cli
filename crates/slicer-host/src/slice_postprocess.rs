//! Slice-postprocess paint annotation execution contract.

use std::sync::Arc;

use slicer_ir::{PaintRegionIR, PaintSemantic, PaintValue, SliceIR};

/// One per-layer paint annotation invocation.
#[derive(Debug, Clone)]
pub struct SlicePostProcessPaintAnnotationRequest {
    /// Slice geometry after all `Layer::SlicePostProcess` polygon edits.
    pub slice_ir: SliceIR,
    /// Immutable per-layer paint regions produced by `PrePass::PaintSegmentation`.
    pub paint_regions: Arc<PaintRegionIR>,
    /// Semantics that must be annotatable for this layer.
    pub required_semantics: Vec<PaintSemantic>,
}

/// Output of the built-in paint annotation finalization step.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicePostProcessPaintAnnotationResult {
    /// Slice IR with `boundary_paint` rewritten for all regions.
    pub slice_ir: SliceIR,
    /// True when non-fatal fallback behavior was required.
    pub degraded: bool,
    /// Structured non-fatal warnings suitable for progress events.
    pub warnings: Vec<SlicePostProcessPaintAnnotationWarning>,
}

/// Structured non-fatal fallback record.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicePostProcessPaintAnnotationWarning {
    /// Stable warning code for frontend/event routing.
    pub code: u16,
    /// Layer where the fallback occurred.
    pub global_layer_index: u32,
    /// Region object identifier.
    pub object_id: String,
    /// Region identifier.
    pub region_id: u64,
    /// Semantic that was defaulted.
    pub semantic: PaintSemantic,
    /// Polygon index within `SlicedRegion.polygons`.
    pub polygon_index: usize,
    /// Contour-point index within the polygon contour.
    pub contour_point_index: usize,
    /// Deterministic fallback value that was written.
    pub fallback_value: PaintValue,
    /// Stable machine-readable warning kind.
    pub reason: SlicePostProcessPaintAnnotationWarningReason,
}

/// Warning kinds emitted by the host paint annotator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicePostProcessPaintAnnotationWarningReason {
    /// Point classification remained numerically unresolved after polygon edits.
    NumericalEdgeAmbiguity,
}

/// Fatal paint annotation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlicePostProcessPaintAnnotationError {
    /// Required paint data is missing for the layer entirely.
    MissingPaintRegionLayer {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Missing semantic family.
        semantic: PaintSemantic,
    },
    /// Required paint data is missing for one semantic on this layer.
    MissingPaintRegionSemantic {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Missing semantic family.
        semantic: PaintSemantic,
    },
    /// Existing boundary paint no longer matches final contour cardinality.
    BoundaryPaintCardinalityMismatch {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Region object identifier.
        object_id: String,
        /// Region identifier.
        region_id: u64,
        /// Semantic family carrying stale cardinality.
        semantic: PaintSemantic,
        /// Polygon index within `SlicedRegion.polygons`.
        polygon_index: usize,
        /// Final contour point count.
        expected_points: usize,
        /// Existing boundary-paint point count.
        actual_points: usize,
    },
    /// Equal-precedence conflicting custom values were encountered deterministically.
    DeterministicConflict {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Region object identifier.
        object_id: String,
        /// Region identifier.
        region_id: u64,
        /// Conflicting semantic family.
        semantic: PaintSemantic,
        /// Polygon index within `SlicedRegion.polygons`.
        polygon_index: usize,
        /// Contour-point index that triggered the conflict.
        contour_point_index: usize,
    },
}

/// Annotate one final `SliceIR` layer with contour-parallel `boundary_paint`.
pub fn execute_slice_postprocess_paint_annotation(
    request: SlicePostProcessPaintAnnotationRequest,
) -> Result<SlicePostProcessPaintAnnotationResult, SlicePostProcessPaintAnnotationError> {
    let _ = request;
    todo!("TASK-030 red scaffold: implement SlicePostProcess paint annotation executor")
}
