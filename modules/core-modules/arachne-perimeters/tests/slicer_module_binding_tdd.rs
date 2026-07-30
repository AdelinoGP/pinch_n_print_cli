//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! arachne-perimeters matches its manifest's declared stage package.

#![allow(missing_docs)]

use arachne_perimeters::ArachnePerimeters;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        ArachnePerimeters::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(ArachnePerimeters::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        ArachnePerimeters::__slicer_stage_name(),
        "Layer::Perimeters"
    );
    assert_eq!(ArachnePerimeters::__slicer_stage_export_name(), "run");
    assert_eq!(
        ArachnePerimeters::__slicer_module_schema().stage_export,
        "slicer:layer-perimeters/perimeters@1.0.0#run"
    );
    let exports = ArachnePerimeters::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-perimeters/perimeters@1.0.0#run"));
}
