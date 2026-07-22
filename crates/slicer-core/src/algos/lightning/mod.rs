//! Lightning sparse-infill tree generator.
//!
//! Contract landed in packet 137; the per-layer 2-point integer-unit
//! `tree_edge_segments` storage and per-object/per-layer `LightningTreeEntry`
//! shape are stable. The actual generator algorithm (distance field +
//! `TreeNode` + cross-layer `Generator`) is ported in packets 138
//! (data structures) and 139 (wiring into this seam).

use std::sync::Arc;

use slicer_ir::LightningTreeIR;

use crate::algos::lightning::error::LightningTreeError;

/// Discrete support-distance field used by the Lightning tree generator.
pub mod distance_field;
pub mod error;
/// Parent/child graph primitive used by the Lightning tree generator.
pub mod tree_node;

pub use distance_field::DistanceField;

/// Generate the per-object, per-layer `LightningTreeIR` for one print.
///
/// The 137 contract returns an empty-but-valid `LightningTreeIR` (the
/// `Default` impl uses `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`). 139
/// replaces the body with the real generator; this signature is the seam
/// both implementations share.
#[allow(clippy::missing_errors_doc)]
pub fn generate_lightning_trees() -> Result<Arc<LightningTreeIR>, LightningTreeError> {
    // 139 wiring point: replace this body with the real cross-layer
    // distance-field + tree-node generator; the return type stays the same.
    Ok(Arc::new(LightningTreeIR::default()))
}
