use slicer_ir::SemVer;
use slicer_scheduler::{validate_support_family_pairing, LoadedModule, LoadedModuleBuilder};
use std::path::PathBuf;

fn module(id: &str, claims: &[&str]) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "PrePass::SupportGeometry",
        slicer_schema::TIER_LAYER,
        PathBuf::from(format!("{id}.wasm")),
    )
    .claims(claims.iter().map(|claim| (*claim).into()).collect())
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

pub fn support_family_pairing_rejected() {
    let warnings = validate_support_family_pairing(&[module(
        "planner-only",
        &["support-planner", "support-family:planner-only"],
    )])
    .expect("planner without renderer must degrade with a warning");
    let error = &warnings[0];
    assert_eq!(error.missing_renderers, vec!["planner-only"]);
    assert!(error.missing_planners.is_empty());

    let warnings = validate_support_family_pairing(&[module(
        "renderer-only",
        &["support-generator", "support-family:renderer-only"],
    )])
    .expect("renderer without planner must degrade with a warning");
    let error = &warnings[0];
    assert!(error.missing_renderers.is_empty());
    assert_eq!(error.missing_planners, vec!["renderer-only"]);

    let warnings = validate_support_family_pairing(&[
        module(
            "planner",
            &["support-planner", "support-family:planner-family"],
        ),
        module(
            "renderer",
            &["support-generator", "support-family:renderer-family"],
        ),
    ])
    .expect("mismatched families must degrade with warnings");
    let error = &warnings[0];
    assert_eq!(error.missing_renderers, vec!["planner-family"]);
    assert_eq!(error.missing_planners, vec!["renderer-family"]);
}
