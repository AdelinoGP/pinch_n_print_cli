//! Stage-order helpers derived from the single canonical list.
//!
//! The authoritative ordered stage list is [`crate::execution_plan::STAGE_ORDER`]
//! (docs/04 §Fixed Stage Order), already pinned against the WIT export table by
//! `tests/contract/stage_list_consistency_tdd.rs`. Several validators used to
//! keep their *own* copies of it, and two of them had silently drifted:
//! `manifest::known_stage_ids` and `validation::stage_order_index` both omitted
//! `PrePass::SeamPlanning`, `PrePass::SupportGeometry`, and
//! `Layer::PaintRegionAnnotation`. Because `stage_order_index` doubles as the
//! allowlist for a module's own declared `module.stage` (see
//! `validation::validate_stage_ids`), a module legitimately declaring one of
//! those stages (e.g. `seam-planner-default`) was rejected at startup as an
//! `UnknownStage`.
//!
//! This module is the one place those helpers are derived, so the drift cannot
//! recur. It adds no new list — it forwards to `STAGE_ORDER`.

use std::collections::BTreeMap;

use crate::execution_plan::STAGE_ORDER;

/// Returns the canonical stage ids as a slice (membership allowlist).
#[must_use]
pub fn known_stage_ids() -> &'static [&'static str] {
    STAGE_ORDER
}

/// Returns true when `stage` is a recognised pipeline stage id.
#[must_use]
pub fn is_known_stage(stage: &str) -> bool {
    STAGE_ORDER.contains(&stage)
}

/// Builds the `stage -> canonical index` map used by the scheduler's ordering
/// passes (cross-stage dependency legality, unfulfilled-reads, dead-writes).
#[must_use]
pub fn stage_order_index() -> BTreeMap<&'static str, usize> {
    STAGE_ORDER
        .iter()
        .enumerate()
        .map(|(index, stage)| (*stage, index))
        .collect()
}

/// Derives the coarse tier label (`"prepass"`, `"per_layer"`, `"postpass"`)
/// from a stage id prefix. Unknown prefixes return `"unknown"`.
#[must_use]
pub fn tier_of(stage: &str) -> &'static str {
    if stage.starts_with("PrePass::") {
        "prepass"
    } else if stage.starts_with("Layer::") {
        "per_layer"
    } else if stage.starts_with("PostPass::") {
        "postpass"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three stages that the old `validation::stage_order_index` dropped
    /// must be present in the canonical list, or modules declaring them are
    /// wrongly rejected as `UnknownStage` at startup. This is the regression
    /// guard for the bug 3d fixed.
    #[test]
    fn previously_dropped_stages_are_known() {
        for stage in [
            "PrePass::SeamPlanning",
            "PrePass::SupportGeometry",
            "Layer::PaintRegionAnnotation",
        ] {
            assert!(is_known_stage(stage), "{stage} must be a known stage");
            assert!(
                stage_order_index().contains_key(stage),
                "{stage} must have an ordering index"
            );
        }
    }

    #[test]
    fn stage_order_index_covers_every_canonical_stage() {
        assert_eq!(stage_order_index().len(), STAGE_ORDER.len());
    }

    #[test]
    fn tier_of_classifies_each_canonical_stage() {
        for stage in STAGE_ORDER {
            assert_ne!(tier_of(stage), "unknown", "{stage} has no tier");
        }
    }
}
