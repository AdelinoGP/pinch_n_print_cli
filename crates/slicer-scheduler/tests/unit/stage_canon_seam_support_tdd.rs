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
use slicer_scheduler::dag::ModuleNode;
use slicer_scheduler::validation::{ClaimHolder, ConflictScope, StageDag};
use slicer_scheduler::{
    validate_startup_dag, DagValidationPass, DagValidationRequest, SchedulerError,
};

fn module(id: &str, stage: &str, writes: &[&str]) -> slicer_scheduler::LoadedModule {
    slicer_scheduler::manifest::LoadedModuleBuilder::new(
        id.to_string(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage.to_string(),
        String::new(),
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

fn dag_validation_request_base() -> DagValidationRequest {
    // Shared non-default fixture; stage tests override only the module list.
    // exhaustive: this helper is the single construction point for the no-default base.
    DagValidationRequest {
        modules: Vec::new(),
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
    }
}

fn support_claim_holders(module_ids: &[&str]) -> Vec<ClaimHolder> {
    [
        "support-generator",
        "support-planner",
        "support-family:traditional",
        "support-family:tree",
    ]
    .into_iter()
    .flat_map(|claim| {
        module_ids.iter().map(move |module_id| ClaimHolder {
            claim: claim.to_string(),
            module_id: (*module_id).to_string(),
            scope: ConflictScope::Global,
        })
    })
    .collect()
}

fn support_write_stage(module_ids: &[&str]) -> StageDag {
    StageDag {
        stage: "PrePass::SupportGeometry".to_string(),
        nodes: module_ids
            .iter()
            .map(|module_id| ModuleNode {
                module_id: (*module_id).to_string(),
                edges_to: Vec::new(),
                ir_reads: Vec::new(),
                ir_writes: vec!["SupportPlanIR".to_string(), "SupportIR".to_string()],
            })
            .collect(),
    }
}

#[test]
fn family_scoped_support_claims_do_not_conflict_globally() {
    let module_ids = ["support-traditional", "support-tree"];
    let request = DagValidationRequest {
        modules: module_ids
            .iter()
            .map(|id| {
                module(
                    id,
                    "PrePass::SupportGeometry",
                    &["SupportPlanIR", "SupportIR"],
                )
            })
            .collect(),
        claim_holders: support_claim_holders(&module_ids),
        stage_dags: vec![support_write_stage(&module_ids)],
        ..dag_validation_request_base()
    };

    let report = validate_startup_dag(&request);
    assert!(
        report.errors.iter().all(|diagnostic| {
            !matches!(diagnostic.detail, SchedulerError::ClaimConflict { .. })
                && !matches!(diagnostic.detail, SchedulerError::WriteConflict { .. })
        }),
        "family-scoped support topology emitted advisories: {:?}",
        report.errors
    );
}

#[test]
fn genuine_claim_conflict_still_rejected_after_family_exemption() {
    let request = DagValidationRequest {
        claim_holders: vec![
            ClaimHolder {
                claim: "mesh-analyzer".to_string(),
                module_id: "analyzer-a".to_string(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: "mesh-analyzer".to_string(),
                module_id: "analyzer-b".to_string(),
                scope: ConflictScope::Global,
            },
        ],
        ..dag_validation_request_base()
    };

    let report = validate_startup_dag(&request);
    assert!(report.errors.iter().any(|diagnostic| matches!(
        diagnostic.detail,
        SchedulerError::ClaimConflict { ref claim, .. } if claim == "mesh-analyzer"
    )));
}

#[test]
fn genuine_write_conflict_still_rejected_after_aggregation_recognition() {
    let module_ids = ["unrelated-a", "unrelated-b"];
    let request = DagValidationRequest {
        modules: module_ids
            .iter()
            .map(|id| module(id, "PrePass::SupportGeometry", &["SharedIR.field"]))
            .collect(),
        stage_dags: vec![StageDag {
            stage: "PrePass::SupportGeometry".to_string(),
            nodes: module_ids
                .iter()
                .map(|module_id| ModuleNode {
                    module_id: (*module_id).to_string(),
                    edges_to: Vec::new(),
                    ir_reads: Vec::new(),
                    ir_writes: vec!["SharedIR.field".to_string()],
                })
                .collect(),
        }],
        ..dag_validation_request_base()
    };

    let report = validate_startup_dag(&request);
    assert!(report.errors.iter().any(|diagnostic| matches!(
        diagnostic.detail,
        SchedulerError::WriteConflict { ref field, .. } if field == "SharedIR.field"
    )));
}

#[test]
fn prepass_seam_planning_module_is_not_unknown_stage() {
    let m = module(
        "com.example.seam-planner",
        "PrePass::SeamPlanning",
        &["SeamPlanIR.entries"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        ..dag_validation_request_base()
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
        &["SupportGeometryIR.regions"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        ..dag_validation_request_base()
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
        &["PaintRegionIR.per_layer"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        ..dag_validation_request_base()
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
        &["SomeIR.field"],
    );
    let request = DagValidationRequest {
        modules: vec![m],
        ..dag_validation_request_base()
    };
    let report = validate_startup_dag(&request);
    assert!(
        unknown_stage_errors(&report)
            .iter()
            .any(|e| matches!(e, SchedulerError::UnknownStage { .. })),
        "a misspelled stage must still raise UnknownStage"
    );
}
