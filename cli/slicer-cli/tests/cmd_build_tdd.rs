//! TDD tests for `slicer build` subcommand.
//!
//! Tests cover: Cargo.toml parsing, error paths, release flag,
//! output path construction, cdylib detection, and command assembly.

use slicer_cli::cmd_build;
use std::fs;
use tempfile::TempDir;

/// Helper: create a minimal Cargo.toml in the given directory.
fn write_cargo_toml(dir: &std::path::Path, name: &str, cdylib: bool) {
    let crate_type = if cdylib {
        r#"crate-type = ["cdylib"]"#
    } else {
        r#"crate-type = ["rlib"]"#
    };
    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
{crate_type}
"#,
    );
    fs::write(dir.join("Cargo.toml"), content).unwrap();
}

// --- Module name parsing ---

#[test]
fn parse_module_name_from_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    write_cargo_toml(tmp.path(), "my-cool-infill", true);

    let name = cmd_build::parse_module_name(tmp.path()).unwrap();
    assert_eq!(name, "my-cool-infill");
}

#[test]
fn parse_module_name_underscores() {
    let tmp = TempDir::new().unwrap();
    write_cargo_toml(tmp.path(), "my_infill_module", true);

    let name = cmd_build::parse_module_name(tmp.path()).unwrap();
    assert_eq!(name, "my_infill_module");
}

#[test]
fn parse_module_name_missing_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    let result = cmd_build::parse_module_name(tmp.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, cmd_build::BuildError::MissingCargoToml),
        "expected MissingCargoToml, got: {err:?}"
    );
}

#[test]
fn parse_module_name_malformed_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "not valid toml {{{{").unwrap();

    let result = cmd_build::parse_module_name(tmp.path());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        cmd_build::BuildError::CargoTomlParseError(_)
    ));
}

#[test]
fn parse_module_name_missing_package_name() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let result = cmd_build::parse_module_name(tmp.path());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        cmd_build::BuildError::CargoTomlParseError(_)
    ));
}

// --- cdylib detection ---

#[test]
fn detect_cdylib_present() {
    let tmp = TempDir::new().unwrap();
    write_cargo_toml(tmp.path(), "my-module", true);

    assert!(cmd_build::has_cdylib(tmp.path()).unwrap());
}

#[test]
fn detect_cdylib_absent() {
    let tmp = TempDir::new().unwrap();
    write_cargo_toml(tmp.path(), "my-module", false);

    assert!(!cmd_build::has_cdylib(tmp.path()).unwrap());
}

#[test]
fn detect_cdylib_no_lib_section() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"my-module\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    assert!(!cmd_build::has_cdylib(tmp.path()).unwrap());
}

// --- Output path construction ---

#[test]
fn wasm_output_path_debug() {
    let p = cmd_build::core_wasm_output_path("my-infill", false);
    assert!(p.ends_with("debug/my_infill.wasm"), "got: {}", p.display());
    assert!(p.to_str().unwrap().contains("wasm32-unknown-unknown/debug"));
}

#[test]
fn wasm_output_path_release() {
    let p = cmd_build::core_wasm_output_path("my-infill", true);
    assert!(
        p.ends_with("release/my_infill.wasm"),
        "got: {}",
        p.display()
    );
    assert!(p
        .to_str()
        .unwrap()
        .contains("wasm32-unknown-unknown/release"));
}

#[test]
fn final_output_path() {
    let p = cmd_build::final_output_path("my-infill");
    assert_eq!(p, std::path::PathBuf::from("target/slicer/my-infill.wasm"));
}

// --- Cargo build command assembly ---

#[test]
fn cargo_build_args_debug() {
    let args = cmd_build::cargo_build_args(false);
    assert_eq!(args, vec!["build", "--target", "wasm32-unknown-unknown"]);
}

#[test]
fn cargo_build_args_release() {
    let args = cmd_build::cargo_build_args(true);
    assert_eq!(
        args,
        vec!["build", "--target", "wasm32-unknown-unknown", "--release"]
    );
}

// --- wasm-tools command assembly ---

#[test]
fn wasm_tools_component_args() {
    let core = std::path::PathBuf::from("target/wasm32-unknown-unknown/debug/my_infill.wasm");
    let output = std::path::PathBuf::from("target/slicer/my-infill.wasm");
    let args = cmd_build::wasm_tools_args(&core, &output);
    assert_eq!(
        args,
        vec![
            "component",
            "new",
            "target/wasm32-unknown-unknown/debug/my_infill.wasm",
            "-o",
            "target/slicer/my-infill.wasm",
        ]
    );
}

// --- BuildError Display ---

#[test]
fn build_error_display() {
    let err = cmd_build::BuildError::MissingCargoToml;
    let msg = format!("{err}");
    assert!(
        msg.contains("Cargo.toml"),
        "error message should mention Cargo.toml: {msg}"
    );
}

#[test]
fn build_error_cargo_build_failed_display() {
    let err = cmd_build::BuildError::CargoBuildFailed("some error output".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("cargo build"), "got: {msg}");
}

#[test]
fn build_error_wasm_tools_failed_display() {
    let err = cmd_build::BuildError::WasmToolsFailed("wasm-tools error".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("wasm-tools") || msg.contains("component"),
        "got: {msg}"
    );
}
