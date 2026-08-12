use slicer_scheduler::IntegratedModuleRegistration;
use slicer_sdk::native::NativeStageEntry;

#[cfg(feature = "fuzzy-skin")]
use fuzzy_skin::FuzzySkinModule;
#[cfg(feature = "gyroid-infill")]
use gyroid_infill::GyroidInfill;
#[cfg(feature = "infill-linker")]
use infill_linker::InfillLinker;
#[cfg(feature = "layer-planner-default")]
use layer_planner_default::DefaultLayerPlanner;
#[cfg(feature = "lightning-infill")]
use lightning_infill::LightningInfill;
#[cfg(feature = "machine-gcode-emit")]
use machine_gcode_emit::MachineGcodeEmit;
#[cfg(feature = "overhang-classifier-default")]
use overhang_classifier_default::OverhangClassifierDefault;
#[cfg(feature = "part-cooling")]
use part_cooling::PartCooling;
#[cfg(feature = "path-optimization-default")]
use path_optimization_default::PathOptimizationDefault;
#[cfg(feature = "rectilinear-infill")]
use rectilinear_infill::RectilinearInfill;
#[cfg(feature = "seam-placer")]
use seam_placer::SeamPlacer;
#[cfg(feature = "seam-planner-default")]
use seam_planner_default::SeamPlannerDefault;
#[cfg(feature = "skirt-brim")]
use skirt_brim::SkirtBrim;
#[cfg(feature = "support-surface-ironing")]
use support_surface_ironing::SupportSurfaceIroning;
#[cfg(feature = "top-surface-ironing")]
use top_surface_ironing::TopSurfaceIroning;
#[cfg(feature = "traditional-support")]
use traditional_support::TraditionalSupport;
#[cfg(feature = "tree-support")]
use tree_support::TreeSupport;
#[cfg(feature = "wipe-tower")]
use wipe_tower::WipeTower;

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
macro_rules! manifest_const {
    ($name:ident, $feature:literal, $path:literal) => {
        #[cfg(feature = $feature)]
        const $name: &str = include_str!($path);
    };
}
manifest_const!(
    FUZZY_SKIN_MANIFEST,
    "fuzzy-skin",
    "../../../modules/core-modules/fuzzy-skin/fuzzy-skin.toml"
);
manifest_const!(
    GYROID_INFILL_MANIFEST,
    "gyroid-infill",
    "../../../modules/core-modules/gyroid-infill/gyroid-infill.toml"
);
manifest_const!(
    INFILL_LINKER_MANIFEST,
    "infill-linker",
    "../../../modules/core-modules/infill-linker/infill-linker.toml"
);
manifest_const!(
    LAYER_PLANNER_DEFAULT_MANIFEST,
    "layer-planner-default",
    "../../../modules/core-modules/layer-planner-default/layer-planner-default.toml"
);
manifest_const!(
    LIGHTNING_INFILL_MANIFEST,
    "lightning-infill",
    "../../../modules/core-modules/lightning-infill/lightning-infill.toml"
);
manifest_const!(
    OVERHANG_CLASSIFIER_DEFAULT_MANIFEST,
    "overhang-classifier-default",
    "../../../modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml"
);
manifest_const!(
    PART_COOLING_MANIFEST,
    "part-cooling",
    "../../../modules/core-modules/part-cooling/part-cooling.toml"
);
manifest_const!(
    RECTILINEAR_INFILL_MANIFEST,
    "rectilinear-infill",
    "../../../modules/core-modules/rectilinear-infill/rectilinear-infill.toml"
);
manifest_const!(
    SEAM_PLACER_MANIFEST,
    "seam-placer",
    "../../../modules/core-modules/seam-placer/seam-placer.toml"
);
manifest_const!(
    SEAM_PLANNER_DEFAULT_MANIFEST,
    "seam-planner-default",
    "../../../modules/core-modules/seam-planner-default/seam-planner-default.toml"
);
manifest_const!(
    SKIRT_BRIM_MANIFEST,
    "skirt-brim",
    "../../../modules/core-modules/skirt-brim/skirt-brim.toml"
);
manifest_const!(
    SUPPORT_SURFACE_IRONING_MANIFEST,
    "support-surface-ironing",
    "../../../modules/core-modules/support-surface-ironing/support-surface-ironing.toml"
);
manifest_const!(
    TOP_SURFACE_IRONING_MANIFEST,
    "top-surface-ironing",
    "../../../modules/core-modules/top-surface-ironing/top-surface-ironing.toml"
);
manifest_const!(
    TRADITIONAL_SUPPORT_MANIFEST,
    "traditional-support",
    "../../../modules/core-modules/traditional-support/traditional-support.toml"
);
manifest_const!(
    TREE_SUPPORT_MANIFEST,
    "tree-support",
    "../../../modules/core-modules/tree-support/tree-support.toml"
);
manifest_const!(
    WIPE_TOWER_MANIFEST,
    "wipe-tower",
    "../../../modules/core-modules/wipe-tower/wipe-tower.toml"
);
manifest_const!(
    PATH_OPTIMIZATION_DEFAULT_MANIFEST,
    "path-optimization-default",
    "../../../modules/core-modules/path-optimization-default/path-optimization-default.toml"
);
manifest_const!(
    MACHINE_GCODE_EMIT_MANIFEST,
    "machine-gcode-emit",
    "../../../modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml"
);

