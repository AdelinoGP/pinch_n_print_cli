//! Packet 247 — schema 1.2.0 `silhouette` end-to-end bundle contract
//! (AC-1, AC-7, AC-9).
//!
//! These drive the real CLI pipeline (`pnp_cli::visual_debug::
//! run_visual_debug`, `Model` source) over `resources/regression_wedge.stl`,
//! because every assertion here is about the bundle/manifest handoff
//! (`run_model_source`'s silhouette branch) rather than the pure renderer —
//! the renderer's own projection/class behavior is pinned separately in
//! `slicer-runtime`.
//!
//! Only `CapturedIr::Slice` taps are used: the wedge fixture has no support
//! demand, so a support-tap end-to-end run would render an empty group, and
//! `render_silhouette_composite` deliberately fails closed on one. Support
//! silhouettes are pinned at the renderer level instead.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugError, VisualDebugRequest,
    VisualDebugSource, VisualizationSpec,
};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────── fixtures ────────────────────────────
// Mirrors `visual_debug_intermediate_renderer_tdd.rs`'s helpers (standalone
// integration-test binary; the small duplication is the Rust convention).

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

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// `layer_height` at the schema max (1.0mm) so the ~40mm-tall
/// regression_wedge bounds to ~40 layers instead of ~200.
fn write_bounded_config(dir: &Path) -> PathBuf {
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height": 1.0}"#).expect("write bounded config");
    path
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

/// A schema 1.2.0 silhouette request. `options` is passed verbatim so a test
/// can omit `view` entirely (AC-1's default-`front` case).
fn silhouette_request(
    taps: Vec<&str>,
    layers: Vec<LayerSelector>,
    config: PathBuf,
    options: Vec<Value>,
) -> VisualDebugRequest {
    // exhaustive: model-source silhouette request boundary fixture
    VisualDebugRequest {
        schema_version: "1.2.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(wedge_path()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers,
        taps: taps
            .into_iter()
            .map(|t| TapSelector::Name(t.to_string()))
            .collect(),
        visualizations: options
            .into_iter()
            .map(|options| VisualizationSpec::Detail {
                kind: "silhouette".to_string(),
                options,
            })
            .collect(),
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

/// Re-expand a manifest `layers_rendered` list back into the flat index set
/// it encodes — the round-trip that makes the range encoding lossless.
fn expand_layer_ranges(value: &Value) -> Vec<i64> {
    let mut out = Vec::new();
    for range in value.as_array().expect("layers_rendered is an array") {
        let start = range["start"].as_i64().expect("range start is an integer");
        let end = range["end"].as_i64().expect("range end is an integer");
        assert!(
            start <= end,
            "range must be non-empty and ascending: {range}"
        );
        out.extend(start..=end);
    }
    out
}

// ───────────────────────────── AC-1 ──────────────────────────────

#[test]
fn silhouette_bundle_entry_shape_and_default_front_view() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    // No `options.view` at all: the bundle must default to `front` (X-Z).
    let req = silhouette_request(
        vec!["Layer::Slice"],
        vec![LayerSelector::Range { start: 0, end: 3 }],
        config,
        vec![json!({})],
    );

    let manifest_path = run_visual_debug(req, &output, false).expect("silhouette bundle succeeds");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "one composite per (tap, view) group; got {images:#?}"
    );
    let entry = &images[0];

    assert_eq!(entry["visualization"], "silhouette");
    assert_eq!(entry["view"], "front");
    assert_eq!(entry["tap"], "Layer::Slice");
    assert_eq!(
        entry["png_path"], "images/Layer__Slice_silhouette_front.png",
        "composite filename is {{sanitized_tap}}_silhouette_{{view}}.png with no _l{{layer}} suffix"
    );
    assert!(
        output
            .join("images/Layer__Slice_silhouette_front.png")
            .exists(),
        "the referenced composite PNG must exist on disk"
    );

    // D7: a composite spans many layers, so neither per-layer key may be
    // present at all (not null, not 0 — absent).
    let obj = entry.as_object().expect("image entry is an object");
    assert!(
        !obj.contains_key("layer_index"),
        "silhouette entries must omit `layer_index` entirely; got {entry:#?}"
    );
    assert!(
        !obj.contains_key("layer_z"),
        "silhouette entries must omit `layer_z` entirely; got {entry:#?}"
    );

    // Lossless round-trip: re-expanding the ranges must reproduce exactly the
    // resolved layer index set (the request's inclusive 0..=3).
    let rendered = expand_layer_ranges(&entry["layers_rendered"]);
    assert_eq!(
        rendered,
        vec![0, 1, 2, 3],
        "layers_rendered must re-expand to exactly the resolved layer indices"
    );

    let bounds = entry["world_bounds_mm"]
        .as_object()
        .expect("world_bounds_mm is an object");
    for key in ["min_x", "min_y", "max_x", "max_y"] {
        assert!(
            bounds.get(key).and_then(Value::as_f64).is_some(),
            "world_bounds_mm.{key} must be a number; got {bounds:#?}"
        );
    }

    // Assigned fix: 1.2.0 records the existing v1.1 legend (silhouettes add
    // fill classes, not glyphs — LEGEND_VERSION is deliberately not bumped).
    assert_eq!(
        entry["legend_version"], "1.1.0",
        "a 1.2.0 bundle records the 1.1.0 legend, not the 1.0.0 one"
    );
}

// ───────────────────────────── AC-7 ──────────────────────────────

#[test]
fn z_frame_is_model_wide_not_selection_wide() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());

    let subset_out = tmp.path().join("subset");
    let subset_manifest = run_visual_debug(
        silhouette_request(
            vec!["Layer::Slice"],
            vec![LayerSelector::Range { start: 0, end: 2 }],
            config.clone(),
            vec![json!({})],
        ),
        &subset_out,
        false,
    )
    .expect("subset silhouette bundle succeeds");

    let all_out = tmp.path().join("all");
    let all_manifest = run_visual_debug(
        silhouette_request(
            vec!["Layer::Slice"],
            // Range resolution clamps to the real schedule, so this selects
            // every scheduled layer.
            vec![LayerSelector::Range {
                start: 0,
                end: 1_000_000,
            }],
            config,
            vec![json!({})],
        ),
        &all_out,
        false,
    )
    .expect("all-layer silhouette bundle succeeds");

    let subset = manifest_at(&subset_manifest);
    let all = manifest_at(&all_manifest);
    let subset_entry = &subset["images"][0];
    let all_entry = &all["images"][0];

    // Sanity: the two requests really did select different layer sets, so the
    // bounds equality below is not vacuous.
    let subset_layers = expand_layer_ranges(&subset_entry["layers_rendered"]);
    let all_layers = expand_layer_ranges(&all_entry["layers_rendered"]);
    assert_eq!(subset_layers, vec![0, 1, 2]);
    assert!(
        all_layers.len() > subset_layers.len(),
        "the all-layer request must render strictly more layers; got {all_layers:?}"
    );

    assert_eq!(
        serde_json::to_string(&subset_entry["world_bounds_mm"]).expect("serialize bounds"),
        serde_json::to_string(&all_entry["world_bounds_mm"]).expect("serialize bounds"),
        "the silhouette Z frame is model-wide (MeshIR::build_volume), so a layer \
         subset and the full model must record byte-identical world_bounds_mm"
    );
}

