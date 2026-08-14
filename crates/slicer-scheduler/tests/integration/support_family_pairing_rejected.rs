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
    let error = validate_support_family_pairing(&[module(
        "planner-only",
        &["support-planner", "support-family:planner-only"],
    )])
    .expect_err("planner without renderer must fail startup");
    assert_eq!(error.missing_renderers, vec!["planner-only"]);
    assert!(error.missing_planners.is_empty());

    let error = validate_support_family_pairing(&[module(
        "renderer-only",
        &["support-generator", "support-family:renderer-only"],
    )])
    .expect_err("renderer without planner must fail startup");
    assert!(error.missing_renderers.is_empty());
    assert_eq!(error.missing_planners, vec!["renderer-only"]);

    let error = validate_support_family_pairing(&[
        module(
            "planner",
            &["support-planner", "support-family:planner-family"],
        ),
        module(
            "renderer",
            &["support-generator", "support-family:renderer-family"],
        ),
    ])
    .expect_err("mismatched planner and renderer families must fail startup");
    assert_eq!(error.missing_renderers, vec!["planner-family"]);
    assert_eq!(error.missing_planners, vec!["renderer-family"]);
}
