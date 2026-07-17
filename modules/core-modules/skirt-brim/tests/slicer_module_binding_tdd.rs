//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! skirt-brim matches its manifest's declared finalization world/stage.

#![allow(missing_docs)]

use skirt_brim::SkirtBrim;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        SkirtBrim::__slicer_world_id(),
        slicer_schema::WORLD_FINALIZATION
    );
    assert_eq!(SkirtBrim::__slicer_trait_name(), "FinalizationModule");
    assert_eq!(
        SkirtBrim::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(SkirtBrim::__slicer_stage_export_name(), "run-finalization");
    let exports = SkirtBrim::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-finalization"));
}