#[test]
fn region_mapping_bundle_entry_and_model_wide_frame() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let subset_out = tmp.path().join("region-subset");
    let subset_manifest = run_visual_debug(
        silhouette_request(
            vec!["PrePass::RegionMapping"],
            vec![LayerSelector::Range { start: 0, end: 2 }],
            config.clone(),
            vec![json!({"view": "front"})],
        ),
        &subset_out,
        false,
    )
    .expect("subset RegionMapping silhouette bundle succeeds");
    let all_out = tmp.path().join("region-all");
    let all_manifest = run_visual_debug(
        silhouette_request(
            vec!["PrePass::RegionMapping"],
            vec![LayerSelector::Range {
                start: 0,
                end: 1_000_000,
            }],
            config,
            vec![json!({"view": "front"})],
        ),
        &all_out,
        false,
    )
    .expect("all-layer RegionMapping silhouette bundle succeeds");
    let subset = manifest_at(&subset_manifest);
    let all = manifest_at(&all_manifest);
    for (bundle, output) in [(&subset, &subset_out), (&all, &all_out)] {
        let images = bundle["images"].as_array().expect("images array");
        assert_eq!(images.len(), 1, "one RegionMapping silhouette image");
        let entry = &images[0];
        assert_eq!(
            entry["png_path"],
            "images/PrePass__RegionMapping_silhouette_front.png"
        );
        assert_eq!(entry["visualization"], "silhouette");
        assert_eq!(entry["view"], "front");
        assert!(entry["layers_rendered"].is_array());
        let object = entry.as_object().expect("image entry object");
        assert!(!object.contains_key("layer_index"));
        assert!(!object.contains_key("layer_z"));
        assert!(output
            .join("images/PrePass__RegionMapping_silhouette_front.png")
            .exists());
    }
    assert_eq!(
        serde_json::to_string(&subset["images"][0]["world_bounds_mm"]).unwrap(),
        serde_json::to_string(&all["images"][0]["world_bounds_mm"]).unwrap(),
        "RegionMapping silhouette framing is model-wide"
    );
}

