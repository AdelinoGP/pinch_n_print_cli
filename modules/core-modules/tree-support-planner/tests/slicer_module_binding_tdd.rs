//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! support-planner matches its manifest's declared prepass world/stage.

#![allow(missing_docs)]

use tree_support_planner::SupportPlanner;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        SupportPlanner::__slicer_tier_id(),
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(SupportPlanner::__slicer_trait_name(), "PrepassModule");
    assert_eq!(
        SupportPlanner::__slicer_stage_name(),
        "PrePass::SupportGeometry"
    );
    assert_eq!(SupportPlanner::__slicer_stage_export_name(), "run");
    assert_eq!(
        SupportPlanner::__slicer_module_schema().stage_export,
        "slicer:prepass-support-geometry/support-geometry@1.0.0#run"
    );
    let exports = SupportPlanner::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:prepass-support-geometry/support-geometry@1.0.0#run"));
}
