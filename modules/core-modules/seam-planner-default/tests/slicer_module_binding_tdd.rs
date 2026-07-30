//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! seam-planner-default matches its manifest's declared prepass world/stage.

#![allow(missing_docs)]

use seam_planner_default::SeamPlannerDefault;

#[test]
fn binding_surface_matches_seam_planning_stage() {
    assert_eq!(
        SeamPlannerDefault::__slicer_tier_id(),
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(SeamPlannerDefault::__slicer_trait_name(), "PrepassModule");
    assert_eq!(
        SeamPlannerDefault::__slicer_stage_name(),
        "PrePass::SeamPlanning"
    );
    assert_eq!(SeamPlannerDefault::__slicer_stage_export_name(), "run");
    assert_eq!(
        SeamPlannerDefault::__slicer_module_schema().stage_export,
        "slicer:prepass-seam-planning/seam-planning@1.0.0#run"
    );
    let exports = SeamPlannerDefault::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:prepass-seam-planning/seam-planning@1.0.0#run"));
}
