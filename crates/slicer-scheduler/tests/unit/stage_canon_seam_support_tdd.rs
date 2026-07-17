//! Regression coverage for the stage-order canonicalisation (packet 76, 3d).
//!
//! The validator's stage allowlist (`validation::stage_order_index`) had
//! silently dropped `PrePass::SeamPlanning`, `PrePass::SupportGeometry`, and
//! `Layer::PaintRegionAnnotation`. Because that map doubles as the membership
//! check for a module's own declared `module.stage`
//! (`validation::validate_stage_ids`), a module legitimately declaring one of
//! those stages was rejected at startup with `SchedulerError::UnknownStage`,
//! even though `slicer_schema::STAGES` lists seam/support as module-declarable
//! and the `seam-planner-default` / `support-planner` core modules exist.
//!
//! These tests pin the fix: such a module must pass stage-id validation.

use std::path::PathBuf;

use slicer_ir::SemVer;
use slicer_scheduler::{
    validate_startup_dag, DagValidationPass, DagValidationRequest, SchedulerError,
};

fn module(
    id: &str,
    stage: &str,
    wit_world: &str,
    writes: &[&str],
) -> slicer_scheduler::LoadedModule {
    slicer_scheduler::manifest::LoadedModuleBuilder::new(
        id.to_string(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage.to_string(),
        wit_world.to_string(),
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_writes(writes.iter().map(|s| s.to_string()).collect())
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build()
}

fn unknown_stage_errors(
    report: &slicer_scheduler::validation::DagValidationReport,
) -> Vec<&SchedulerError> {
    report
        .errors
        .iter()
        .filter(|d| d.pass == DagValidationPass::StageIdValidation)
        .map(|d| &d.detail)
        .collect()
}

#[test]
fn prepass_seam_planning_module_is_not_unknown_stage() {
    let m = module(
        "com.example.seam-planner",
        "PrePass::SeamPlanning",
        slicer_schema::WORLD_PREPASS,
        &["SeamPlanIR.entries"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        stage_dags: Vec::new(),
        host_ir_schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        host_version: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };
    let report = validate_startup_dag(&request);
    assert!(
        unknown_stage_errors(&report).is_empty(),
        "PrePass::SeamPlanning must be an accepted module stage, got: {:?}",
        unknown_stage_errors(&report)
    );
}

#[test]
fn prepass_support_geometry_module_is_not_unknown_stage() {
    let m = module(
        "com.example.support-planner",
        "PrePass::SupportGeometry",
        slicer_schema::WORLD_PREPASS,
        &["SupportGeometryIR.regions"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        stage_dags: Vec::new(),
        host_ir_schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        host_version: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };
    let report = validate_startup_dag(&request);
    assert!(
        unknown_stage_errors(&report).is_empty(),
        "PrePass::SupportGeometry must be an accepted module stage, got: {:?}",
        unknown_stage_errors(&report)
    );
}

#[test]
fn layer_paint_region_annotation_module_is_not_unknown_stage() {
    let m = module(
        "com.example.paint-region-annotator",
        "Layer::PaintRegionAnnotation",
        slicer_schema::WORLD_LAYER,
        &["PaintRegionIR.per_layer"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        stage_dags: Vec::new(),
        host_ir_schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        host_version: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };
    let report = validate_startup_dag(&request);
    assert!(
        unknown_stage_errors(&report).is_empty(),
        "Layer::PaintRegionAnnotation must be an accepted module stage, got: {:?}",
        unknown_stage_errors(&report)
    );
}

#[test]
fn genuinely_unknown_stage_is_still_rejected() {
    let m = module(
        "com.example.bogus",
        "PrePass::NotARealStage",
        slicer_schema::WORLD_PREPASS,
        &["SomeIR.field"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        stage_dags: Vec::new(),
        host_ir_schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        host_version: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };
    let report = validate_startup_dag(&request);
    assert!(
        unknown_stage_errors(&report)
            .iter()
            .any(|e| matches!(e, SchedulerError::UnknownStage { .. })),
        "a misspelled stage must still raise UnknownStage"
    );
}
