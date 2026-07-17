//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! classic-perimeters matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use classic_perimeters::ClassicPerimeters;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        ClassicPerimeters::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(ClassicPerimeters::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        ClassicPerimeters::__slicer_stage_name(),
        "Layer::Perimeters"
    );
    assert_eq!(
        ClassicPerimeters::__slicer_stage_export_name(),
        "run-perimeters"
    );
    let exports = ClassicPerimeters::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-perimeters"));
}
