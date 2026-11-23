//! Module-private plan types for the lightning kernel.
//!
//! These are the relocated homes of the host producer's public plan types.
//! They are deliberately private to this module: no `serde`, no
//! `schema_version` field and no schema-version constant. The empty plan is
//! `KernelPlan { entries: Vec::new() }`; there is nothing to version because
//! no host contract reads these values.

use slicer_ir::{ObjectId, PaintValue, Point2, RegionId};

/// One 2-point tree-edge segment for a lightning sparse-infill tree on a
/// single `(object, region, layer)` identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PlanEntry {
    /// Object this entry belongs to.
    pub(crate) object_id: ObjectId,
    /// Global model layer index this entry applies to.
    pub(crate) global_layer_index: i32,
    /// Region inside the object this entry belongs to.
    pub(crate) region_id: RegionId,
    /// 2-point tree-edge segments in integer units.
    pub(crate) tree_edge_segments: Vec<[Point2; 2]>,
    /// Ordered `(paint_semantic_name, value)` chain of the source region.
    /// Empty is an exact identity for a genuinely chain-less source, never a
    /// wildcard.
    pub(crate) variant_chain: Vec<(String, PaintValue)>,
}

/// Per-object, per-region, per-layer tree-edge segments produced by the
/// module's preparation pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct KernelPlan {
    /// One entry per active `(object, region, layer)` triple that received
    /// lightning tree-edge segments. Multiple entries may share an
    /// `(object_id, global_layer_index)` when an object has multiple regions.
    pub(crate) entries: Vec<PlanEntry>,
}
