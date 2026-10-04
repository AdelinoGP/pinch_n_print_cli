//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! overhang-classifier-default matches its manifest's declared finalization
//! world/stage. (Despite the "classifier" name, this is a FinalizationModule.)

#![allow(missing_docs)]

use overhang_classifier_default::OverhangClassifierDefault;

#[test]
fn binding_surface_matches_manifest() {
    let expected_export =
        slicer_schema::qualified_export_for_stage_id("PostPass::LayerFinalization")
            .expect("finalization stage has a qualified WIT export");

    assert_eq!(
        OverhangClassifierDefault::__slicer_tier_id(),
        slicer_schema::TIER_FINALIZATION
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_trait_name(),
        "FinalizationModule"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_stage_export_name(),
        "run"
    );
    assert_eq!(
        OverhangClassifierDefault::__slicer_module_schema().stage_export,
        expected_export
    );
    let exports = OverhangClassifierDefault::__slicer_wit_exports();
    assert!(exports.contains(&expected_export.as_str()));
}
