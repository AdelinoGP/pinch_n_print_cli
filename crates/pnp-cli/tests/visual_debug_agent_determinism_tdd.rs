//! Packet 161, Step 10 — determinism closure gate for the visual-debug agent
//! (AC-5): two independent full runs of the SAME request, for BOTH source
//! modes (model-source with a whole-print `PostPass` tap, and standalone
//! gcode-source), must produce byte-identical `manifest.json`, identical
//! image/warning/layer/tap ordering, identical PNG relative paths, and
//! byte-identical PNG contents.
//!
//! This is a standalone integration-test binary — Rust has no shared-`mod`
//! convention across separately-compiled test binaries — so the small
//! fixture/helper duplication below is copied verbatim from the smallest
//! existing pnp-cli visual-debug tests rather than authoring anything new:
//! - Model-source request shape: `visual_debug_typed_tap_capture_tdd.rs`
//!   (packet 158) / `visual_debug_intermediate_renderer_tdd.rs`'s
//!   `model_request_with_viz` (packet 159) — `resources/regression_wedge.stl`
//!   + `modules/core-modules`, bounded to ~40 layers via `layer_height: 1.0`.
//! - Standalone gcode-source fixture: `visual_debug_gcode_renderer_tdd.rs`'s
//!   (packet 160) `SUPPORTED_SINGLE_LAYER_GCODE` inline fixture.
//! - PNG dimension/signature parsing: the same hand-rolled IHDR reader used
//!   by both of those files (no `png` crate dependency in scope here).

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, LayerSelector, TapSelector, VisualDebugRequest, VisualDebugSource,
    VisualizationSpec,
};
use serde_json::Value;
use tempfile::TempDir;

// ─────────────────────────── shared fixtures/helpers ───────────────────────

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
/// regression_wedge fixture bounds to ~40 layers instead of ~200 — mirrors
/// packet 158/159's own bound.
fn write_bounded_config(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).expect("config directory");
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height": 1.0}"#).expect("write bounded config");
    path
}

/// Model-source request carrying one whole-print `PostPass` tap
/// (`PostPass::GCodeEmit`, the third tap class per ADR-0040 — its source IR
/// only exists after the whole per-layer -> finalization -> postpass prefix
/// runs) plus a `filament_lines` visualization, so the run actually produces
/// a PNG (not just an unrendered `typed_ir` entry). `filled_areas` is NOT
/// usable here: `GCodeEmit`'s `GCodeCommand::Move` carries no
/// `Point3WithWidth.width`, and the renderer refuses to infer a bead width
/// for it (`RenderFailed`) — `filament_lines` has no such requirement.
fn model_request_with_postpass_tap(config: PathBuf) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(wedge_path()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers: vec![LayerSelector::Index(0)],
        taps: vec![TapSelector::Name("PostPass::GCodeEmit".to_string())],
        visualizations: vec![VisualizationSpec::Name("filament_lines".to_string())],
        resolution_scale: 1,
        gcode_line_width_mm: None,
    }
}

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

fn write_gcode(dir: &Path, file_name: &str, contents: &str) -> PathBuf {
    let path = dir.join(file_name);
    fs::write(&path, contents).expect("write gcode fixture");
    path
}

fn gcode_request(gcode_path: PathBuf) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Gcode {
            path: Some(gcode_path),
            model: None,
        },
        layers: vec![LayerSelector::Index(0)],
        taps: vec![TapSelector::Name("final_gcode".to_string())],
        visualizations: vec![VisualizationSpec::Name("filament_lines".to_string())],
        resolution_scale: 1,
        gcode_line_width_mm: None,
    }
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    assert_eq!(&bytes[0..8], &SIGNATURE, "not a PNG file");
    assert_eq!(&bytes[12..16], b"IHDR", "IHDR must be the first PNG chunk");
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("4 bytes"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("4 bytes"));
    (width, height)
}

