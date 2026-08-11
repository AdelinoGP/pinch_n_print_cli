use slicer_scheduler::IntegratedModuleRegistration;
use slicer_sdk::native::NativeStageEntry;

#[cfg(feature = "arachne-perimeters")]
use arachne_perimeters::ArachnePerimeters;
#[cfg(feature = "classic-perimeters")]
use classic_perimeters::ClassicPerimeters;
#[cfg(feature = "support-planner")]
use support_planner::SupportPlanner;

#[cfg(feature = "classic-perimeters")]
const CLASSIC_PERIMETERS_MANIFEST: &str =
    include_str!("../../../modules/core-modules/classic-perimeters/classic-perimeters.toml");
#[cfg(feature = "arachne-perimeters")]
const ARACHNE_PERIMETERS_MANIFEST: &str =
    include_str!("../../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml");
#[cfg(feature = "support-planner")]
const SUPPORT_PLANNER_MANIFEST: &str =
    include_str!("../../../modules/core-modules/support-planner/support-planner.toml");

/// Return the integrated module manifests enabled for this build.
pub fn integrated_registrations() -> Vec<IntegratedModuleRegistration> {
    #[allow(unused_mut)]
    let mut registrations = Vec::new();

    #[cfg(feature = "classic-perimeters")]
    {
        registrations.push(IntegratedModuleRegistration {
            manifest_toml: CLASSIC_PERIMETERS_MANIFEST,
            origin_label: "integrated://classic-perimeters",
        });
    }

    #[cfg(feature = "arachne-perimeters")]
    {
        registrations.push(IntegratedModuleRegistration {
            manifest_toml: ARACHNE_PERIMETERS_MANIFEST,
            origin_label: "integrated://arachne-perimeters",
        });
    }

    #[cfg(feature = "support-planner")]
    {
        registrations.push(IntegratedModuleRegistration {
            manifest_toml: SUPPORT_PLANNER_MANIFEST,
            origin_label: "integrated://support-planner",
        });
    }

    registrations
}

/// Return native entry points for the integrated modules enabled for this build.
pub fn native_entries() -> Vec<(String, NativeStageEntry)> {
    #[allow(unused_mut)]
    let mut entries = Vec::new();

    #[cfg(feature = "classic-perimeters")]
    {
        entries.push((
            String::from("com.core.classic-perimeters"),
            ClassicPerimeters::__slicer_native_entry(),
        ));
    }

    #[cfg(feature = "arachne-perimeters")]
    {
        entries.push((
            String::from("com.core.arachne-perimeters"),
            ArachnePerimeters::__slicer_native_entry(),
        ));
    }

    #[cfg(feature = "support-planner")]
    {
        entries.push((
            String::from("com.core.support-planner"),
            SupportPlanner::__slicer_native_entry(),
        ));
    }

    entries
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

#[cfg(all(
    test,
    feature = "classic-perimeters",
    feature = "arachne-perimeters",
    feature = "support-planner"
))]
mod hybrid_pilot_tests {
    use super::{integrated_registrations, native_entries};
    use slicer_sdk::native::NativeStageEntry;

    const IDS: [&str; 3] = [
        "com.core.classic-perimeters",
        "com.core.arachne-perimeters",
        "com.core.support-planner",
    ];

    #[test]
    fn hybrid_pilot_registrations_are_exactly_three() {
        let registrations = integrated_registrations();
        assert_eq!(registrations.len(), 3);

        for id in IDS {
            let origin = format!("integrated://{}", id.strip_prefix("com.core.").unwrap());
            let registration = registrations
                .iter()
                .find(|registration| registration.origin_label == origin)
                .expect("pilot registration should be present");
            assert!(registration
                .manifest_toml
                .contains(&format!("id           = \"{id}\"")));
        }
    }

    #[test]
    fn hybrid_pilot_native_entry_families_match_stage_ids() {
        let entries = native_entries();
        assert_eq!(entries.len(), 3);

        for id in IDS {
            assert!(entries.iter().any(|(entry_id, _)| entry_id == id));
        }

        let classic = entries
            .iter()
            .find(|(id, _)| id == "com.core.classic-perimeters")
            .unwrap();
        assert!(matches!(classic.1, NativeStageEntry::Layer(_)));

        let arachne = entries
            .iter()
            .find(|(id, _)| id == "com.core.arachne-perimeters")
            .unwrap();
        assert!(matches!(arachne.1, NativeStageEntry::Layer(_)));

        let support = entries
            .iter()
            .find(|(id, _)| id == "com.core.support-planner")
            .unwrap();
        assert!(matches!(support.1, NativeStageEntry::Prepass(_)));
    }
}
