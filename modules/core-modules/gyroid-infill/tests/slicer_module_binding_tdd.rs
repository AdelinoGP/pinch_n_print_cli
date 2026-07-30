//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! gyroid-infill matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use gyroid_infill::GyroidInfill;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        GyroidInfill::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(GyroidInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(GyroidInfill::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(GyroidInfill::__slicer_stage_export_name(), "run");
    assert_eq!(
        GyroidInfill::__slicer_module_schema().stage_export,
        "slicer:layer-infill/infill@1.0.0#run"
    );
    let exports = GyroidInfill::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-infill/infill@1.0.0#run"));
}
