//! Packet 160, Step 1 (red-phase TDD scaffolding) — standalone final-G-code
//! visual-debug renderer.
//!
//! `pnp_cli::visual_debug`'s `VisualDebugSource::Gcode` arm (currently a
//! verified placeholder: it never opens the file, never parses G-code, only
//! ever uses `req.layers.first()` for every image, and never writes PNG
//! bytes — see `crates/pnp-cli/src/visual_debug.rs`) must become a real
//! parser + renderer for the documented PnP G0/G1 subset
//! (`docs/specs/visual-pipeline-debug.md`, `docs/19_visual_debug.md`):
//! `;LAYER_CHANGE`, `;Z:`, `;TYPE:` markers, G0/G1 X/Y/Z/E/F moves, absolute
//! vs. relative extrusion mode.
//!
//! This file is scaffolding only (Step 1): it drives the real CLI entry
//! point (`pnp_cli::visual_debug::run_visual_debug`, mirroring the
//! `Model`-source invocation pattern already used by
//! `visual_debug_typed_tap_capture_tdd.rs` (packet 158) and
//! `visual_debug_intermediate_renderer_tdd.rs` (packet 159)) against inline
//! deterministic G-code fixtures written to a tempdir. It is EXPECTED that
//! some/most assertions below currently FAIL (or the placeholder "succeeds"
//! in a way that violates the AC) — the parser/renderer implementation is a
//! separate step (Step 2) on a different worker. This file's own job is
//! only to exist, name the 8 acceptance-criteria tests exactly, and compile.
//!
//! None of these tests depend on the `png` crate (out of this packet's edit
//! scope for `Cargo.toml`): PNG raster dimensions are read by hand-parsing
//! the fixed 8-byte PNG signature + IHDR chunk header, mirroring
//! `visual_debug_intermediate_renderer_tdd.rs`'s own `png_dimensions` helper.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::visual_debug_gcode::parse_gcode;
use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, ValidationError, VisualDebugError,
    VisualDebugRequest, VisualDebugSource, VisualizationSpec,
};
use serde_json::Value;
use tempfile::TempDir;

// ─────────────────────────────── fixtures ───────────────────────────────────

fn write_gcode(dir: &Path, file_name: &str, contents: &str) -> PathBuf {
    let path = dir.join(file_name);
    fs::write(&path, contents).expect("write gcode fixture");
    path
}

fn gcode_request(
    gcode_path: PathBuf,
    layers: Vec<i64>,
    taps: Vec<&str>,
    visualizations: Vec<VisualizationSpec>,
    resolution_scale: u32,
    gcode_line_width_mm: Option<f64>,
) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Gcode {
            path: Some(gcode_path),
            model: None,
        },
        layers: layers.into_iter().map(LayerSelector::Index).collect(),
        taps: taps
            .into_iter()
            .map(|t| TapSelector::Name(t.to_string()))
            .collect(),
        visualizations,
        resolution_scale,
        gcode_line_width_mm,
        frame: FrameMode::Model,
    }
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

/// Parse a PNG's raster width/height from its IHDR chunk without a
/// PNG-decoding crate dependency. Per the PNG spec, IHDR is always the first
/// chunk, immediately after the fixed 8-byte signature.
fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    assert_eq!(&bytes[0..8], &SIGNATURE, "not a PNG file");
    assert_eq!(&bytes[12..16], b"IHDR", "IHDR must be the first PNG chunk");
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("4 bytes"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("4 bytes"));
    (width, height)
}

/// Collect every warning string a bundle carries: the bundle-wide
/// `manifest.warnings` plus every per-image `warnings` entry. The AC list
/// doesn't pin which of the two the parser/renderer will use, so tests that
/// assert "manifest.json records a warning" search both.
fn all_warnings(manifest: &Value) -> Vec<String> {
    let mut out: Vec<String> = manifest["warnings"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if let Some(images) = manifest["images"].as_array() {
        for image in images {
            if let Some(warnings) = image["warnings"].as_array() {
                out.extend(
                    warnings
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string)),
                );
            }
        }
    }
    out
}

