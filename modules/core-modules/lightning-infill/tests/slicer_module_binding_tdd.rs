//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! lightning-infill matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use lightning_infill::LightningInfill;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        LightningInfill::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(LightningInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(LightningInfill::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(LightningInfill::__slicer_stage_export_name(), "run-infill");
    let exports = LightningInfill::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill"));
}
