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
    assert_eq!(InfillLinker::__slicer_stage_export_name(), "run");
    assert_eq!(
        InfillLinker::__slicer_module_schema().stage_export,
        "slicer:layer-infill-postprocess/infill-postprocess@1.0.0#run"
    );
    let exports = InfillLinker::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-infill-postprocess/infill-postprocess@1.0.0#run"));
}
