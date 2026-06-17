//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! wipe-tower matches its manifest's declared finalization world/stage.

#![allow(missing_docs)]

use wipe_tower::WipeTower;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        WipeTower::__slicer_world_id(),
        "slicer:world-finalization@1.0.0"
    );
    assert_eq!(WipeTower::__slicer_trait_name(), "FinalizationModule");
    assert_eq!(
        WipeTower::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(WipeTower::__slicer_stage_export_name(), "run-finalization");
    let exports = WipeTower::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-finalization"));
}