#[test]
fn silhouette_tool_on_remaining_taps_fails_tool_color_unavailable() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    for tap in ["PrePass::RegionMapping", "PrePass::OverhangAnnotation"] {
        let result = run_visual_debug(
            silhouette_request(
                vec![tap],
                vec![LayerSelector::Range { start: 0, end: 2 }],
                config.clone(),
                vec![json!({"view": "front", "color_by": "tool"})],
            ),
            &tmp.path().join(tap.replace("::", "-")),
            false,
        );
        let error = result.expect_err("tool-colored remaining tap must fail");
        let rendered = format!("{error:?}\n{error}");
        assert!(
            matches!(error, VisualDebugError::RenderFailed(ref message)
                if message.contains("color_by \"tool\" is unavailable")),
            "expected the wrapped ToolColorUnavailable contract: {rendered}"
        );
        assert!(
            rendered.contains(tap),
            "error must name tap {tap}: {rendered}"
        );
    }
}

// ───────────────────────────── AC-9 ──────────────────────────────

#[test]
fn one_image_per_tap_view_group_and_unique_filenames() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    // Two *identical* silhouette specs over two taps: duplicate specs collapse
    // into one (tap, view) group, so exactly two images may exist.
    let req = silhouette_request(
        vec!["Layer::Slice", "Layer::SlicePostProcess"],
        vec![LayerSelector::Range { start: 0, end: 2 }],
        config,
        vec![json!({"view": "front"}), json!({"view": "front"})],
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("two-tap silhouette bundle succeeds");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        2,
        "one composite per tap; duplicate specs must collapse. Got {images:#?}"
    );

    let mut paths: Vec<&str> = images
        .iter()
        .map(|e| e["png_path"].as_str().expect("png_path is a string"))
        .collect();
    let taps: Vec<&str> = images
        .iter()
        .map(|e| e["tap"].as_str().expect("tap is a string"))
        .collect();
    // Groups are ordered by STAGE_ORDER position, then tap string — the same
    // key `run_model_source` already sorts its merged captures by. `Layer::
    // Slice` is a Blackboard-read tap id with no STAGE_ORDER entry (the
    // scheduler stage is `PrePass::Slice`), so it takes the `usize::MAX`
    // position and sorts after `Layer::SlicePostProcess`, which is a real
    // STAGE_ORDER member. Deterministic, and consistent with the per-capture
    // manifest order elsewhere in the bundle.
    assert_eq!(
        taps,
        vec!["Layer::SlicePostProcess", "Layer::Slice"],
        "groups are ordered by STAGE_ORDER position, then tap"
    );

    for path in &paths {
        assert!(
            output.join(path).exists(),
            "every manifest png_path must exist on disk: {path}"
        );
    }
    let count = paths.len();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(count, paths.len(), "no two entries may share a png_path");
}
