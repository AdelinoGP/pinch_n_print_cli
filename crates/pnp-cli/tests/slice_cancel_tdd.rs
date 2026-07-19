//! End-to-end coverage for graceful slice cancellation and stdin EOF handling.

use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn benchy_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

#[test]
fn stdin_eof_cancels() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("cancelled.gcode");

    let output = Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .arg("slice")
        .arg("--model")
        .arg(benchy_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode)
        .arg("--cancel-on-stdin-eof")
        .write_stdin(Vec::<u8>::new())
        .output()
        .expect("spawn pnp_cli");

    assert_eq!(output.status.code(), Some(130));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("\"event\":\"cancelled\""),
        "stderr: {stderr}"
    );
    assert!(!gcode.exists(), "cancelled slice must not write output");
}

#[test]
fn no_flag_stdin_eof_completes() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("completed.gcode");

    let output = Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .arg("slice")
        .arg("--model")
        .arg(benchy_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode)
        .write_stdin(Vec::<u8>::new())
        .output()
        .expect("spawn pnp_cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("\"event\":\"cancelled\""),
        "stderr: {stderr}"
    );
    assert!(gcode.exists(), "completed slice must write output");
}

#[test]
fn slice_help_documents_cancel_flag() {
    let output = Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .args(["slice", "--help"])
        .output()
        .expect("run pnp_cli help");

    assert!(output.status.success());
    let help = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(help.contains("cancel-on-stdin-eof"), "help: {help}");
    assert!(help.contains("130"), "help: {help}");
}
