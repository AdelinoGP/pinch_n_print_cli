//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! arachne-perimeters matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use arachne_perimeters::ArachnePerimeters;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        ArachnePerimeters::__slicer_world_id(),
        "slicer:world-layer@1.0.0"
    );
    assert_eq!(ArachnePerimeters::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        ArachnePerimeters::__slicer_stage_name(),
        "Layer::Perimeters"
    );
    assert_eq!(
        ArachnePerimeters::__slicer_stage_export_name(),
        "run-perimeters"
    );
    let exports = ArachnePerimeters::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-perimeters"));
}
