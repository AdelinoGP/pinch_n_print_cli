//! Implementation of the `slicer new` subcommand.
//!
//! Scaffolds a new module project directory with the correct structure,
//! Cargo.toml, manifest template, and a passing test suite.

use std::fmt;
use std::fs;
use std::path::Path;

/// The fifteen valid pipeline stages a module can target.
const VALID_STAGES: &[&str] = &[
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
];

/// Errors that can occur during scaffolding.
#[derive(Debug)]
pub enum NewError {
    /// Module name is not valid kebab-case.
    InvalidName(String),
    /// Stage is not one of the recognized pipeline stages.
    InvalidStage(String),
    /// The target directory already exists.
    DirectoryExists(String),
    /// An I/O error occurred while creating files.
    Io(std::io::Error),
}

impl fmt::Display for NewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NewError::InvalidName(name) => write!(
                f,
                "invalid module name '{name}': must be kebab-case (lowercase letters, digits, hyphens; \
                 must start with a letter, must not start or end with a hyphen, no consecutive hyphens)"
            ),
            NewError::InvalidStage(stage) => {
                write!(f, "unknown stage '{stage}'. Valid stages: {}", VALID_STAGES.join(", "))
            }
            NewError::DirectoryExists(path) => {
                write!(f, "directory '{path}' already exists")
            }
            NewError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for NewError {
    fn from(e: std::io::Error) -> Self {
        NewError::Io(e)
    }
}

/// Returns true if `name` is valid kebab-case for a module name.
///
/// Rules:
/// - Only lowercase ASCII letters, digits, and hyphens.
/// - Must start with a lowercase letter.
/// - Must not end with a hyphen.
/// - No consecutive hyphens.
/// - At least one character.
pub fn is_valid_module_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = name.as_bytes();
    // Must start with lowercase letter.
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    // Must not end with hyphen.
    if bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    // Only allowed characters, no consecutive hyphens.
    let mut prev_hyphen = false;
    for &b in bytes {
        if b == b'-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }
    true
}

/// Returns true if `stage` is one of the fifteen valid pipeline stages.
pub fn is_valid_stage(stage: &str) -> bool {
    VALID_STAGES.contains(&stage)
}

/// Execute the `slicer new` command in the current directory.
pub fn execute(name: &str, stage: &str) -> Result<(), NewError> {
    execute_in(name, stage, Path::new("."))
}

/// Execute the `slicer new` command, creating the module under `parent_dir`.
///
/// This is the core implementation used by both the CLI entry point and tests.
pub fn execute_in(name: &str, stage: &str, parent_dir: &Path) -> Result<(), NewError> {
    if !is_valid_module_name(name) {
        return Err(NewError::InvalidName(name.to_string()));
    }
    if !is_valid_stage(stage) {
        return Err(NewError::InvalidStage(stage.to_string()));
    }

    let base = parent_dir.join(name);
    if base.exists() {
        return Err(NewError::DirectoryExists(name.to_string()));
    }

    // Create directory structure.
    fs::create_dir_all(base.join("src"))?;
    fs::create_dir_all(base.join("tests/fixtures"))?;

    // Write Cargo.toml.
    fs::write(base.join("Cargo.toml"), generate_cargo_toml(name))?;

    // Write module manifest.
    fs::write(
        base.join(format!("{name}.toml")),
        generate_manifest(name, stage),
    )?;

    // Write lib.rs.
    fs::write(base.join("src/lib.rs"), generate_lib_rs(name, stage))?;

    // Write basic.rs test.
    fs::write(base.join("tests/basic.rs"), generate_basic_test(name))?;

    // Write fixture JSON.
    fs::write(
        base.join("tests/fixtures/square_20mm.json"),
        generate_fixture_json(),
    )?;

    println!("Created module '{name}' with stage {stage}");
    println!("  {name}/Cargo.toml");
    println!("  {name}/{name}.toml");
    println!("  {name}/src/lib.rs");
    println!("  {name}/tests/basic.rs");
    println!("  {name}/tests/fixtures/square_20mm.json");

    Ok(())
}

