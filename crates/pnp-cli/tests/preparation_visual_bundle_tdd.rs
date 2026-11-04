//! Packet 06 Step 16: real CLI gate B for prepared-plan XY diagnostics.
//!
//! These tests drive the public `run_visual_debug` API with the actual generated
//! `preparation-visual-guest` component staged beside its manifest. Nothing here
//! injects a Ready plan, a test-local store, or a test-local resolver.
//!
//! AC-5 pins the mixed preparation + `Layer::PathOptimization` bundle, its typed
//! capture envelope, the shared viewport and independent projector/pixel
//! expectations. AC-6 pins whole-scene framing/legend stability across a layer
//! subset, distinct executed/render axes, and the explicit-empty-versus-missing
//! split.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tempfile::TempDir;

use pnp_cli::visual_debug::{
    run_visual_debug, validate_request, ValidationError, VisualDebugError, VisualDebugRequest,
};

const MODULE_ID: &str = "test.preparation-visual";
const VIEW: &str = "probe_xy";
const MODULE_STEM: &str = "preparation-visual-guest";
const MODULE_MANIFEST: &str =
    include_str!("../../slicer-wasm-host/test-guests/preparation-visual-guest/module.toml");

/// Background of the preparation raster (matches `visual_debug_render`'s
/// canvas fill; pinned independently of any capture run).
const BACKGROUND: [u8; 3] = [255, 255, 255];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

/// Read one Markdown section (heading plus body, ending at the next heading of
/// the same or higher level). AC-8 asserts concrete content, not just headings,
/// so dropping a promised contract from a normative section fails the test
/// instead of leaving a green heading grep behind.
fn doc_section(relative: &str, heading: &str) -> String {
    let path = workspace_root().join(relative);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read normative doc {}: {error}", path.display()));
    let level = heading.chars().take_while(|c| *c == '#').count();
    assert!(level > 0, "heading must start with '#': {heading}");
    let mut section = String::new();
    let mut inside = false;
    for line in text.lines() {
        if !inside {
            if line.trim_end() == heading {
                inside = true;
                section.push_str(line);
                section.push('\n');
            }
            continue;
        }
        let next_level = line.chars().take_while(|c| *c == '#').count();
        if next_level > 0 && next_level <= level {
            break;
        }
        section.push_str(line);
        section.push('\n');
    }
    assert!(
        inside,
        "missing normative section {heading:?} in {relative}"
    );
    section
}

/// Stage the actual generated component beside its manifest, as module
/// discovery expects (`{stem}.toml` + `{stem}.wasm`), plus every core module
/// except the core `Layer::PathOptimization` so the fixture wins the real
/// claim arbitration.
struct Fixture {
    staging: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let staging = TempDir::new().expect("stage the generated visual fixture");
        let component = workspace_root()
            .join("crates/slicer-wasm-host/test-guests")
            .join(format!("{MODULE_STEM}.component.wasm"));
        assert!(
            component.is_file(),
            "required generated preparation visual component absent: {} \
             (run `cargo xtask build-guests`; a silent skip or substitute is not permitted)",
            component.display()
        );
        let manifest_path = staging.path().join(format!("{MODULE_STEM}.toml"));
        let component_path = staging.path().join(format!("{MODULE_STEM}.wasm"));
        fs::write(&manifest_path, MODULE_MANIFEST).unwrap_or_else(|error| {
            panic!("stage manifest at {}: {error}", manifest_path.display())
        });
        fs::copy(&component, &component_path).unwrap_or_else(|error| {
            panic!(
                "stage component {} -> {}: {error}",
                component.display(),
                component_path.display()
            )
        });
        Self { staging }
    }

    fn module_dirs(&self) -> Vec<PathBuf> {
        let core = workspace_root().join("modules").join("core-modules");
        assert!(
            core.is_dir(),
            "required core module root absent: {}",
            core.display()
        );
        let mut dirs = vec![self.staging.path().to_path_buf()];
        for entry in fs::read_dir(&core).expect("read core module roots") {
            let path = entry.expect("core module directory").path();
            if !path.is_dir() {
                continue;
            }
            let stem = path.file_name().expect("core module name");
            let manifest_path = path.join(stem).with_extension("toml");
            let manifest: toml::Value = fs::read_to_string(&manifest_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()))
                .parse()
                .expect("parse core module manifest");
            if manifest["stage"]["id"].as_str() == Some("Layer::PathOptimization") {
                continue;
            }
            dirs.push(path);
        }
        dirs.sort();
        dirs
    }

    fn config(&self, dir: &Path, mode: &str) -> PathBuf {
        let path = dir.join(format!("config-{mode}.json"));
        let body = json!({
            "layer_height": 1.0,
            "first_layer_height": 1.0,
            "preparation_visual_mode": mode,
            "preparation_visual_token": "PRB1",
        });
        fs::write(&path, body.to_string()).expect("write config");
        path
    }

    /// Raw JSON for a schema `1.4.0` model-source request with one preparation
    /// tap, one ordinary `Layer::PathOptimization` tap, and both matching
    /// visualization kinds.
    fn request_json(&self, config: &Path, layers: &[i64], _mode: &str) -> Value {
        json!({
            "schema_version": "1.4.0",
            "source": {
                "kind": "model",
                "model": wedge_path(),
                "config": config,
                "module_dirs": self.module_dirs(),
            },
            "layers": layers,
            "taps": [
                {"preparation": {"module_id": MODULE_ID, "view": VIEW}},
                {"id": "Layer::PathOptimization"},
            ],
            "visualizations": [
                {"type": "preparation_geometry"},
                {"type": "filament_lines"},
            ],
            "resolution_scale": 1,
            "frame": "model",
        })
    }

    /// The parsed schema `1.4.0` request used by the successful bundle tests.
    fn request(&self, config: &Path, layers: &[i64], mode: &str) -> VisualDebugRequest {
        serde_json::from_value(self.request_json(config, layers, mode))
            .expect("schema 1.4.0 preparation request must parse")
    }
}

