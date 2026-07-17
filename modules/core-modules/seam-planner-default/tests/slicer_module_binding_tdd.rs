//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! seam-planner-default matches its manifest's declared prepass world/stage.

#![allow(missing_docs)]

use seam_planner_default::SeamPlannerDefault;

#[test]
fn binding_surface_matches_seam_planning_stage() {
    assert_eq!(
        SeamPlannerDefault::__slicer_world_id(),
        slicer_schema::WORLD_PREPASS
    );
    assert_eq!(SeamPlannerDefault::__slicer_trait_name(), "PrepassModule");
    assert_eq!(
        SeamPlannerDefault::__slicer_stage_name(),
        "PrePass::SeamPlanning"
    );
    assert_eq!(
        SeamPlannerDefault::__slicer_stage_export_name(),
        "run-seam-planning"
    );
    let exports = SeamPlannerDefault::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-seam-planning"));
}