/// Return the integrated module manifests enabled for this build.
#[allow(clippy::vec_init_then_push)]
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

    #[cfg(feature = "fuzzy-skin")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: FUZZY_SKIN_MANIFEST,
        origin_label: "integrated://fuzzy-skin",
    });
    #[cfg(feature = "gyroid-infill")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: GYROID_INFILL_MANIFEST,
        origin_label: "integrated://gyroid-infill",
    });
    #[cfg(feature = "infill-linker")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: INFILL_LINKER_MANIFEST,
        origin_label: "integrated://infill-linker",
    });
    #[cfg(feature = "layer-planner-default")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: LAYER_PLANNER_DEFAULT_MANIFEST,
        origin_label: "integrated://layer-planner-default",
    });
    #[cfg(feature = "lightning-infill")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: LIGHTNING_INFILL_MANIFEST,
        origin_label: "integrated://lightning-infill",
    });
    #[cfg(feature = "overhang-classifier-default")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: OVERHANG_CLASSIFIER_DEFAULT_MANIFEST,
        origin_label: "integrated://overhang-classifier-default",
    });
    #[cfg(feature = "part-cooling")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: PART_COOLING_MANIFEST,
        origin_label: "integrated://part-cooling",
    });
    #[cfg(feature = "rectilinear-infill")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: RECTILINEAR_INFILL_MANIFEST,
        origin_label: "integrated://rectilinear-infill",
    });
    #[cfg(feature = "seam-placer")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: SEAM_PLACER_MANIFEST,
        origin_label: "integrated://seam-placer",
    });
    #[cfg(feature = "seam-planner-default")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: SEAM_PLANNER_DEFAULT_MANIFEST,
        origin_label: "integrated://seam-planner-default",
    });
    #[cfg(feature = "skirt-brim")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: SKIRT_BRIM_MANIFEST,
        origin_label: "integrated://skirt-brim",
    });
    #[cfg(feature = "support-surface-ironing")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: SUPPORT_SURFACE_IRONING_MANIFEST,
        origin_label: "integrated://support-surface-ironing",
    });
    #[cfg(feature = "top-surface-ironing")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: TOP_SURFACE_IRONING_MANIFEST,
        origin_label: "integrated://top-surface-ironing",
    });
    #[cfg(feature = "traditional-support")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: TRADITIONAL_SUPPORT_MANIFEST,
        origin_label: "integrated://traditional-support",
    });
    #[cfg(feature = "tree-support")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: TREE_SUPPORT_MANIFEST,
        origin_label: "integrated://tree-support",
    });
    #[cfg(feature = "wipe-tower")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: WIPE_TOWER_MANIFEST,
        origin_label: "integrated://wipe-tower",
    });
    #[cfg(feature = "path-optimization-default")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: PATH_OPTIMIZATION_DEFAULT_MANIFEST,
        origin_label: "integrated://path-optimization-default",
    });
    #[cfg(feature = "machine-gcode-emit")]
    registrations.push(IntegratedModuleRegistration {
        manifest_toml: MACHINE_GCODE_EMIT_MANIFEST,
        origin_label: "integrated://machine-gcode-emit",
    });

    registrations
}