/// A single-layer, fully-supported fixture: layer-change/Z/type markers, a
/// travel move, and a small extrusion loop with monotonically increasing
/// absolute E — every construct the documented G0/G1 subset covers.
const SUPPORTED_SINGLE_LAYER_GCODE: &str = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G1 X10 Y10 E2.0
G1 X0 Y10 E3.0
G1 X0 Y0 E4.0
";

// ─────────────────────────────── AC-1 ────────────────────────────────────────

#[test]
fn ac1_supported_final_gcode_produces_manifest_and_pngs() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let manifest_path = run_visual_debug(req, &output, false)
        .expect("a fully-supported final-gcode request should succeed");
    let manifest = manifest_at(&manifest_path);

    assert_eq!(
        manifest["source"]["kind"], "gcode",
        "manifest top-level source.kind must record the gcode source"
    );
    assert!(
        !manifest["gcode_parser_version"].is_null(),
        "the bundle-wide gcode_parser_version must be populated for a gcode-source bundle"
    );

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "exactly one rendered layer was selected/renderable; got {images:#?}"
    );
    let entry = &images[0];
    assert_eq!(
        entry["source"], "gcode",
        "image entry must record source=gcode"
    );
    assert_eq!(entry["tap"], "final_gcode");
    assert_eq!(entry["layer_index"], 0);
    assert!(
        !entry["layer_z"].is_null(),
        "the parsed ;Z: marker must populate layer_z"
    );
    assert!(
        !entry["gcode_parser_version"].is_null(),
        "each image entry must also carry the gcode parser version"
    );
    assert_eq!(entry["legend_version"], manifest["legend_version"]);
    assert_eq!(
        entry["viewport"], manifest["viewport"],
        "every image shares the one bundle-wide XY viewport"
    );

    let png_path = entry["png_path"].as_str().expect("png_path is a string");
    assert!(!png_path.is_empty(), "png_path must be populated");
    let png_file = output.join(png_path);
    let bytes = fs::read(&png_file).expect("the referenced PNG must exist on disk");
    let (w, h) = png_dimensions(&bytes);
    assert_eq!((w, h), (1024, 1024), "resolution_scale 1 -> 1024x1024");
}

// ─────────────────────────────── AC-2 ────────────────────────────────────────

#[test]
fn ac2_preserves_unclassified_extrusion() {
    // A real extrusion move (`E` increases) occurs before any ;TYPE: marker
    // is seen, so it has no active role: it must be retained with role
    // "unclassified" (never dropped, never guessed as the following role),
    // and the bundle must carry a warning saying so.
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
G1 X0 Y0 F3000
G1 X5 Y0 E0.5 F1200
;TYPE:Outer wall
G1 X10 Y0 E1.0
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", gcode);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let manifest_path = run_visual_debug(req, &output, false)
        .expect("role-less extrusion must still render, not fail the whole bundle");
    let manifest = manifest_at(&manifest_path);

    let warnings = all_warnings(&manifest);
    assert!(
        warnings
            .iter()
            .any(|w| w.to_lowercase().contains("unclassified")),
        "manifest must carry an unclassified-extrusion warning; got warnings={warnings:?}"
    );

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "the single selected layer must still render"
    );
    let png_path = images[0]["png_path"]
        .as_str()
        .expect("png_path is a string");
    assert!(
        output.join(png_path).exists(),
        "the layer must still produce a PNG despite the role-less segment"
    );
}

// ─────────────────────────────── AC-3 ────────────────────────────────────────

