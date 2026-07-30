//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! overhang-classifier-default matches its manifest's declared finalization
//! world/stage. (Despite the "classifier" name, this is a FinalizationModule.)

#![allow(missing_docs)]

use overhang_classifier_default::OverhangClassifierDefault;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        OverhangClassifierDefault::__slicer_tier_id(),
        slicer_schema::TIER_FINALIZATION
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_trait_name(),
        "FinalizationModule"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_stage_export_name(),
        "run"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_module_schema().stage_export,
        "slicer:finalization-layer-finalization/layer-finalization@1.0.0#run"
    );
    let exports = OverhangClassifierDefault::__slicer_wit_exports();
    assert!(
        exports.contains(&"slicer:finalization-layer-finalization/layer-finalization@1.0.0#run")
    );
}