/// Generate the Cargo.toml for a new module crate.
fn generate_cargo_toml(name: &str) -> String {
    let underscore_name = name.replace('-', "_");
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
name = "{underscore_name}"
crate-type = ["cdylib"]

[dependencies]
slicer-sdk = {{ path = "../../crates/slicer-sdk" }}

[dev-dependencies]
slicer-test = {{ path = "../../crates/slicer-test" }}
"#
    )
}

/// Generate the module manifest TOML.
fn generate_manifest(name: &str, stage: &str) -> String {
    let wit_world = wit_world_for_stage(stage);
    let parallel_safe = stage != "PostPass::LayerFinalization";
    format!(
        r#"[module]
id           = "com.example.{name}"
version      = "0.1.0"
display-name = "{display}"
description  = "A {stage} module"
author       = "developer"
license      = "MIT"
wit-world    = "{wit_world}"

[stage]
id = "{stage}"

[ir-access]
reads  = []
writes = []

[claims]
holds    = []
requires = []

[compatibility]
incompatible-with = []
requires          = []
min-host-version  = "0.1.0"
min-ir-schema     = "1.0.0"
max-ir-schema     = "2.0.0"

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe    = {parallel_safe}
"#,
        display = display_name_from_kebab(name),
    )
}

/// Map a stage ID to the WIT world package string.
fn wit_world_for_stage(stage: &str) -> &'static str {
    match stage {
        "PrePass::MeshSegmentation"
        | "PrePass::MeshAnalysis"
        | "PrePass::LayerPlanning"
        | "PrePass::PaintSegmentation" => "slicer:world-prepass@1.0.0",
        "PostPass::LayerFinalization" => "slicer:world-finalization@1.0.0",
        "PostPass::GCodePostProcess" | "PostPass::TextPostProcess" => "slicer:world-postpass@1.0.0",
        _ => "slicer:world-layer@1.0.0",
    }
}

/// Convert kebab-case name to a display name (title case).
fn display_name_from_kebab(name: &str) -> String {
    name.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Default body expression for a stage method stub.
fn default_body_for_stage(stage: &str) -> &'static str {
    match stage {
        "PostPass::TextPostProcess" => "Ok(gcode_text.to_string())",
        _ => "Ok(())",
    }
}

