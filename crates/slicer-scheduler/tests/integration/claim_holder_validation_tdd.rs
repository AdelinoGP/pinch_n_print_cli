//! Startup validation coverage for configured claim-holder selections.

use std::path::PathBuf;

use slicer_ir::SemVer;
use slicer_scheduler::manifest::LoadedModuleBuilder;
use slicer_scheduler::{
    validate_configured_claim_holders, validate_startup_dag_with_configured_holders,
    DagValidationPass, DagValidationRequest, LoadedModule, SchedulerError,
};

fn module(id: &str, claims: &[&str]) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Infill",
        String::new(),
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .claims(claims.iter().map(|claim| (*claim).to_string()).collect())
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

fn request(modules: Vec<LoadedModule>) -> DagValidationRequest {
    // exhaustive: this helper intentionally supplies the complete validation request boundary.
    DagValidationRequest {
        modules,
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

#[test]
fn unmatched_holder_is_a_structured_global_validation_error() {
    let report = validate_startup_dag_with_configured_holders(
        &request(vec![
            module("com.core.gyroid-infill", &["claim:sparse-fill"]),
            module("com.core.rectilinear-infill", &["claim:sparse-fill"]),
        ]),
        &[("claim:sparse-fill", "com.core.missing-infill")],
    );

    let diagnostic = report
        .errors
        .iter()
        .find(|diagnostic| {
            matches!(
                &diagnostic.detail,
                SchedulerError::UnmatchedClaimHolder { .. }
            )
        })
        .expect("an unmatched configured holder must fail validation");
    assert_eq!(diagnostic.pass, DagValidationPass::GlobalClaimConflicts);
    assert!(matches!(
        &diagnostic.detail,
        SchedulerError::UnmatchedClaimHolder {
            claim,
            holder,
            candidates,
        } if claim == "claim:sparse-fill"
            && holder == "com.core.missing-infill"
            && candidates == &["com.core.gyroid-infill", "com.core.rectilinear-infill"]
    ));
}

#[test]
fn matching_module_without_selected_claim_is_a_distinct_validation_error() {
    let report = validate_startup_dag_with_configured_holders(
        &request(vec![
            module("com.core.rectilinear-infill", &["claim:top-fill"]),
            module("com.core.gyroid-infill", &["claim:sparse-fill"]),
        ]),
        &[("claim:sparse-fill", "rectilinear-infill")],
    );

    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::GlobalClaimConflicts
            && matches!(
                &diagnostic.detail,
                SchedulerError::ClaimHolderDoesNotDeclareClaim {
                    claim,
                    holder,
                    matched_modules,
                    candidates,
                } if claim == "claim:sparse-fill"
                    && holder == "rectilinear-infill"
                    && matched_modules == &["com.core.rectilinear-infill"]
                    && candidates == &["com.core.gyroid-infill"]
            )
    }));
}

#[test]
fn full_and_short_holder_names_still_match_declared_claims() {
    let modules = vec![
        module("com.core.rectilinear-infill", &["claim:sparse-fill"]),
        module("com.acme.custom-infill", &["claim:sparse-fill"]),
    ];
    let configured = [
        ("claim:sparse-fill", "rectilinear-infill"),
        ("claim:sparse-fill", "com.acme.custom-infill"),
    ];

    assert!(validate_configured_claim_holders(&modules, &configured).is_empty());
}
