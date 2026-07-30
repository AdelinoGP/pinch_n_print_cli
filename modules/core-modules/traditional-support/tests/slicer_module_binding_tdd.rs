//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! traditional-support matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use traditional_support::TraditionalSupport;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        TraditionalSupport::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(TraditionalSupport::__slicer_trait_name(), "LayerModule");
    assert_eq!(TraditionalSupport::__slicer_stage_name(), "Layer::Support");
    assert_eq!(TraditionalSupport::__slicer_stage_export_name(), "run");
    assert_eq!(
        TraditionalSupport::__slicer_module_schema().stage_export,
        "slicer:layer-support/support@1.0.0#run"
    );
    let exports = TraditionalSupport::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-support/support@1.0.0#run"));
}
