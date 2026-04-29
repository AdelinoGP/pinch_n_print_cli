#![allow(missing_docs)]

//! Gap-bridging tests for docs/01_system_architecture.md §"Allowed Claim
//! Transition Matrix" (lines 524-538).
//!
//! Docs specify, normatively:
//!
//!   | Claim                | Allowed per-layer transition | Notes |
//!   |----------------------|------------------------------|-------|
//!   | infill-generator     | Yes                          | ...   |
//!   | support-generator    | Yes                          | ...   |
//!   | perimeter-generator  | No                           | ...   |
//!   | seam-placer          | No                           | ...   |
//!   | layer-planner        | No                           | ...   |
//!   | mesh-analyzer        | No                           | ...   |
//!
//!   "If a claim is marked non-transitionable, any layer-varying holder
//!    selection is a startup validation error."
//!
//! The current `validate_startup_dag` pipeline reports two holders of the
//! same claim in the same scope, but because `ConflictScope::Region.global_
//! layer_index` is part of the scope key, different layer indices are
//! treated as *different* scopes — so per-layer transitions of a
//! non-transitionable claim silently pass. These tests lock down the
//! required behavior and expose the gap.

use slicer_host::{
    build_intra_stage_dag, validate_startup_dag, ClaimHolder, ConflictScope, DagValidationRequest,
    LoadedModule, SchedulerError, StageDag,
};
use slicer_ir::SemVer;
use std::path::PathBuf;

// ── Helpers ───────────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn loaded_module(id: &str, stage: &str, claim: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(0, 1, 0),
        stage: stage.to_string(),
        wit_world: String::from("slicer:world-layer@1.0.0"),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: vec![claim.to_string()],
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: slicer_host::ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: true,
        wasm_path: PathBuf::from("/tmp/placeholder.wasm"),
        placeholder_wasm: true,
    }
}

fn stage_dag_for(stage: &str, modules: &[LoadedModule]) -> StageDag {
    let nodes = build_intra_stage_dag(stage.to_string(), modules).expect("intra-stage DAG build");
    StageDag {
        stage: stage.to_string(),
        nodes,
    }
}

fn region_scope_at(layer: u32) -> ConflictScope {
    ConflictScope::Region {
        object_id: String::from("cube"),
        region_id: String::from("shell"),
        global_layer_index: Some(layer),
    }
}