/// Return native entry points for the integrated modules enabled for this build.
#[allow(clippy::vec_init_then_push)]
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

    #[cfg(feature = "fuzzy-skin")]
    entries.push((
        String::from("com.core.fuzzy-skin"),
        FuzzySkinModule::__slicer_native_entry(),
    ));
    #[cfg(feature = "gyroid-infill")]
    entries.push((
        String::from("com.core.gyroid-infill"),
        GyroidInfill::__slicer_native_entry(),
    ));
    #[cfg(feature = "infill-linker")]
    entries.push((
        String::from("com.core.infill-linker"),
        InfillLinker::__slicer_native_entry(),
    ));
    #[cfg(feature = "layer-planner-default")]
    entries.push((
        String::from("com.core.layer-planner-default"),
        DefaultLayerPlanner::__slicer_native_entry(),
    ));
    #[cfg(feature = "lightning-infill")]
    entries.push((
        String::from("com.core.lightning-infill"),
        LightningInfill::__slicer_native_entry(),
    ));
    #[cfg(feature = "overhang-classifier-default")]
    entries.push((
        String::from("com.core.overhang-classifier-default"),
        OverhangClassifierDefault::__slicer_native_entry(),
    ));
    #[cfg(feature = "part-cooling")]
    entries.push((
        String::from("com.core.part-cooling"),
        PartCooling::__slicer_native_entry(),
    ));
    #[cfg(feature = "rectilinear-infill")]
    entries.push((
        String::from("com.core.rectilinear-infill"),
        RectilinearInfill::__slicer_native_entry(),
    ));
    #[cfg(feature = "seam-placer")]
    entries.push((
        String::from("com.core.seam-placer"),
        SeamPlacer::__slicer_native_entry(),
    ));
    #[cfg(feature = "seam-planner-default")]
    entries.push((
        String::from("com.core.seam-planner-default"),
        SeamPlannerDefault::__slicer_native_entry(),
    ));
    #[cfg(feature = "skirt-brim")]
    entries.push((
        String::from("com.core.skirt-brim"),
        SkirtBrim::__slicer_native_entry(),
    ));
    #[cfg(feature = "support-surface-ironing")]
    entries.push((
        String::from("com.core.support-surface-ironing"),
        SupportSurfaceIroning::__slicer_native_entry(),
    ));
    #[cfg(feature = "top-surface-ironing")]
    entries.push((
        String::from("com.core.top-surface-ironing"),
        TopSurfaceIroning::__slicer_native_entry(),
    ));
    #[cfg(feature = "traditional-support")]
    entries.push((
        String::from("com.core.traditional-support"),
        TraditionalSupport::__slicer_native_entry(),
    ));
    #[cfg(feature = "tree-support")]
    entries.push((
        String::from("com.core.tree-support"),
        TreeSupport::__slicer_native_entry(),
    ));
    #[cfg(feature = "wipe-tower")]
    entries.push((
        String::from("com.core.wipe-tower"),
        WipeTower::__slicer_native_entry(),
    ));
    #[cfg(feature = "path-optimization-default")]
    entries.push((
        String::from("com.core.path-optimization-default"),
        PathOptimizationDefault::__slicer_native_entry(),
    ));
    #[cfg(feature = "machine-gcode-emit")]
    entries.push((
        String::from("com.core.machine-gcode-emit"),
        MachineGcodeEmit::__slicer_native_entry(),
    ));

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

#[cfg(all(
    test,
    not(feature = "classic-perimeters"),
    not(feature = "arachne-perimeters"),
    not(feature = "support-planner"),
    not(feature = "fuzzy-skin"),
    not(feature = "gyroid-infill"),
    not(feature = "infill-linker"),
    not(feature = "layer-planner-default"),
    not(feature = "lightning-infill"),
    not(feature = "overhang-classifier-default"),
    not(feature = "part-cooling"),
    not(feature = "rectilinear-infill"),
    not(feature = "seam-placer"),
    not(feature = "seam-planner-default"),
    not(feature = "skirt-brim"),
    not(feature = "support-surface-ironing"),
    not(feature = "top-surface-ironing"),
    not(feature = "traditional-support"),
    not(feature = "tree-support"),
    not(feature = "wipe-tower"),
    not(feature = "path-optimization-default"),
    not(feature = "machine-gcode-emit")
))]
#[test]
fn integrated_registrations_are_empty_by_default() {
    assert!(integrated_registrations().is_empty());
}

#[cfg(all(
    test,
    feature = "classic-perimeters",
    feature = "arachne-perimeters",
    feature = "support-planner",
    not(feature = "fuzzy-skin"),
    not(feature = "gyroid-infill"),
    not(feature = "infill-linker"),
    not(feature = "layer-planner-default"),
    not(feature = "lightning-infill"),
    not(feature = "overhang-classifier-default"),
    not(feature = "part-cooling"),
    not(feature = "rectilinear-infill"),
    not(feature = "seam-placer"),
    not(feature = "seam-planner-default"),
    not(feature = "skirt-brim"),
    not(feature = "support-surface-ironing"),
    not(feature = "top-surface-ironing"),
    not(feature = "traditional-support"),
    not(feature = "tree-support"),
    not(feature = "wipe-tower"),
    not(feature = "path-optimization-default"),
    not(feature = "machine-gcode-emit")
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

