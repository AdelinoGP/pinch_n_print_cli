//! End-to-end silhouette postpass assembly and rendering contracts.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    postpass_stage_captures, run_visual_debug, FrameMode, LayerSelector, PostpassCaptureShape,
    TapSelector, VisualDebugError, VisualDebugRequest, VisualDebugSource, VisualizationSpec,
};
use serde_json::Value;
use slicer_ir::LayerCollectionIR;
use slicer_runtime::postpass::PostPassCapture;
use slicer_runtime::CapturedIr;
use tempfile::tempdir;

fn fixture() -> PostPassCapture {
    PostPassCapture {
        finalized_layers: vec![
            LayerCollectionIR {
                global_layer_index: 0,
                z: 0.2,
                ..Default::default()
            },
            LayerCollectionIR {
                global_layer_index: 1,
                z: 0.4,
                ..Default::default()
            },
            LayerCollectionIR {
                global_layer_index: 2,
                z: 0.6,
                ..Default::default()
            },
        ],
        gcode_ir: Default::default(),
    }
}

fn z_lookup(index: u32) -> f32 {
    [0.2, 0.4, 0.6][index as usize]
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn write_bounded_config(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height": 1.0}"#).unwrap();
    path
}

fn silhouette_request(
    config: PathBuf,
    tap: &str,
    specs: Vec<Value>,
    layers: Vec<LayerSelector>,
) -> VisualDebugRequest {
    // exhaustive: model-source silhouette request boundary fixture
    VisualDebugRequest {
        schema_version: "1.2.0".into(),
        source: VisualDebugSource::Model {
            model: Some(wedge_path()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers,
        taps: vec![TapSelector::Name(tap.into())],
        visualizations: specs
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

fn role_spec() -> Value {
    Value::Null
}

fn tool_spec() -> Value {
    serde_json::json!({"color_by": "tool"})
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn image_bytes(output: &Path, entry: &Value) -> Vec<u8> {
    fs::read(output.join(entry["png_path"].as_str().unwrap())).unwrap()
}

fn assert_empty(output: &Path) {
    assert!(!output.exists() || fs::read_dir(output).unwrap().next().is_none());
}

#[test]
fn postpass_whole_print_shape_one_capture_per_tap() {
    let capture = fixture();
    let stage_ids = ["PostPass::LayerFinalization".to_string()];
    let applicable = [0, 1, 2];

    let whole_print = postpass_stage_captures(
        &capture,
        &stage_ids,
        &applicable,
        &z_lookup,
        PostpassCaptureShape::WholePrint,
    );
    assert_eq!(whole_print.len(), 1);
    assert_eq!(whole_print[0].layer_index, 0);
    assert_eq!(whole_print[0].layer_z, 0.0);
    match &whole_print[0].ir {
        CapturedIr::LayerFinalization(layers) => {
            assert_eq!(layers, capture.finalized_layers.as_slice());
        }
        other => panic!("unexpected capture: {other:?}"),
    }

    let per_layer = postpass_stage_captures(
        &capture,
        &stage_ids,
        &applicable,
        &z_lookup,
        PostpassCaptureShape::PerLayer,
    );
    assert_eq!(per_layer.len(), 3);
    for (capture_row, expected_index) in per_layer.iter().zip(applicable) {
        assert_eq!(capture_row.layer_index, expected_index);
        assert_eq!(capture_row.layer_z, z_lookup(expected_index));
        match &capture_row.ir {
            CapturedIr::LayerFinalization(layers) => {
                assert_eq!(layers, capture.finalized_layers.as_slice());
            }
            other => panic!("unexpected capture: {other:?}"),
        }
    }

    let gcode_print = postpass_stage_captures(
        &capture,
        &["PostPass::GCodeEmit".to_string()],
        &applicable,
        &z_lookup,
        PostpassCaptureShape::WholePrint,
    );
    assert_eq!(gcode_print.len(), 1);
    assert_eq!(gcode_print[0].layer_index, 0);
    assert_eq!(gcode_print[0].layer_z, 0.0);
    assert!(matches!(
        &gcode_print[0].ir,
        CapturedIr::GCodeEmit(ir) if ir == &capture.gcode_ir
    ));
}

#[test]
fn postpass_silhouette_bundle_entry_shape() {
    let tmp = tempdir().unwrap();
    let output = tmp.path().join("bundle");
    let req = silhouette_request(
        write_bounded_config(&tmp.path().join("config")),
        "PostPass::LayerFinalization",
        vec![role_spec()],
        vec![LayerSelector::Range { start: 0, end: 1 }],
    );
    let manifest = manifest_at(&run_visual_debug(req, &output, false).unwrap());
    let images = manifest["images"].as_array().unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(
        images[0]["png_path"],
        "images/PostPass__LayerFinalization_silhouette_front.png"
    );
    assert_eq!(images[0]["view"], "front");
    assert_eq!(
        images[0]["layers_rendered"],
        serde_json::json!([{"start": 0, "end": 1}])
    );
    assert!(images[0].get("layer_index").is_none());
    assert!(images[0].get("layer_z").is_none());
}

#[test]
fn silhouette_role_and_tool_specs_render_distinct_images() {
    let tmp = tempdir().unwrap();
    let output = tmp.path().join("bundle");
    let req = silhouette_request(
        write_bounded_config(&tmp.path().join("config")),
        "PostPass::LayerFinalization",
        vec![role_spec(), tool_spec()],
        vec![LayerSelector::Range { start: 0, end: 1 }],
    );
    let manifest_path = run_visual_debug(req, &output, false).unwrap();
    let manifest = manifest_at(&manifest_path);
    let images = manifest["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(
        images[0]["png_path"],
        "images/PostPass__LayerFinalization_silhouette_front.png"
    );
    assert_eq!(
        images[1]["png_path"],
        "images/PostPass__LayerFinalization_silhouette_front_tool.png"
    );
    assert_eq!(images[1]["color_by"], "tool");
    assert_eq!(images[1]["tool_color_source"], "palette");
    assert!(manifest["tool_palette"].is_array());
    assert_ne!(
        image_bytes(&output, &images[0]),
        image_bytes(&output, &images[1])
    );
}

#[test]
fn postpass_z_frame_is_model_wide_not_selection_wide() {
    let tmp = tempdir().unwrap();
    let config = write_bounded_config(&tmp.path().join("config"));
    let subset = tmp.path().join("subset");
    let all = tmp.path().join("all");
    let subset_manifest = manifest_at(
        &run_visual_debug(
            silhouette_request(
                config.clone(),
                "PostPass::LayerFinalization",
                vec![role_spec()],
                vec![LayerSelector::Range { start: 0, end: 1 }],
            ),
            &subset,
            false,
        )
        .unwrap(),
    );
    let all_manifest = manifest_at(
        &run_visual_debug(
            silhouette_request(
                config,
                "PostPass::LayerFinalization",
                vec![role_spec()],
                vec![LayerSelector::Range { start: 0, end: 39 }],
            ),
            &all,
            false,
        )
        .unwrap(),
    );
    assert_eq!(
        subset_manifest["images"][0]["world_bounds_mm"],
        all_manifest["images"][0]["world_bounds_mm"]
    );
}

#[test]
fn subset_selection_gates_rendered_layers() {
    let tmp = tempdir().unwrap();
    let config = write_bounded_config(&tmp.path().join("config"));
    let subset = tmp.path().join("subset");
    let all = tmp.path().join("all");
    let subset_manifest = manifest_at(
        &run_visual_debug(
            silhouette_request(
                config.clone(),
                "PostPass::LayerFinalization",
                vec![role_spec()],
                vec![LayerSelector::Range { start: 0, end: 1 }],
            ),
            &subset,
            false,
        )
        .unwrap(),
    );
    let all_manifest = manifest_at(
        &run_visual_debug(
            silhouette_request(
                config,
                "PostPass::LayerFinalization",
                vec![role_spec()],
                vec![LayerSelector::Range { start: 0, end: 39 }],
            ),
            &all,
            false,
        )
        .unwrap(),
    );
    assert_eq!(
        subset_manifest["images"][0]["layers_rendered"],
        serde_json::json!([{"start": 0, "end": 1}])
    );
    assert_eq!(
        all_manifest["images"][0]["layers_rendered"],
        serde_json::json!([{"start": 0, "end": 39}])
    );
    assert_ne!(
        image_bytes(&subset, &subset_manifest["images"][0]),
        image_bytes(&all, &all_manifest["images"][0])
    );
}

#[test]
fn silhouette_tool_on_blackboard_tap_fails_tool_color_unavailable() {
    let tmp = tempdir().unwrap();
    let output = tmp.path().join("bundle");
    let err = run_visual_debug(
        silhouette_request(
            write_bounded_config(&tmp.path().join("config")),
            "Layer::Slice",
            vec![tool_spec()],
            vec![LayerSelector::Index(0)],
        ),
        &output,
        false,
    )
    .unwrap_err();
    assert!(
        matches!(err, VisualDebugError::RenderFailed(ref message)
        if message.contains("color_by \"tool\" is unavailable") && message.contains("Layer::Slice")),
        "{err:?}"
    );
    assert_empty(&output);
}

#[test]
fn duplicate_tool_specs_collapse_to_one_group() {
    let tmp = tempdir().unwrap();
    let output = tmp.path().join("bundle");
    let manifest = manifest_at(
        &run_visual_debug(
            silhouette_request(
                write_bounded_config(&tmp.path().join("config")),
                "PostPass::LayerFinalization",
                vec![tool_spec(), tool_spec()],
                vec![LayerSelector::Index(0)],
            ),
            &output,
            false,
        )
        .unwrap(),
    );
    assert_eq!(manifest["images"].as_array().unwrap().len(), 1);
}