#[test]
fn ac3_filled_areas_use_requested_line_width() {
    // Two otherwise-identical requests differing only in
    // `gcode_line_width_mm` must rasterize visibly different geometry —
    // proving `filled_areas` used the requested width rather than deriving a
    // bead width from E (E is identical across both requests here).
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);

    let narrow_output = tmp.path().join("bundle-narrow");
    let narrow_req = gcode_request(
        gcode_path.clone(),
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
        Some(0.2),
    );
    let narrow_manifest_path = run_visual_debug(narrow_req, &narrow_output, false)
        .expect("filled_areas with an explicit line width should succeed");
    let narrow_manifest = manifest_at(&narrow_manifest_path);
    let narrow_png = narrow_manifest["images"][0]["png_path"]
        .as_str()
        .expect("png_path is a string")
        .to_string();
    let narrow_bytes = fs::read(narrow_output.join(&narrow_png)).expect("read narrow PNG");

    let wide_output = tmp.path().join("bundle-wide");
    let wide_req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
        Some(1.2),
    );
    let wide_manifest_path = run_visual_debug(wide_req, &wide_output, false)
        .expect("filled_areas with a different explicit line width should succeed");
    let wide_manifest = manifest_at(&wide_manifest_path);
    let wide_png = wide_manifest["images"][0]["png_path"]
        .as_str()
        .expect("png_path is a string")
        .to_string();
    let wide_bytes = fs::read(wide_output.join(&wide_png)).expect("read wide PNG");

    assert_ne!(
        narrow_bytes, wide_bytes,
        "changing only gcode_line_width_mm (E values held constant) must change the \
         rasterized filled_areas output; the width must come from the request, not E"
    );
}

// ─────────────────────────────── AC-4 ────────────────────────────────────────

#[test]
fn ac4_tracks_motion_state_layers_roles_and_shared_viewport() {
    // Two layers, absolute extrusion mode (M82), a travel move (G0, no E),
    // and a role change carried across the layer boundary. Selecting BOTH
    // layers is deliberate: the documented placeholder bug renders
    // `req.layers.first()` for every image, so a correct implementation must
    // produce two DIFFERENT (layer_index, layer_z) image entries here, not
    // the same layer duplicated twice.
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
M82
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G0 X10 Y10 F3000
G1 X0 Y10 E2.0
;LAYER_CHANGE
;Z:0.4
G1 Z0.4 F600
;TYPE:Solid infill
G1 X0 Y0 F3000
G1 X10 Y0 E3.0 F1200
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", gcode);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0, 1],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("multi-layer gcode render should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        2,
        "both selected layers must produce their own image entry; got {images:#?}"
    );

    let layer_indices: Vec<i64> = images
        .iter()
        .map(|e| e["layer_index"].as_i64().expect("layer_index is i64"))
        .collect();
    assert!(
        layer_indices.contains(&0) && layer_indices.contains(&1),
        "falsifies the req.layers.first()-only placeholder bug: both layer 0 and \
         layer 1 must appear, not layer 0 twice; got {layer_indices:?}"
    );

    let layer_zs: Vec<f64> = images
        .iter()
        .map(|e| e["layer_z"].as_f64().expect("layer_z is f64"))
        .collect();
    assert_ne!(
        layer_zs[0], layer_zs[1],
        "the two layers' parsed ;Z: markers must differ (0.2 vs 0.4), not both \
         report the first layer's Z; got {layer_zs:?}"
    );

    // Every image entry shares one model-wide XY viewport, identical to the
    // bundle-wide manifest viewport.
    for entry in images {
        assert_eq!(
            entry["viewport"], manifest["viewport"],
            "every image entry must carry the shared, bundle-wide XY viewport"
        );
    }

    let png_paths: Vec<&str> = images
        .iter()
        .map(|e| e["png_path"].as_str().expect("png_path is a string"))
        .collect();
    assert_ne!(
        png_paths[0], png_paths[1],
        "the two layers must produce two distinct PNGs, not overwrite one another"
    );
    for path in &png_paths {
        assert!(
            output.join(path).exists(),
            "each layer's PNG must exist on disk: {path}"
        );
    }
}

// ─────────────────────────────── AC-5 ────────────────────────────────────────

