//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! tree-support matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use tree_support::TreeSupport;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(TreeSupport::__slicer_tier_id(), slicer_schema::TIER_LAYER);
    assert_eq!(TreeSupport::__slicer_trait_name(), "LayerModule");
    assert_eq!(TreeSupport::__slicer_stage_name(), "Layer::Support");
    assert_eq!(TreeSupport::__slicer_stage_export_name(), "run");
    assert_eq!(
        TreeSupport::__slicer_module_schema().stage_export,
        "slicer:layer-support/support@1.0.0#run"
    );
    let exports = TreeSupport::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-support/support@1.0.0#run"));
}
