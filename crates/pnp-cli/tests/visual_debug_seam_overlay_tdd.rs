#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    require_seam_plan, run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugRequest,
    VisualDebugSource, VisualizationSpec,
};
use serde_json::{json, Value};
use tempfile::TempDir;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn config(dir: &Path) -> PathBuf {
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height":1.0}"#).unwrap();
    path
}

fn support_config(dir: &Path) -> PathBuf {
    let path = dir.join("support-config.json");
    fs::write(&path, br#"{"layer_height":1.0,"enable_support":true,"support_filament":2,"support_interface_filament":3}"#).unwrap();
    path
}

fn request(version: &str, options: Value, dir: &Path) -> VisualDebugRequest {
    // exhaustive: model-source seam overlay fixture
    VisualDebugRequest {
        schema_version: version.into(),
        source: VisualDebugSource::Model {
            model: Some(root().join("resources").join("regression_wedge.stl")),
            config: Some(if options["color_by"] == "tool" {
                support_config(dir)
            } else {
                config(dir)
            }),
            module_dirs: vec![root().join("modules").join("core-modules")],
            path: None,
        },
        layers: vec![LayerSelector::Range { start: 0, end: 3 }],
        taps: vec![TapSelector::Name(
            if version == "1.1.0" {
                "Layer::Perimeters"
            } else if options["color_by"] == "tool" {
                "PostPass::LayerFinalization"
            } else {
                "Layer::Slice"
            }
            .into(),
        )],
        visualizations: vec![VisualizationSpec::Detail {
            kind: if version == "1.1.0" {
                "diagnostic_overlay"
            } else {
                "silhouette"
            }
            .into(),
            options,
        }],
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn manifest(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path.join("manifest.json")).unwrap()).unwrap()
}

const BACKGROUND: [u8; 3] = [255, 255, 255];
const FAINT_BASE: [u8; 3] = [210, 210, 210];
const SEAM: [u8; 3] = [220, 0, 0];

fn decode_rgb(path: &Path) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(fs::read(path).unwrap()));
    let mut reader = decoder.read_info().expect("PNG must be decodable");
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .expect("PNG frame must be decodable");
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

fn assert_pixels(path: &Path, required: &[[u8; 3]], allowed: &[[u8; 3]]) {
    let (width, _height, rgb) = decode_rgb(path);
    let pixels = rgb.chunks_exact(3).map(|p| [p[0], p[1], p[2]]);
    let pixels: Vec<[u8; 3]> = pixels.collect();
    for color in required {
        assert!(
            pixels.contains(color),
            "{path:?} must contain pixel {color:?}"
        );
    }
    assert!(width > 0);
    assert!(
        pixels
            .iter()
            .all(|pixel| allowed.contains(pixel) || *pixel == BACKGROUND),
        "{path:?} contains a non-background color outside the contract"
    );
}

fn seam_events(entry: &Value) -> &Vec<Value> {
    entry["overlay_events"]
        .as_array()
        .expect("overlay_events array")
}

fn run(options: Value, dir: &TempDir) -> (Value, PathBuf) {
    let out = dir.path().join("bundle");
    let path = run_visual_debug(request("1.2.0", options, dir.path()), &out, false).unwrap();
    (manifest(&out), path)
}

