#![allow(missing_docs)]

//! AC-N1 (packet 151): spiral-vase dispatch — arachne stays selected when
//! spiral is INACTIVE; classic is forced when spiral is ACTIVE (even with
//! wall_generator=arachne).
//!
//! The G8 gap test (`arachne_parity_pipeline_spiral_vase_forces_classic_generator`)
//! is a static source-text probe and does not exercise the actual behavior.
//! This contract test exercises the real `dedup_same_claim_modules_*` API.

use slicer_ir::SemVer;
use slicer_scheduler::{
    dedup_same_claim_modules_with_wall_generator, LoadDiagnostic, LoadedModule, LoadedModuleBuilder,
};
use std::path::PathBuf;

// Module ids match the private `CLASSIC_PERIMETERS_MODULE_ID` /
// `ARACHNE_PERIMETERS_MODULE_ID` consts in execution_plan.rs.
const CLASSIC_PERIMETERS_MODULE_ID: &str = "com.core.classic-perimeters";
const ARACHNE_PERIMETERS_MODULE_ID: &str = "com.core.arachne-perimeters";

const PERIMETER_STAGE: &str = "Layer::Perimeters";
const PERIMETER_GENERATOR_CLAIM: &str = "perimeter-generator";

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn perimeter_module(id: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(0, 1, 0),
        PERIMETER_STAGE,
        slicer_schema::WORLD_LAYER,
        PathBuf::from("/tmp/placeholder.wasm"),
    )
    .claims(vec![PERIMETER_GENERATOR_CLAIM.to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .placeholder_wasm(true)
    .build()
}

fn two_module_set() -> Vec<LoadedModule> {
    // Both modules claim the perimeter-generator role in the same stage; dedup
    // must pick exactly one based on wall_generator + spiral_vase.
    vec![
        perimeter_module(CLASSIC_PERIMETERS_MODULE_ID),
        perimeter_module(ARACHNE_PERIMETERS_MODULE_ID),
    ]
}

fn kept_ids(
    modules: &mut Vec<LoadedModule>,
    wall_generator: Option<&str>,
    spiral_vase: bool,
) -> Vec<String> {
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let kept = dedup_same_claim_modules_with_wall_generator(
        modules,
        &mut diagnostics,
        wall_generator,
        spiral_vase,
    );
    kept.iter().map(|m| m.id().to_string()).collect()
}

#[test]
fn spiral_vase_arachne_dispatch_inactive_keeps_arachne() {
    // wall_generator=arachne, spiral_vase=false → arachne wins.
    let mut modules = two_module_set();
    let ids = kept_ids(&mut modules, Some("arachne"), false);
    assert!(
        ids.contains(&ARACHNE_PERIMETERS_MODULE_ID.to_string())
            && !ids.contains(&CLASSIC_PERIMETERS_MODULE_ID.to_string()),
        "spiral_vase=false + wall_generator=arachne must keep the arachne module; got {:?}",
        ids
    );
}

#[test]
fn spiral_vase_arachne_dispatch_active_forces_classic() {
    // wall_generator=arachne, spiral_vase=true → classic must be forced.
    let mut modules = two_module_set();
    let ids = kept_ids(&mut modules, Some("arachne"), true);
    assert!(
        ids.contains(&CLASSIC_PERIMETERS_MODULE_ID.to_string())
            && !ids.contains(&ARACHNE_PERIMETERS_MODULE_ID.to_string()),
        "spiral_vase=true must force classic-perimeters regardless of \
         wall_generator=arachne; got {:?}",
        ids
    );
}
