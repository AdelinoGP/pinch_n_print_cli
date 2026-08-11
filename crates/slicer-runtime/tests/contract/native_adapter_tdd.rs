use sdk_finalization_guest::SdkFinalizationModule;
use sdk_layer_infill_guest::SdkLayerInfillModule;
use sdk_postpass_text_guest::SdkPostpassTextModule;
use sdk_prepass_guest::SdkPrepassModule;
use slicer_sdk::native::NativeStageEntry;

#[test]
fn native_entry_layer_family() {
    assert!(matches!(
        SdkLayerInfillModule::__slicer_native_entry(),
        NativeStageEntry::Layer(_)
    ));
}

#[test]
fn native_entry_prepass_family() {
    assert!(matches!(
        SdkPrepassModule::__slicer_native_entry(),
        NativeStageEntry::Prepass(_)
    ));
}

#[test]
fn native_entry_postpass_family() {
    assert!(matches!(
        SdkPostpassTextModule::__slicer_native_entry(),
        NativeStageEntry::Postpass(_)
    ));
}

#[test]
fn native_entry_finalization_family() {
    assert!(matches!(
        SdkFinalizationModule::__slicer_native_entry(),
        NativeStageEntry::Finalization(_)
    ));
}
