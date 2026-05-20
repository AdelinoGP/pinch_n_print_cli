//! Public helper façade for dispatch utilities used by tests and external callers.
//!
//! This module re-exports or wraps internal dispatch helpers that need to be
//! accessible from integration tests without making the full dispatch module public.

use slicer_core::paint_region::PaintRegionRTreeIndex;
use slicer_ir::PaintRegionIR;

use crate::wit_host::HostExecutionContext;

/// Harvest the paint-region IR and companion R-tree index from an
/// [`HostExecutionContext`] that has had paint-region entries collected
/// via [`HostPaintSegmentationOutput::push_paint_region`].
///
/// This is the public façade over the private `harvest_paint_segmentation_ir`
/// function in `dispatch.rs`, exposed for integration test use.
pub fn harvest_paint_segmentation_ir_from_ctx(
    ctx: HostExecutionContext,
) -> (PaintRegionIR, PaintRegionRTreeIndex) {
    crate::dispatch::harvest_paint_segmentation_ir_pub(ctx)
}
