//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! fuzzy-skin matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use fuzzy_skin::FuzzySkinModule;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        FuzzySkinModule::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(FuzzySkinModule::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        FuzzySkinModule::__slicer_stage_name(),
        "Layer::PerimetersPostProcess"
    );
    assert_eq!(FuzzySkinModule::__slicer_stage_export_name(), "run");
    assert_eq!(
        FuzzySkinModule::__slicer_module_schema().stage_export,
        "slicer:layer-perimeters-postprocess/perimeters-postprocess@1.0.0#run"
    );
    let exports = FuzzySkinModule::__slicer_wit_exports();
    assert!(
        exports.contains(&"slicer:layer-perimeters-postprocess/perimeters-postprocess@1.0.0#run")
    );
}
