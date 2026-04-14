#![allow(missing_docs)]

use slicer_cli::cmd_validate::{parse_manifest, validate_stage, validate_wit_world};

#[test]
fn validate_accepts_architectural_stage_matrix() {
    for stage in [
        "PrePass::MeshSegmentation",
        "PrePass::MeshAnalysis",
        "PrePass::LayerPlanning",
        "PrePass::PaintSegmentation",
        "Layer::SlicePostProcess",
        "Layer::Perimeters",
        "Layer::PerimetersPostProcess",
        "Layer::Infill",
        "Layer::InfillPostProcess",
        "Layer::Support",
        "Layer::SupportPostProcess",
        "Layer::PathOptimization",
        "PostPass::LayerFinalization",
        "PostPass::GCodePostProcess",
        "PostPass::TextPostProcess",
    ] {
        let manifest = parse_manifest(&format!(
            r#"
            [module]
            id = "com.example.test"
            version = "0.1.0"
            display-name = "Test"
            description = "fixture"
            author = "tester"
            license = "MIT"
            wit-world = "slicer:world-layer@1.0.0"

            [stage]
            id = "{stage}"
            "#
        ))
        .expect("manifest should parse");

        validate_stage(&manifest)
            .unwrap_or_else(|error| panic!("documented stage should validate: {stage}: {error}"));
    }
}

#[test]
fn validate_accepts_documented_wit_worlds() {
    for wit_world in [
        "slicer:world-layer@1.0.0",
        "slicer:world-prepass@1.0.0",
        "slicer:world-finalization@1.0.0",
        "slicer:world-postpass@1.0.0",
    ] {
        let manifest = parse_manifest(&format!(
            r#"
            [module]
            id = "com.example.test"
            version = "0.1.0"
            display-name = "Test"
            description = "fixture"
            author = "tester"
            license = "MIT"
            wit-world = "{wit_world}"

            [stage]
            id = "Layer::Infill"
            "#
        ))
        .expect("manifest should parse");

        validate_wit_world(&manifest).unwrap_or_else(|error| {
            panic!("documented wit world should validate: {wit_world}: {error}")
        });
    }
}
