//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! part-cooling matches its manifest's declared finalization world/stage.

#![allow(missing_docs)]

use part_cooling::PartCooling;

#[test]
fn binding_surface_matches_finalization_stage() {
    assert_eq!(
        PartCooling::__slicer_world_id(),
        slicer_schema::WORLD_FINALIZATION
    );
    assert_eq!(PartCooling::__slicer_trait_name(), "FinalizationModule");
    assert_eq!(
        PartCooling::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(PartCooling::__slicer_stage_export_name(), "run");
    assert_eq!(
        PartCooling::__slicer_module_schema().stage_export,
        "slicer:finalization-layer-finalization/layer-finalization@1.0.0#run"
    );
    let exports = PartCooling::__slicer_wit_exports();
    assert!(
        exports.contains(&"slicer:finalization-layer-finalization/layer-finalization@1.0.0#run")
    );
}
