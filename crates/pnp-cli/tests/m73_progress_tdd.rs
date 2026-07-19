//! M73 progress injection + filament comment block end-to-end coverage
//! (docs/19 visual debug ↔ M73 progress contract).
//!
//! Verifies that a default `pnp_cli slice` writes at least one `M73 P0 R`
//! line, the final-region `M73 P100 R0`, an adjacent `M73 Q`/`S` stealth
//! pair for each progress marker, and the filament / estimated-time comment
//! block. The `disable_m73` config must suppress every `M73` line while
//! keeping the comment block intact.

use std::path::PathBuf;
use std::process::Output;

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

fn model_path() -> PathBuf {
    workspace_root().join("resources").join("20mm_cube.obj")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

fn run_slice(config: Option<&str>) -> (Output, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");
    let mut cmd = Command::cargo_bin("pnp_cli").expect("pnp_cli binary");
    cmd.arg("slice")
        .arg("--model")
        .arg(model_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode);
    if let Some(config) = config {
        let config_path = tmp.path().join("config.json");
        std::fs::write(&config_path, config).expect("write config");
        cmd.arg("--config").arg(config_path);
    }
    let output = cmd.output().expect("spawn pnp_cli");
    assert!(
        output.status.success(),
        "pnp_cli slice must succeed; stderr tail:\n{}",
        tail(&String::from_utf8_lossy(&output.stderr), 20)
    );
    (output, tmp)
}

#[test]
fn slice_emits_m73_and_filament_comments() {
    let (_output, tmp) = run_slice(None);
    let gcode = std::fs::read_to_string(tmp.path().join("out.gcode")).expect("gcode written");
    let lines: Vec<&str> = gcode.lines().collect();

    assert!(lines.iter().any(|line| line.starts_with("M73 P0 R")));
    assert!(lines.contains(&"M73 P100 R0"));
    let progress_count = lines
        .iter()
        .filter(|line| line.starts_with("M73 P"))
        .count();
    let stealth_pair_count = lines
        .windows(2)
        .filter(|pair| {
            pair[0].starts_with("M73 P") && pair[1].starts_with("M73 Q") && pair[1].contains(" S")
        })
        .count();
    assert_eq!(
        stealth_pair_count, progress_count,
        "each M73 marker needs an adjacent Q/S pair"
    );
    assert!(gcode.contains("; filament used [mm]"));
    assert!(gcode.contains("; estimated printing time (normal mode)"));
}

#[test]
fn disable_m73_suppresses_m73_keeps_comments() {
    let (_output, tmp) = run_slice(Some(r#"{"disable_m73":true}"#));
    let gcode = std::fs::read_to_string(tmp.path().join("out.gcode")).expect("gcode written");

    assert!(
        gcode.lines().all(|line| !line.starts_with("M73")),
        "disable_m73 must suppress every M73 line"
    );
    assert!(gcode.contains("; filament used [mm]"));
    assert!(gcode.contains("; estimated printing time (normal mode)"));
}
