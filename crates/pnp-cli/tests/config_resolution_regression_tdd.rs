//! Config regressions through the shipped CLI, rather than a test-local resolver.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

fn cube(path: &Path) {
    let vertices = [
        [0., 0., 0.],
        [4., 0., 0.],
        [4., 4., 0.],
        [0., 4., 0.],
        [0., 0., 1.],
        [4., 0., 1.],
        [4., 4., 1.],
        [0., 4., 1.],
    ];
    let faces = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let triangles: Vec<_> = faces
        .into_iter()
        .map(|face| stl_io::Triangle {
            normal: stl_io::Normal::new([0., 0., 0.]),
            vertices: face.map(|i| stl_io::Vertex::new(vertices[i])),
        })
        .collect();
    stl_io::write_stl(&mut std::fs::File::create(path).unwrap(), triangles.iter()).unwrap();
}

fn slice(config: Value) -> (std::process::Output, TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let model = dir.path().join("cube.stl");
    let config_path = dir.path().join("config.json");
    let output_path = dir.path().join("out.gcode");
    cube(&model);
    std::fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let modules = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");
    let output = Command::cargo_bin("pnp_cli")
        .unwrap()
        .args([
            "slice",
            "--no-default-module-paths",
            "--no-integrated-modules",
            "--no-progress-events",
        ])
        .arg("--model")
        .arg(model)
        .arg("--module-dir")
        .arg(modules)
        .arg("--config")
        .arg(config_path)
        .arg("--output")
        .arg(&output_path)
        .timeout(std::time::Duration::from_secs(120))
        .output()
        .unwrap();
    (output, dir, output_path)
}

fn commands(gcode: &str) -> Vec<&str> {
    gcode
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .collect()
}

#[test]
fn cli_alias_and_string_boolean_match_canonical_typed_slice() {
    let (canonical, _canonical_dir, canonical_path) = slice(json!({
        "initial_layer_line_width": 0.6,
        "use_relative_e_distances": false,
        "machine_start_gcode": "",
        "machine_end_gcode": "",
    }));
    assert!(
        canonical.status.success(),
        "{}",
        String::from_utf8_lossy(&canonical.stderr)
    );
    let (alias, _alias_dir, alias_path) = slice(json!({
        "first_layer_line_width": 0.6,
        "use_relative_e_distances": "0",
        "machine_start_gcode": "",
        "machine_end_gcode": "",
    }));
    assert!(
        alias.status.success(),
        "{}",
        String::from_utf8_lossy(&alias.stderr)
    );
    let canonical_gcode = std::fs::read_to_string(canonical_path).unwrap();
    let alias_gcode = std::fs::read_to_string(alias_path).unwrap();
    let canonical_commands = commands(&canonical_gcode);
    let alias_commands = commands(&alias_gcode);
    assert!(
        canonical_commands
            .iter()
            .any(|line| line.starts_with("G1 ") && line.contains(" E")),
        "fixture must produce extrusion, not just a preamble"
    );
    assert!(
        canonical_commands.contains(&"M82"),
        "absolute E must be observable in the CLI output:\n{}",
        canonical_commands.join("\n")
    );
    assert!(
        !alias_commands.contains(&"M83"),
        "string 0 must not fall back to relative E"
    );
    assert_eq!(
        alias_commands, canonical_commands,
        "alias admission and string typing must preserve executable output"
    );
}

#[test]
fn cli_rejects_typed_layer_height_above_registry_max_before_output() {
    let (output, _dir, path) = slice(json!({"layer_height": 2.0}));
    assert!(
        !output.status.success(),
        "out-of-range layer_height must not slice"
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("layer_height"), "{error}");
    assert!(error.contains("outside allowed range"), "{error}");
    assert!(!path.exists(), "invalid config must not publish G-code");
}
