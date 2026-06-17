//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! support-planner matches its manifest's declared prepass world/stage.

#![allow(missing_docs)]

use support_planner::SupportPlanner;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        SupportPlanner::__slicer_world_id(),
        "slicer:world-prepass@1.0.0"
    );
    assert_eq!(SupportPlanner::__slicer_trait_name(), "PrepassModule");
    assert_eq!(
        SupportPlanner::__slicer_stage_name(),
        "PrePass::SupportGeometry"
    );
    assert_eq!(
        SupportPlanner::__slicer_stage_export_name(),
        "run-support-geometry"
    );
    let exports = SupportPlanner::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-support-geometry"));
}
