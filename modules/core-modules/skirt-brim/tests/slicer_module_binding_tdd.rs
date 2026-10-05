//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! skirt-brim matches its manifest's declared finalization world/stage.

#![allow(missing_docs)]

use skirt_brim::SkirtBrim;

#[test]
fn binding_surface_matches_manifest() {
    let expected_export =
        slicer_schema::qualified_export_for_stage_id("PostPass::LayerFinalization")
            .expect("finalization stage has a qualified WIT export");

    assert_eq!(
        SkirtBrim::__slicer_tier_id(),
        slicer_schema::TIER_FINALIZATION
    );
    assert_eq!(SkirtBrim::__slicer_trait_name(), "FinalizationModule");
    assert_eq!(
        SkirtBrim::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(SkirtBrim::__slicer_stage_export_name(), "run");
    assert_eq!(
        SkirtBrim::__slicer_module_schema().stage_export,
        expected_export
    );
    let exports = SkirtBrim::__slicer_wit_exports();
    assert!(exports.contains(&expected_export.as_str()));
}
