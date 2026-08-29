//! Packet 250 — GCodeEmit silhouette bundle and emitter-config contracts.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugRequest, VisualDebugSource,
    VisualizationSpec,
};
use serde_json::{json, Value};
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn request(config: PathBuf, visualizations: Vec<Value>) -> VisualDebugRequest {
    // exhaustive: model-source GCodeEmit silhouette fixture
    VisualDebugRequest {
        schema_version: "1.2.0".into(),
        source: VisualDebugSource::Model {
            model: Some(workspace_root().join("resources/regression_wedge.stl")),
            config: Some(config),
            module_dirs: vec![workspace_root().join("modules/core-modules")],
            path: None,
        },
        layers: vec![LayerSelector::Range { start: 0, end: 3 }],
        taps: vec![TapSelector::Name("PostPass::GCodeEmit".into())],
        visualizations: visualizations
            .into_iter()
            .map(|options| VisualizationSpec::Detail {
                kind: "silhouette".into(),
                options,
            })
            .collect(),
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn config(dir: &Path, diameter: Option<f32>) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join("config.json");
    let contents = match diameter {
        Some(d) => format!(r#"{{"layer_height":1.0,"filament_diameter":[{d}]}}"#),
        None => r#"{"layer_height":1.0}"#.into(),
    };
    fs::write(&path, contents).unwrap();
    path
}

fn manifest(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn gcode_emit_silhouette_bundle_entry_shape() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("bundle");
    let m = manifest(
        &run_visual_debug(
            request(config(tmp.path(), None), vec![json!({})]),
            &output,
            false,
        )
        .unwrap(),
    );
    let images = m["images"].as_array().unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(
        images[0]["png_path"],
        "images/PostPass__GCodeEmit_silhouette_front.png"
    );
    assert_eq!(images[0]["view"], "front");
    assert_eq!(images[0]["layers_rendered"], json!([{"start":0,"end":3}]));
    assert!(images[0].get("layer_index").is_none());
    assert!(images[0].get("layer_z").is_none());
    assert!(output
        .join("images/PostPass__GCodeEmit_silhouette_front.png")
        .exists());
}

#[test]
fn gcode_emit_role_and_tool_specs_render_distinct_images() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("bundle");
    let m = manifest(
        &run_visual_debug(
            request(
                config(tmp.path(), None),
                vec![Value::Null, json!({"color_by":"tool"})],
            ),
            &output,
            false,
        )
        .unwrap(),
    );
    let images = m["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(
        images[0]["png_path"],
        "images/PostPass__GCodeEmit_silhouette_front.png"
    );
    assert_eq!(
        images[1]["png_path"],
        "images/PostPass__GCodeEmit_silhouette_front_tool.png"
    );
    assert_eq!(images[1]["color_by"], "tool");
    assert!(images[1]["tool_palette"].is_array() || m["tool_palette"].is_array());
    assert_ne!(
        fs::read(output.join(images[0]["png_path"].as_str().unwrap())).unwrap(),
        fs::read(output.join(images[1]["png_path"].as_str().unwrap())).unwrap()
    );
}

fn first_e(manifest: &Value) -> f64 {
    let value = &manifest["images"][0]["typed_capture"];
    value["value"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|c| (c["Move"]["e"].is_number()).then(|| c["Move"]["e"].as_f64().unwrap()))
        .expect("GCodeEmit capture has an extruding Move")
}

#[test]
fn postpass_capture_emitter_uses_request_config_diameter() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    let ma = manifest(
        &run_visual_debug(
            request(config(&tmp.path().join("config-a"), Some(1.75)), vec![]),
            &a,
            false,
        )
        .unwrap(),
    );
    let mb = manifest(
        &run_visual_debug(
            request(config(&tmp.path().join("config-b"), Some(2.85)), vec![]),
            &b,
            false,
        )
        .unwrap(),
    );
    let e_a = first_e(&ma);
    let e_b = first_e(&mb);
    assert!(e_a > 0.0, "1.75 mm stream must extrude positively: {e_a}");
    assert!(e_b > 0.0, "2.85 mm stream must extrude positively: {e_b}");

    // Filament-area ratio (2.85/1.75)⁻².
    let ratio = e_b / e_a;
    let expected = (1.75f32 / 2.85f32).powi(2) as f64;
    assert!(
        (ratio - expected).abs() <= expected.abs() * 1e-3,
        "ratio {ratio} expected {expected}"
    );

    let ratio2 = e_a / e_b;
    let expected2 = (2.85f32 / 1.75f32).powi(2) as f64;
    assert!(
        (ratio2 - expected2).abs() <= expected2.abs() * 1e-3,
        "reciprocal ratio {ratio2} expected {expected2}"
    );
    assert!(
        (ratio2 - 1.0).abs() > 0.5,
        "diameter config had no effect: {ratio2}"
    );
    assert_eq!(first_e(&ma), e_a, "1.75 mm capture is not self-consistent");
    assert_eq!(first_e(&mb), e_b, "2.85 mm capture is not self-consistent");
}
