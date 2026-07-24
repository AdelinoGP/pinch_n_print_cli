//! Support-preview verb contract tests.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use slicer_ir::{
    ConfigValue, ExPolygon, GlobalLayer, Point2, Polygon, SemVer, SupportGeometryIR,
    SupportGeometryKey,
};
use tempfile::TempDir;

#[path = "../src/support_preview.rs"]
mod support_preview;

use support_preview::{build_preview_doc, run_support_preview};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pnp-cli has a parent crate directory")
        .parent()
        .expect("crates has a workspace root")
        .to_path_buf()
}

fn fixture_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("bridge_support_enforcers.3mf")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn config_source(support_enabled: bool) -> HashMap<String, ConfigValue> {
    HashMap::from([(
        "enable_support".to_owned(),
        ConfigValue::Bool(support_enabled),
    )])
}

fn write_config(tmp: &TempDir, support_enabled: bool) -> PathBuf {
    let path = tmp.path().join("config.json");
    let json = serde_json::json!({ "enable_support": support_enabled });
    fs::write(
        &path,
        serde_json::to_vec(&json).expect("config JSON serialization"),
    )
    .expect("write config");
    path
}

fn read_doc(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read preview JSON")).expect("parse preview JSON")
}

fn snapshot_tree(root: &Path) -> BTreeSet<PathBuf> {
    fn visit(path: &Path, root: &Path, paths: &mut BTreeSet<PathBuf>) {
        for entry in fs::read_dir(path).expect("read snapshot directory") {
            let entry = entry.expect("read snapshot entry");
            let path = entry.path();
            paths.insert(
                path.strip_prefix(root)
                    .expect("snapshot path under root")
                    .to_path_buf(),
            );
            if path.is_dir() {
                visit(&path, root, paths);
            }
        }
    }

    let mut paths = BTreeSet::new();
    visit(root, root, &mut paths);
    paths
}

fn prepared_context(support_enabled: bool) -> slicer_runtime::PrepassContext {
    let mesh = slicer_model_io::load_model(&fixture_path()).expect("load support fixture");
    slicer_runtime::prepare_prepass_context(
        Arc::new(mesh),
        config_source(support_enabled),
        &[module_dir()],
        true,
    )
    .expect("prepare support preview context")
}

fn assert_finite_point(point: &Value, context: &str) {
    let pair = point
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be [x, y]"));
    assert_eq!(pair.len(), 2, "{context} must have two coordinates");
    for coordinate in pair {
        assert!(
            coordinate.as_f64().is_some_and(f64::is_finite),
            "{context} coordinate must be finite"
        );
    }
}

fn assert_finite_support(layer: &Value, layer_index: usize) {
    let support = layer["support"]
        .as_array()
        .expect("support must be an array");
    for (polygon_index, polygon) in support.iter().enumerate() {
        let contour = polygon["contour"]
            .as_array()
            .expect("contour must be an array");
        for (point_index, point) in contour.iter().enumerate() {
            assert_finite_point(
                point,
                &format!("layers[{layer_index}].support[{polygon_index}].contour[{point_index}]"),
            );
        }
        let holes = polygon["holes"].as_array().expect("holes must be an array");
        for (hole_index, hole) in holes.iter().enumerate() {
            for (point_index, point) in hole
                .as_array()
                .expect("hole must be an array")
                .iter()
                .enumerate()
            {
                assert_finite_point(
                    point,
                    &format!(
                        "layers[{layer_index}].support[{polygon_index}].holes[{hole_index}][{point_index}]"
                    ),
                );
            }
        }
    }
}

fn unit_square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(1.0, 0.0),
                Point2::from_mm(1.0, 1.0),
                Point2::from_mm(0.0, 1.0),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn preview_json_schema_and_nonempty_support() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("support-preview.json");
    let config = write_config(&tmp, true);

    assert!(run_support_preview(
        &fixture_path(),
        &output,
        Some(&config),
        &[module_dir()],
        true
    )
    .is_ok());
    let doc = read_doc(&output);
    assert_eq!(doc["schema_version"], "1.0.0");
    assert_eq!(doc["units"], "mm");
    assert!(doc["layer_count"].as_u64().is_some_and(|count| count > 0));

    let layers = doc["layers"].as_array().expect("layers must be an array");
    assert!(!layers.is_empty());
    assert!(
        layers.iter().any(|layer| {
            layer["support"]
                .as_array()
                .is_some_and(|support| !support.is_empty())
        }),
        "at least one layer must contain support"
    );

    let mut has_nonempty_contour = false;
    for (layer_index, layer) in layers.iter().enumerate() {
        assert!(
            layer["layer_index"]
                .as_u64()
                .is_some_and(|index| u32::try_from(index).is_ok()),
            "layer_index must be a u32 JSON number"
        );
        assert!(
            layer["z_mm"]
                .as_f64()
                .is_some_and(|z| z.is_finite() && z > 0.0),
            "z_mm must be finite and positive"
        );
        assert_finite_support(layer, layer_index);
        if layer["support"].as_array().is_some_and(|support| {
            support.iter().any(|polygon| {
                polygon["contour"]
                    .as_array()
                    .is_some_and(|contour| !contour.is_empty())
            })
        }) {
            has_nonempty_contour = true;
        }
    }
    assert!(has_nonempty_contour, "a support contour must be non-empty");
}

