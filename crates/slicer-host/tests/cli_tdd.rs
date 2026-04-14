//! TDD tests for the slicer-host CLI argument parsing.

use clap::Parser;
use slicer_host::cli::{CliError, HostCli, HostCommands};
use std::io::Write;

#[test]
fn run_requires_module_and_model() {
    let result = HostCli::try_parse_from(["slicer-host", "run"]);
    assert!(
        result.is_err(),
        "run without --module and --model should fail"
    );
}

#[test]
fn run_parses_all_flags() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
        "--module",
        "/tmp/mod.wasm",
        "--model",
        "/tmp/model.stl",
        "--config",
        "/tmp/config.json",
        "--output",
        "/tmp/out.gcode",
        "--module-dir",
        "/modules",
    ])
    .expect("should parse all flags");

    match cli.command {
        HostCommands::Run {
            module,
            model,
            config,
            output,
            module_dir,
        } => {
            assert_eq!(module, "/tmp/mod.wasm");
            assert_eq!(model, "/tmp/model.stl");
            assert_eq!(config.as_deref(), Some("/tmp/config.json"));
            assert_eq!(output.as_deref(), Some("/tmp/out.gcode"));
            assert_eq!(module_dir, "/modules");
        }
        _ => panic!("expected Run command"),
    }
}

#[test]
fn run_optional_config_and_output() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
        "--module",
        "/tmp/mod.wasm",
        "--model",
        "/tmp/model.stl",
    ])
    .expect("should parse with only required flags");

    match cli.command {
        HostCommands::Run {
            config,
            output,
            module_dir,
            ..
        } => {
            assert!(config.is_none(), "config should be None");
            assert!(output.is_none(), "output should be None");
            assert_eq!(module_dir, ".", "module_dir should default to '.'");
        }
        _ => panic!("expected Run command"),
    }
}

#[test]
fn config_schema_default_dir() {
    let cli = HostCli::try_parse_from(["slicer-host", "config-schema"])
        .expect("config-schema with no args should parse");

    match cli.command {
        HostCommands::ConfigSchema { module_dir } => {
            assert_eq!(module_dir, ".", "module_dir should default to '.'");
        }
        _ => panic!("expected ConfigSchema command"),
    }
}

#[test]
fn config_schema_custom_dir() {
    let cli = HostCli::try_parse_from(["slicer-host", "config-schema", "--module-dir", "/foo"])
        .expect("config-schema with --module-dir should parse");

    match cli.command {
        HostCommands::ConfigSchema { module_dir } => {
            assert_eq!(module_dir, "/foo");
        }
        _ => panic!("expected ConfigSchema command"),
    }
}

#[test]
fn validate_run_options_missing_model() {
    // Create a real module file but use a nonexistent model path.
    let dir = tempfile::tempdir().unwrap();
    let module_path = dir.path().join("mod.wasm");
    std::fs::File::create(&module_path).unwrap();

    let result = slicer_host::cli::validate_run_options(
        module_path.to_str().unwrap(),
        "/nonexistent/model.stl",
        None,
        None,
        ".",
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        CliError::MissingModel(p) => {
            assert_eq!(p.to_str().unwrap(), "/nonexistent/model.stl");
        }
        other => panic!("expected MissingModel, got: {:?}", other),
    }
}

#[test]
fn validate_run_options_missing_module() {
    let result = slicer_host::cli::validate_run_options(
        "/nonexistent/mod.wasm",
        "/nonexistent/model.stl",
        None,
        None,
        ".",
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        CliError::MissingModule(p) => {
            assert_eq!(p.to_str().unwrap(), "/nonexistent/mod.wasm");
        }
        other => panic!("expected MissingModule, got: {:?}", other),
    }
}

#[test]
fn validate_run_options_valid() {
    let dir = tempfile::tempdir().unwrap();
    let module_path = dir.path().join("mod.wasm");
    let model_path = dir.path().join("model.stl");

    let mut f = std::fs::File::create(&module_path).unwrap();
    f.write_all(b"fake wasm").unwrap();
    let mut f = std::fs::File::create(&model_path).unwrap();
    f.write_all(b"fake stl").unwrap();

    let opts = slicer_host::cli::validate_run_options(
        module_path.to_str().unwrap(),
        model_path.to_str().unwrap(),
        None,
        Some("/tmp/out.gcode"),
        dir.path().to_str().unwrap(),
    )
    .expect("should validate successfully");

    assert_eq!(opts.module_path, module_path);
    assert_eq!(opts.model_path, model_path);
    assert!(opts.config_path.is_none());
    assert_eq!(
        opts.output_path.as_deref(),
        Some(std::path::Path::new("/tmp/out.gcode"))
    );
    assert_eq!(opts.module_dir, dir.path());
}
