//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! overhang-classifier-default matches its manifest's declared finalization
//! world/stage. (Despite the "classifier" name, this is a FinalizationModule.)

#![allow(missing_docs)]

use overhang_classifier_default::OverhangClassifierDefault;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        OverhangClassifierDefault::__slicer_world_id(),
        "slicer:world-finalization@1.0.0"
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
        "run-finalization"
    );
    let exports = OverhangClassifierDefault::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-finalization"));
}