fn request_with_per_layer_transition(
    stage: &str,
    claim: &str,
    module_a: &LoadedModule,
    module_b: &LoadedModule,
) -> DagValidationRequest {
    DagValidationRequest {
        modules: vec![module_a.clone(), module_b.clone()],
        stage_dags: vec![stage_dag_for(stage, &[module_a.clone(), module_b.clone()])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: vec![
            ClaimHolder {
                claim: claim.to_string(),
                module_id: module_a.id.clone(),
                scope: region_scope_at(0),
            },
            ClaimHolder {
                claim: claim.to_string(),
                module_id: module_b.id.clone(),
                scope: region_scope_at(10),
            },
        ],
        access_audits: Vec::new(),
    }
}

fn report_has_transition_error(report: &slicer_host::DagValidationReport, claim: &str) -> bool {
    report.errors.iter().any(|d| match &d.detail {
        // Either a new dedicated variant...
        //   SchedulerError::ClaimTransitionViolation { claim: c, .. } => c == claim,
        // ...or a re-use of ClaimConflict with the (object,region) pair and
        // differing layer indices. Accept either encoding until the doc is
        // tightened.
        SchedulerError::ClaimConflict {
            claim: c,
            scope: ConflictScope::Region { .. },
            ..
        } => c == claim,
        _ => false,
    })
}

// ──────────────────────────────────────────────────────────────────────────
// Test 1 — perimeter-generator layer variance is a fatal validation error
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn non_transitionable_perimeter_generator_rejects_layer_varying_holder() {
    let alpha = loaded_module(
        "com.example.alpha-perimeters",
        "Layer::Perimeters",
        "perimeter-generator",
    );
    let beta = loaded_module(
        "com.example.beta-perimeters",
        "Layer::Perimeters",
        "perimeter-generator",
    );

    let request = request_with_per_layer_transition(
        "Layer::Perimeters",
        "perimeter-generator",
        &alpha,
        &beta,
    );
    let report = validate_startup_dag(&request);

    assert!(
        !report.is_valid(),
        "docs/01 §'Allowed Claim Transition Matrix' marks perimeter-generator \
         as non-transitionable. Layer-varying holder selection must fail \
         startup validation, but the report is clean: {:#?}",
        report
    );
    assert!(
        report_has_transition_error(&report, "perimeter-generator"),
        "expected a ClaimConflict/ClaimTransitionViolation diagnostic for \
         'perimeter-generator'; got errors: {:#?}",
        report.errors
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 2 — seam-placer, layer-planner, mesh-analyzer same treatment
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn non_transitionable_seam_placer_layer_planner_and_mesh_analyzer_reject_variance() {
    for (stage, claim) in [
        ("Layer::PerimetersPostProcess", "seam-placer"),
        ("PrePass::LayerPlanning", "layer-planner"),
        ("PrePass::MeshAnalysis", "mesh-analyzer"),
    ] {
        let alpha = loaded_module(&format!("com.example.{claim}-alpha"), stage, claim);
        let beta = loaded_module(&format!("com.example.{claim}-beta"), stage, claim);

        let request = request_with_per_layer_transition(stage, claim, &alpha, &beta);
        let report = validate_startup_dag(&request);

        assert!(
            !report.is_valid(),
            "claim '{claim}' is non-transitionable per docs/01 but validation \
             accepted a layer-varying holder selection: {:#?}",
            report
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Test 3 — transitionable claims are allowed to vary per layer
// ──────────────────────────────────────────────────────────────────────────
//
// Positive test: guards against an over-zealous fix that would reject ALL
// per-layer claim variance. Only non-transitionable rows must fail.

#[test]
fn transitionable_infill_and_support_generators_allow_per_layer_variance() {
    for (stage, claim) in [
        ("Layer::Infill", "infill-generator"),
        ("Layer::Support", "support-generator"),
    ] {
        let alpha = loaded_module(&format!("com.example.{claim}-alpha"), stage, claim);
        let beta = loaded_module(&format!("com.example.{claim}-beta"), stage, claim);

        let request = request_with_per_layer_transition(stage, claim, &alpha, &beta);
        let report = validate_startup_dag(&request);

        let has_transition_err = report_has_transition_error(&report, claim);
        assert!(
            !has_transition_err,
            "claim '{claim}' is marked 'Yes' for per-layer transition in docs/01 \
             §'Allowed Claim Transition Matrix', but validation rejected the \
             configuration: {:#?}",
            report.errors
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Test 4 — same holder across layers stays valid (no false positive)
// ──────────────────────────────────────────────────────────────────────────
//
// Sanity check: two claim-holder records for the SAME module at different
// layer indices is "stable across layers" — explicitly allowed by the
// "claim holder must be stable across layers for the same (object, claim)"
// rule at docs/01 line 510.

#[test]
fn stable_holder_across_layers_is_valid_for_non_transitionable_claim() {
    let alpha = loaded_module(
        "com.example.only-perimeters",
        "Layer::Perimeters",
        "perimeter-generator",
    );
    let request = DagValidationRequest {
        modules: vec![alpha.clone()],
        stage_dags: vec![stage_dag_for("Layer::Perimeters", &[alpha.clone()])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: vec![
            ClaimHolder {
                claim: String::from("perimeter-generator"),
                module_id: alpha.id.clone(),
                scope: region_scope_at(0),
            },
            ClaimHolder {
                claim: String::from("perimeter-generator"),
                module_id: alpha.id.clone(),
                scope: region_scope_at(10),
            },
        ],
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(
        report.is_valid(),
        "stable single-holder-across-layers must NOT trigger a transition \
         violation, but got errors: {:#?}",
        report.errors
    );
}