#[test]
fn ac5_records_unsupported_construct_line_warning() {
    // Line 6 (1-indexed) is a G2 arc move — a raw construct outside the
    // documented G0/G1 subset. It must not be approximated; the manifest
    // must record a warning naming that source line number, and the
    // remaining supported moves (line 7) must still render.
    let lines: Vec<&str> = vec![
        ";LAYER_CHANGE",
        ";Z:0.2",
        "G1 Z0.2 F600",
        ";TYPE:Outer wall",
        "G1 X0 Y0 F3000",
        "G2 X10 Y0 I5 J0 E1.0 F1200",
        "G1 X10 Y10 E2.0",
    ];
    let unsupported_line_number = 6usize; // 1-indexed position of the G2 line above.
    assert_eq!(
        lines[unsupported_line_number - 1],
        "G2 X10 Y0 I5 J0 E1.0 F1200",
        "sanity: the hardcoded line number must match the fixture's actual G2 line"
    );
    let gcode = format!("{}\n", lines.join("\n"));

    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", &gcode);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let manifest_path = run_visual_debug(req, &output, false)
        .expect("supported moves elsewhere in the file must let the bundle complete");
    let manifest = manifest_at(&manifest_path);

    let warnings = all_warnings(&manifest);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains(&unsupported_line_number.to_string())),
        "a warning must name the unsupported construct's source line number \
         ({unsupported_line_number}); got warnings={warnings:?}"
    );

    let images = manifest["images"].as_array().expect("images array");
    assert!(
        !images.is_empty(),
        "supported moves must still render even though one line was rejected"
    );
}

// ─────────────────────────────── AC-6 ────────────────────────────────────────

#[test]
fn ac6_final_gcode_render_is_deterministic() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);

    let req_a = gcode_request(
        gcode_path.clone(),
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );
    let output_a = tmp.path().join("bundle-a");
    let manifest_path_a =
        run_visual_debug(req_a, &output_a, false).expect("first run should succeed");
    let manifest_a = manifest_at(&manifest_path_a);

    let req_b = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );
    let output_b = tmp.path().join("bundle-b");
    let manifest_path_b =
        run_visual_debug(req_b, &output_b, false).expect("second run should succeed");
    let manifest_b = manifest_at(&manifest_path_b);

    assert_eq!(
        manifest_a, manifest_b,
        "two clean runs of the same request must produce byte/value-identical \
         manifests, including warning ordering and image-entry ordering"
    );

    let images_a = manifest_a["images"].as_array().expect("images array");
    let images_b = manifest_b["images"].as_array().expect("images array");
    assert_eq!(images_a.len(), images_b.len());
    assert!(!images_a.is_empty(), "sanity: at least one image entry");

    for (entry_a, entry_b) in images_a.iter().zip(images_b.iter()) {
        let png_path_a = entry_a["png_path"].as_str().expect("png_path is a string");
        let png_path_b = entry_b["png_path"].as_str().expect("png_path is a string");
        let png_a = fs::read(output_a.join(png_path_a)).expect("read PNG a");
        let png_b = fs::read(output_b.join(png_path_b)).expect("read PNG b");
        assert_eq!(
            png_a, png_b,
            "on-disk PNG bytes must be byte-identical across two independent runs"
        );
    }
}

// ─────────────────────────────── AC-N1 ───────────────────────────────────────

#[test]
fn ac_n1_rejects_filled_areas_without_line_width() {
    // Pre-existing packet-157 validation (visual_debug.rs:197-215): a
    // filled_areas request must supply gcode_line_width_mm explicitly. This
    // test only confirms that existing behavior still holds for the gcode
    // source, and that rejection happens before parsing/PNG creation.
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
        None,
    );

    let err = run_visual_debug(req, &output, false)
        .expect_err("filled_areas without an explicit gcode_line_width_mm must be rejected");
    assert!(
        matches!(
            err,
            VisualDebugError::Validation(ValidationError::GcodeLineWidth)
        ),
        "the rejection must be the specific GcodeLineWidth validation error, rejected \
         before parsing/PNG creation; got: {err:?}"
    );

    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when the request is rejected \
         before parsing/PNG creation"
    );
}

