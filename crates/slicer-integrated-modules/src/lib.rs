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
#[cfg(feature = "wave-overhangs")]
use wave_overhangs::WaveOverhangs;
#[cfg(feature = "wipe-tower")]
use wipe_tower::WipeTower;

#[cfg(feature = "arachne-perimeters")]
use arachne_perimeters::ArachnePerimeters;
#[cfg(feature = "classic-perimeters")]
use classic_perimeters::ClassicPerimeters;
#[cfg(feature = "tree-support-planner")]
use tree_support_planner::SupportPlanner;

#[cfg(feature = "classic-perimeters")]
const CLASSIC_PERIMETERS_MANIFEST: &str =
    include_str!("../../../modules/core-modules/classic-perimeters/classic-perimeters.toml");
#[cfg(feature = "arachne-perimeters")]
const ARACHNE_PERIMETERS_MANIFEST: &str =
    include_str!("../../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml");
#[cfg(feature = "tree-support-planner")]
const SUPPORT_PLANNER_MANIFEST: &str =
    include_str!("../../../modules/core-modules/tree-support-planner/tree-support-planner.toml");
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
manifest_const!(
    WAVE_OVERHANGS_MANIFEST,
    "wave-overhangs",
    "../../../modules/core-modules/wave-overhangs/wave-overhangs.toml"
);

macro_rules! integrated_registry {
    ($(($feature:literal, $manifest:ident, $module:ty, $id:literal, $origin:literal, $family:ident)),+ $(,)?) => {
        /// Return the integrated module manifests enabled for this build.
        pub fn integrated_registrations() -> Vec<IntegratedModuleRegistration> {
            vec![$(
                {
                    #[cfg(feature = $feature)]
                    {
                        Some(IntegratedModuleRegistration {
                        manifest_toml: $manifest,
                        origin_label: $origin,
                        })
                    }
                    #[cfg(not(feature = $feature))]
                    {
                        None
                    }
                }
            ),+]
            .into_iter()
            .flatten()
            .collect()
        }

        /// Return native entry points for the integrated modules enabled for this build.
        pub fn native_entries() -> Vec<(String, NativeStageEntry)> {
            vec![$(
                {
                    #[cfg(feature = $feature)]
                    {
                        Some((String::from($id), <$module>::__slicer_native_entry()))
                    }
                    #[cfg(not(feature = $feature))]
                    {
                        None
                    }
                }
            ),+]
            .into_iter()
            .flatten()
            .collect()
        }

        /// Return metadata for every integrated module enabled in this build.
        pub fn integrated_inventory() -> Vec<IntegratedModuleInventory> {
            vec![$(
                {
                    #[cfg(feature = $feature)]
                    {
                        Some(IntegratedModuleInventory {
                            id: $id,
                            origin_label: $origin,
                            stage_family: StageFamily::$family,
                        })
                    }
                    #[cfg(not(feature = $feature))]
                    {
                        None
                    }
                }
            ),+]
            .into_iter()
            .flatten()
            .collect()
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageFamily {
    Layer,
    Prepass,
    Finalization,
    Postpass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegratedModuleInventory {
    pub id: &'static str,
    pub origin_label: &'static str,
    pub stage_family: StageFamily,
}

integrated_registry!(
    (
        "classic-perimeters",
        CLASSIC_PERIMETERS_MANIFEST,
        ClassicPerimeters,
        "com.core.classic-perimeters",
        "integrated://classic-perimeters",
        Layer
    ),
    (
        "arachne-perimeters",
        ARACHNE_PERIMETERS_MANIFEST,
        ArachnePerimeters,
        "com.core.arachne-perimeters",
        "integrated://arachne-perimeters",
        Layer
    ),
    (
        "tree-support-planner",
        SUPPORT_PLANNER_MANIFEST,
        SupportPlanner,
        "com.core.tree-support-planner",
        "integrated://support-planner",
        Prepass
    ),
    (
        "fuzzy-skin",
        FUZZY_SKIN_MANIFEST,
        FuzzySkinModule,
        "com.core.fuzzy-skin",
        "integrated://fuzzy-skin",
        Layer
    ),
    (
        "gyroid-infill",
        GYROID_INFILL_MANIFEST,
        GyroidInfill,
        "com.core.gyroid-infill",
        "integrated://gyroid-infill",
        Layer
    ),
    (
        "infill-linker",
        INFILL_LINKER_MANIFEST,
        InfillLinker,
        "com.core.infill-linker",
        "integrated://infill-linker",
        Layer
    ),
    (
        "layer-planner-default",
        LAYER_PLANNER_DEFAULT_MANIFEST,
        DefaultLayerPlanner,
        "com.core.layer-planner-default",
        "integrated://layer-planner-default",
        Prepass
    ),
    (
        "lightning-infill",
        LIGHTNING_INFILL_MANIFEST,
        LightningInfill,
        "com.core.lightning-infill",
        "integrated://lightning-infill",
        Layer
    ),
    (
        "overhang-classifier-default",
        OVERHANG_CLASSIFIER_DEFAULT_MANIFEST,
        OverhangClassifierDefault,
        "com.core.overhang-classifier-default",
        "integrated://overhang-classifier-default",
        Finalization
    ),
    (
        "part-cooling",
        PART_COOLING_MANIFEST,
        PartCooling,
        "com.core.part-cooling",
        "integrated://part-cooling",
        Finalization
    ),
    (
        "rectilinear-infill",
        RECTILINEAR_INFILL_MANIFEST,
        RectilinearInfill,
        "com.core.rectilinear-infill",
        "integrated://rectilinear-infill",
        Layer
    ),
    (
        "wave-overhangs",
        WAVE_OVERHANGS_MANIFEST,
        WaveOverhangs,
        "com.core.wave-overhangs",
        "integrated://wave-overhangs",
        Layer
    ),
    (
        "seam-placer",
        SEAM_PLACER_MANIFEST,
        SeamPlacer,
        "com.core.seam-placer",
        "integrated://seam-placer",
        Layer
    ),
    (
        "seam-planner-default",
        SEAM_PLANNER_DEFAULT_MANIFEST,
        SeamPlannerDefault,
        "com.core.seam-planner-default",
        "integrated://seam-planner-default",
        Prepass
    ),
    (
        "skirt-brim",
        SKIRT_BRIM_MANIFEST,
        SkirtBrim,
        "com.core.skirt-brim",
        "integrated://skirt-brim",
        Finalization
    ),
    (
        "support-surface-ironing",
        SUPPORT_SURFACE_IRONING_MANIFEST,
        SupportSurfaceIroning,
        "com.core.support-surface-ironing",
        "integrated://support-surface-ironing",
        Layer
    ),
    (
        "top-surface-ironing",
        TOP_SURFACE_IRONING_MANIFEST,
        TopSurfaceIroning,
        "com.core.top-surface-ironing",
        "integrated://top-surface-ironing",
        Layer
    ),
    (
        "traditional-support",
        TRADITIONAL_SUPPORT_MANIFEST,
        TraditionalSupport,
        "com.core.traditional-support",
        "integrated://traditional-support",
        Layer
    ),
    (
        "tree-support",
        TREE_SUPPORT_MANIFEST,
        TreeSupport,
        "com.core.tree-support",
        "integrated://tree-support",
        Layer
    ),
    (
        "wipe-tower",
        WIPE_TOWER_MANIFEST,
        WipeTower,
        "com.core.wipe-tower",
        "integrated://wipe-tower",
        Finalization
    ),
    (
        "path-optimization-default",
        PATH_OPTIMIZATION_DEFAULT_MANIFEST,
        PathOptimizationDefault,
        "com.core.path-optimization-default",
        "integrated://path-optimization-default",
        Layer
    ),
    (
        "machine-gcode-emit",
        MACHINE_GCODE_EMIT_MANIFEST,
        MachineGcodeEmit,
        "com.core.machine-gcode-emit",
        "integrated://machine-gcode-emit",
        Postpass
    ),
);

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
    not(feature = "tree-support-planner"),
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
    not(feature = "machine-gcode-emit"),
    not(feature = "wave-overhangs")
))]
#[test]
fn integrated_registrations_are_empty_by_default() {
    assert!(integrated_registrations().is_empty());
}

#[cfg(all(
    test,
    feature = "classic-perimeters",
    feature = "arachne-perimeters",
    feature = "tree-support-planner",
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
    not(feature = "machine-gcode-emit"),
    not(feature = "wave-overhangs")
))]
mod hybrid_pilot_tests {
    use super::{integrated_registrations, native_entries};
    use slicer_sdk::native::NativeStageEntry;

