//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! seam-placer matches its manifest's declared layer world/stage. (seam-placer
//! is a LayerModule at PerimetersPostProcess, not a seam-planning prepass.)

#![allow(missing_docs)]

use seam_placer::SeamPlacer;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(SeamPlacer::__slicer_world_id(), "slicer:world-layer@1.0.0");
    assert_eq!(SeamPlacer::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        SeamPlacer::__slicer_stage_name(),
        "Layer::PerimetersPostProcess"
    );
    assert_eq!(
        SeamPlacer::__slicer_stage_export_name(),
        "run-wall-postprocess"
    );
    let exports = SeamPlacer::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-wall-postprocess"));
}
