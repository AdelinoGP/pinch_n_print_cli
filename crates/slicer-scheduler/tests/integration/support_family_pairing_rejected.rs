use slicer_ir::SemVer;
use slicer_scheduler::{validate_support_family_pairing, LoadedModuleBuilder};
use std::path::PathBuf;

pub fn support_family_pairing_rejected() {
    let module = LoadedModuleBuilder::new(
        "com.test.planner",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "PrePass::SupportGeometry",
        slicer_schema::TIER_LAYER,
        PathBuf::from("planner.wasm"),
    )
    .claims(vec![
        "support-planner".into(),
        "support-family:missing".into(),
    ])
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
    .build();
    let error =
        validate_support_family_pairing(&[module]).expect_err("unpaired family must fail startup");
    assert_eq!(error.missing_renderers, vec!["missing"]);
    assert!(error.missing_planners.is_empty());
}