// ─────────────────────────────── AC-N2 ───────────────────────────────────────

#[test]
fn ac_n2_rejects_input_with_no_supported_renderable_moves() {
    // Only unsupported motion constructs (G2/G3 arcs), no G0/G1 at all: no
    // supported renderable move exists anywhere in the file, so the command
    // must fail outright rather than report a successful empty/partial
    // bundle.
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
G2 X10 Y0 I5 J0
G3 X0 Y0 I-5 J0
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", gcode);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![0],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let err = run_visual_debug(req, &output, false).expect_err(
        "a file with no supported G0/G1 renderable moves must fail, not report a \
         successful partial bundle",
    );
    assert!(
        matches!(err, VisualDebugError::NoRenderableGcodeMoves(_)),
        "the rejection must be the specific NoRenderableGcodeMoves error; got: {err:?}"
    );
    if let VisualDebugError::NoRenderableGcodeMoves(message) = &err {
        assert!(
            !message.is_empty(),
            "the rejection must carry a diagnostic message"
        );
    }

    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when no renderable move exists"
    );
    assert!(
        !output.exists()
            || fs::read_dir(&output)
                .expect("read output dir")
                .next()
                .is_none(),
        "no stray PNG or other file may land in the output directory on total rejection"
    );
}

// ───────────────────────── follow-up gap 1: named layer selector ────────────

#[test]
fn ac_gap1_gcode_rejects_named_layer_selector() {
    // `LayerSelector::Name` has no meaning for the standalone final-G-code
    // source (only `;LAYER_CHANGE`/`;Z:` markers indexed by parse order
    // exist — there is no named-layer table). It must be rejected with a
    // clear validation error, not silently aliased to layer 0 via
    // `layer_info`'s catch-all arm.
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);
    let output = tmp.path().join("bundle");

    let req = VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Gcode {
            path: Some(gcode_path),
            model: None,
        },
        layers: vec![LayerSelector::Name("first_layer".to_string())],
        taps: vec![TapSelector::Name("final_gcode".to_string())],
        visualizations: vec![VisualizationSpec::Name("filament_lines".to_string())],
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    };

    let err = run_visual_debug(req, &output, false).expect_err(
        "a Name layer selector is meaningless for a gcode source and must be rejected, \
         not silently treated as layer 0",
    );
    assert!(
        matches!(
            err,
            VisualDebugError::Validation(ValidationError::GcodeUnsupportedLayerSelector)
        ),
        "the rejection must be the specific GcodeUnsupportedLayerSelector validation \
         error; got: {err:?}"
    );
    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when the request is rejected"
    );
}

// ───────────────────────── follow-up gap 2: empty layers ────────────────────

#[test]
fn ac_gap2_gcode_rejects_empty_layers_with_visualizations() {
    // Empty `layers` with non-empty `visualizations` must fail the whole
    // command (mirroring the Model source's `NoApplicableLayer` guard), not
    // silently succeed with an empty `images` array and a written
    // manifest.json.
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![],
        vec!["final_gcode"],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
        1,
        None,
    );

    let err = run_visual_debug(req, &output, false).expect_err(
        "empty layers with non-empty visualizations must fail the whole command, not \
         silently succeed with an empty bundle",
    );
    assert!(
        matches!(err, VisualDebugError::NoApplicableLayer),
        "the rejection must be the specific NoApplicableLayer error; got: {err:?}"
    );
    assert!(
        !output.join("manifest.json").exists(),
        "no manifest.json may be written when no layer applies"
    );
}

// ───────────────────────── follow-up gap 3: stale position tracking ─────────

