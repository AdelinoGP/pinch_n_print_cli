//! TDD tests for multi-root module search-path assembly.
//!
//! Logic tests call the `pub(crate)` inner helper
//! `assemble_search_roots_with` directly so the process environment is
//! never mutated. CLI-parse tests exercise the flag surface via clap.

use clap::Parser;
use slicer_host::cli::{HostCli, HostCommands};
use slicer_host::module_search_path::assemble_search_roots_with;
use std::path::PathBuf;

// -----------------------------------------------------------------------------
// CLI parsing of the new flag shape
// -----------------------------------------------------------------------------

#[test]
fn cli_module_dir_is_empty_vec_when_flag_absent() {
    let cli = HostCli::try_parse_from(["slicer-host", "run", "--model", "/tmp/model.stl"])
        .expect("parse with no --module-dir");

    match cli.command {
        HostCommands::Run { module_dir, .. } => assert!(module_dir.is_empty()),
        _ => panic!("expected Run"),
    }
}

#[test]
fn cli_module_dir_single_value() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
        "--model",
        "/tmp/model.stl",
        "--module-dir",
        "/a",
    ])
    .expect("parse single --module-dir");

    match cli.command {
        HostCommands::Run { module_dir, .. } => {
            assert_eq!(module_dir, vec![PathBuf::from("/a")]);
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn cli_module_dir_repeated_preserves_order() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
        "--model",
        "/tmp/model.stl",
        "--module-dir",
        "/a",
        "--module-dir",
        "/b",
        "--module-dir",
        "/c",
    ])
    .expect("parse repeated --module-dir");

    match cli.command {
        HostCommands::Run { module_dir, .. } => {
            assert_eq!(
                module_dir,
                vec![
                    PathBuf::from("/a"),
                    PathBuf::from("/b"),
                    PathBuf::from("/c")
                ]
            );
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn cli_no_default_module_paths_defaults_false() {
    let cli = HostCli::try_parse_from(["slicer-host", "run", "--model", "/tmp/model.stl"])
        .expect("parse without flag");

    match cli.command {
        HostCommands::Run {
            no_default_module_paths,
            ..
        } => assert!(!no_default_module_paths),
        _ => panic!("expected Run"),
    }
}

#[test]
fn cli_no_default_module_paths_true_when_given() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "run",
        "--model",
        "/tmp/model.stl",
        "--no-default-module-paths",
    ])
    .expect("parse with flag");

    match cli.command {
        HostCommands::Run {
            no_default_module_paths,
            ..
        } => assert!(no_default_module_paths),
        _ => panic!("expected Run"),
    }
}

#[test]
fn config_schema_module_dir_repeated() {
    let cli = HostCli::try_parse_from([
        "slicer-host",
        "config-schema",
        "--module-dir",
        "/x",
        "--module-dir",
        "/y",
    ])
    .expect("parse repeated --module-dir on config-schema");

    match cli.command {
        HostCommands::ConfigSchema { module_dir, .. } => {
            assert_eq!(module_dir, vec![PathBuf::from("/x"), PathBuf::from("/y")]);
        }
        _ => panic!("expected ConfigSchema"),
    }
}

#[test]
fn config_schema_no_default_module_paths_flag() {
    let cli =
        HostCli::try_parse_from(["slicer-host", "config-schema", "--no-default-module-paths"])
            .expect("parse with flag");

    match cli.command {
        HostCommands::ConfigSchema {
            no_default_module_paths,
            ..
        } => assert!(no_default_module_paths),
        _ => panic!("expected ConfigSchema"),
    }
}

// -----------------------------------------------------------------------------
// assemble_search_roots_with logic
// -----------------------------------------------------------------------------

#[test]
fn cli_dirs_appear_first_in_output() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let cli_dirs = vec![a.path().to_path_buf(), b.path().to_path_buf()];

    let roots = assemble_search_roots_with(&cli_dirs, None, true);

    assert_eq!(roots, cli_dirs);
}

#[test]
fn env_dirs_appear_after_cli_dirs() {
    let cli_dir = tempfile::tempdir().unwrap();
    let env_dir = tempfile::tempdir().unwrap();
    let env_value = std::env::join_paths([env_dir.path()]).unwrap();
    let env_value = env_value.into_string().unwrap();

    let cli_dirs = vec![cli_dir.path().to_path_buf()];
    let roots = assemble_search_roots_with(&cli_dirs, Some(&env_value), true);

    assert_eq!(
        roots,
        vec![cli_dir.path().to_path_buf(), env_dir.path().to_path_buf()]
    );
}

#[test]
fn cli_and_env_same_path_deduped() {
    let dir = tempfile::tempdir().unwrap();
    let env_value = std::env::join_paths([dir.path()]).unwrap();
    let env_value = env_value.into_string().unwrap();

    let cli_dirs = vec![dir.path().to_path_buf()];
    let roots = assemble_search_roots_with(&cli_dirs, Some(&env_value), true);

    assert_eq!(roots.len(), 1, "canonical dedup should collapse to one");
    assert_eq!(roots[0], dir.path());
}

#[test]
fn duplicate_cli_dirs_are_deduped() {
    let dir = tempfile::tempdir().unwrap();
    let cli_dirs = vec![dir.path().to_path_buf(), dir.path().to_path_buf()];

    let roots = assemble_search_roots_with(&cli_dirs, None, true);

    assert_eq!(roots.len(), 1);
}

#[test]
fn no_default_paths_with_empty_input_returns_empty() {
    let roots = assemble_search_roots_with(&[], None, true);
    assert!(roots.is_empty());
}

#[test]
fn missing_platform_defaults_silently_skipped() {
    // With no_default_paths=false on a typical test host, the platform
    // config and executable-relative modules dirs almost certainly do
    // not exist. The helper must skip them silently rather than error;
    // the output should therefore equal whatever CLI/env contributes
    // (empty here).
    let roots = assemble_search_roots_with(&[], None, false);
    // Don't assert exact emptiness — a CI machine might happen to have
    // `~/.config/modular-slicer/modules/` present. But every entry must
    // be a real directory (the helper filters by `.is_dir()`).
    for p in &roots {
        assert!(p.is_dir(), "default-path entry must exist: {}", p.display());
    }
}

#[test]
fn env_path_list_with_multiple_entries_splits_correctly() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let env_value = std::env::join_paths([a.path(), b.path()]).unwrap();
    let env_value = env_value.into_string().unwrap();

    let roots = assemble_search_roots_with(&[], Some(&env_value), true);

    assert_eq!(roots, vec![a.path().to_path_buf(), b.path().to_path_buf()]);
}

#[test]
fn empty_env_string_is_ignored() {
    let roots = assemble_search_roots_with(&[], Some(""), true);
    assert!(roots.is_empty());
}