#[test]
fn isolated_seam_overlay_faint_base_and_events_carry_z() {
    let dir = TempDir::new().unwrap();
    let (m, _) = run(json!({"overlays":["seams"]}), &dir);
    let e = &m["images"][0];
    assert_eq!(e["visualization"], "silhouette");
    assert_eq!(e["view"], "front");
    assert!(!e["layers_rendered"].as_array().unwrap().is_empty());
    assert_eq!(e["overlay"], "seams");
    assert_eq!(
        e["png_path"],
        "images/Layer__Slice_silhouette_front_overlay_seams.png"
    );
    assert!(e["overlay_events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["z"].is_number()));
    assert!(!e.as_object().unwrap().contains_key("layer_index"));
    assert!(!e.as_object().unwrap().contains_key("layer_z"));
    assert_pixels(
        &dir.path()
            .join("bundle")
            .join(e["png_path"].as_str().unwrap()),
        &[FAINT_BASE, SEAM],
        &[FAINT_BASE, SEAM],
    );
}

#[test]
fn composited_seams_draw_on_colored_base_no_extra_file() {
    let dir = TempDir::new().unwrap();
    let (m, _) = run(json!({"composited_overlays":["seams"]}), &dir);
    let e = &m["images"][0];
    assert_eq!(e["png_path"], "images/Layer__Slice_silhouette_front.png");
    assert_eq!(e["composited_overlays"], json!(["seams"]));
    assert!(e["overlay_events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["z"].is_number()));
    let (_, _, rgb) = decode_rgb(
        &dir.path()
            .join("bundle")
            .join(e["png_path"].as_str().unwrap()),
    );
    assert!(rgb.chunks_exact(3).any(|p| [p[0], p[1], p[2]] == SEAM));
    assert!(m["images"].as_array().unwrap().iter().all(|image| {
        !image["png_path"]
            .as_str()
            .unwrap()
            .contains("overlay_seams")
    }));
}

#[test]
fn both_forms_coexist_one_isolated_one_composited() {
    let dir = TempDir::new().unwrap();
    let (m, _) = run(
        json!({"overlays":["seams"],"composited_overlays":["seams"]}),
        &dir,
    );
    let images = m["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert_ne!(images[0]["png_path"], images[1]["png_path"]);
    let isolated = images.iter().find(|e| e["overlay"] == "seams").unwrap();
    let composited = images
        .iter()
        .find(|e| e["composited_overlays"] == json!(["seams"]))
        .unwrap();
    assert!(!seam_events(isolated).is_empty());
    assert!(!seam_events(composited).is_empty());
    assert!(seam_events(isolated)
        .iter()
        .all(|v| v["event"] == "seam" && v["z"].is_number()));
    assert!(seam_events(composited)
        .iter()
        .all(|v| v["event"] == "seam" && v["z"].is_number()));
}

#[test]
fn legacy_seam_events_serialize_without_z_key() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("bundle");
    let path = run_visual_debug(
        request(
            "1.1.0",
            json!({"base":"filled_areas","overlays":["seams"]}),
            dir.path(),
        ),
        &out,
        false,
    )
    .unwrap();
    let events = manifest(&out)["images"][0]["overlay_events"]
        .as_array()
        .unwrap()
        .clone();
    assert!(!events.is_empty());
    for event in events {
        let object = event.as_object().unwrap();
        assert!(
            object.contains_key("event") && object.contains_key("x") && object.contains_key("y")
        );
        assert!(!object.contains_key("z"));
    }
    assert!(path.exists());
}

#[test]
fn composited_seams_on_tool_colored_base() {
    let dir = TempDir::new().unwrap();
    let (m, _) = run(
        json!({"color_by":"tool","composited_overlays":["seams"]}),
        &dir,
    );
    let e = &m["images"][0];
    assert_eq!(e["composited_overlays"], json!(["seams"]));
    assert_eq!(
        e["png_path"],
        "images/PostPass__LayerFinalization_silhouette_front_tool.png"
    );
    let (_, _, rgb) = decode_rgb(
        &dir.path()
            .join("bundle")
            .join(e["png_path"].as_str().unwrap()),
    );
    assert!(rgb.chunks_exact(3).any(|p| [p[0], p[1], p[2]] == SEAM));
}

#[test]
fn side_view_seam_glyphs_project_via_y() {
    let dir = TempDir::new().unwrap();
    let (m, _) = run(json!({"view":"side","overlays":["seams"]}), &dir);
    let e = &m["images"][0];
    assert_eq!(e["view"], "side");
    assert!(!seam_events(e).is_empty());
    assert!(seam_events(e).iter().all(|v| v["z"].is_number()));
    let (_, _, rgb) = decode_rgb(
        &dir.path()
            .join("bundle")
            .join(e["png_path"].as_str().unwrap()),
    );
    assert!(rgb.chunks_exact(3).any(|p| [p[0], p[1], p[2]] == SEAM));
}

#[test]
fn missing_seam_plan_fails_closed() {
    let err = require_seam_plan(None).unwrap_err();
    assert!(err.to_string().contains("seam plan"));
}

/// The isolated seam image is emitted once per (tap, view), never once per
/// color-mode group: the faint base ignores `color_by`, so a role spec and a
/// tool spec that both request seams must share one image file and one
/// manifest entry. Before the per-(tap, view) dedup, this request wrote the
/// same filename twice (duplicate manifest entries, double `fs::write`).
/// Pinning the root cause of review finding F1 plus the color-tag root
/// cause: since the entry is color-agnostic, it must never carry the
/// `color_by: "tool"` marker — pre-fix, which group's `is_tool` the single
/// entry inherited depended on bundle iteration order.
#[test]
fn isolated_seam_image_is_emitted_once_across_color_modes_and_stays_unmarked() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("bundle");
    // exhaustive: role+tool dual-group seam overlay fixture
    let request = VisualDebugRequest {
        taps: vec![TapSelector::Name("PostPass::LayerFinalization".into())],
        visualizations: vec![
            VisualizationSpec::Detail {
                kind: "silhouette".into(),
                options: json!({"overlays":["seams"],"composited_overlays":["seams"]}),
            },
            VisualizationSpec::Detail {
                kind: "silhouette".into(),
                options: json!({"color_by":"tool","overlays":["seams"],"composited_overlays":["seams"]}),
            },
        ],
        ..request("1.2.0", json!({"overlays":["seams"]}), dir.path())
    };
    run_visual_debug(request, &out, false).unwrap();
    let m = manifest(&out);
    let isolated_path = "images/PostPass__LayerFinalization_silhouette_front_overlay_seams.png";
    let entries: Vec<&Value> = m["images"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["png_path"] == *isolated_path)
        .collect();
    assert_eq!(entries.len(), 1, "one isolated seam image per (tap, view)");
    let e = entries[0];
    assert!(!e.as_object().unwrap().contains_key("color_by"));
    assert!(!e.as_object().unwrap().contains_key("tool_color_source"));
    // The composited side keeps per-color-mode bases; role gets the plain
    // name, tool gets the `_tool` base, each mirroring its own events.
    let composited: Vec<&Value> = m["images"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["composited_overlays"] == json!(["seams"]))
        .collect();
    assert_eq!(composited.len(), 2);
    let mut paths: Vec<&str> = composited
        .iter()
        .map(|e| e["png_path"].as_str().unwrap())
        .collect();
    paths.sort();
    assert_ne!(paths[0], paths[1]);
}