fn run_bundle(
    fixture: &Fixture,
    bundle_dir: &Path,
    config: &Path,
    layers: &[i64],
    mode: &str,
) -> (PathBuf, Value) {
    let manifest_path = run_visual_debug(fixture.request(config, layers, mode), bundle_dir, true)
        .unwrap_or_else(|error| panic!("preparation bundle ({mode}, {layers:?}) failed: {error}"));
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest.json"))
            .expect("manifest is valid JSON");
    let output = manifest_path
        .parent()
        .expect("manifest.json has a bundle directory")
        .to_path_buf();
    (output, manifest)
}

fn entries<'a>(manifest: &'a Value, capture_phase: &str) -> Vec<&'a Value> {
    manifest["images"]
        .as_array()
        .expect("images array")
        .iter()
        .filter(|entry| entry["capture_phase"].as_str() == Some(capture_phase))
        .collect()
}

fn prep_entry<'a>(manifest: &'a Value) -> &'a Value {
    let prep = entries(manifest, "preparation");
    assert_eq!(prep.len(), 1, "exactly one preparation entry: {prep:?}");
    prep[0]
}

fn stage_entry<'a>(manifest: &'a Value, visualization: &str) -> &'a Value {
    let found: Vec<&Value> = manifest["images"]
        .as_array()
        .expect("images array")
        .iter()
        .filter(|entry| {
            entry["capture_phase"].is_null()
                && entry["visualization"].as_str() == Some(visualization)
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one ordinary {visualization} entry: {found:?}"
    );
    found[0]
}

fn bounds_from_manifest(entry: &Value) -> slicer_runtime::ViewportBoundsMm {
    let b = &entry["world_bounds_mm"];
    slicer_runtime::ViewportBoundsMm {
        min_x: b["min_x"].as_f64().expect("min_x") as f32,
        min_y: b["min_y"].as_f64().expect("min_y") as f32,
        max_x: b["max_x"].as_f64().expect("max_x") as f32,
        max_y: b["max_y"].as_f64().expect("max_y") as f32,
    }
}

fn class_rgb(entry: &Value, class: &str) -> [u8; 3] {
    let legend = entry["class_legend"]
        .as_array()
        .expect("class_legend array");
    let row = legend
        .iter()
        .find(|row| row["class"].as_str() == Some(class))
        .unwrap_or_else(|| panic!("class {class} in legend: {legend:?}"));
    let rgb = row["rgb"].as_array().expect("rgb array");
    [
        rgb[0].as_u64().expect("r") as u8,
        rgb[1].as_u64().expect("g") as u8,
        rgb[2].as_u64().expect("b") as u8,
    ]
}

fn decode_rgb(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG info");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG output buffer size")];
    let info = reader.next_frame(&mut buf).expect("PNG frame");
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    (info.width, info.height, buf)
}

fn pixel_at(rgb: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let index = ((y * width + x) * 3) as usize;
    [rgb[index], rgb[index + 1], rgb[index + 2]]
}

/// World-mm sample through the public `Projector`, rounded to the nearest
/// pixel. The projector is the renderer's sole world→pixel transform; this
/// test derives the same pixel independently from the manifest's bounds.
fn sample(
    rgb: &[u8],
    width: u32,
    height: u32,
    bounds: slicer_runtime::ViewportBoundsMm,
    x_mm: f64,
    y_mm: f64,
) -> [u8; 3] {
    let projector = slicer_runtime::Projector::new(bounds, width, height);
    let (px, py) = projector.project(x_mm, y_mm);
    pixel_at(rgb, width, px.round() as u32, py.round() as u32)
}

