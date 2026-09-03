//! Host-side support-territory clipping (SchemaBridgeMap ticket 19).
//!
//! A parameter modifier that changes `support_type` mints a sub-region whose
//! support family differs from its parent's. Both family planners emit bodies
//! under the same overhang, and the tree planner's branches drift into the
//! free air under the modifier half, so the cross-family overlap guard in
//! `support_aggregation` rejected BOTH sides on every overlapping layer.
//! Measured on `resources/support_test_modifier_normal_in_tree.3mf`: tree
//! support stopped at z = 18.0 and no traditional support was published at
//! all.
//!
//! The host now publishes each minted sub-region's own territory
//! (`SupportAnalysisIR::support_territory`) and this module applies the one
//! clip rule shared with the planners
//! (`slicer_sdk::prepass_types::SupportAnalysisView::territory_partition`):
//! a sub-region body keeps `roles ∩ own`; a base-region body keeps
//! `roles - inflate(foreign, clearance)`. The clearance sits on the base side
//! only, so the two families never touch and the downstream swept-path guard
//! (`LayerStageCommit::Support` in
//! `crates/slicer-runtime/src/layer_executor.rs`) has nothing to reject.
//!
//! Orca has no per-region support family, so none of this has a canonical
//! counterpart (see `docs/DEVIATION_LOG.md`).

use slicer_core::polygon_ops::{difference, intersection, offset, OffsetJoinType};
use slicer_ir::{ExPolygon, SupportAnalysisIR};
use slicer_sdk::prepass_types::SupportAnalysisView;

use crate::marshal::native::support_analysis_view_from_ir;

/// `SupportAnalysisIR::shared_settings` key carrying the clearance, in mm,
/// that base-family bodies keep from foreign territory. The producer publishes
/// the resolved support line width.
pub const SUPPORT_TERRITORY_CLEARANCE_KEY: &str = "support_territory_clearance_mm";

/// Clips support bodies to the territory their family owns.
pub struct TerritoryClipper {
    view: SupportAnalysisView,
    clearance_mm: f32,
}

impl TerritoryClipper {
    /// `None` when the analysis carries no territory at all, so callers keep
    /// the legacy (territory-free) guard path without projecting the IR.
    pub fn from_ir(analysis: &SupportAnalysisIR) -> Option<Self> {
        if analysis.support_territory.is_empty() {
            return None;
        }
        let clearance_mm = analysis
            .shared_settings
            .get(SUPPORT_TERRITORY_CLEARANCE_KEY)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0);
        Some(Self {
            view: support_analysis_view_from_ir(analysis),
            clearance_mm,
        })
    }

    /// Whether `(object_id, layer)` carries any territory.
    pub fn has_territory(&self, object_id: &str, layer: u32) -> bool {
        self.view
            .support_territory
            .iter()
            .any(|entry| entry.object_id == object_id && entry.global_support_layer_index == layer)
    }

    /// Clip one body to what `family_id` may print for `region_id` at
    /// `(object_id, layer)`. Returns `None` when the layer carries no
    /// territory (nothing to clip against).
    pub fn clip(
        &self,
        object_id: &str,
        layer: u32,
        region_id: &str,
        family_id: &str,
        polygons: &[ExPolygon],
    ) -> Option<Vec<ExPolygon>> {
        if let Some(own) = self.view.region_territory(object_id, layer, region_id) {
            return Some(intersection(polygons, own));
        }
        let partition = self.view.territory_partition(object_id, layer, family_id)?;
        if partition.foreign.is_empty() {
            return Some(polygons.to_vec());
        }
        Some(difference(
            polygons,
            &inflate_foreign(&partition.foreign, self.clearance_mm),
        ))
    }
}

/// Foreign territory grown by the base-side clearance. A zero clearance keeps
/// the raw footprint.
pub fn inflate_foreign(foreign: &[ExPolygon], clearance_mm: f32) -> Vec<ExPolygon> {
    if clearance_mm <= 0.0 {
        return foreign.to_vec();
    }
    let grown = offset(foreign, clearance_mm, OffsetJoinType::Miter, 0.0);
    if grown.is_empty() {
        foreign.to_vec()
    } else {
        grown
    }
}
