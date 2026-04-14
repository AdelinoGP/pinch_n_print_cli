//! Implementation of the `slicer run` subcommand.
//!
//! Runs the local module against a real model by orchestrating:
//! manifest validation → WASM binary discovery → model file check →
//! host binary availability → host invocation with module+model+config → output.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cmd_build;
use crate::cmd_validate;

/// Errors that can occur during the run process.
#[derive(Debug)]
pub enum RunError {
    /// No module manifest found in the current directory.
    MissingManifest,
    /// Manifest validation failed.
    ManifestValidationFailed(String),
    /// No compiled WASM binary found at the expected path.
    MissingWasm(PathBuf),
    /// The specified model file does not exist.
    MissingModel(PathBuf),
    /// The `slicer-host` binary is not available on PATH.
    MissingHostBinary,
    /// The host process returned a non-zero exit code.
    HostExecutionFailed(String),
    /// Config file could not be parsed as JSON.
    ConfigParseError(String),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingManifest => write!(f, "no module manifest found in the current directory"),
            Self::ManifestValidationFailed(msg) => write!(f, "manifest validation failed: {msg}"),
            Self::MissingWasm(path) => {
                write!(f, "WASM binary not found at {}", path.display())
            }
            Self::MissingModel(path) => {
                write!(f, "model file not found: {}", path.display())
            }
            Self::MissingHostBinary => {
                write!(f, "slicer-host binary not found on PATH (install with: cargo install slicer-host)")
            }
            Self::HostExecutionFailed(msg) => write!(f, "host execution failed: {msg}"),
            Self::ConfigParseError(msg) => write!(f, "config file parse error: {msg}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for RunError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Discover the compiled WASM binary for the module in the given directory.
///
/// Parses `Cargo.toml` to get the module name, then checks
/// `target/slicer/<module-name>.wasm` for existence.
///
/// # Errors
///
/// Returns [`RunError::MissingManifest`] if `Cargo.toml` cannot be parsed,
/// or [`RunError::MissingWasm`] if the expected binary does not exist.
pub fn find_wasm_binary(dir: &Path) -> Result<PathBuf, RunError> {
    let module_name = cmd_build::parse_module_name(dir).map_err(|_| RunError::MissingManifest)?;
    let wasm_path = dir.join(cmd_build::final_output_path(&module_name));
    if !wasm_path.exists() {
        return Err(RunError::MissingWasm(wasm_path));
    }
    Ok(wasm_path)
}

/// Validate that the given model file exists.
///
/// # Errors
///
/// Returns [`RunError::MissingModel`] if the path does not exist.
pub fn check_model_exists(path: &Path) -> Result<(), RunError> {
    if !path.exists() {
        return Err(RunError::MissingModel(path.to_path_buf()));
    }
    Ok(())
}

/// Parse a JSON config file at the given path.
///
/// # Errors
///
/// Returns [`RunError::ConfigParseError`] if the file cannot be read or parsed as JSON.
pub fn parse_config_file(path: &Path) -> Result<serde_json::Value, RunError> {
    let content = fs::read_to_string(path)
        .map_err(|e| RunError::ConfigParseError(format!("cannot read {}: {e}", path.display())))?;
    serde_json::from_str(&content)
        .map_err(|e| RunError::ConfigParseError(format!("invalid JSON in {}: {e}", path.display())))
}

/// Check whether the `slicer-host` binary is available on PATH.
pub fn check_host_binary() -> bool {
    Command::new("slicer-host")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build the argument list for invoking `slicer-host` with the module and model.
///
/// The resulting args are: `run --module <wasm> --model <model> [--config <config>] [--output <output>]`.
pub fn build_host_args(
    wasm: &Path,
    model: &Path,
    config: Option<&Path>,
    output: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--module".to_string(),
        wasm.to_string_lossy().to_string(),
        "--model".to_string(),
        model.to_string_lossy().to_string(),
    ];
    if let Some(cfg) = config {
        args.push("--config".to_string());
        args.push(cfg.to_string_lossy().to_string());
    }
    if let Some(out) = output {
        args.push("--output".to_string());
        args.push(out.to_string_lossy().to_string());
    }
    args
}

/// Validate the module manifest in the given directory.
///
/// # Errors
///
/// Returns [`RunError::MissingManifest`] or [`RunError::ManifestValidationFailed`].
pub fn validate_manifest(dir: &Path) -> Result<(), RunError> {
    cmd_validate::execute_in(dir).map_err(|e| match e {
        cmd_validate::ValidateError::ManifestNotFound => RunError::MissingManifest,
        other => RunError::ManifestValidationFailed(other.to_string()),
    })
}

/// Execute the `slicer run` workflow in the given directory.
///
/// 1. Validate manifest.
/// 2. Discover WASM binary.
/// 3. Check model file exists.
/// 4. Parse config file (optional).
/// 5. Check host binary availability.
/// 6. Invoke the host with the module and model.
///
/// # Errors
///
/// Returns a [`RunError`] if any step fails.
pub fn execute_in(
    dir: &Path,
    model: &str,
    config: Option<&str>,
    output: Option<&str>,
) -> Result<(), RunError> {
    // Step 1: Validate manifest
    validate_manifest(dir)?;

    // Step 2: Find WASM binary
    let wasm_path = find_wasm_binary(dir)?;

    // Step 3: Check model file
    let model_path = Path::new(model);
    check_model_exists(model_path)?;

    // Step 4: Parse config (optional)
    if let Some(cfg) = config {
        let cfg_path = Path::new(cfg);
        let _config_value = parse_config_file(cfg_path)?;
    }

    // Step 5: Check host binary
    if !check_host_binary() {
        return Err(RunError::MissingHostBinary);
    }

    // Step 6: Invoke host
    let config_path = config.map(Path::new);
    let output_path = output.map(Path::new);
    let args = build_host_args(&wasm_path, model_path, config_path, output_path);

    println!("Running module: {}", wasm_path.display());
    println!("Model: {model}");

    let host_output = Command::new("slicer-host")
        .args(&args)
        .status()
        .map_err(RunError::Io)?;

    if !host_output.success() {
        return Err(RunError::HostExecutionFailed(format!(
            "exit code: {}",
            host_output.code().unwrap_or(-1)
        )));
    }

    Ok(())
}

/// Execute the `slicer run` workflow in the current directory.
///
/// # Errors
///
/// Returns a [`RunError`] if any step fails.
pub fn execute(model: &str, config: Option<&str>, output: Option<&str>) -> Result<(), RunError> {
    let cwd = std::env::current_dir()?;
    execute_in(&cwd, model, config, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: create a minimal valid module project directory ───────────

    fn write_cargo_toml(dir: &Path, name: &str) {
        fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
    }

    fn write_valid_manifest(dir: &Path) {
        fs::write(
            dir.join("my-module.toml"),
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "A test module"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = ["infill-generator"]
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#,
        )
        .unwrap();
    }

    fn write_wasm_binary(dir: &Path, module_name: &str) {
        let wasm_dir = dir.join("target").join("slicer");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join(format!("{module_name}.wasm")), b"fake-wasm").unwrap();
    }

    fn write_model_file(dir: &Path, name: &str) {
        fs::write(dir.join(name), b"fake-stl-data").unwrap();
    }

    fn write_config_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    // ── WASM binary discovery ────────────────────────────────────────────

    #[test]
    fn find_wasm_binary_found() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-infill");
        write_wasm_binary(dir.path(), "my-infill");

        let result = find_wasm_binary(dir.path());
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("my-infill.wasm"));
    }

