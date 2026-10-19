//! AC-6: the real visual-debug model-source pipeline over the committed
//! single-range fixture.
//!
//! Drives `run_visual_debug` (not the CLI binary) on
//! `resources/layer_range_one_range.3mf`, selecting a `Layer::Slice` silhouette
//! that spans layers below, inside, and above the fixture's `[0.4, 0.8)`
//! `layer_height = 0.1` range. The bundle must succeed, write a manifest with
//! non-empty rendered layers, and report the range-derived mixed schedule
//! `[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]` — never the uniform
//! `[0.2, 0.4, 0.6, 0.8, 1.0]` grid the same model produces without its range.
//!
//! The manifest Z sequence is compared element-by-element against literal
//! expectations; no assertion derives its schedule from the produced bundle.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugRequest, VisualDebugSource,
    VisualizationSpec,
};
use serde_json::Value;
use tempfile::TempDir;

/// The fixture's range-derived schedule, as literal millimetre layer tops.
const EXPECTED_SCHEDULE: [f64; 7] = [0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0];
/// The uniform grid the same model produces when the range is ignored.
const UNIFORM_SCHEDULE: [f64; 5] = [0.2, 0.4, 0.6, 0.8, 1.0];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn fixture_model() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("layer_range_one_range.3mf")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// The fixture 3MF carries no `Metadata/project_settings.config`, so the model
/// request needs one sidecar the base scalars can be stated in. Only the base
/// heights are written here: the range's own `layer_height = 0.1` comes from
/// the fixture's `Metadata/layer_config_ranges.xml`, which is the input under
/// test.
fn write_base_config(dir: &Path) -> PathBuf {
    let path = dir.join("config.json");
    fs::write(&path, r#"{"layer_height": 0.2, "first_layer_height": 0.2}"#)
        .expect("write base config");
    path
}

fn request(config: PathBuf) -> VisualDebugRequest {
    // exhaustive: layer-range fixture request-boundary fixture
    VisualDebugRequest {
        schema_version: "1.3.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(fixture_model()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        // A range spanning below, inside, and above `[0.4, 0.8)`: the fixture
        // model is 1.0 mm tall, so 0..=11 covers every scheduled layer.
        layers: vec![LayerSelector::Range { start: 0, end: 11 }],
        taps: vec![TapSelector::Name("Layer::Slice".to_string())],
        visualizations: vec![VisualizationSpec::Detail {
            kind: "silhouette".to_string(),
            options: serde_json::json!({"view": "front"}),
        }],
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

#[test]
fn layer_range_fixture_changes_real_visual_debug_z_schedule() {
    let tmp = TempDir::new().expect("temporary output directory");
    let config = write_base_config(tmp.path());
    let output = tmp.path().join("bundle");

    let manifest_path = run_visual_debug(request(config), &output, false)
        .expect("the single-range fixture must render a real visual-debug bundle");
    let manifest = manifest_at(&manifest_path);

    assert_eq!(
        manifest["schema_version"], "1.3.0",
        "the bundle must record the declared schema version"
    );

    // The bundle must be a real bundle: non-empty rendered layers and their
    // PNGs on disk.
    let images = manifest["images"]
        .as_array()
        .expect("manifest images must be an array");
    assert!(
        !images.is_empty(),
        "the silhouette request must render at least one image"
    );
    let layers_rendered = images[0]["layers_rendered"]
        .as_array()
        .expect("a silhouette entry must carry layers_rendered ranges");
    assert!(
        !layers_rendered.is_empty(),
        "the silhouette must record the layers it rendered"
    );
    let png_path = images[0]["png_path"]
        .as_str()
        .expect("the silhouette entry must carry a string png_path");
    let png_file = output.join(png_path);
    assert!(
        png_file.is_file() && fs::metadata(&png_file).expect("png metadata").len() > 0,
        "the manifest's silhouette PNG must exist and be non-empty: {}",
        png_file.display()
    );

    // The schedule is the range-derived mixed sequence, compared literally.
    let scheduled = manifest["scheduled_layer_zs"]
        .as_array()
        .expect("a 1.3.0 model bundle must carry scheduled_layer_zs")
        .iter()
        .map(|value| {
            value
                .as_f64()
                .expect("scheduled_layer_zs entries must be numbers")
        })
        .collect::<Vec<f64>>();

    assert_eq!(
        scheduled.len(),
        EXPECTED_SCHEDULE.len(),
        "the fixture's [0.4, 0.8) layer_height = 0.1 range must yield the mixed \
         schedule; got {scheduled:?}"
    );
    for (index, (actual, expected)) in scheduled.iter().zip(EXPECTED_SCHEDULE).enumerate() {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "scheduled_layer_zs[{index}] must be {expected}, got {actual} \
             (full schedule {scheduled:?})"
        );
    }
    assert_ne!(
        scheduled, UNIFORM_SCHEDULE,
        "the range-derived schedule must differ from the uniform grid \
         {UNIFORM_SCHEDULE:?}; got {scheduled:?}"
    );
    assert!(
        scheduled.iter().any(|value| (value - 0.5).abs() < 1.0e-5),
        "the schedule must contain the range's 0.1 mm step top 0.5, which the \
         uniform grid never reaches; got {scheduled:?}"
    );
}
