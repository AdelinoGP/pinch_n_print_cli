//! Red-phase integration tests for `pnp_cli visual-debug` — request
//! validation, bundle manifest shape, atomicity, and negative cases.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

fn write_request(dir: &Path, body: Value) -> PathBuf {
    fs::create_dir_all(dir).expect("request directory");
    let path = dir.join("request.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&body).expect("request JSON should serialize"),
    )
    .expect("write request");
    path
}

fn run(request: &Path, output: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .args(["visual-debug", "--request"])
        .arg(request)
        .args(["--output"])
        .arg(output)
        .assert()
}

fn run_with_overwrite(request: &Path, output: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .args(["visual-debug", "--request"])
        .arg(request)
        .args(["--output"])
        .arg(output)
        .arg("--overwrite")
        .assert()
}

fn model_request() -> Value {
    json!({
        "schema_version": "1.0.0",
        "source": {
            "kind": "model",
            "model": "resources/benchy.stl",
            "config": "config.toml",
            "module_dirs": ["modules"]
        },
        "layers": [0],
        "taps": [],
        "visualizations": []
    })
}

fn gcode_request() -> Value {
    json!({
        "schema_version": "1.0.0",
        "source": {
            "kind": "gcode",
            "path": "reported.gcode"
        },
        "layers": [0],
        "taps": [],
        "visualizations": []
    })
}

fn manifest(output: &Path) -> Value {
    serde_json::from_slice(&fs::read(output.join("manifest.json")).expect("manifest exists"))
        .expect("manifest is JSON")
}