    #[test]
    fn find_wasm_binary_not_found() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-infill");
        // No WASM binary written

        let result = find_wasm_binary(dir.path());
        assert!(matches!(result, Err(RunError::MissingWasm(_))));
    }

    #[test]
    fn find_wasm_binary_correct_path_from_module_name() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "cool-perimeters");
        write_wasm_binary(dir.path(), "cool-perimeters");

        let path = find_wasm_binary(dir.path()).unwrap();
        assert!(
            path.to_string_lossy()
                .contains("target/slicer/cool-perimeters.wasm"),
            "path should use module name with hyphens preserved: {path:?}"
        );
    }

    #[test]
    fn find_wasm_binary_no_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        // No Cargo.toml at all

        let result = find_wasm_binary(dir.path());
        assert!(matches!(result, Err(RunError::MissingManifest)));
    }

    // ── Model file check ─────────────────────────────────────────────────

    #[test]
    fn check_model_exists_found() {
        let dir = tempfile::tempdir().unwrap();
        write_model_file(dir.path(), "cube.stl");

        let result = check_model_exists(&dir.path().join("cube.stl"));
        assert!(result.is_ok());
    }

    #[test]
    fn check_model_exists_missing() {
        let dir = tempfile::tempdir().unwrap();

        let result = check_model_exists(&dir.path().join("nonexistent.stl"));
        assert!(matches!(result, Err(RunError::MissingModel(_))));
    }

    // ── Config file parsing ──────────────────────────────────────────────

    #[test]
    fn parse_config_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        write_config_file(
            dir.path(),
            "config.json",
            r#"{"density": 0.15, "enabled": true}"#,
        );

        let result = parse_config_file(&dir.path().join("config.json"));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["density"], 0.15);
        assert_eq!(val["enabled"], true);
    }

    #[test]
    fn parse_config_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        write_config_file(dir.path(), "config.json", "not valid json {{{");

        let result = parse_config_file(&dir.path().join("config.json"));
        assert!(matches!(result, Err(RunError::ConfigParseError(_))));
    }

    #[test]
    fn parse_config_missing_file() {
        let dir = tempfile::tempdir().unwrap();

        let result = parse_config_file(&dir.path().join("missing.json"));
        assert!(matches!(result, Err(RunError::ConfigParseError(_))));
    }

    #[test]
    fn parse_config_empty_json_object() {
        let dir = tempfile::tempdir().unwrap();
        write_config_file(dir.path(), "config.json", "{}");

        let result = parse_config_file(&dir.path().join("config.json"));
        assert!(result.is_ok());
    }

    // ── Host argument construction ───────────────────────────────────────

    #[test]
    fn build_host_args_minimal() {
        let args = build_host_args(
            Path::new("target/slicer/my-infill.wasm"),
            Path::new("cube.stl"),
            None,
            None,
        );
        assert_eq!(
            args,
            vec![
                "run",
                "--module",
                "target/slicer/my-infill.wasm",
                "--model",
                "cube.stl"
            ]
        );
    }

    #[test]
    fn build_host_args_with_config() {
        let args = build_host_args(
            Path::new("target/slicer/my-infill.wasm"),
            Path::new("cube.stl"),
            Some(Path::new("config.json")),
            None,
        );
        assert_eq!(
            args,
            vec![
                "run",
                "--module",
                "target/slicer/my-infill.wasm",
                "--model",
                "cube.stl",
                "--config",
                "config.json"
            ]
        );
    }

    #[test]
    fn build_host_args_with_output() {
        let args = build_host_args(
            Path::new("target/slicer/my-infill.wasm"),
            Path::new("cube.stl"),
            None,
            Some(Path::new("output.gcode")),
        );
        assert_eq!(
            args,
            vec![
                "run",
                "--module",
                "target/slicer/my-infill.wasm",
                "--model",
                "cube.stl",
                "--output",
                "output.gcode"
            ]
        );
    }

    #[test]
    fn build_host_args_all_options() {
        let args = build_host_args(
            Path::new("target/slicer/my-infill.wasm"),
            Path::new("cube.stl"),
            Some(Path::new("config.json")),
            Some(Path::new("output.gcode")),
        );
        assert_eq!(
            args,
            vec![
                "run",
                "--module",
                "target/slicer/my-infill.wasm",
                "--model",
                "cube.stl",
                "--config",
                "config.json",
                "--output",
                "output.gcode"
            ]
        );
    }

    // ── Error display ────────────────────────────────────────────────────

    #[test]
    fn error_display_missing_manifest() {
        let err = RunError::MissingManifest;
        let msg = err.to_string();
        assert!(msg.contains("manifest"), "should mention manifest: {msg}");
    }

    #[test]
    fn error_display_missing_wasm() {
        let err = RunError::MissingWasm(PathBuf::from("target/slicer/foo.wasm"));
        let msg = err.to_string();
        assert!(msg.contains("foo.wasm"), "should include path: {msg}");
    }

    #[test]
    fn error_display_missing_model() {
        let err = RunError::MissingModel(PathBuf::from("cube.stl"));
        let msg = err.to_string();
        assert!(msg.contains("cube.stl"), "should include path: {msg}");
    }

    #[test]
    fn error_display_missing_host() {
        let err = RunError::MissingHostBinary;
        let msg = err.to_string();
        assert!(
            msg.contains("slicer-host"),
            "should mention slicer-host: {msg}"
        );
    }

    #[test]
    fn error_display_config_parse() {
        let err = RunError::ConfigParseError("bad json".into());
        let msg = err.to_string();
        assert!(msg.contains("bad json"), "should include reason: {msg}");
    }

    #[test]
    fn error_display_host_execution_failed() {
        let err = RunError::HostExecutionFailed("exit code: 1".into());
        let msg = err.to_string();
        assert!(msg.contains("exit code"), "should include exit info: {msg}");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err = RunError::from(io_err);
        assert!(matches!(err, RunError::Io(_)));
    }

    // ── Manifest validation ──────────────────────────────────────────────

    #[test]
    fn validate_manifest_valid() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        write_valid_manifest(dir.path());

        let result = validate_manifest(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_manifest_no_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        // No manifest .toml written

        let result = validate_manifest(dir.path());
        assert!(matches!(result, Err(RunError::MissingManifest)));
    }

    #[test]
    fn validate_manifest_invalid_stage() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        fs::write(
            dir.path().join("bad.toml"),
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "test"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Bogus"
"#,
        )
        .unwrap();

        let result = validate_manifest(dir.path());
        assert!(matches!(result, Err(RunError::ManifestValidationFailed(_))));
    }

    // ── Full integration (execute_in) ────────────────────────────────────
    // Note: These tests exercise the orchestration up to the host binary check.
    // Since slicer-host is not installed in the test environment, they verify
    // the pipeline stops with MissingHostBinary (or earlier errors).

    #[test]
    fn execute_in_missing_manifest_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        // No manifest

        let result = execute_in(dir.path(), "cube.stl", None, None);
        assert!(matches!(result, Err(RunError::MissingManifest)));
    }

    #[test]
    fn execute_in_missing_wasm_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        write_valid_manifest(dir.path());
        write_model_file(dir.path(), "cube.stl");
        // No WASM binary

        let result = execute_in(
            dir.path(),
            &dir.path().join("cube.stl").to_string_lossy(),
            None,
            None,
        );
        assert!(matches!(result, Err(RunError::MissingWasm(_))));
    }

    #[test]
    fn execute_in_missing_model_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        write_valid_manifest(dir.path());
        write_wasm_binary(dir.path(), "my-module");
        // No model file

        let result = execute_in(dir.path(), "/nonexistent/cube.stl", None, None);
        assert!(matches!(result, Err(RunError::MissingModel(_))));
    }

    #[test]
    fn execute_in_invalid_config_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        write_valid_manifest(dir.path());
        write_wasm_binary(dir.path(), "my-module");
        write_model_file(dir.path(), "cube.stl");
        write_config_file(dir.path(), "bad.json", "not json");

        let model_path = dir.path().join("cube.stl");
        let config_path = dir.path().join("bad.json");
        let result = execute_in(
            dir.path(),
            &model_path.to_string_lossy(),
            Some(&config_path.to_string_lossy()),
            None,
        );
        assert!(matches!(result, Err(RunError::ConfigParseError(_))));
    }

    #[test]
    fn execute_in_reaches_host_check() {
        // With valid manifest, WASM, model, and config — should fail at host binary check
        let dir = tempfile::tempdir().unwrap();
        write_cargo_toml(dir.path(), "my-module");
        write_valid_manifest(dir.path());
        write_wasm_binary(dir.path(), "my-module");
        write_model_file(dir.path(), "cube.stl");
        write_config_file(dir.path(), "config.json", r#"{"density": 0.15}"#);

        let model_path = dir.path().join("cube.stl");
        let config_path = dir.path().join("config.json");
        let result = execute_in(
            dir.path(),
            &model_path.to_string_lossy(),
            Some(&config_path.to_string_lossy()),
            Some("output.gcode"),
        );
        // slicer-host is not installed in test env, so this should fail at the host check
        assert!(
            matches!(result, Err(RunError::MissingHostBinary)),
            "expected MissingHostBinary, got: {result:?}"
        );
    }
}
