//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! classic-perimeters matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use classic_perimeters::ClassicPerimeters;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        ClassicPerimeters::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(ClassicPerimeters::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        ClassicPerimeters::__slicer_stage_name(),
        "Layer::Perimeters"
    );
    assert_eq!(ClassicPerimeters::__slicer_stage_export_name(), "run");
    assert_eq!(
        ClassicPerimeters::__slicer_module_schema().stage_export,
        "slicer:layer-perimeters/perimeters@1.0.0#run"
    );
    let exports = ClassicPerimeters::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-perimeters/perimeters@1.0.0#run"));
}
