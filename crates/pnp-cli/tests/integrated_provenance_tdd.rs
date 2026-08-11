use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pnp-cli has a parent")
        .parent()
        .expect("crates has a parent")
        .to_path_buf()
}

fn cli() -> Command {
    let mut command = Command::cargo_bin("pnp_cli").expect("pnp_cli binary");
    command.env_remove("SLICER_MODULE_PATH");
    command
}

fn fixture_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("test_stl")
        .join("ASCII")
        .join("20mmbox-LF.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn json_stdout(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "pnp_cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should be JSON")
}

#[test]
fn slice_flag_disables_integrated_tier() {
    let first = cli()
        .args(["slice", "--model"])
        .arg(fixture_path())
        .args(["--module-dir"])
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .output()
        .expect("run slice");
    assert!(first.status.success());
    assert!(String::from_utf8_lossy(&first.stderr).contains("shadows integrated module"));

    let second = cli()
        .args(["slice", "--model"])
        .arg(fixture_path())
        .args(["--module-dir"])
        .arg(module_dir())
        .args(["--no-default-module-paths", "--no-integrated-modules"])
        .output()
        .expect("run slice without integrated modules");
    assert!(second.status.success());
    assert!(!String::from_utf8_lossy(&second.stderr).contains("shadows integrated module"));
}

#[test]
fn diagnose_lists_integrated_provenance() {
    let output = cli()
        .args(["module", "diagnose", "--no-default-module-paths"])
        .output()
        .expect("run module diagnose");
    let json = json_stdout(&output);
    assert_eq!(json["modules_loaded"], 1);
    assert_eq!(
        json["modules"],
        serde_json::json!([{"id": "com.core.classic-perimeters", "provenance": "integrated"}])
    );
}

#[test]
fn dag_stages_sees_integrated_tier() {
    let first = cli()
        .args(["dag", "stages", "--no-default-module-paths"])
        .output()
        .expect("run dag stages");
    assert!(first.status.success());
    assert!(String::from_utf8_lossy(&first.stdout).contains("Layer::Perimeters"));

    let second = cli()
        .args([
            "dag",
            "stages",
            "--no-default-module-paths",
            "--no-integrated-modules",
        ])
        .output()
        .expect("run dag stages without integrated modules");
    assert!(second.status.success());
    assert!(!String::from_utf8_lossy(&second.stdout).contains("Layer::Perimeters"));
}

#[test]
fn config_schema_includes_integrated_module() {
    let first = cli()
        .args(["module", "config-schema", "--no-default-module-paths"])
        .output()
        .expect("run config schema");
    let json = json_stdout(&first);
    assert!(json["schema"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .any(|entry| { entry["module"] == "com.core.classic-perimeters" }));

    let second = cli()
        .args([
            "module",
            "config-schema",
            "--no-default-module-paths",
            "--no-integrated-modules",
        ])
        .output()
        .expect("run config schema without integrated modules");
    assert!(second.status.success());
    assert!(!String::from_utf8_lossy(&second.stdout).contains("classic-perimeters"));
}

#[test]
fn no_integrated_modules_empties_diagnose() {
    let output = cli()
        .args([
            "module",
            "diagnose",
            "--no-default-module-paths",
            "--no-integrated-modules",
        ])
        .output()
        .expect("run module diagnose without integrated modules");
    let json = json_stdout(&output);
    assert_eq!(json["modules_loaded"], 0);
    assert_eq!(json["modules"], serde_json::json!([]));
    assert!(!output
        .stdout
        .windows("com.core.classic-perimeters".len())
        .any(|window| window == b"com.core.classic-perimeters"));
}

#[test]
fn diagnose_shows_external_shadowing_integrated() {
    let output = cli()
        .args([
            "module",
            "diagnose",
            "--no-default-module-paths",
            "--module-dir",
        ])
        .arg(module_dir())
        .output()
        .expect("run module diagnose with external modules");
    let json = json_stdout(&output);
    let modules = json["modules"].as_array().expect("modules array");
    let matching: Vec<_> = modules
        .iter()
        .filter(|module| module["id"] == "com.core.classic-perimeters")
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["provenance"], "external");
    assert!(json["diagnostics"].as_array().expect("diagnostics array").iter().any(|diagnostic| {
        diagnostic["level"] == "warning"
            && diagnostic["message"]
                == "external module com.core.classic-perimeters shadows integrated module com.core.classic-perimeters"
    }));
}