fn assert_no_private_fields(value: &Value) {
    const FORBIDDEN: &[&str] = &[
        "plan",
        "store",
        "piece",
        "codec",
        "handle",
        "payload",
        "probe",
        "owner",
        "accounting",
    ];
    fn walk(value: &Value) {
        match value {
            Value::Object(map) => {
                for key in map.keys() {
                    assert!(
                        !FORBIDDEN.contains(&key.as_str()),
                        "typed capture leaks private field {key:?}: {value}"
                    );
                    walk(&map[key]);
                }
            }
            Value::Array(items) => items.iter().for_each(walk),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    walk(value);
}

#[test]
fn mixed_plan_and_stage_bundle() {
    let fixture = Fixture::new();
    let config_dir = TempDir::new().expect("config dir");
    let config = fixture.config(config_dir.path(), "valid");
    let bundle_dir = TempDir::new().expect("bundle dir");
    let (output, manifest) = run_bundle(&fixture, bundle_dir.path(), &config, &[0], "valid");

    assert_eq!(manifest["schema_version"], "1.4.0");
    assert_eq!(
        manifest["images"].as_array().expect("images").len(),
        2,
        "one preparation image plus one ordinary stage image"
    );
    let prep = prep_entry(&manifest);
    let stage = stage_entry(&manifest, "filament_lines");
    assert_eq!(
        prep["world_bounds_mm"], stage["world_bounds_mm"],
        "both mixed entries share the single bundle-wide viewport"
    );

    // Actual execution evidence: exactly one owner ran, and the rendered entry
    // names that same committed artifact/version rather than a second guess.
    let executed = manifest["executed_preparation"]
        .as_array()
        .expect("executed_preparation array");
    assert_eq!(
        executed.len(),
        1,
        "one actually executed owner: {executed:?}"
    );
    assert_eq!(executed[0]["module_id"], MODULE_ID);
    assert_eq!(executed[0]["module_version"], "0.1.0");
    assert_eq!(
        prep["artifact_identity"], executed[0]["artifact_identity"],
        "the rendered entry and the execution record name the same artifact"
    );

    // Preparation entry identity and truthfulness.
    assert_eq!(prep["capture_phase"], "preparation");
    assert_eq!(prep["module_id"], MODULE_ID);
    assert_eq!(prep["module_version"], "0.1.0");
    assert!(
        prep["artifact_identity"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "artifact identity is recorded: {prep}"
    );
    assert_eq!(prep["consumer_stage"], "Layer::PathOptimization");
    assert_eq!(prep["view"], VIEW);
    assert_eq!(prep["projection_schema"], "1.0.0");
    assert_eq!(prep["layer_index"], 0);
    assert_eq!(
        prep["layer_z"], 1.0,
        "real layer-0 schedule Z for first_layer_height/layer_height 1.0"
    );
    assert_eq!(prep["empty"], false);

    // Exact typed-capture envelope: kind + value only, value with four public
    // scene fields only.
    let typed = &prep["typed_capture"];
    let typed_keys: BTreeSet<&str> = typed
        .as_object()
        .expect("typed_capture object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(typed_keys, BTreeSet::from(["kind", "value"]), "{typed}");
    assert_eq!(typed["kind"], "PreparationProjection");
    let scene = &typed["value"];
    let scene_keys: BTreeSet<&str> = scene
        .as_object()
        .expect("scene object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        scene_keys,
        BTreeSet::from(["scene_id", "view", "classes", "primitives"]),
        "{scene}"
    );
    assert_eq!(scene["scene_id"], "final");
    assert_eq!(scene["view"], VIEW);
    let classes: BTreeSet<&str> = scene["classes"]
        .as_array()
        .expect("classes")
        .iter()
        .map(|c| c.as_str().expect("class string"))
        .collect();
    assert_eq!(
        classes,
        BTreeSet::from(["anchors", "domains", "paths", "remote"]),
        "whole-scene classes include the second scheduled layer's remote class"
    );

    // Independent literal geometry for the rendered layer.
    let primitives = scene["primitives"].as_array().expect("primitives");
    let by_id: std::collections::BTreeMap<&str, &Value> = primitives
        .iter()
        .map(|p| (p["id"].as_str().expect("primitive id"), p))
        .collect();
    assert_eq!(
        by_id.keys().copied().collect::<Vec<_>>(),
        vec!["anchor_a", "domain_a", "outline_a", "path_a"],
        "four layer-0 primitives, exact ids"
    );
    assert_eq!(by_id["anchor_a"]["class"], "anchors");
    assert_eq!(by_id["anchor_a"]["label"], "anchor");
    assert_eq!(by_id["anchor_a"]["global_layer_index"], 0);
    assert_eq!(
        by_id["anchor_a"]["geometry"],
        json!({"Point": {"x": 10000, "y": 20000}})
    );
    assert_eq!(by_id["path_a"]["class"], "paths");
    assert_eq!(by_id["path_a"]["label"], "route");
    assert_eq!(
        by_id["path_a"]["geometry"],
        json!({"Polyline": [{"x": 0, "y": 0}, {"x": 20000, "y": 0}]})
    );
    assert_eq!(by_id["domain_a"]["class"], "domains");
    assert_eq!(by_id["domain_a"]["label"], "area");
    let filled = &by_id["domain_a"]["geometry"];
    assert_eq!(filled["Polygon"]["filled"], true);
    let filled_contour = filled["Polygon"]["polygon"]["contour"]["points"]
        .as_array()
        .expect("contour points");
    assert_eq!(filled_contour.len(), 4);
    assert_eq!(
        filled_contour[0],
        json!({"x": 0, "y": 0}),
        "ExPolygon point encoding is the canonical integer Point2 shape"
    );
    assert_eq!(filled_contour[2], json!({"x": 40000, "y": 40000}));
    let holes = filled["Polygon"]["polygon"]["holes"]
        .as_array()
        .expect("holes");
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0]["points"].as_array().expect("hole ring").len(), 4);
    assert_eq!(
        holes[0]["points"][0],
        json!({"x": 10000, "y": 10000}),
        "hole starts at the clockwise (10,10) mm vertex"
    );
    assert_eq!(by_id["outline_a"]["class"], "domains");
    assert_eq!(by_id["outline_a"]["label"], "outline");
    assert_eq!(by_id["outline_a"]["geometry"]["Polygon"]["filled"], false);

    // No diagnostic primitive is an extrusion claim.
    for primitive in primitives {
        let object = primitive.as_object().expect("primitive object");
        for forbidden in ["InfillIR", "width", "tool", "volume"] {
            assert!(
                !object.contains_key(forbidden),
                "diagnostic primitive must not carry {forbidden:?}: {primitive}"
            );
        }
    }
    assert_no_private_fields(typed);

    // Class legend: sorted by class, exact glyph contract.
    let legend = prep["class_legend"].as_array().expect("class_legend");
    let legend_classes: Vec<&str> = legend
        .iter()
        .map(|row| row["class"].as_str().expect("legend class"))
        .collect();
    let mut sorted = legend_classes.clone();
    sorted.sort_unstable();
    assert_eq!(legend_classes, sorted, "class legend is class-sorted");
    for row in legend {
        assert_eq!(row["point_glyph"], "circle");
        assert!(row["stroke_px"].as_u64().is_some());
        assert_eq!(row["rgb"].as_array().expect("rgb").len(), 3);
    }

    // Independent pixel oracles from the manifest's class colors and bounds.
    let prep_png = output.join(prep["png_path"].as_str().expect("png_path"));
    let stage_png = output.join(stage["png_path"].as_str().expect("png_path"));
    assert!(
        prep_png.is_file(),
        "preparation PNG exists: {}",
        prep_png.display()
    );
    assert!(
        stage_png.is_file(),
        "stage PNG exists: {}",
        stage_png.display()
    );
    let prep_bytes = fs::read(&prep_png).expect("read preparation PNG");
    let (width, height, rgb) = decode_rgb(&prep_bytes);
    assert_eq!((width, height), (1024, 1024));
    let bounds = bounds_from_manifest(prep);
    let point_rgb = class_rgb(prep, "anchors");
    let line_rgb = class_rgb(prep, "paths");
    let domain_rgb = class_rgb(prep, "domains");

    // The AC-1 literal units are 100 nm each: (10000,20000) units is (1,2) mm.
    assert_eq!(
        sample(&rgb, width, height, bounds, 1.0, 2.0),
        point_rgb,
        "point (1,2) mm is the anchors glyph color"
    );
    assert_eq!(
        sample(&rgb, width, height, bounds, 1.0, 0.0),
        line_rgb,
        "open line midpoint (1,0) mm is the paths stroke"
    );
    assert_eq!(
        sample(&rgb, width, height, bounds, 0.5, 0.5),
        domain_rgb,
        "filled contour interior (0.5,0.5) mm is the domains fill"
    );
    assert_eq!(
        sample(&rgb, width, height, bounds, 1.2, 2.8),
        BACKGROUND,
        "closed hole corner (1.2,2.8) mm is background (clear of the area label)"
    );
    assert_eq!(
        sample(&rgb, width, height, bounds, 6.0, 1.0),
        domain_rgb,
        "outline left edge (6,1) mm is the domains stroke"
    );
    assert_eq!(
        sample(&rgb, width, height, bounds, 7.0, 1.0),
        BACKGROUND,
        "outline interior (7,1) mm is unfilled"
    );

    // The ordinary entry is a real nonempty PNG.
    let stage_bytes = fs::read(&stage_png).expect("read stage PNG");
    let (stage_width, stage_height, _) = decode_rgb(&stage_bytes);
    assert_eq!((stage_width, stage_height), (1024, 1024));
}

#[test]
fn subset_snapshot_and_explicit_empty() {
    let fixture = Fixture::new();
    let config_dir = TempDir::new().expect("config dir");
    let config = fixture.config(config_dir.path(), "valid");

    // All-layer (the fixture publishes on layers 0 and 1) versus subset.
    let all_bundle = TempDir::new().expect("all-layer bundle dir");
    let subset_bundle = TempDir::new().expect("subset bundle dir");
    let (all_output, all) = run_bundle(&fixture, all_bundle.path(), &config, &[0, 1], "valid");
    let (subset_output, subset) =
        run_bundle(&fixture, subset_bundle.path(), &config, &[0], "valid");
    assert_eq!(
        all["viewport"], subset["viewport"],
        "full-plan framing does not depend on the rendered subset"
    );
    let all_prep = entries(&all, "preparation");
    let subset_prep = entries(&subset, "preparation");
    assert_eq!(all_prep.len(), 2, "one image per selected layer");
    assert_eq!(subset_prep.len(), 1);
    let all_layer0 = all_prep
        .iter()
        .find(|entry| entry["layer_index"] == 0)
        .expect("all-layer entry for layer 0");
    assert_eq!(
        all_layer0["world_bounds_mm"], subset_prep[0]["world_bounds_mm"],
        "subset framing equals all-layer framing"
    );
    assert_eq!(
        all_layer0["class_legend"], subset_prep[0]["class_legend"],
        "class legend is derived from the whole scene, not the rendered subset"
    );
    assert_eq!(
        all_layer0["typed_capture"], subset_prep[0]["typed_capture"],
        "the same whole-print scene snapshot is mirrored for layer 0"
    );
    assert_eq!(
        all["rendered_layer_indices"],
        json!([0, 1]),
        "top-level rendered layers list every requested layer"
    );
    assert_eq!(
        subset["rendered_layer_indices"],
        json!([0]),
        "subset bundle reports only its rendered layer"
    );
    assert_eq!(
        all["executed_preparation"]
            .as_array()
            .expect("executed")
            .len(),
        1,
        "one actually executed owner, independent of rendered layers"
    );
    assert_eq!(
        subset["executed_preparation"]
            .as_array()
            .expect("executed")
            .len(),
        1,
        "execution evidence is not filtered to the rendered subset"
    );

    // The all-layer bundle renders a real layer-1 PNG too.
    for entry in &all_prep {
        let path = all_output.join(entry["png_path"].as_str().expect("png path"));
        assert!(path.is_file(), "layer PNG exists: {}", path.display());
    }
    let subset_path = subset_output.join(subset_prep[0]["png_path"].as_str().expect("png path"));
    assert!(subset_path.is_file());

    // Explicit completed empty scene: successful bundle, empty true, zero
    // mirrored primitives, a real PNG.
    let empty_config = fixture.config(config_dir.path(), "empty");
    let empty_bundle = TempDir::new().expect("empty bundle dir");
    let (empty_output, empty) =
        run_bundle(&fixture, empty_bundle.path(), &empty_config, &[0], "empty");
    let empty_prep = prep_entry(&empty);
    assert_eq!(empty_prep["empty"], true);
    assert_eq!(
        empty_prep["typed_capture"]["value"]["primitives"],
        json!([]),
        "explicit empty completion mirrors zero primitives"
    );
    let empty_png = empty_output.join(empty_prep["png_path"].as_str().expect("png path"));
    assert!(
        empty_png.is_file(),
        "explicit empty completion still writes a PNG"
    );

    // Missing completion is a distinct failure, never `empty: true`.
    let missing_config = fixture.config(config_dir.path(), "missing");
    let missing = run_visual_debug(
        fixture.request(&missing_config, &[0], "missing"),
        TempDir::new().expect("missing bundle dir").path(),
        true,
    );
    match missing {
        Ok(_) => panic!("missing scene completion must not produce a successful bundle"),
        Err(VisualDebugError::PreparationProjectionRejected {
            module_id,
            view,
            reason,
        }) => {
            assert_eq!(module_id, MODULE_ID);
            assert_eq!(view, VIEW);
            assert_eq!(
                reason,
                slicer_schema::preparation::PreparationProjectionError::MissingScene,
                "missing completion carries the typed reason"
            );
        }
        Err(other) => panic!("missing completion returned {other:?}"),
    }
}

/// A stage-only model request under a legacy schema. It names a real per-layer
/// tap and a single geometry visualization, and contains no preparation
/// selector or preparation visualization.
fn legacy_request(fixture: &Fixture, config: &Path, schema: &str) -> VisualDebugRequest {
    let body = json!({
        "schema_version": schema,
        "source": {
            "kind": "model",
            "model": wedge_path(),
            "config": config,
            "module_dirs": fixture.module_dirs(),
        },
        "layers": [0],
        "taps": [{"id": "Layer::Perimeters"}],
        "visualizations": [{"type": "filament_lines"}],
        "resolution_scale": 1,
    });
    serde_json::from_value(body).expect("legacy stage-only request parses")
}

fn error_for(request: VisualDebugRequest) -> VisualDebugError {
    let output = TempDir::new().expect("failing bundle dir");
    run_visual_debug(request, output.path(), true)
        .expect_err("the request must fail closed, never write a successful bundle")
}

#[test]
fn legacy_schema_compatibility() {
    let fixture = Fixture::new();
    let config_dir = TempDir::new().expect("config dir");
    let config = fixture.config(config_dir.path(), "valid");

    for schema in ["1.0.0", "1.1.0", "1.2.0", "1.3.0"] {
        let bundle = TempDir::new().expect("legacy bundle dir");
        let manifest_path = run_visual_debug(
            legacy_request(&fixture, &config, schema),
            bundle.path(),
            true,
        )
        .unwrap_or_else(|error| panic!("legacy {schema} stage-only bundle failed: {error}"));
        let first_manifest_bytes = fs::read(&manifest_path).expect("legacy manifest bytes");
        let manifest: Value =
            serde_json::from_slice(&first_manifest_bytes).expect("legacy manifest JSON");

        assert_eq!(manifest["schema_version"], schema);
        assert!(
            manifest.get("executed_preparation").is_none(),
            "pre-1.4 manifests carry no executed_preparation: {manifest}"
        );
        assert!(
            manifest.get("rendered_layer_indices").is_none(),
            "pre-1.4 manifests carry no rendered_layer_indices: {manifest}"
        );
        let images = manifest["images"].as_array().expect("legacy images");
        assert!(!images.is_empty(), "legacy {schema} still renders an image");
        let mut legacy_outputs = Vec::new();
        for image in images {
            assert!(
                image.get("capture_phase").is_none(),
                "legacy image has no preparation capture_phase: {image}"
            );
            assert!(
                image.get("projection_schema").is_none(),
                "legacy image has no projection_schema: {image}"
            );
            let relative = image["png_path"].as_str().expect("legacy png_path");
            let png = bundle.path().join(relative);
            assert!(png.is_file(), "legacy PNG exists: {}", png.display());
            legacy_outputs.push((
                relative.to_owned(),
                fs::read(&png).expect("legacy PNG bytes"),
            ));
        }
        let expected_legend = if schema == "1.0.0" {
            "1.0.0"
        } else {
            slicer_runtime::LEGEND_VERSION
        };
        assert_eq!(
            manifest["legend_version"], expected_legend,
            "legacy legend string is not repinned at 1.4.0"
        );

        // The untouched legacy targets own the golden pins; this repeat run
        // proves the updated path still produces the same bytes deterministically
        // (a re-pin to 1.4.0 or an accidental preparation field would differ).
        let repeat = TempDir::new().expect("repeat legacy bundle dir");
        let repeat_manifest_path = run_visual_debug(
            legacy_request(&fixture, &config, schema),
            repeat.path(),
            true,
        )
        .unwrap_or_else(|error| panic!("legacy {schema} repeat bundle failed: {error}"));
        assert_eq!(
            fs::read(&repeat_manifest_path).expect("repeat legacy manifest"),
            first_manifest_bytes,
            "legacy {schema} manifest must be byte-identical on a repeat run"
        );
        for (relative, bytes) in legacy_outputs {
            let repeat_png = repeat.path().join(&relative);
            assert_eq!(
                fs::read(&repeat_png).expect("repeat legacy PNG"),
                bytes,
                "legacy {schema} PNG {relative} must be byte-identical on a repeat run"
            );
        }
    }

    // A 1.3.0 preparation request is rejected by the named version gate, not
    // silently accepted with preparation fields.
    let mut json = fixture.request_json(&config, &[0], "valid");
    json["schema_version"] = json!("1.3.0");
    let request: VisualDebugRequest = serde_json::from_value(json).expect("1.3 request parses");
    assert!(
        matches!(
            validate_request(request).expect_err("1.3 preparation must fail validation"),
            ValidationError::PreparationRequiresSchema14
        ),
        "schema gate is the named requires-1.4.0 error"
    );
}

#[test]
fn fail_closed_requests_and_bundle() {
    let fixture = Fixture::new();
    let config_dir = TempDir::new().expect("config dir");
    let config = fixture.config(config_dir.path(), "valid");

    // Validation-level rejections, before any module invocation.
    let mut missing_prep_viz = fixture.request_json(&config, &[0], "valid");
    missing_prep_viz["visualizations"] = json!([{"type": "filament_lines"}]);
    let request: VisualDebugRequest =
        serde_json::from_value(missing_prep_viz).expect("request parses");
    assert!(matches!(
        validate_request(request).expect_err("preparation tap without preparation_geometry"),
        ValidationError::InvalidPreparationSelector { .. }
    ));

    let mut gcode = fixture.request_json(&config, &[0], "valid");
    gcode["source"] = json!({"kind": "gcode", "path": "/definitely/not/a/real.gcode"});
    let request: VisualDebugRequest = serde_json::from_value(gcode).expect("gcode request parses");
    assert!(matches!(
        validate_request(request).expect_err("preparation tap on gcode source"),
        ValidationError::PreparationUnsupportedSource
    ));

    let mut front = fixture.request_json(&config, &[0], "valid");
    front["visualizations"][0] =
        json!({"type": "preparation_geometry", "options": {"view": "front"}});
    let request: VisualDebugRequest = serde_json::from_value(front).expect("front request parses");
    assert!(matches!(
        validate_request(request).expect_err("front dimension is not a plan view"),
        ValidationError::PreparationUnsupportedDimension { ref view } if view == "front"
    ));

    // A preparation selector under a pre-1.4 schema is the named version gate.
    let mut legacy_prep = fixture.request_json(&config, &[0], "valid");
    legacy_prep["schema_version"] = json!("1.3.0");
    let request: VisualDebugRequest =
        serde_json::from_value(legacy_prep).expect("1.3 preparation request parses");
    assert!(
        matches!(
            validate_request(request).expect_err("1.3 preparation must fail validation"),
            ValidationError::PreparationRequiresSchema14
        ),
        "a preparation selector below 1.4.0 is the named schema-14 error"
    );

    // Malformed structured selector cannot fall through to the legacy Detail
    // form: the preparation object must be exactly {module_id, view}.
    let malformed = json!({"preparation": {"module_id": MODULE_ID, "view": VIEW, "extra": 1}});
    assert!(
        serde_json::from_value::<pnp_cli::visual_debug::TapSelector>(malformed).is_err(),
        "extra preparation keys are rejected, never silently dropped"
    );

    // Runtime-level named rejections.
    let mut unknown_owner = fixture.request_json(&config, &[0], "valid");
    unknown_owner["taps"][0]["preparation"]["module_id"] = json!("test.unknown");
    let err =
        error_for(serde_json::from_value(unknown_owner.clone()).expect("unknown owner request"));
    assert!(
        matches!(err, VisualDebugError::UnknownPreparationOwner { ref module_id } if module_id == "test.unknown"),
        "unknown owner: {err:?}"
    );

    let mut unknown_view = fixture.request_json(&config, &[0], "valid");
    unknown_view["taps"][0]["preparation"]["view"] = json!("undeclared_xy");
    let err = error_for(serde_json::from_value(unknown_view).expect("unknown view request"));
    assert!(
        matches!(err, VisualDebugError::UnknownPreparationView { ref module_id, ref view } if module_id == MODULE_ID && view == "undeclared_xy"),
        "unknown view: {err:?}"
    );

    let mut unknown_layer = fixture.request_json(&config, &[99], "valid");
    unknown_layer["layers"] = json!([99]);
    let err = error_for(serde_json::from_value(unknown_layer).expect("unknown layer request"));
    assert!(
        matches!(err, VisualDebugError::NoApplicableLayer),
        "unknown layer: {err:?}"
    );

    // `PreparationUnsupportedOwner` / `InactivePreparationOwner` are not
    // reachable through this single always-invoked preparation-capable
    // fixture: the frozen preparation set contains only declared owners, so a
    // non-preparation module id is `UnknownPreparationOwner`, and the fixture
    // participates on every scheduled layer. Their exact CLI mappings are
    // pinned by the production-unit test
    // `preparation_visual_error_mapping_is_exact_for_every_variant`; the
    // runtime validation paths are
    // `freeze_selected_preparation_bindings` (`UnsupportedOwner`) and
    // `resolve_preparation_visual_request` (`InactiveOwner`). The corresponding
    // no-invocation property for unknown owner/view/layer is asserted by
    // `preparation_visual_capture_tdd::pre_execution_rejections_never_invoke_preparation`.

    // A failed request must not replace an existing sentinel bundle, even with
    // overwrite=true: validation/capture finishes before the writer runs.
    let sentinel = TempDir::new().expect("sentinel bundle dir");
    let sentinel_manifest = sentinel.path().join("manifest.json");
    fs::write(&sentinel_manifest, b"sentinel-bytes").expect("write sentinel");
    let err = run_visual_debug(
        serde_json::from_value(unknown_owner).expect("unknown owner request"),
        sentinel.path(),
        true,
    )
    .expect_err("unknown owner fails closed");
    assert!(matches!(
        err,
        VisualDebugError::UnknownPreparationOwner { .. }
    ));
    assert_eq!(
        fs::read(&sentinel_manifest).expect("sentinel survives"),
        b"sentinel-bytes",
        "a rejected request must leave an existing bundle byte-identical"
    );

    // Typed projection faults: each named first rejection survives to the CLI.
    let cases = [
        (
            "missing",
            slicer_schema::preparation::PreparationProjectionError::MissingScene,
        ),
        (
            "duplicate",
            slicer_schema::preparation::PreparationProjectionError::DuplicateScene,
        ),
        (
            "undeclared_view",
            slicer_schema::preparation::PreparationProjectionError::UndeclaredView,
        ),
        (
            "undeclared_class",
            slicer_schema::preparation::PreparationProjectionError::InvalidReference,
        ),
        (
            "unknown_layer",
            slicer_schema::preparation::PreparationProjectionError::InvalidReference,
        ),
        (
            "invalid_line",
            slicer_schema::preparation::PreparationProjectionError::InvalidShape,
        ),
        (
            "invalid_ring",
            slicer_schema::preparation::PreparationProjectionError::InvalidShape,
        ),
        (
            "invalid_hole",
            slicer_schema::preparation::PreparationProjectionError::InvalidShape,
        ),
        (
            "invalid_coordinate",
            slicer_schema::preparation::PreparationProjectionError::InvalidCoordinate,
        ),
        (
            // The first rejection is the malformed line; the later valid write
            // is still refused with PoisonedOutput, and the first reason is the
            // one the host sidecar retains.
            "caught_error",
            slicer_schema::preparation::PreparationProjectionError::InvalidShape,
        ),
    ];
    for (mode, expected) in cases {
        let config = fixture.config(config_dir.path(), mode);
        let err = error_for(fixture.request(&config, &[0], mode));
        match err {
            VisualDebugError::PreparationProjectionRejected { reason, .. } => {
                assert_eq!(reason, expected, "mode {mode} typed reason");
            }
            other => panic!("mode {mode} returned {other:?}, expected {expected:?}"),
        }
    }

    // Original module/trap failure keeps precedence over the projection poison
    // the fault modes inject before failing.
    for mode in ["module_error", "trap"] {
        let config = fixture.config(config_dir.path(), mode);
        let err = error_for(fixture.request(&config, &[0], mode));
        assert!(
            !matches!(err, VisualDebugError::PreparationProjectionRejected { .. }),
            "mode {mode} must report the producer failure, not a projection rejection: {err:?}"
        );
        assert!(
            matches!(err, VisualDebugError::CaptureFailed(_)),
            "mode {mode} is a capture/producer failure: {err:?}"
        );
    }
}

#[test]
fn documented_request_shape() {
    let fixture = Fixture::new();
    let config_dir = TempDir::new().expect("config dir");
    let config = fixture.config(config_dir.path(), "valid");
    let example = fixture.request_json(&config, &[0], "valid");

    // The normative example parses into the real public request type and
    // passes the same validation the CLI runs.
    let parsed: VisualDebugRequest =
        serde_json::from_value(example.clone()).expect("documented request parses");
    validate_request(parsed).expect("documented request passes production validation");

    // The documented shape is executable, not merely parseable.
    let bundle = TempDir::new().expect("documented bundle dir");
    let manifest_path = run_visual_debug(
        serde_json::from_value(example).expect("documented request parses"),
        bundle.path(),
        true,
    )
    .expect("documented request produces a real bundle");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("documented manifest"))
            .expect("documented manifest JSON");
    assert_eq!(manifest["schema_version"], "1.4.0");
    assert_eq!(
        entries(&manifest, "preparation").len(),
        1,
        "the documented mixed request produces one preparation image"
    );

    // AC-8 is a documentation contract: headings alone are not evidence. Each
    // normative section must still state the shape this packet implemented.
    let visual = doc_section(
        "docs/19_visual_debug.md",
        "## Prepared plan XY views (schema 1.4.0)",
    );
    assert!(
        visual.contains(r#"{"preparation": {"module_id": ..., "view": ...}}"#),
        "docs/19 must state the exact preparation selector shape"
    );
    assert!(
        visual.contains(r#"{"view": "xy"}"#),
        "docs/19 must state the only accepted preparation_geometry option"
    );
    assert!(
        visual.contains("`executed_preparation`") && visual.contains("`rendered_layer_indices`"),
        "docs/19 must keep the execution-versus-render evidence axes"
    );
    assert!(
        visual.contains("PostPass::LayerFinalization"),
        "docs/19 must disclose the preparation+postpass fail-closed limitation"
    );

    let wit = doc_section(
        "docs/03_wit_and_manifest.md",
        "### Preparation diagnostic projection transport (Normative)",
    );
    assert!(
        wit.contains("package slicer:layer-preparation@1.0.0;"),
        "docs/03 must keep the imported-interface-only @1.0.0 package policy"
    );
    assert!(
        wit.contains("resource preparation-projection-output")
            && wit.contains(
                "projection: func() -> result<option<preparation-projection-output>, preparation-projection-error>"
            ),
        "docs/03 must keep the typed sink resource and optional projection method"
    );

    let sdk = doc_section(
        "docs/05_module_sdk.md",
        "### Optional preparation projection sink (Normative)",
    );
    assert!(
        sdk.contains("PreparedPlanOutput::projection()")
            && sdk.contains("PreparationProjectionWrite: Send + Sync")
            && sdk.contains("from_host_writer"),
        "docs/05 must keep the SDK acquisition point and native writer facade"
    );

    let evidence = doc_section(
        "docs/17_agent_debugging.md",
        "### Preparation visual execution evidence (Normative)",
    );
    assert!(
        evidence.contains("`executed_preparation`")
            && evidence.contains("`rendered_layer_indices`")
            && evidence.contains("once per owner/view"),
        "docs/17 must keep the execution axes and counted-once accounting exclusion"
    );
}
