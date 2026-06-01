//! TDD integration tests for the `pnp_cli module new` command.
//!
//! These tests exercise the full scaffolding pipeline by running `execute_in()`
//! in a temporary directory and verifying the generated file structure.

use std::fs;
use tempfile::TempDir;

/// Helper: run `execute_in` inside a temporary directory and return the temp dir.
fn run_new(name: &str, stage: &str) -> Result<TempDir, pnp_cli::module_new::NewError> {
    let tmp = TempDir::new().unwrap();
    pnp_cli::module_new::execute_in(tmp.path(), name, stage)?;
    Ok(tmp)
}

// ── Default stage ────────────────────────────────────────────────────────────

#[test]
fn default_stage_creates_correct_structure() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let base = tmp.path().join("my-infill");

    assert!(base.join("Cargo.toml").is_file());
    assert!(base.join("my-infill.toml").is_file());
    assert!(base.join("src/lib.rs").is_file());
    assert!(base.join("tests/basic.rs").is_file());
    assert!(base.join("tests/fixtures/square_20mm.json").is_file());
}

#[test]
fn default_stage_cargo_toml_has_cdylib() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let cargo = fs::read_to_string(tmp.path().join("my-infill/Cargo.toml")).unwrap();
    assert!(cargo.contains(r#"crate-type = ["cdylib"]"#));
    assert!(cargo.contains("slicer-sdk"));
    assert!(
        cargo.contains(r#"slicer-sdk = { path = "../../crates/slicer-sdk", features = ["test"] }"#)
    );
    assert!(!cargo.contains("slicer-test"));
}

#[test]
fn default_stage_manifest_is_valid_toml() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let manifest = fs::read_to_string(tmp.path().join("my-infill/my-infill.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&manifest).expect("manifest must be valid TOML");

    // Check required sections.
    assert!(parsed.get("module").is_some());
    assert!(parsed.get("stage").is_some());
    assert!(parsed.get("ir-access").is_some());
    assert!(parsed.get("claims").is_some());
    assert!(parsed.get("compatibility").is_some());
    assert!(parsed.get("hints").is_some());
}

#[test]
fn default_stage_manifest_has_correct_stage_id() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let manifest = fs::read_to_string(tmp.path().join("my-infill/my-infill.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&manifest).unwrap();
    assert_eq!(parsed["stage"]["id"].as_str().unwrap(), "Layer::Infill");
}

#[test]
fn default_stage_lib_rs_has_correct_struct_and_fn() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let lib = fs::read_to_string(tmp.path().join("my-infill/src/lib.rs")).unwrap();
    assert!(lib.contains("pub struct MyInfill"));
    assert!(lib.contains("run_infill"));
}

#[test]
fn fixture_json_is_valid_and_has_20mm_square() {
    let tmp = run_new("my-infill", "Layer::Infill").unwrap();
    let json =
        fs::read_to_string(tmp.path().join("my-infill/tests/fixtures/square_20mm.json")).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).expect("fixture must be valid JSON");
    assert_eq!(val["object_id"], "default");
    // 20mm = 200_000 units (1 unit = 100nm).
    let contour_pts = val["contour"]["points"].as_array().unwrap();
    assert_eq!(contour_pts.len(), 4);
    assert_eq!(contour_pts[1]["x"], 200000);
}

// ── All stages ───────────────────────────────────────────────────────────────

#[test]
fn stage_perimeters() {
    let tmp = run_new("wall-gen", "Layer::Perimeters").unwrap();
    let lib = fs::read_to_string(tmp.path().join("wall-gen/src/lib.rs")).unwrap();
    assert!(lib.contains("run_perimeters"));
    let manifest = fs::read_to_string(tmp.path().join("wall-gen/wall-gen.toml")).unwrap();
    assert!(manifest.contains("Layer::Perimeters"));
}

#[test]
fn stage_perimeters_postprocess() {
    let tmp = run_new("wall-pp", "Layer::PerimetersPostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("wall-pp/src/lib.rs")).unwrap();
    assert!(lib.contains("run_wall_postprocess"));
}

#[test]
fn stage_infill_postprocess() {
    let tmp = run_new("infill-pp", "Layer::InfillPostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("infill-pp/src/lib.rs")).unwrap();
    assert!(lib.contains("run_infill_postprocess"));
}

#[test]
fn stage_slice_postprocess() {
    let tmp = run_new("slice-pp", "Layer::SlicePostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("slice-pp/src/lib.rs")).unwrap();
    assert!(lib.contains("run_slice_postprocess"));
}

#[test]
fn stage_mesh_analysis() {
    let tmp = run_new("mesh-tool", "PrePass::MeshAnalysis").unwrap();
    let lib = fs::read_to_string(tmp.path().join("mesh-tool/src/lib.rs")).unwrap();
    assert!(lib.contains("run_mesh_analysis"));
    let manifest = fs::read_to_string(tmp.path().join("mesh-tool/mesh-tool.toml")).unwrap();
    assert!(manifest.contains("slicer:world-prepass@1.0.0"));
}

#[test]
fn stage_layer_planning() {
    let tmp = run_new("plan-tool", "PrePass::LayerPlanning").unwrap();
    let lib = fs::read_to_string(tmp.path().join("plan-tool/src/lib.rs")).unwrap();
    assert!(lib.contains("run_layer_planning"));
}

#[test]
fn stage_mesh_segmentation_is_scaffoldable_per_architecture() {
    let tmp = run_new("mesh-seg", "PrePass::MeshSegmentation").unwrap();
    let lib = fs::read_to_string(tmp.path().join("mesh-seg/src/lib.rs")).unwrap();
    assert!(lib.contains("run_mesh_segmentation"));
    let manifest = fs::read_to_string(tmp.path().join("mesh-seg/mesh-seg.toml")).unwrap();
    assert!(manifest.contains("slicer:world-prepass@1.0.0"));
}

#[test]
fn stage_paint_segmentation_is_scaffoldable_per_architecture() {
    let tmp = run_new("paint-seg", "PrePass::PaintSegmentation").unwrap();
    let lib = fs::read_to_string(tmp.path().join("paint-seg/src/lib.rs")).unwrap();
    assert!(lib.contains("run_paint_segmentation"));
    let manifest = fs::read_to_string(tmp.path().join("paint-seg/paint-seg.toml")).unwrap();
    assert!(manifest.contains("slicer:world-prepass@1.0.0"));
}

#[test]
fn stage_support_is_scaffoldable_per_architecture() {
    let tmp = run_new("support-gen", "Layer::Support").unwrap();
    let lib = fs::read_to_string(tmp.path().join("support-gen/src/lib.rs")).unwrap();
    assert!(lib.contains("run_support"));
}

#[test]
fn stage_support_postprocess_is_scaffoldable_per_architecture() {
    let tmp = run_new("support-pp", "Layer::SupportPostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("support-pp/src/lib.rs")).unwrap();
    assert!(lib.contains("run_support_postprocess"));
}

#[test]
fn stage_path_optimization_is_scaffoldable_per_architecture() {
    let tmp = run_new("path-opt", "Layer::PathOptimization").unwrap();
    let lib = fs::read_to_string(tmp.path().join("path-opt/src/lib.rs")).unwrap();
    assert!(lib.contains("run_path_optimization"));
}

#[test]
fn stage_layer_infill_scaffolds_layer_world_v1_0_0_for_backcompat() {
    let tmp = run_new("plain-infill", "Layer::Infill").unwrap();
    let manifest = fs::read_to_string(tmp.path().join("plain-infill/plain-infill.toml")).unwrap();
    assert!(manifest.contains("slicer:world-layer@1.0.0"));
}

#[test]
fn stage_gcode_postprocess() {
    let tmp = run_new("gcode-fix", "PostPass::GCodePostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("gcode-fix/src/lib.rs")).unwrap();
    assert!(lib.contains("run_gcode_postprocess"));
    let manifest = fs::read_to_string(tmp.path().join("gcode-fix/gcode-fix.toml")).unwrap();
    assert!(manifest.contains("slicer:world-postpass@1.0.0"));
}

#[test]
fn stage_text_postprocess() {
    let tmp = run_new("text-fix", "PostPass::TextPostProcess").unwrap();
    let lib = fs::read_to_string(tmp.path().join("text-fix/src/lib.rs")).unwrap();
    assert!(lib.contains("run_text_postprocess"));
}

#[test]
fn stage_layer_finalization_uses_finalization_world() {
    let tmp = run_new("layer-finalizer", "PostPass::LayerFinalization").unwrap();
    let manifest =
        fs::read_to_string(tmp.path().join("layer-finalizer/layer-finalizer.toml")).unwrap();
    assert!(manifest.contains("slicer:world-finalization@1.0.0"));
    let lib = fs::read_to_string(tmp.path().join("layer-finalizer/src/lib.rs")).unwrap();
    assert!(lib.contains("run_finalization"));
}

// ── Error cases ──────────────────────────────────────────────────────────────

#[test]
fn invalid_name_uppercase() {
    let tmp = TempDir::new().unwrap();
    let result = pnp_cli::module_new::execute_in(tmp.path(), "MyModule", "Layer::Infill");
    assert!(matches!(
        result,
        Err(pnp_cli::module_new::NewError::InvalidName(_))
    ));
}

#[test]
fn invalid_name_underscore() {
    let tmp = TempDir::new().unwrap();
    let result = pnp_cli::module_new::execute_in(tmp.path(), "my_module", "Layer::Infill");
    assert!(matches!(
        result,
        Err(pnp_cli::module_new::NewError::InvalidName(_))
    ));
}

#[test]
fn invalid_name_starts_with_digit() {
    let tmp = TempDir::new().unwrap();
    let result = pnp_cli::module_new::execute_in(tmp.path(), "1module", "Layer::Infill");
    assert!(matches!(
        result,
        Err(pnp_cli::module_new::NewError::InvalidName(_))
    ));
}

#[test]
fn invalid_stage_rejected() {
    let tmp = TempDir::new().unwrap();
    let result = pnp_cli::module_new::execute_in(tmp.path(), "my-mod", "Layer::Unknown");
    assert!(matches!(
        result,
        Err(pnp_cli::module_new::NewError::InvalidStage(_))
    ));
}

#[test]
fn directory_already_exists() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("my-mod")).unwrap();
    let result = pnp_cli::module_new::execute_in(tmp.path(), "my-mod", "Layer::Infill");
    assert!(matches!(
        result,
        Err(pnp_cli::module_new::NewError::DirectoryExists(_))
    ));
}

// ── AC-10: extended scaffold files ───────────────────────────────────────────

#[test]
fn emits_cargo_config_alias() {
    let tmp = run_new("test-module", "Layer::Infill").unwrap();
    let base = tmp.path().join("test-module");

    // (i) .cargo/config.toml must exist with the exact alias string
    let cargo_config_path = base.join(".cargo/config.toml");
    assert!(
        cargo_config_path.is_file(),
        ".cargo/config.toml must be created by the scaffold"
    );
    let cargo_config = fs::read_to_string(&cargo_config_path).unwrap();
    assert!(
        cargo_config.contains("build-wasm = \"build --target wasm32-unknown-unknown --release\""),
        "'.cargo/config.toml' must contain the wasm build alias; got:\n{cargo_config}"
    );

    // (ii) README.md must exist with the wasm-tools command
    let readme_path = base.join("README.md");
    assert!(
        readme_path.is_file(),
        "README.md must be created by the scaffold"
    );
    let readme = fs::read_to_string(&readme_path).unwrap();
    assert!(
        readme.contains("wasm-tools component new"),
        "README.md must contain 'wasm-tools component new'; got:\n{readme}"
    );
}