fn stderr_text(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

#[test]
fn ac_model_request_accepts_and_creates_manifest_state() {
    let tmp = TempDir::new().expect("tempdir");
    let request = write_request(tmp.path(), model_request());
    let output = tmp.path().join("bundle");

    run(&request, &output).success();

    let value = manifest(&output);
    assert_eq!(value["schema_version"], "1.0.0");
    assert!(value["images"].is_array());
    assert_eq!(value["source"]["kind"], "model");
}

#[test]
fn ac_gcode_request_accepts_as_exclusive_source() {
    let tmp = TempDir::new().expect("tempdir");
    let request = write_request(tmp.path(), gcode_request());
    let output = tmp.path().join("bundle");

    run(&request, &output).success();

    let value = manifest(&output);
    assert_eq!(value["source"]["kind"], "gcode");
    assert_eq!(value["source"]["path"], "reported.gcode");
    assert!(value["source"]["model"].is_null());
}

#[test]
fn ac_manifest_serializes_required_index_and_entry_fields() {
    let tmp = TempDir::new().expect("tempdir");
    let mut request_body = model_request();
    request_body["taps"] = json!(["taps.ir_view"]);
    request_body["visualizations"] = json!(["filament_lines"]);
    let request = write_request(tmp.path(), request_body);
    let output = tmp.path().join("bundle");

    run(&request, &output).success();

    let value = manifest(&output);
    for key in ["schema_version", "source", "images"] {
        assert!(value.get(key).is_some(), "manifest missing {key}");
    }
    let image = &value["images"][0];
    for key in [
        "source",
        "tap",
        "layer_index",
        "layer_z",
        "visualization",
        "png_path",
        "viewport",
        "legend_version",
        "ir_schema_version",
        "warnings",
    ] {
        assert!(image.get(key).is_some(), "image entry missing {key}");
    }
}

#[test]
fn ac_resolution_scale_contract() {
    let tmp = TempDir::new().expect("tempdir");

    for (name, scale, expected_width) in [
        ("omitted", None, 1024),
        ("one", Some(1), 1024),
        ("two", Some(2), 2048),
        ("three", Some(3), 3072),
    ] {
        let mut body = model_request();
        if let Some(scale) = scale {
            body["resolution_scale"] = json!(scale);
        }
        let request = write_request(&tmp.path().join(name), body);
        let output = tmp.path().join(format!("bundle-{name}"));
        run(&request, &output).success();

        let value = manifest(&output);
        assert_eq!(value["resolution_scale"], scale.unwrap_or(1));
        // docs/19_visual_debug.md:30-31 defines the 1024 base and scale multiplier.
        assert_eq!(value["viewport"]["width"], expected_width);
        assert_eq!(value["viewport"]["height"], expected_width);
    }
}

#[test]
fn ac_explicit_overwrite_replaces_non_empty_bundle() {
    let tmp = TempDir::new().expect("tempdir");
    let request = write_request(tmp.path(), model_request());
    let output = tmp.path().join("bundle");
    fs::create_dir(&output).expect("bundle dir");
    fs::write(output.join("manifest.json"), br#"{"test_marker":"stale"}"#).expect("sentinel");

    run_with_overwrite(&request, &output).success();

    let value = manifest(&output);
    assert_ne!(value["test_marker"], "stale");
    assert_eq!(value["schema_version"], "1.0.0");
}

#[test]
fn ac_n1_rejects_mixed_source_modes() {
    let tmp = TempDir::new().expect("tempdir");
    let mut body = model_request();
    body["source"]["path"] = json!("reported.gcode");
    let request = write_request(tmp.path(), body);
    let output = tmp.path().join("bundle");

    let result = run(&request, &output).failure();
    let stderr = stderr_text(&result).to_lowercase();
    assert!(stderr.contains("source"));
    assert!(stderr.contains("exclusive") || stderr.contains("mutually exclusive"));
    assert!(!output.join("manifest.json").exists());
}

#[test]
fn ac_n2_rejects_missing_source_mode() {
    let tmp = TempDir::new().expect("tempdir");
    let mut body = model_request();
    body["source"] = json!({"kind": "model"});
    let request = write_request(tmp.path(), body);
    let output = tmp.path().join("bundle");

    let result = run(&request, &output).failure();
    assert!(stderr_text(&result).to_lowercase().contains("source"));
    assert!(!output.join("manifest.json").exists());
}

#[test]
fn ac_model_source_requires_config_and_module_dirs() {
    let tmp = TempDir::new().expect("tempdir");
    // AC-1: model source requires `model`, `config`, and `module_dirs`.
    for field in ["config", "module_dirs"] {
        let mut body = model_request();
        body["source"].as_object_mut().unwrap().remove(field);
        let request = write_request(&tmp.path().join(field), body);
        let output = tmp.path().join(format!("bundle-{field}"));

        let result = run(&request, &output).failure();
        assert!(stderr_text(&result).contains(field));
        assert!(!output.join("manifest.json").exists());
    }
}

#[test]
fn ac_n3_rejects_out_of_range_resolution_scale() {
    let tmp = TempDir::new().expect("tempdir");
    for scale in [0, 4] {
        let mut body = model_request();
        body["resolution_scale"] = json!(scale);
        let request = write_request(&tmp.path().join(format!("scale-{scale}")), body);
        let output = tmp.path().join(format!("bundle-{scale}"));

        let result = run(&request, &output).failure();
        let stderr = stderr_text(&result).to_lowercase();
        assert!(stderr.contains('1') && stderr.contains('2') && stderr.contains('3'));
        assert!(!output.join("manifest.json").exists());
    }
}

#[test]
fn ac_n4_requires_gcode_line_width_for_standalone_filled_areas() {
    let tmp = TempDir::new().expect("tempdir");
    let mut body = gcode_request();
    body["visualizations"] = json!(["filled_areas"]);
    let request = write_request(tmp.path(), body);
    let output = tmp.path().join("bundle");

    let result = run(&request, &output).failure();
    assert!(stderr_text(&result).contains("gcode_line_width_mm"));
    assert!(!output.join("manifest.json").exists());
}

#[test]
fn ac_n5_rejects_non_empty_output_without_overwrite() {
    let tmp = TempDir::new().expect("tempdir");
    let request = write_request(tmp.path(), model_request());
    let output = tmp.path().join("bundle");
    fs::create_dir(&output).expect("bundle dir");
    let sentinel = br#"{"test_marker":"preserve"}"#;
    fs::write(output.join("manifest.json"), sentinel).expect("sentinel");

    let result = run(&request, &output).failure();
    assert!(stderr_text(&result).contains("--overwrite"));
    assert_eq!(fs::read(output.join("manifest.json")).unwrap(), sentinel);
}

#[test]
fn ac_n6_write_failure_is_fatal_and_not_success() {
    let tmp = TempDir::new().expect("tempdir");
    let request = write_request(tmp.path(), model_request());
    let output = tmp.path().join("blocker_file");
    fs::write(&output, b"not a directory").expect("blocker file");

    let result = run(&request, &output).failure();
    assert!(!stderr_text(&result).is_empty());
    assert!(!output.join("manifest.json").exists());
}