#[test]
fn ac_gap3_position_tracking_survives_skipped_unsupported_axis_line() {
    // `G1 X5 Y5 U1` carries an unsupported axis (U) but ALSO recognized X/Y:
    // a real printer would still physically move to (5, 5), so the tracked
    // position must advance there even though this partially-unsupported
    // move itself is never rendered (never approximate what we don't fully
    // understand). The following fully-supported `G1 X10 Y10 E1.0` move
    // must then compute its segment as a delta from the CORRECTED (5, 5),
    // not the stale initial (0, 0).
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X5 Y5 U1 F1200
G1 X10 Y10 E1.0
";
    let parsed = parse_gcode(gcode);
    assert_eq!(parsed.layers.len(), 1);
    let segments = &parsed.layers[0].segments;
    assert_eq!(
        segments.len(),
        1,
        "only the fully-supported second move renders a segment; the unsupported-axis \
         line must never itself be rendered; got {segments:?}"
    );
    let seg = &segments[0];
    assert_eq!(
        (seg.from.x, seg.from.y),
        (5.0, 5.0),
        "the supported move's segment must start from the position after the \
         skipped-but-position-advancing unsupported-axis line (5, 5), not a stale \
         position; got {:?}",
        seg.from
    );
    assert_eq!(
        (seg.to.x, seg.to.y),
        (10.0, 10.0),
        "the supported move's segment must end at its own X/Y target"
    );

    assert!(
        parsed.warnings.iter().any(|w| w.contains("line 5")),
        "the unsupported-axis line must still be recorded as a warning naming its \
         source line; got {:?}",
        parsed.warnings
    );
}

// ──────────────── model-wide bounds must not include a phantom origin ───────

/// Real slicer output starts printing wherever the part sits on the bed, after
/// a homing/start macro this parser does not model. Its first `G0`/`G1` is a
/// move to that spot — not from the bed origin.
///
/// This fixture mirrors that: every coordinate lives in a 20x10 mm box near
/// (80, 90), far from (0, 0). Every in-repo fixture before this one opened with
/// `G1 X0 Y0`, which is exactly why the bug below survived: at the origin, a
/// fabricated origin is indistinguishable from a real one.
const OFFSET_FROM_ORIGIN_GCODE: &str = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X80 Y90 F3000
G1 X100 Y90 E1.0 F1200
G1 X100 Y100 E2.0
G1 X80 Y100 E3.0
G1 X80 Y90 E4.0
";

/// The model-wide bounding box must cover the geometry the file actually
/// contains — nothing else.
///
/// The parser used to assume the toolhead began at `(0, 0)` and treat the
/// opening `G1 X80 Y90` as a real travel *from the bed origin*, pulling
/// `(0, 0)` into the bounds. Every render was then framed from the bed origin
/// to the model's far corner, so the part appeared shrunk into the corner of
/// what looked like a full-plate view. On the real Benchy fixture this stretched
/// the viewport from x[79.7..140.2] to x[0..140.2].
#[test]
fn bounds_exclude_the_unstated_start_position() {
    let parsed = parse_gcode(OFFSET_FROM_ORIGIN_GCODE);
    let (min_x, min_y, max_x, max_y) = parsed.bounds_mm.expect("fixture has motion");

    assert_eq!(
        (min_x, min_y, max_x, max_y),
        (80.0, 90.0, 100.0, 100.0),
        "bounds must be the stated geometry's own box; a min at (0, 0) means the \
         unstated start position is being fabricated as the bed origin again"
    );
}

/// The opening move's *destination* is real, stated geometry and must bound the
/// viewport — only the un-drawable travel *to* it is dropped. The four extruded
/// walls remain, and no phantom segment from the origin is invented.
#[test]
fn opening_move_destination_counts_but_invents_no_segment() {
    let parsed = parse_gcode(OFFSET_FROM_ORIGIN_GCODE);
    let layer = &parsed.layers[0];

    for seg in &layer.segments {
        for (name, p) in [("from", seg.from), ("to", seg.to)] {
            assert!(
                p.x >= 80.0 && p.y >= 90.0,
                "segment {name} ({}, {}) lies outside the fixture's geometry — a travel \
                 from a fabricated origin was drawn",
                p.x,
                p.y
            );
        }
    }

    let extrusions = layer.segments.iter().filter(|s| s.is_extrusion).count();
    assert_eq!(extrusions, 4, "all four extruded walls must survive");
}
