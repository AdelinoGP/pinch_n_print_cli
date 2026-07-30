//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! lightning-infill matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use lightning_infill::LightningInfill;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        LightningInfill::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(LightningInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(LightningInfill::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(LightningInfill::__slicer_stage_export_name(), "run");
    assert_eq!(
        LightningInfill::__slicer_module_schema().stage_export,
        "slicer:layer-infill/infill@1.0.0#run"
    );
    let exports = LightningInfill::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-infill/infill@1.0.0#run"));
}
