//! Runtime-to-visual-emitter regression for a resolved tool-scoped limit.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn run_model_request(root: &Path, request: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_pnp_cli"))
        .current_dir(root)
        .args(["visual-debug", "--request"])
        .arg(request)
        .arg("--output")
        .arg(output)
        .output()
        .expect("model visual-debug process must launch");
    assert!(
        result.status.success(),
        "visual-debug failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&fs::read(output.join("manifest.json")).expect("manifest must exist"))
        .expect("manifest must be valid JSON")
}

fn outer_wall_feedrates(manifest: &Value) -> Vec<f64> {
    let image = manifest["images"]
        .as_array()
        .expect("manifest images must be an array")
        .iter()
        .find(|image| {
            image["tap"] == "PostPass::GCodeEmit" && image["visualization"] == "filament_lines"
        })
        .expect("model request must capture emitted filament lines");
    let capture = &image["typed_capture"];
    assert_eq!(capture["kind"], "GCodeEmit");
    let feedrates = capture["value"]["commands"]
        .as_array()
        .expect("capture must contain emitted G-code commands")
        .iter()
        .filter_map(|command| {
            let movement = command.get("Move")?;
            (movement["role"] == "OuterWall" && movement["e"].as_f64()? > 0.0).then(|| {
                movement["f"]
                    .as_f64()
                    .expect("extrusion must have a feedrate")
            })
        })
        .collect::<Vec<_>>();
    assert!(
        !feedrates.is_empty(),
        "capture must contain outer-wall extrusion"
    );
    assert!(feedrates.iter().all(|f| f.is_finite() && *f > 0.0));
    feedrates
}

/// A model-helper test alone cannot detect `prepare_prepass_context` discarding
/// tool configs. Compare actual CLI captures with the same model/global limit,
/// once with tool 0's narrower 12 mm³/s and once with only global 8 mm³/s.
#[test]
fn visual_debug_volumetric_auto_runtime_tool_override_changes_emitted_feedrates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = root.join("crates/pnp-cli/tests/fixtures/config_scope_resolution_10");
    let mut config: Value = serde_json::from_slice(
        &fs::read(fixture.join("visual-debug-config.json")).expect("companion config must exist"),
    )
    .expect("companion config must parse");
    assert_eq!(config["outer_wall_speed"], 0);
    assert_eq!(config["filament_max_volumetric_speed"], 8.0);
    assert_eq!(config["tool_config:0:filament_max_volumetric_speed"], 12.0);
    let temp = tempfile::tempdir().expect("temporary request directory must be writable");
    let overridden = run_model_request(
        &root,
        &fixture.join("visual-debug.json"),
        &temp.path().join("override"),
    );

    assert_eq!(
        config
            .as_object_mut()
            .expect("config must be an object")
            .remove("tool_config:0:filament_max_volumetric_speed"),
        Some(Value::from(12.0))
    );
    let control_config = temp.path().join("global-only.json");
    fs::write(&control_config, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut request: Value = serde_json::from_slice(
        &fs::read(fixture.join("visual-debug.json")).expect("model request must exist"),
    )
    .expect("model request must parse");
    request["source"]["config"] = Value::from(control_config.to_str().expect("UTF-8 config path"));
    let control_request = temp.path().join("global-only-request.json");
    fs::write(&control_request, serde_json::to_vec(&request).unwrap()).unwrap();
    let control = run_model_request(&root, &control_request, &temp.path().join("control"));

    let override_f = outer_wall_feedrates(&overridden);
    let control_f = outer_wall_feedrates(&control);
    assert_eq!(
        override_f.len(),
        control_f.len(),
        "same model must produce comparable moves"
    );
    for (actual, baseline) in override_f.iter().zip(control_f) {
        // Geometry/factors are unchanged. 12 / 8 is independently 1.5;
        // tolerance accommodates the documented three-decimal F rounding.
        assert!(
            (*actual - baseline * 1.5).abs() < 0.01,
            "tool override must reach the runtime emitter: override={actual}, global-only={baseline}"
        );
    }
}
