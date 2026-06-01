//! Tests for CLI-local I/O helpers.
//!
//! Packet 82 moved `write_with_parents` and `OutputFormat` out of
//! `slicer-runtime::cli` (deleted) into `pnp_cli::io`. The legacy `HostCli` /
//! `HostCommands` parser-shape tests covered dead code and were removed.
//!
//! The remaining tests pin:
//!   * `write_with_parents` correctly creates missing parent directories.
//!   * `write_with_parents` handles a bare filename (no parent component).
//!   * The HTML report Collector also creates missing parent dirs.

use pnp_cli::io::write_with_parents;
use std::path::PathBuf;

#[test]
fn report_path_creates_parent_dir() {
    use slicer_runtime::report::Collector;

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