    const IDS: [&str; 3] = [
        "com.core.classic-perimeters",
        "com.core.arachne-perimeters",
        "com.core.tree-support-planner",
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
            .find(|(id, _)| id == "com.core.tree-support-planner")
            .unwrap();
        assert!(matches!(support.1, NativeStageEntry::Prepass(_)));
    }
}

#[cfg(all(
    test,
    feature = "classic-perimeters",
    feature = "arachne-perimeters",
    feature = "tree-support-planner",
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
    feature = "machine-gcode-emit",
    feature = "wave-overhangs"
))]
mod full_coverage_tests {
    use super::{integrated_inventory, integrated_registrations, native_entries, StageFamily};
    use slicer_sdk::native::NativeStageEntry;
    use std::collections::BTreeSet;

    fn expected_ids() -> BTreeSet<String> {
        integrated_inventory()
            .iter()
            .map(|module| module.id.to_owned())
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

        let expected_order: Vec<_> = integrated_inventory()
            .iter()
            .map(|module| module.origin_label)
            .collect();
        let actual_order: Vec<_> = integrated_registrations()
            .iter()
            .map(|registration| registration.origin_label)
            .collect();
        assert_eq!(actual_order, expected_order);

        for module in integrated_inventory() {
            let registration = integrated_registrations()
                .into_iter()
                .find(|registration| registration.origin_label == module.origin_label)
                .unwrap();
            assert!(registration
                .manifest_toml
                .contains(&format!("id           = \"{}\"", module.id)));
        }
    }

    #[test]
    fn full_coverage_native_entry_families_match_stage_ids() {
        let expected = expected_ids();
        let entries = native_entries();
        let actual: BTreeSet<_> = entries.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(actual, expected);

        let expected_order: Vec<_> = integrated_inventory()
            .iter()
            .map(|module| module.id)
            .collect();
        let actual_order: Vec<_> = entries.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(actual_order, expected_order);

        for module in integrated_inventory() {
            let entry = entries.iter().find(|(id, _)| id == module.id).unwrap();
            match module.stage_family {
                StageFamily::Layer => assert!(matches!(entry.1, NativeStageEntry::Layer(_))),
                StageFamily::Prepass => assert!(matches!(entry.1, NativeStageEntry::Prepass(_))),
                StageFamily::Finalization => {
                    assert!(matches!(entry.1, NativeStageEntry::Finalization(_)))
                }
                StageFamily::Postpass => assert!(matches!(entry.1, NativeStageEntry::Postpass(_))),
            }
        }
    }
}
