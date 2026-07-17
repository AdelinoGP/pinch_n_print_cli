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
    assert_eq!(
        PartCooling::__slicer_stage_export_name(),
        "run-finalization"
    );
    let exports = PartCooling::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-finalization"));
}
