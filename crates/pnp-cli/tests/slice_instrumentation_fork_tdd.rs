//! Coverage for the 4-way instrumentation fork inside
//! `slicer_runtime::run::run_slice`.
//!
//! Step 4 moved the fork (`--report` × `--instrument-stderr`) out of the
//! deleted `main.rs::HostCommands::Run` arm into `run.rs::run_pipeline_fork`.
//! AC-2 exercises the (no-report, no-instrument) leaf. The remaining three
//! leaves are pinned here so the fork's composition can't silently regress.
//!
//! Fork combinations:
//! | --report | --instrument-stderr | Covered by                    |
//! | ---      | ---                 | ---                           |
//! | absent   | absent              | AC-2 (e2e_integration_tdd)    |
//! | present  | absent              | `slice_with_report_*`         |
//! | absent   | present             | `slice_with_instrument_*`     |
//! | present  | present             | `slice_with_report_and_instr_*` |
//!
//! Since the default-on core stream landed, the absent/absent leaf also
//! emits JSONL (core-contract events); the default stream's content is
//! covered by `slice_progress_events_default_tdd.rs`.

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
fn slice_with_report_emits_nonempty_html() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");
    let report = tmp.path().join("report.html");

    Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .arg("slice")
        .arg("--model")
        .arg(benchy_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode)
        .arg("--report")
        .arg(&report)
        .assert()
        .success();

    let html = std::fs::read_to_string(&report).expect("report HTML must exist");
    assert!(!html.is_empty(), "report HTML must be non-empty");
    assert!(
        html.contains("<html") || html.contains("<!DOCTYPE"),
        "report file must look like HTML, first 200 bytes: {:?}",
        &html.chars().take(200).collect::<String>()
    );
}

#[test]
fn slice_with_instrument_stderr_emits_jsonl_events() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");

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
        .arg("--instrument-stderr")
        .output()
        .expect("spawn pnp_cli");
    assert!(
        output.status.success(),
        "pnp_cli slice must succeed; stderr tail:\n{}",
        tail(&String::from_utf8_lossy(&output.stderr), 20)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let jsonl_lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("\"schema_version\"") && line.contains("\"slice_id\""))
        .collect();
    assert!(
        !jsonl_lines.is_empty(),
        "expected JSONL ProgressEvent lines on stderr (schema_version + slice_id fields); stderr tail:\n{}",
        tail(&stderr, 20)
    );
}

#[test]
fn slice_with_report_and_instrument_stderr_emits_both() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");
    let report = tmp.path().join("report.html");

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
        .arg("--report")
        .arg(&report)
        .arg("--instrument-stderr")
        .output()
        .expect("spawn pnp_cli");
    assert!(
        output.status.success(),
        "pnp_cli slice must succeed; stderr tail:\n{}",
        tail(&String::from_utf8_lossy(&output.stderr), 20)
    );

    let html = std::fs::read_to_string(&report).expect("report HTML must exist");
    assert!(!html.is_empty(), "report HTML must be non-empty");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let jsonl_present = stderr
        .lines()
        .any(|line| line.contains("\"schema_version\"") && line.contains("\"slice_id\""));
    assert!(
        jsonl_present,
        "composite fork must still emit JSONL ProgressEvents on stderr; tail:\n{}",
        tail(&stderr, 20)
    );
}

fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}
