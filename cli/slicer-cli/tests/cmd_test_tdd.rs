//! TDD tests for `slicer test` subcommand.
//!
//! Tests cover: Cargo.toml parsing, error paths, arg forwarding,
//! nextest/cargo-test fallback, coverage path construction, and command assembly.

use slicer_cli::cmd_test;
use std::fs;
use tempfile::TempDir;

/// Helper: create a minimal Cargo.toml in the given directory.
fn write_cargo_toml(dir: &std::path::Path, name: &str) {
    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#,
    );
    fs::write(dir.join("Cargo.toml"), content).unwrap();
}

// --- Module name parsing (reuses cmd_build::parse_module_name internally) ---

#[test]
fn parse_module_name_from_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    write_cargo_toml(tmp.path(), "my-test-module");

    let name = cmd_test::parse_module_name(tmp.path()).unwrap();
    assert_eq!(name, "my-test-module");
}

#[test]
fn parse_module_name_missing_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    let result = cmd_test::parse_module_name(tmp.path());
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), cmd_test::TestError::MissingCargoToml),
        "expected MissingCargoToml"
    );
}

#[test]
fn parse_module_name_malformed_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "not valid toml {{{{").unwrap();

    let result = cmd_test::parse_module_name(tmp.path());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        cmd_test::TestError::CargoTomlParseError(_)
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

    let result = cmd_test::parse_module_name(tmp.path());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        cmd_test::TestError::CargoTomlParseError(_)
    ));
}

// --- Nextest detection ---

#[test]
fn check_nextest_available_returns_bool() {
    // Just verify the function exists and returns a bool — actual availability
    // depends on the host toolchain.
    let available = cmd_test::is_nextest_available();
    // We don't assert a specific value; the function must not panic.
    let _: bool = available;
}

// --- Test runner command assembly ---

#[test]
fn nextest_args_no_extra() {
    let args = cmd_test::nextest_args(&[]);
    assert_eq!(args, vec!["nextest", "run"]);
}

#[test]
fn nextest_args_with_forwarded() {
    let args = cmd_test::nextest_args(&["--test-threads=1".to_string(), "--nocapture".to_string()]);
    assert_eq!(
        args,
        vec!["nextest", "run", "--test-threads=1", "--nocapture"]
    );
}

#[test]
fn cargo_test_args_no_extra() {
    let args = cmd_test::cargo_test_args(&[]);
    assert_eq!(args, vec!["test"]);
}

#[test]
fn cargo_test_args_with_forwarded() {
    let args = cmd_test::cargo_test_args(&[
        "--lib".to_string(),
        "--".to_string(),
        "--nocapture".to_string(),
    ]);
    assert_eq!(args, vec!["test", "--lib", "--", "--nocapture"]);
}

// --- Coverage path construction ---

#[test]
fn coverage_output_dir() {
    let p = cmd_test::coverage_output_dir();
    assert_eq!(p, std::path::PathBuf::from("target/slicer/coverage"));
}

// --- Coverage command assembly ---

#[test]
fn llvm_cov_args() {
    let cov_dir = cmd_test::coverage_output_dir();
    let args = cmd_test::llvm_cov_args(&cov_dir);
    assert!(args.iter().any(|a| a == "llvm-cov"));
    assert!(args.iter().any(|a| a.contains("target/slicer/coverage")));
}

#[test]
fn check_llvm_cov_available_returns_bool() {
    let available = cmd_test::is_llvm_cov_available();
    let _: bool = available;
}

// --- TestError Display ---

#[test]
fn test_error_display_missing_cargo_toml() {
    let err = cmd_test::TestError::MissingCargoToml;
    let msg = format!("{err}");
    assert!(
        msg.contains("Cargo.toml"),
        "error message should mention Cargo.toml: {msg}"
    );
}

#[test]
fn test_error_display_parse_error() {
    let err = cmd_test::TestError::CargoTomlParseError("bad field".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("Cargo.toml") || msg.contains("parse"),
        "got: {msg}"
    );
}

#[test]
fn test_error_display_test_failed() {
    let err = cmd_test::TestError::TestRunnerFailed("test output".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("test"), "got: {msg}");
}

#[test]
fn test_error_display_coverage_failed() {
    let err = cmd_test::TestError::CoverageFailed("cov error".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("coverage") || msg.contains("cov"),
        "got: {msg}"
    );
}

#[test]
fn test_error_display_io() {
    let err = cmd_test::TestError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file gone",
    ));
    let msg = format!("{err}");
    assert!(
        msg.contains("I/O") || msg.contains("file gone"),
        "got: {msg}"
    );
}
