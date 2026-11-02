//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! lightning-infill matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use lightning_infill::LightningInfill;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        LightningInfill::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(LightningInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(LightningInfill::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(LightningInfill::__slicer_stage_export_name(), "run");
    assert_eq!(
        LightningInfill::__slicer_module_schema().stage_export,
        "slicer:layer-infill/infill@2.0.0#run"
    );
    let exports = LightningInfill::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:layer-infill/infill@2.0.0#run"));
}

/// The generated ordinary/preparation pair is the module's real compiled export
/// surface, pinned without a tree fixture: the schema names the capability's
/// qualified export and ordinary world, and the exported entry is the same
/// ordinary `Layer` stage the module implements.
#[test]
fn paired_preparation_export_matches_manifest() {
    let schema = LightningInfill::__SLICER_PREPARATION_SCHEMA;
    assert_eq!(
        schema.qualified_export,
        slicer_schema::PREPARATION_QUALIFIED_EXPORT
    );
    assert_eq!(
        schema.qualified_export,
        "slicer:layer-preparation/prepare@1.0.0#prepare-print"
    );
    assert_eq!(schema.ordinary_world, "infill-module");
    let entry = LightningInfill::__slicer_prepared_native_entry();
    assert!(
        matches!(entry.stage, slicer_sdk::native::NativeStageEntry::Layer(_)),
        "the prepared pair must compose the module's ordinary Layer entry"
    );
}