/// Generate the lib.rs stub for the appropriate stage.
fn generate_lib_rs(name: &str, stage: &str) -> String {
    let underscore_name = name.replace('-', "_");
    let struct_name = struct_name_from_kebab(&underscore_name);
    let (trait_name, fn_name, fn_sig) = trait_info_for_stage(stage);
    let fn_body = default_body_for_stage(stage);

    format!(
        r#"//! {display} — a ModularSlicer module.

use slicer_sdk::prelude::*;

/// The main module struct.
pub struct {struct_name};

#[slicer_module]
impl {trait_name} for {struct_name} {{
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {{
        Ok(Self)
    }}

    {fn_sig} {{
        {fn_body}
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn module_struct_exists() {{
        let _ = {struct_name};
    }}

    #[test]
    fn on_print_start_succeeds() {{
        let config = ConfigView {{ fields: std::collections::HashMap::new() }};
        let result = {struct_name}::on_print_start(&config);
        assert!(result.is_ok());
    }}

    #[test]
    fn {fn_name}_succeeds() {{
        let _ = {struct_name};
    }}
}}
"#,
        display = display_name_from_kebab(name),
    )
}

/// Convert underscore_name to PascalCase struct name.
fn struct_name_from_kebab(underscore_name: &str) -> String {
    underscore_name
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Return (trait_name, fn_name, fn_signature) for a given stage.
///
/// The trait_name maps to the SDK trait the module should implement.
/// The fn_name is the stage method the module overrides.
/// The fn_signature is the full method signature as it appears in the trait impl.
fn trait_info_for_stage(stage: &str) -> (&'static str, &'static str, &'static str) {
    match stage {
        // Layer world stages → LayerModule trait
        "Layer::Infill" => (
            "LayerModule",
            "run_infill",
            "fn run_infill(\n        &self,\n        _layer_index: u32,\n        _regions: &[SliceRegionView],\n        _output: &mut InfillOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::Perimeters" => (
            "LayerModule",
            "run_perimeters",
            "fn run_perimeters(\n        &self,\n        _layer_index: u32,\n        _regions: &[SliceRegionView],\n        _paint: &PaintRegionLayerView,\n        _output: &mut PerimeterOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::PerimetersPostProcess" => (
            "LayerModule",
            "run_wall_postprocess",
            "fn run_wall_postprocess(\n        &self,\n        _layer_index: u32,\n        _regions: &[PerimeterRegionView],\n        _output: &mut PerimeterOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::InfillPostProcess" => (
            "LayerModule",
            "run_infill_postprocess",
            "fn run_infill_postprocess(\n        &self,\n        _layer_index: u32,\n        _regions: &[PerimeterRegionView],\n        _output: &mut InfillOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::SlicePostProcess" => (
            "LayerModule",
            "run_slice_postprocess",
            "fn run_slice_postprocess(\n        &self,\n        _layer_index: u32,\n        _regions: &[SliceRegionView],\n        _paint: &PaintRegionLayerView,\n        _output: &mut SlicePostprocessBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::Support" => (
            "LayerModule",
            "run_support",
            "fn run_support(\n        &self,\n        _layer_index: u32,\n        _regions: &[SliceRegionView],\n        _paint: &PaintRegionLayerView,\n        _output: &mut SupportOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::SupportPostProcess" => (
            "LayerModule",
            "run_support_postprocess",
            "fn run_support_postprocess(\n        &self,\n        _layer_index: u32,\n        _regions: &[SliceRegionView],\n        _output: &mut SupportOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "Layer::PathOptimization" => (
            "LayerModule",
            "run_path_optimization",
            "fn run_path_optimization(\n        &self,\n        _layer_index: u32,\n        _regions: &[PerimeterRegionView],\n        _output: &mut GcodeOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        // PrePass world stages → PrepassModule trait
        "PrePass::MeshAnalysis" => (
            "PrepassModule",
            "run_mesh_analysis",
            "fn run_mesh_analysis(\n        &self,\n        _objects: &[ObjectId],\n        _output: &mut MeshAnalysisOutput,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "PrePass::LayerPlanning" => (
            "PrepassModule",
            "run_layer_planning",
            "fn run_layer_planning(\n        &self,\n        _objects: &[ObjectId],\n        _output: &mut LayerPlanOutput,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "PrePass::MeshSegmentation" => (
            "PrepassModule",
            "run_mesh_segmentation",
            "fn run_mesh_segmentation(\n        &self,\n        _objects: &[MeshObjectView],\n        _output: &mut MeshSegmentationOutput,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "PrePass::PaintSegmentation" => (
            "PrepassModule",
            "run_paint_segmentation",
            "fn run_paint_segmentation(\n        &self,\n        _objects: &[PaintSegmentationObjectView],\n        _output: &mut PaintSegmentationOutput,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        // Finalization world → FinalizationModule trait
        "PostPass::LayerFinalization" => (
            "FinalizationModule",
            "run_layer_finalization",
            "fn run_finalization(\n        &self,\n        _layers: &[LayerCollectionView],\n        _output: &mut FinalizationOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        // PostPass world stages → PostpassModule trait
        "PostPass::GCodePostProcess" => (
            "PostpassModule",
            "run_gcode_postprocess",
            "fn run_gcode_postprocess(\n        &self,\n        _commands: &[GcodeCommandView],\n        _output: &mut GcodeOutputBuilder,\n        _config: &ConfigView,\n    ) -> Result<(), ModuleError>",
        ),
        "PostPass::TextPostProcess" => (
            "PostpassModule",
            "run_text_postprocess",
            "fn run_text_postprocess(\n        &self,\n        gcode_text: &str,\n        _config: &ConfigView,\n    ) -> Result<String, ModuleError>",
        ),
        _ => (
            "LayerModule",
            "run",
            "fn run(&self) -> Result<(), ModuleError>",
        ),
    }
}

/// Generate the basic.rs test stub.
fn generate_basic_test(name: &str) -> String {
    let underscore_name = name.replace('-', "_");
    let struct_name = struct_name_from_kebab(&underscore_name);
    format!(
        r#"//! Basic tests for the {display} module.

use {underscore_name}::{struct_name};

#[test]
fn module_can_be_instantiated() {{
    let _module = {struct_name};
}}
"#,
        display = display_name_from_kebab(name),
    )
}

/// Generate a simple 20mm square SliceRegionView fixture as JSON.
fn generate_fixture_json() -> String {
    // A 20mm square at z=0.2mm, scaled to integer coordinates (1 unit = 100nm).
    // 20mm = 200_000 units.
    serde_json::json!({
        "object_id": "default",
        "region_id": 0,
        "z": 0.2,
        "effective_layer_height": 0.2,
        "contour": {
            "points": [
                {"x": 0, "y": 0},
                {"x": 200000, "y": 0},
                {"x": 200000, "y": 200000},
                {"x": 0, "y": 200000}
            ]
        },
        "holes": [],
        "infill_areas": [{
            "contour": {
                "points": [
                    {"x": 4000, "y": 4000},
                    {"x": 196000, "y": 4000},
                    {"x": 196000, "y": 196000},
                    {"x": 4000, "y": 196000}
                ]
            },
            "holes": []
        }],
        "has_nonplanar": false,
        "boundary_paint": []
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_module_names() {
        assert!(is_valid_module_name("my-infill"));
        assert!(is_valid_module_name("tpms"));
        assert!(is_valid_module_name("a"));
        assert!(is_valid_module_name("my-cool-module2"));
        assert!(is_valid_module_name("x1"));
    }

    #[test]
    fn invalid_module_names() {
        assert!(!is_valid_module_name(""));
        assert!(!is_valid_module_name("-foo"));
        assert!(!is_valid_module_name("foo-"));
        assert!(!is_valid_module_name("foo--bar"));
        assert!(!is_valid_module_name("Foo"));
        assert!(!is_valid_module_name("foo_bar"));
        assert!(!is_valid_module_name("foo bar"));
        assert!(!is_valid_module_name("123"));
        assert!(!is_valid_module_name("1foo"));
        assert!(!is_valid_module_name("foo.bar"));
    }

    #[test]
    fn valid_stages() {
        for stage in VALID_STAGES {
            assert!(is_valid_stage(stage), "stage '{stage}' should be valid");
        }
    }

    #[test]
    fn invalid_stages() {
        assert!(!is_valid_stage("Layer::Unknown"));
        assert!(!is_valid_stage("infill"));
        assert!(!is_valid_stage(""));
    }

    #[test]
    fn display_name_conversion() {
        assert_eq!(display_name_from_kebab("my-infill"), "My Infill");
        assert_eq!(display_name_from_kebab("tpms"), "Tpms");
        assert_eq!(display_name_from_kebab("a-b-c"), "A B C");
    }

    #[test]
    fn struct_name_conversion() {
        assert_eq!(struct_name_from_kebab("my_infill"), "MyInfill");
        assert_eq!(struct_name_from_kebab("tpms"), "Tpms");
    }

    #[test]
    fn wit_world_mapping() {
        assert_eq!(
            wit_world_for_stage("Layer::Infill"),
            "slicer:world-layer@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("Layer::Support"),
            "slicer:world-layer@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("Layer::PathOptimization"),
            "slicer:world-layer@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PrePass::MeshAnalysis"),
            "slicer:world-prepass@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PrePass::MeshSegmentation"),
            "slicer:world-prepass@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PrePass::PaintSegmentation"),
            "slicer:world-prepass@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PostPass::GCodePostProcess"),
            "slicer:world-postpass@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PostPass::LayerFinalization"),
            "slicer:world-finalization@1.0.0"
        );
    }

    #[test]
    fn cargo_toml_generation() {
        let toml = generate_cargo_toml("my-infill");
        assert!(toml.contains(r#"name = "my-infill""#));
        assert!(toml.contains(r#"name = "my_infill""#));
        assert!(toml.contains(r#"crate-type = ["cdylib"]"#));
        assert!(toml.contains("slicer-sdk"));
        assert!(toml.contains("slicer-test"));
    }

    #[test]
    fn manifest_generation() {
        let manifest = generate_manifest("my-infill", "Layer::Infill");
        assert!(manifest.contains(r#"id           = "com.example.my-infill""#));
        assert!(manifest.contains(r#"id = "Layer::Infill""#));
        assert!(manifest.contains(r#"wit-world    = "slicer:world-layer@1.0.0""#));
        // Verify it's valid TOML.
        let parsed: Result<toml::Value, _> = toml::from_str(&manifest);
        assert!(parsed.is_ok(), "generated manifest must be valid TOML");
    }

    #[test]
    fn manifest_prepass_stage() {
        let manifest = generate_manifest("mesh-tool", "PrePass::MeshAnalysis");
        assert!(manifest.contains(r#"wit-world    = "slicer:world-prepass@1.0.0""#));
        assert!(manifest.contains(r#"id = "PrePass::MeshAnalysis""#));
    }

    #[test]
    fn manifest_prepass_mesh_segmentation_stage() {
        let manifest = generate_manifest("mesh-seg", "PrePass::MeshSegmentation");
        assert!(manifest.contains(r#"wit-world    = "slicer:world-prepass@1.0.0""#));
        assert!(manifest.contains(r#"id = "PrePass::MeshSegmentation""#));
    }

    #[test]
    fn manifest_postpass_stage() {
        let manifest = generate_manifest("gcode-fix", "PostPass::GCodePostProcess");
        assert!(manifest.contains(r#"wit-world    = "slicer:world-postpass@1.0.0""#));
    }

    #[test]
    fn manifest_finalization_stage() {
        let manifest = generate_manifest("layer-fin", "PostPass::LayerFinalization");
        assert!(manifest.contains(r#"wit-world    = "slicer:world-finalization@1.0.0""#));
        assert!(manifest.contains(r#"id = "PostPass::LayerFinalization""#));
    }

    #[test]
    fn lib_rs_default_stage() {
        let lib = generate_lib_rs("my-infill", "Layer::Infill");
        assert!(lib.contains("pub struct MyInfill"));
        assert!(lib.contains("run_infill"));
    }

    #[test]
    fn lib_rs_perimeter_stage() {
        let lib = generate_lib_rs("wall-gen", "Layer::Perimeters");
        assert!(lib.contains("pub struct WallGen"));
        assert!(lib.contains("run_perimeters"));
    }

    #[test]
    fn lib_rs_prepass_stage() {
        let lib = generate_lib_rs("mesh-tool", "PrePass::MeshAnalysis");
        assert!(lib.contains("pub struct MeshTool"));
        assert!(lib.contains("run_mesh_analysis"));
    }

    #[test]
    fn lib_rs_postpass_stage() {
        let lib = generate_lib_rs("gcode-fix", "PostPass::TextPostProcess");
        assert!(lib.contains("pub struct GcodeFix"));
        assert!(lib.contains("run_text_postprocess"));
    }

    #[test]
    fn fixture_json_is_valid() {
        let json = generate_fixture_json();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok(), "fixture JSON must be valid");
        let val = parsed.unwrap();
        assert_eq!(val["object_id"], "default");
        assert_eq!(val["z"], 0.2);
    }

    #[test]
    fn basic_test_generation() {
        let test = generate_basic_test("my-infill");
        assert!(test.contains("use my_infill::MyInfill"));
        assert!(test.contains("fn module_can_be_instantiated"));
    }

    // ── Generated skeleton correctness ──────────────────────────────────

    #[test]
    fn lib_rs_uses_slicer_module_macro() {
        let lib = generate_lib_rs("my-infill", "Layer::Infill");
        assert!(
            lib.contains("#[slicer_module]"),
            "generated lib.rs must use #[slicer_module] attribute"
        );
        assert!(
            !lib.contains("TODO"),
            "generated lib.rs must not contain TODO placeholders"
        );
    }

    #[test]
    fn lib_rs_uses_prelude_import() {
        let lib = generate_lib_rs("my-infill", "Layer::Infill");
        assert!(
            lib.contains("use slicer_sdk::prelude::*;"),
            "generated lib.rs must import the SDK prelude"
        );
    }

    #[test]
    fn lib_rs_layer_stage_uses_layer_module_trait() {
        for stage in &[
            "Layer::Infill",
            "Layer::Perimeters",
            "Layer::PerimetersPostProcess",
            "Layer::InfillPostProcess",
            "Layer::SlicePostProcess",
            "Layer::Support",
            "Layer::SupportPostProcess",
            "Layer::PathOptimization",
        ] {
            let lib = generate_lib_rs("test-mod", stage);
            assert!(
                lib.contains("impl LayerModule for TestMod"),
                "Layer stage {stage} must use LayerModule trait"
            );
        }
    }

    #[test]
    fn lib_rs_prepass_stage_uses_prepass_module_trait() {
        for stage in &[
            "PrePass::MeshAnalysis",
            "PrePass::LayerPlanning",
            "PrePass::MeshSegmentation",
            "PrePass::PaintSegmentation",
        ] {
            let lib = generate_lib_rs("test-mod", stage);
            assert!(
                lib.contains("impl PrepassModule for TestMod"),
                "PrePass stage {stage} must use PrepassModule trait"
            );
        }
    }

    #[test]
    fn lib_rs_finalization_stage_uses_finalization_module_trait() {
        let lib = generate_lib_rs("test-mod", "PostPass::LayerFinalization");
        assert!(
            lib.contains("impl FinalizationModule for TestMod"),
            "LayerFinalization must use FinalizationModule trait"
        );
    }

    #[test]
    fn lib_rs_postpass_stage_uses_postpass_module_trait() {
        for stage in &["PostPass::GCodePostProcess", "PostPass::TextPostProcess"] {
            let lib = generate_lib_rs("test-mod", stage);
            assert!(
                lib.contains("impl PostpassModule for TestMod"),
                "PostPass stage {stage} must use PostpassModule trait"
            );
        }
    }

    #[test]
    fn lib_rs_has_on_print_start_lifecycle() {
        let lib = generate_lib_rs("my-infill", "Layer::Infill");
        assert!(
            lib.contains("fn on_print_start"),
            "generated lib.rs must include on_print_start lifecycle"
        );
    }

    #[test]
    fn lib_rs_text_postprocess_returns_string() {
        let lib = generate_lib_rs("text-pp", "PostPass::TextPostProcess");
        assert!(
            lib.contains("gcode_text.to_string()"),
            "TextPostProcess body must return the input string"
        );
    }

    #[test]
    fn manifest_finalization_forces_parallel_safe_false() {
        let manifest = generate_manifest("layer-fin", "PostPass::LayerFinalization");
        assert!(
            manifest.contains("layer-parallel-safe    = false"),
            "LayerFinalization manifest must set layer-parallel-safe = false"
        );
    }

    #[test]
    fn manifest_layer_stage_defaults_parallel_safe_true() {
        let manifest = generate_manifest("my-infill", "Layer::Infill");
        assert!(
            manifest.contains("layer-parallel-safe    = true"),
            "Layer stages should default to layer-parallel-safe = true"
        );
    }

    #[test]
    fn every_stage_produces_valid_manifest_toml() {
        for stage in VALID_STAGES {
            let manifest = generate_manifest("test-mod", stage);
            let parsed: Result<toml::Value, _> = toml::from_str(&manifest);
            assert!(
                parsed.is_ok(),
                "manifest for stage '{stage}' must be valid TOML: {:?}",
                parsed.err()
            );
        }
    }
}
