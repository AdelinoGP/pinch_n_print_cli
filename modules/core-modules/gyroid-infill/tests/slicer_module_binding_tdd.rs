//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! gyroid-infill matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use gyroid_infill::GyroidInfill;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        GyroidInfill::__slicer_world_id(),
        "slicer:world-layer@1.0.0"
    );
    assert_eq!(GyroidInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(GyroidInfill::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(GyroidInfill::__slicer_stage_export_name(), "run-infill");
    let exports = GyroidInfill::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill"));
}
