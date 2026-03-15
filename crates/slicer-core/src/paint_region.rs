//! Paint-region point query helpers.

use slicer_ir::{PaintRegionIR, PaintSemantic, PaintValue, Point2};

/// Boundary handling mode for point-in-polygon paint queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryInclusion {
    /// Treat points on polygon boundaries as contained.
    Include,
    /// Treat points on polygon boundaries as excluded unless strictly interior.
    Exclude,
}

/// Deterministic point-query failures for paint-region lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintRegionQueryError {
    /// Equal-precedence conflicting paint values were encountered.
    DeterministicConflict,
}

/// Queries the paint value for a single point on one layer and semantic.
pub fn point_in_paint_region(
    paint_regions: &PaintRegionIR,
    layer_index: u32,
    semantic: &PaintSemantic,
    point: Point2,
    boundary_inclusion: BoundaryInclusion,
) -> Result<Option<PaintValue>, PaintRegionQueryError> {
    let _ = (paint_regions, layer_index, semantic, point, boundary_inclusion);
    todo!("TASK-015 point-in-polygon query not implemented yet")
}
