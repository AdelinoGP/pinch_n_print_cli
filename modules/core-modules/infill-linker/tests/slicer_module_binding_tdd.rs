#![allow(missing_docs)]

use infill_linker::InfillLinker;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        InfillLinker::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(InfillLinker::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        InfillLinker::__slicer_stage_name(),
        "Layer::InfillPostProcess"
    );
    assert_eq!(
        InfillLinker::__slicer_stage_export_name(),
        "run-infill-postprocess"
    );
    let exports = InfillLinker::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill-postprocess"));
}
