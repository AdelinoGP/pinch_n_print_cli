//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! support-surface-ironing matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use support_surface_ironing::SupportSurfaceIroning;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        SupportSurfaceIroning::__slicer_world_id(),
        "slicer:world-layer@1.0.0"
    );
    assert_eq!(SupportSurfaceIroning::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        SupportSurfaceIroning::__slicer_stage_name(),
        "Layer::InfillPostProcess"
    );
    assert_eq!(
        SupportSurfaceIroning::__slicer_stage_export_name(),
        "run-infill-postprocess"
    );
    let exports = SupportSurfaceIroning::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill-postprocess"));
}
