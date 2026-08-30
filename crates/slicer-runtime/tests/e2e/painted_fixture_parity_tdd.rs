//! Live-fixture regressions for seam painting and per-region shell settings.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::ConfigValue;
use slicer_runtime::{run_slice, SliceRunOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn slice_fixture(model: &str, config: HashMap<String, ConfigValue>) -> String {
    let root = workspace_root();
    let model_path = root.join(model);
    let mesh = Arc::new(slicer_model_io::load_model(&model_path).expect("model load"));
    run_slice(SliceRunOptions {
        mesh,
        model_label: model.to_string(),
        module_dirs: vec![root.join("modules/core-modules")],
        no_default_module_paths: true,
        config_overrides: config,
        ..Default::default()
    })
    .expect("fixture slice must succeed")
    .gcode_text
}

fn first_outer_wall_start(gcode: &str) -> (f32, f32) {
    let mut outer_wall = false;
    for line in gcode.lines() {
        if line == ";TYPE:Outer wall" {
            outer_wall = true;
            continue;
        }
        if outer_wall {
            if let Some(rest) = line.strip_prefix("G1 X") {
                let (x, rest) = rest.split_once(" Y").expect("outer-wall X/Y move");
                let (y, _) = rest.split_once(' ').expect("outer-wall Y/Z separator");
                return (
                    x.parse().expect("outer-wall X"),
                    y.parse().expect("outer-wall Y"),
                );
            }
        }
    }
    panic!("fixture G-code has no outer-wall start");
}

#[test]
fn painted_seam_fixture_reaches_emitted_outer_wall() {
    let mut config = HashMap::new();
    config.insert(
        "seam_position".to_string(),
        ConfigValue::String("aligned".to_string()),
    );
    let gcode = slice_fixture("resources/painted_seams.3mf", config);
    let (x, y) = first_outer_wall_start(&gcode);

    assert!((x - 129.0471).abs() < 0.5, "unexpected seam X: {x}");
    assert!((y - 87.4170).abs() < 0.5, "unexpected seam Y: {y}");
}

#[test]
fn shell_config_fixture_changes_projection_depth() {
    let root = workspace_root();
    let config_text =
        std::fs::read_to_string(root.join("resources/test_config/cube_4color-shell-config.json"))
            .expect("shell config fixture");
    let config: HashMap<String, ConfigValue> = serde_json::from_str(&config_text)
        .map(|source: serde_json::Map<String, serde_json::Value>| {
            source
                .into_iter()
                .map(|(key, value)| {
                    let config_value = match value {
                        serde_json::Value::Number(number) if number.is_i64() => {
                            ConfigValue::Int(number.as_i64().expect("integer config"))
                        }
                        serde_json::Value::Number(number) => {
                            ConfigValue::Float(number.as_f64().expect("float config"))
                        }
                        other => panic!("unsupported fixture config value: {other}"),
                    };
                    (key, config_value)
                })
                .collect()
        })
        .expect("shell config JSON object");

    let base = slice_fixture(
        "resources/cube_4color.3mf",
        config
            .iter()
            .filter(|(key, _)| !key.starts_with("paint_config:"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    let painted = slice_fixture("resources/cube_4color.3mf", config);

    assert_ne!(
        base, painted,
        "painted shell override must affect emitted G-code"
    );
}
