#![allow(missing_docs)]

//! Gap-bridging tests for docs/01_system_architecture.md Â§"Allowed Claim
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
//! treated as *different* scopes â€” so per-layer transitions of a
//! non-transitionable claim silently pass. These tests lock down the
//! required behavior and expose the gap.

use slicer_ir::SemVer;
use slicer_runtime::{
    build_intra_stage_dag, validate_startup_dag, ClaimHolder, ConflictScope, DagValidationRequest,
    LoadedModule, LoadedModuleBuilder, Producer, SchedulerError, StageDag,
};
use std::path::PathBuf;

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn loaded_module(id: &str, stage: &str, claim: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(0, 1, 0),
        stage,
        "slicer:world-layer@1.0.0",
        PathBuf::from("/tmp/placeholder.wasm"),
    )
    .claims(vec![claim.to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .placeholder_wasm(true)
    .build()
}

fn stage_dag_for(stage: &str, modules: &[LoadedModule]) -> StageDag {
    let producers: Vec<&dyn Producer> = modules.iter().map(|m| m as &dyn Producer).collect();
    let nodes =
        build_intra_stage_dag(stage.to_string(), &producers).expect("intra-stage DAG build");
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
    // Defensive: satisfy packet-37 fill-role coverage so that a future tightening
    // of validate_startup_dag â€” which would surface MissingDependency
    // independent of the conflict short-circuit â€” does not silently start
    // failing these tests. The claim-under-test is unrelated to fill roles.
    let mut claim_holders = vec![
        ClaimHolder {
            claim: claim.to_string(),
            module_id: module_a.id().to_string(),
            scope: region_scope_at(0),
        },
        ClaimHolder {
            claim: claim.to_string(),
            module_id: module_b.id().to_string(),
            scope: region_scope_at(10),
        },
    ];
    for fill in [
        "claim:top-fill",
        "claim:bottom-fill",
        "claim:bridge-fill",
        "claim:sparse-fill",
    ] {
        claim_holders.push(ClaimHolder {
            claim: fill.to_string(),
            module_id: module_a.id().to_string(),
            scope: ConflictScope::Global,
        });
    }
    DagValidationRequest {
        modules: vec![module_a.clone(), module_b.clone()],
        stage_dags: vec![stage_dag_for(stage, &[module_a.clone(), module_b.clone()])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders,
        access_audits: Vec::new(),
    }
}

fn report_has_transition_error(report: &slicer_runtime::DagValidationReport, claim: &str) -> bool {
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Test 1 â€” perimeter-generator layer variance is a fatal validation error
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
        "docs/01 Â§'Allowed Claim Transition Matrix' marks perimeter-generator \
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Test 2 â€” seam-placer, layer-planner, mesh-analyzer same treatment
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Test 3 â€” transitionable claims are allowed to vary per layer
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
             Â§'Allowed Claim Transition Matrix', but validation rejected the \
             configuration: {:#?}",
            report.errors
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Test 4 â€” same holder across layers stays valid (no false positive)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// Sanity check: two claim-holder records for the SAME module at different
// layer indices is "stable across layers" â€” explicitly allowed by the
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
                module_id: alpha.id().to_string(),
                scope: region_scope_at(0),
            },
            ClaimHolder {
                claim: String::from("perimeter-generator"),
                module_id: alpha.id().to_string(),
                scope: region_scope_at(10),
            },
            // Packet 37 fill-role coverage: every fill-role claim must have
            // a holder for `validate_claim_conflicts(global_only=true)`. The
            // test's purpose is non-transitionable-claim cross-layer
            // stability for perimeter-generator; satisfying these is
            // sufficient for global startup validation.
            ClaimHolder {
                claim: String::from("claim:top-fill"),
                module_id: alpha.id().to_string(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: String::from("claim:bottom-fill"),
                module_id: alpha.id().to_string(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: String::from("claim:bridge-fill"),
                module_id: alpha.id().to_string(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: String::from("claim:sparse-fill"),
                module_id: alpha.id().to_string(),
                scope: ConflictScope::Global,
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
