//! TASK-111 regression guard: layer-planner-default has adopted
//! `#[slicer_module]` on its PrepassModule impl; the emitted binding
//! surface matches the documented prepass world/stage.

use layer_planner_default::DefaultLayerPlanner;

#[test]
fn binding_surface_matches_prepass_layer_planning_stage() {
    assert_eq!(
        DefaultLayerPlanner::__slicer_world_id(),
        slicer_schema::WORLD_PREPASS
    );
    assert_eq!(DefaultLayerPlanner::__slicer_trait_name(), "PrepassModule");
    assert_eq!(
        DefaultLayerPlanner::__slicer_stage_name(),
        "PrePass::LayerPlanning"
    );
    assert_eq!(DefaultLayerPlanner::__slicer_stage_export_name(), "run");
    assert_eq!(
        DefaultLayerPlanner::__slicer_module_schema().stage_export,
        "slicer:prepass-layer-planning/layer-planning@1.0.0#run"
    );
    let exports = DefaultLayerPlanner::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:prepass-layer-planning/layer-planning@1.0.0#run"));
}