#[test]
fn coordinates_are_mm_not_internal_units() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("support-preview.json");
    let config = write_config(&tmp, true);
    assert!(run_support_preview(
        &fixture_path(),
        &output,
        Some(&config),
        &[module_dir()],
        true
    )
    .is_ok());
    let doc = read_doc(&output);

    let ctx = prepared_context(true);
    let geometry = ctx
        .blackboard
        .support_geometry()
        .expect("support geometry must be produced");
    let global_layers = ctx.plan.global_layers.as_ref();

    let mut expected: HashMap<u32, Vec<(f64, f64)>> = HashMap::new();
    for (key, polygons) in &geometry.entries {
        if key.global_support_layer_index == u32::MAX {
            continue;
        }
        let points = expected.entry(key.global_support_layer_index).or_default();
        for polygon in polygons {
            for point in &polygon.contour.points {
                let (x, y) = point.to_mm();
                points.push((x as f64, y as f64));
            }
            for hole in &polygon.holes {
                for point in &hole.points {
                    let (x, y) = point.to_mm();
                    points.push((x as f64, y as f64));
                }
            }
        }
    }

    let layers = doc["layers"].as_array().expect("layers must be an array");
    for layer in layers {
        let layer_index = layer["layer_index"].as_u64().expect("layer index") as usize;
        assert!(layer_index < global_layers.len());
        let emitted_z = layer["z_mm"].as_f64().expect("z_mm");
        assert!((emitted_z - global_layers[layer_index].z as f64).abs() < 1e-6);
        for polygon in layer["support"].as_array().expect("support") {
            let mut points = Vec::new();
            points.extend(polygon["contour"].as_array().expect("contour").iter());
            for hole in polygon["holes"].as_array().expect("holes") {
                points.extend(hole.as_array().expect("hole").iter());
            }
            for point in points {
                let pair = point.as_array().expect("point pair");
                let x = pair[0].as_f64().expect("x");
                let y = pair[1].as_f64().expect("y");
                assert!(
                    x.abs() < 1_000.0 && y.abs() < 1_000.0,
                    "coordinates must be in mm, got ({x}, {y})"
                );
                let candidates = expected
                    .get_mut(&(layer_index as u32))
                    .expect("emitted layer must have raw support geometry");
                let match_index = candidates.iter().position(|(expected_x, expected_y)| {
                    (x - expected_x).abs() < 1e-6 && (y - expected_y).abs() < 1e-6
                });
                assert!(
                    match_index.is_some(),
                    "emitted point ({x}, {y}) must match raw units converted to mm; first candidate: {:?}",
                    candidates.first()
                );
                candidates.remove(match_index.expect("matched point"));
            }
        }
    }
    assert!(
        expected.values().all(Vec::is_empty),
        "all raw points must be emitted"
    );
}

#[test]
fn no_gcode_side_effects_exit_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("support-preview.json");
    let config = write_config(&tmp, true);
    let before = snapshot_tree(tmp.path());

    let result = run_support_preview(
        &fixture_path(),
        &output,
        Some(&config),
        &[module_dir()],
        true,
    );
    assert!(
        result.is_ok(),
        "support preview should return Ok: {result:?}"
    );

    let after = snapshot_tree(tmp.path());
    let created: Vec<_> = after.difference(&before).cloned().collect();
    assert_eq!(created, vec![PathBuf::from("support-preview.json")]);
    assert!(output.exists(), "JSON output must exist");
    assert!(
        !after.iter().any(|path| path
            .extension()
            .is_some_and(|extension| extension == "gcode")),
        "support preview must not create G-code"
    );
}

#[test]
fn intermediate_sentinel_entries_skipped_and_counted() {
    let mut entries = HashMap::new();
    for layer_index in 0..3 {
        entries.insert(
            SupportGeometryKey {
                global_support_layer_index: layer_index,
                object_id: format!("real-{layer_index}"),
                region_id: 0,
            },
            vec![unit_square()],
        );
    }
    for sentinel_index in 0..2 {
        entries.insert(
            SupportGeometryKey {
                global_support_layer_index: u32::MAX,
                object_id: format!("sentinel-{sentinel_index}"),
                region_id: 0,
            },
            vec![unit_square()],
        );
    }
    let geometry = SupportGeometryIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        support_layer_height_mm: 0.2,
        support_top_z_distance_mm: 0.1,
        entries,
    };
    let global_layers: Vec<_> = (0..3)
        .map(|index| GlobalLayer {
            index,
            z: 0.2 * (index + 1) as f32,
            ..GlobalLayer::default()
        })
        .collect();

    let doc = build_preview_doc(&geometry, &global_layers);
    assert_eq!(doc.skipped_intermediate_entries, 2);
    assert_eq!(doc.layer_count, 3);
    assert_eq!(doc.layers.len(), 3);
    assert_eq!(
        doc.layers
            .iter()
            .map(|layer| layer.layer_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn support_disabled_yields_empty_layers_exit_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("support-preview.json");
    let config = write_config(&tmp, false);

    assert!(run_support_preview(
        &fixture_path(),
        &output,
        Some(&config),
        &[module_dir()],
        true
    )
    .is_ok());
    let doc = read_doc(&output);
    assert_eq!(doc["schema_version"], "1.0.0");
    assert_eq!(doc["layer_count"], 40);
    assert_eq!(doc["layers"].as_array().expect("layers").len(), 0);
    assert_eq!(doc["skipped_intermediate_entries"], 0);
}

#[test]
fn missing_input_errors_without_output() {
    let tmp = TempDir::new().expect("tempdir");
    let input = workspace_root()
        .join("target")
        .join("does-not-exist-12345.3mf");
    let output = tmp.path().join("support-preview.json");

    let result = run_support_preview(&input, &output, None, &[module_dir()], true);
    assert!(result.is_err(), "missing input must return an error");
    assert!(
        !output.exists(),
        "missing input must not create JSON output"
    );
    assert!(
        !output.with_extension("tmp").exists(),
        "missing input must not leak temporary output"
    );
}