/// Run the same request twice into two independent, clean output directories
/// and assert full byte/order determinism between the two runs: raw
/// `manifest.json` bytes, image/warning/tap/layer ordering, PNG relative
/// paths, and PNG bytes.
fn assert_two_runs_are_byte_deterministic(
    mode: &str,
    req_a: VisualDebugRequest,
    req_b: VisualDebugRequest,
    tmp: &TempDir,
) {
    let output_a = tmp.path().join(format!("bundle-{mode}-a"));
    let output_b = tmp.path().join(format!("bundle-{mode}-b"));

    let manifest_path_a = run_visual_debug(req_a, &output_a, false)
        .unwrap_or_else(|e| panic!("[{mode}] first run must succeed: {e}"));
    let manifest_path_b = run_visual_debug(req_b, &output_b, false)
        .unwrap_or_else(|e| panic!("[{mode}] second run must succeed: {e}"));

    // Raw manifest.json bytes must be identical — the strictest possible
    // check (catches key reordering, whitespace drift, or any nondeterministic
    // field, not just value-equality after JSON parsing).
    let raw_a = fs::read(&manifest_path_a).expect("read manifest.json a");
    let raw_b = fs::read(&manifest_path_b).expect("read manifest.json b");
    assert_eq!(
        raw_a, raw_b,
        "[{mode}] complete manifest.json bytes must be identical across two clean runs \
         of the same request"
    );

    let manifest_a = manifest_at(&manifest_path_a);
    let manifest_b = manifest_at(&manifest_path_b);
    // Redundant with the raw-byte check above, but pins the specific ordering
    // clauses this AC calls out by name.
    assert_eq!(
        manifest_a["images"], manifest_b["images"],
        "[{mode}] image/tap/layer ordering must be identical across runs"
    );
    assert_eq!(
        manifest_a["warnings"], manifest_b["warnings"],
        "[{mode}] warning ordering must be identical across runs"
    );

    let images_a = manifest_a["images"].as_array().expect("images array a");
    let images_b = manifest_b["images"].as_array().expect("images array b");
    assert_eq!(images_a.len(), images_b.len());
    assert!(
        !images_a.is_empty(),
        "[{mode}] sanity: at least one rendered image entry"
    );

    for (entry_a, entry_b) in images_a.iter().zip(images_b.iter()) {
        let png_path_a = entry_a["png_path"]
            .as_str()
            .expect("png_path a is a string");
        let png_path_b = entry_b["png_path"]
            .as_str()
            .expect("png_path b is a string");
        assert_eq!(
            png_path_a, png_path_b,
            "[{mode}] PNG relative paths must be identical across runs"
        );
        assert!(
            !png_path_a.is_empty(),
            "[{mode}] png_path must be populated"
        );

        let png_bytes_a = fs::read(output_a.join(png_path_a))
            .unwrap_or_else(|e| panic!("[{mode}] read PNG a at {png_path_a}: {e}"));
        let png_bytes_b = fs::read(output_b.join(png_path_b))
            .unwrap_or_else(|e| panic!("[{mode}] read PNG b at {png_path_b}: {e}"));
        assert_eq!(
            png_bytes_a, png_bytes_b,
            "[{mode}] every PNG's on-disk bytes must be byte-identical across two \
             independent runs"
        );
        // Sanity: it really is a well-formed PNG in both runs, not two
        // identical-but-empty/garbage files.
        let (w, h) = png_dimensions(&png_bytes_a);
        assert!(
            w > 0 && h > 0,
            "[{mode}] PNG must have non-zero raster dimensions"
        );
    }
}

#[test]
fn visual_debug_bundles_are_byte_deterministic() {
    let tmp = TempDir::new().expect("tempdir");

    // ---- Model-source mode, with a whole-print PostPass tap. ----
    let model_config_a = write_bounded_config(&tmp.path().join("model-config-a"));
    let model_config_b = write_bounded_config(&tmp.path().join("model-config-b"));
    assert_two_runs_are_byte_deterministic(
        "model",
        model_request_with_postpass_tap(model_config_a),
        model_request_with_postpass_tap(model_config_b),
        &tmp,
    );

    // ---- Standalone final-gcode-source mode. ----
    // Deliberately the SAME source file path for both runs (unlike the
    // model-source case above, `ManifestSource` for a gcode source records
    // the queried `path` verbatim — two different source directories would
    // make the manifests differ by that path alone, which is not the
    // nondeterminism this AC is about).
    let gcode_src_dir = tmp.path().join("gcode-src");
    fs::create_dir_all(&gcode_src_dir).expect("gcode src dir");
    let gcode_path = write_gcode(&gcode_src_dir, "final.gcode", SUPPORTED_SINGLE_LAYER_GCODE);
    assert_two_runs_are_byte_deterministic(
        "gcode",
        gcode_request(gcode_path.clone()),
        gcode_request(gcode_path),
        &tmp,
    );
}
