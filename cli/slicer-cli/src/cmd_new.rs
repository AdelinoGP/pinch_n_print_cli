//! Implementation of the `slicer new` subcommand.
//!
//! Scaffolds a new module project directory with the correct structure,
//! Cargo.toml, manifest template, and a passing test suite.

use std::fmt;
use std::fs;
use std::path::Path;

/// The nine valid pipeline stages a module can target.
const VALID_STAGES: &[&str] = &[
    "Layer::Infill",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::InfillPostProcess",
    "Layer::SlicePostProcess",
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
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

/// Returns true if `stage` is one of the nine valid pipeline stages.
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
layer-parallel-safe    = true
"#,
        display = display_name_from_kebab(name),
    )
}

/// Map a stage ID to the WIT world package string.
fn wit_world_for_stage(stage: &str) -> &'static str {
    match stage {
        "PrePass::MeshAnalysis" | "PrePass::LayerPlanning" => "slicer:world-prepass@1.0.0",
        "PostPass::GCodePostProcess" | "PostPass::TextPostProcess" => {
            "slicer:world-postpass@1.0.0"
        }
        // All Layer::* stages use the layer world.
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

/// Generate the lib.rs stub for the appropriate stage.
fn generate_lib_rs(name: &str, stage: &str) -> String {
    let underscore_name = name.replace('-', "_");
    let (_trait_name, fn_sig, fn_body) = trait_info_for_stage(stage);

    format!(
        r#"//! {display} — a ModularSlicer module.

/// The main module struct.
pub struct {struct_name};

// TODO: Add #[slicer_module] attribute once macros are functional.
// For now, implement the trait manually.

impl {struct_name} {{
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
}}
"#,
        display = display_name_from_kebab(name),
        struct_name = struct_name_from_kebab(&underscore_name),
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

/// Return (trait_name, function_signature, function_body) for a given stage.
fn trait_info_for_stage(stage: &str) -> (&'static str, &'static str, &'static str) {
    match stage {
        "Layer::Infill" => (
            "InfillModule",
            "/// Run the infill generation stage.\n    pub fn run_infill(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "Layer::Perimeters" => (
            "PerimeterModule",
            "/// Run the perimeter generation stage.\n    pub fn run_perimeters(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "Layer::PerimetersPostProcess" => (
            "WallPostProcessModule",
            "/// Run the wall post-processing stage.\n    pub fn run_wall_postprocess(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "Layer::InfillPostProcess" => (
            "InfillPostProcessModule",
            "/// Run the infill post-processing stage.\n    pub fn run_infill_postprocess(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "Layer::SlicePostProcess" => (
            "SlicePostProcessModule",
            "/// Run the slice post-processing stage.\n    pub fn run_slice_postprocess(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "PrePass::MeshAnalysis" => (
            "MeshAnalysisModule",
            "/// Run the mesh analysis stage.\n    pub fn run_mesh_analysis(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "PrePass::LayerPlanning" => (
            "LayerPlanningModule",
            "/// Run the layer planning stage.\n    pub fn run_layer_planning(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "PostPass::GCodePostProcess" => (
            "GCodePostProcessModule",
            "/// Run the G-code post-processing stage.\n    pub fn run_gcode_postprocess(&self) -> Result<(), String>",
            "Ok(())",
        ),
        "PostPass::TextPostProcess" => (
            "TextPostProcessModule",
            "/// Run the text post-processing stage.\n    pub fn run_text_postprocess(&self) -> Result<(), String>",
            "Ok(())",
        ),
        _ => (
            "Module",
            "/// Run the module.\n    pub fn run(&self) -> Result<(), String>",
            "Ok(())",
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
            wit_world_for_stage("PrePass::MeshAnalysis"),
            "slicer:world-prepass@1.0.0"
        );
        assert_eq!(
            wit_world_for_stage("PostPass::GCodePostProcess"),
            "slicer:world-postpass@1.0.0"
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
    fn manifest_postpass_stage() {
        let manifest = generate_manifest("gcode-fix", "PostPass::GCodePostProcess");
        assert!(manifest.contains(r#"wit-world    = "slicer:world-postpass@1.0.0""#));
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
}
