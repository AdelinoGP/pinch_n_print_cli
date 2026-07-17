//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! fuzzy-skin matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use fuzzy_skin::FuzzySkinModule;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        FuzzySkinModule::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(FuzzySkinModule::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        FuzzySkinModule::__slicer_stage_name(),
        "Layer::PerimetersPostProcess"
    );
    assert_eq!(
        FuzzySkinModule::__slicer_stage_export_name(),
        "run-wall-postprocess"
    );
    let exports = FuzzySkinModule::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-wall-postprocess"));
}