#[cfg(all(
    test,
    feature = "classic-perimeters",
    feature = "arachne-perimeters",
    feature = "support-planner",
    feature = "fuzzy-skin",
    feature = "gyroid-infill",
    feature = "infill-linker",
    feature = "layer-planner-default",
    feature = "lightning-infill",
    feature = "overhang-classifier-default",
    feature = "part-cooling",
    feature = "rectilinear-infill",
    feature = "seam-placer",
    feature = "seam-planner-default",
    feature = "skirt-brim",
    feature = "support-surface-ironing",
    feature = "top-surface-ironing",
    feature = "traditional-support",
    feature = "tree-support",
    feature = "wipe-tower",
    feature = "path-optimization-default",
    feature = "machine-gcode-emit"
))]
mod full_coverage_tests {
    use super::{integrated_registrations, native_entries};
    use slicer_sdk::native::NativeStageEntry;
    use std::collections::BTreeSet;

    const PILOTS: [&str; 3] = [
        "classic-perimeters",
        "arachne-perimeters",
        "support-planner",
    ];
    const NEW: [&str; 18] = [
        "fuzzy-skin",
        "gyroid-infill",
        "infill-linker",
        "layer-planner-default",
        "lightning-infill",
        "overhang-classifier-default",
        "part-cooling",
        "rectilinear-infill",
        "seam-placer",
        "seam-planner-default",
        "skirt-brim",
        "support-surface-ironing",
        "top-surface-ironing",
        "traditional-support",
        "tree-support",
        "wipe-tower",
        "path-optimization-default",
        "machine-gcode-emit",
    ];

    fn expected_ids() -> BTreeSet<String> {
        PILOTS
            .iter()
            .chain(NEW.iter())
            .map(|name| format!("com.core.{name}"))
            .collect()
    }

    #[test]
    fn full_coverage_registrations_match_registered_set() {
        let expected = expected_ids();
        let actual: BTreeSet<_> = integrated_registrations()
            .iter()
            .map(|registration| {
                registration
                    .origin_label
                    .strip_prefix("integrated://")
                    .unwrap()
                    .to_owned()
            })
            .map(|name| format!("com.core.{name}"))
            .collect();
        assert_eq!(actual, expected);

        for name in NEW {
            let registration = integrated_registrations()
                .into_iter()
                .find(|registration| registration.origin_label == format!("integrated://{name}"))
                .unwrap();
            assert!(registration
                .manifest_toml
                .contains(&format!("id           = \"com.core.{name}\"")));
        }
    }

    #[test]
    fn full_coverage_native_entry_families_match_stage_ids() {
        let expected = expected_ids();
        let entries = native_entries();
        let actual: BTreeSet<_> = entries.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(actual, expected);

        for name in [
            "fuzzy-skin",
            "gyroid-infill",
            "infill-linker",
            "lightning-infill",
            "rectilinear-infill",
            "seam-placer",
            "support-surface-ironing",
            "top-surface-ironing",
            "traditional-support",
            "tree-support",
        ] {
            let entry = entries
                .iter()
                .find(|(id, _)| id == &format!("com.core.{name}"))
                .unwrap();
            assert!(matches!(entry.1, NativeStageEntry::Layer(_)));
        }
        for name in ["path-optimization-default"] {
            let entry = entries
                .iter()
                .find(|(id, _)| id == &format!("com.core.{name}"))
                .unwrap();
            assert!(matches!(entry.1, NativeStageEntry::Layer(_)));
        }
        for name in ["layer-planner-default", "seam-planner-default"] {
            let entry = entries
                .iter()
                .find(|(id, _)| id == &format!("com.core.{name}"))
                .unwrap();
            assert!(matches!(entry.1, NativeStageEntry::Prepass(_)));
        }
        for name in [
            "overhang-classifier-default",
            "part-cooling",
            "skirt-brim",
            "wipe-tower",
        ] {
            let entry = entries
                .iter()
                .find(|(id, _)| id == &format!("com.core.{name}"))
                .unwrap();
            assert!(matches!(entry.1, NativeStageEntry::Finalization(_)));
        }
        for name in ["machine-gcode-emit"] {
            let entry = entries
                .iter()
                .find(|(id, _)| id == &format!("com.core.{name}"))
                .unwrap();
            assert!(matches!(entry.1, NativeStageEntry::Postpass(_)));
        }
    }
}
