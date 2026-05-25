//! TDD tests for the slicer-host CLI argument parsing.

use clap::Parser;
use slicer_host::cli::{write_with_parents, HostCli, HostCommands};
use std::path::PathBuf;

#[test]
fn run_requires_model() {
    let result = HostCli::try_parse_from(["slicer-host", "run"]);
    assert!(result.is_err(), "run without --model should fail");
    let result = HostCli::try_parse_from(["slicer-host", "run", "--model", "model.stl"]);
    assert!(result.is_ok(), "run with --model should succeed");
}

#[test]
fn run_parses_all_flags() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
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
            model,
            config,
            output,
            module_dir,
            no_default_module_paths,
            ..
        } => {
            assert_eq!(model, PathBuf::from("/tmp/model.stl"));
            assert_eq!(config, Some(PathBuf::from("/tmp/config.json")));
            assert_eq!(output, Some(PathBuf::from("/tmp/out.gcode")));
            assert_eq!(module_dir, vec![PathBuf::from("/modules")]);
            assert!(!no_default_module_paths);
        }
        _ => panic!("expected Run command"),
    }
}

#[test]
fn run_optional_config_and_output() {
    let cli = HostCli::try_parse_from(["slicer-host", "run", "--model", "/tmp/model.stl"])
        .expect("should parse with only required flags");

    match cli.command {
        HostCommands::Run {
            config,
            output,
            module_dir,
            no_default_module_paths,
            model: _,
            ..
        } => {
            assert!(config.is_none(), "config should be None");
            assert!(output.is_none(), "output should be None");
            assert!(
                module_dir.is_empty(),
                "module_dir should be an empty Vec when --module-dir is absent"
            );
            assert!(!no_default_module_paths);
        }
        _ => panic!("expected Run command"),
    }
}

#[test]
fn report_path_creates_parent_dir() {
    use slicer_host::report::Collector;

    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("subdir").join("report.html");

    let collector = Collector::new_with_verbose("test", false);
    let result = collector.finish_and_render_to(&nested);
    assert!(
        result.is_ok(),
        "should create parent dir and write report: {:?}",
        result.err()
    );
    assert!(nested.exists(), "report file should exist after write");
    assert!(nested.parent().unwrap().exists(), "parent dir should exist");
}

#[test]
fn output_path_creates_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("subdir").join("nested").join("out.gcode");
    assert!(
        !nested.parent().unwrap().exists(),
        "precondition: nested parent must not exist"
    );

    let result = write_with_parents(&nested, b"; test gcode\n");
    assert!(
        result.is_ok(),
        "write_with_parents should create parents and write file: {:?}",
        result.err()
    );
    assert!(nested.exists(), "output file should exist after write");
    assert_eq!(
        std::fs::read(&nested).unwrap(),
        b"; test gcode\n",
        "output file should contain the written bytes"
    );
}

#[test]
fn write_with_parents_handles_bare_filename() {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let bare = PathBuf::from("bare.gcode");

    let result = write_with_parents(&bare, b"x");

    std::env::set_current_dir(prev).unwrap();
    assert!(
        result.is_ok(),
        "bare filename (no parent) should not error: {:?}",
        result.err()
    );
    assert!(dir.path().join("bare.gcode").exists());
}

#[test]
fn config_schema_default_dir() {
    let cli = HostCli::try_parse_from(["slicer-host", "config-schema"])
        .expect("config-schema with no args should parse");

    match cli.command {
        HostCommands::ConfigSchema {
            module_dir,
            no_default_module_paths,
        } => {
            assert!(
                module_dir.is_empty(),
                "module_dir should be an empty Vec when --module-dir is absent"
            );
            assert!(!no_default_module_paths);
        }
        _ => panic!("expected ConfigSchema command"),
    }
}

#[test]
fn config_schema_custom_dir() {
    let cli = HostCli::try_parse_from(["slicer-host", "config-schema", "--module-dir", "/foo"])
        .expect("config-schema with --module-dir should parse");

    match cli.command {
        HostCommands::ConfigSchema {
            module_dir,
            no_default_module_paths,
        } => {
            assert_eq!(module_dir, vec![PathBuf::from("/foo")]);
            assert!(!no_default_module_paths);
        }
        _ => panic!("expected ConfigSchema command"),
    }
}
