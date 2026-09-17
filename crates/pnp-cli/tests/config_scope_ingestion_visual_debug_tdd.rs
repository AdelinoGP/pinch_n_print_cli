//! Production-reachability regression for typed config-scope ingestion.
//!
//! The request uses the real `cube_4color.3mf` model and its complete
//! `Metadata/project_settings.config` sidecar, then drives the library entry
//! point rather than spawning the CLI binary. One selected layer, one tap, and
//! one rendered visualization make the manifest/image postcondition exact.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugRequest, VisualDebugSource,
    VisualizationSpec,
};
use serde_json::Value;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// Copy the fixture's complete project-settings sidecar to the JSON path that
/// `run_visual_debug` accepts as `VisualDebugSource::Model::config`.
fn write_fixture_config(dir: &Path) -> PathBuf {
    let model = cube_4color_path();
    let file = fs::File::open(&model).expect("cube_4color.3mf should be readable");
    let mut archive = zip::ZipArchive::new(file).expect("cube_4color.3mf should be a ZIP");
    let entry_name = (0..archive.len())
        .find_map(|index| {
            archive
                .by_index(index)
                .ok()
                .filter(|entry| entry.name().ends_with("Metadata/project_settings.config"))
                .map(|entry| entry.name().to_string())
        })
        .expect("cube_4color.3mf should contain project_settings.config");
    let mut entry = archive
        .by_name(&entry_name)
        .expect("project_settings.config entry should be readable");
    let mut contents = String::new();
    entry
        .read_to_string(&mut contents)
        .expect("project_settings.config should be UTF-8 JSON");

    let path = dir.join("project_settings.config.json");
    fs::write(&path, contents).expect("write fixture config");
    path
}

fn model_request(config: PathBuf) -> VisualDebugRequest {
    // exhaustive: cube_4color model request-boundary fixture
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(cube_4color_path()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers: vec![LayerSelector::Index(0)],
        taps: vec![TapSelector::Name("Layer::Perimeters".to_string())],
        visualizations: vec![VisualizationSpec::Name("filled_areas".to_string())],
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
fn cube_4color_typed_ingestion_renders_manifest() {
    let tmp = TempDir::new().expect("temporary output directory");
    let config = write_fixture_config(tmp.path());
    let output = tmp.path().join("bundle");

    let manifest_path = run_visual_debug(model_request(config), &output, false)
        .expect("model-mode visual debug should succeed with typed config ingestion");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"]
        .as_array()
        .expect("manifest images must be an array");
    assert_eq!(
        images.len(),
        1,
        "one selected tap/layer/visualization must produce exactly one indexed image"
    );
    assert_eq!(
        images[0]["layer_index"], 0,
        "the indexed image must identify the requested layer"
    );
    assert_eq!(
        images[0]["tap"], "Layer::Perimeters",
        "the indexed image must identify the requested perimeter tap"
    );

    let png_path = images[0]["png_path"]
        .as_str()
        .expect("the indexed image must carry a string png_path");
    assert!(
        !png_path.is_empty(),
        "the indexed image must carry a real, non-empty png_path"
    );
    let png_file = output.join(png_path);
    assert!(
        png_file.is_file(),
        "the manifest's indexed image path must exist on disk: {}",
        png_file.display()
    );
    let png_bytes = fs::read(&png_file).expect("the indexed image must be readable");
    assert!(
        !png_bytes.is_empty(),
        "the manifest's indexed image file must be non-empty: {}",
        png_file.display()
    );
}
