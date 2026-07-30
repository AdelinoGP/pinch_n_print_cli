//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! support-surface-ironing matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use support_surface_ironing::SupportSurfaceIroning;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        SupportSurfaceIroning::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(SupportSurfaceIroning::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        SupportSurfaceIroning::__slicer_stage_name(),
        "Layer::SupportPostProcess"
    );
    assert_eq!(SupportSurfaceIroning::__slicer_stage_export_name(), "run");
    assert_eq!(
        SupportSurfaceIroning::__slicer_module_schema().stage_export,
        "slicer:layer-support-postprocess/support-postprocess@1.0.0#run"
    );
    let exports = SupportSurfaceIroning::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-support-postprocess/support-postprocess@1.0.0#run"));
}
