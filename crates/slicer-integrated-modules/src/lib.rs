use slicer_scheduler::IntegratedModuleRegistration;
use slicer_sdk::native::NativeStageEntry;

#[cfg(feature = "classic-perimeters")]
use classic_perimeters::ClassicPerimeters;

#[cfg(feature = "classic-perimeters")]
const CLASSIC_PERIMETERS_MANIFEST: &str =
    include_str!("../../../modules/core-modules/classic-perimeters/classic-perimeters.toml");

/// Return the integrated module manifests enabled for this build.
pub fn integrated_registrations() -> Vec<IntegratedModuleRegistration> {
    #[cfg(feature = "classic-perimeters")]
    {
        vec![IntegratedModuleRegistration {
            manifest_toml: CLASSIC_PERIMETERS_MANIFEST,
            origin_label: "integrated://classic-perimeters",
        }]
    }

    #[cfg(not(feature = "classic-perimeters"))]
    {
        Vec::new()
    }
}

/// Return native entry points for the integrated modules enabled for this build.
pub fn native_entries() -> Vec<(String, NativeStageEntry)> {
    #[cfg(feature = "classic-perimeters")]
    {
        vec![(
            String::from("com.core.classic-perimeters"),
            ClassicPerimeters::__slicer_native_entry(),
        )]
    }

    #[cfg(not(feature = "classic-perimeters"))]
    {
        Vec::new()
    }
}

#[cfg(all(test, feature = "classic-perimeters"))]
mod classic_perimeters_tests {
    use super::integrated_registrations;
    use slicer_scheduler::{load_modules_from_roots_with_integrated, ModuleProvenance};

    #[test]
    fn embedded_classic_perimeters_manifest_ingests() {
        let report = load_modules_from_roots_with_integrated(&[], &integrated_registrations())
            .expect("embedded manifest should ingest");
        let module = report
            .modules
            .iter()
            .find(|module| module.id() == "com.core.classic-perimeters")
            .expect("classic perimeters should be present");
        assert_eq!(module.provenance(), ModuleProvenance::Integrated);
    }
}

#[cfg(all(test, not(feature = "classic-perimeters")))]
#[test]
fn integrated_registrations_are_empty_by_default() {
    assert!(integrated_registrations().is_empty());
}
