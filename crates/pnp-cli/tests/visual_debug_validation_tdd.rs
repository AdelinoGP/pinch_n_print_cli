//! Packet 161, Step 8 — two-phase fail-closed request validation
//! (ADR-0041): no requested visualization or layer is ever silently omitted
//! from a bundle the command reports as successful.
//!
//! Exercises `pnp_cli::visual_debug::run_visual_debug` directly (as a
//! library call, matching the pattern already used by
//! `visual_debug_typed_tap_capture_tdd.rs` and
//! `visual_debug_gcode_renderer_tdd.rs`), plus a direct
//! `serde_json`/`LayerSelector` deserialization check for the malformed
//! `{start, end}` range case, since that failure happens before a
//! `VisualDebugRequest` even exists.
//!
//! Fixtures are the smallest already-established ones in this test suite:
//! a two-line inline G-code source (mirroring
//! `visual_debug_gcode_renderer_tdd.rs`'s `SUPPORTED_SINGLE_LAYER_GCODE`)
//! and a `Model` source pointed at deliberately nonexistent paths
//! (mirroring `visual_debug_typed_tap_capture_tdd.rs`'s
//! `unreachable_model_request`) for the checks that must reject before ever
//! touching the filesystem. No new geometry/mesh generator is introduced.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, LayerSelector, TapSelector, ValidationError, VisualDebugError,
    VisualDebugRequest, VisualDebugSource, VisualizationSpec,
};
use tempfile::TempDir;

// ─────────────────────────────── fixtures ───────────────────────────────────

fn write_gcode(dir: &Path, file_name: &str, contents: &str) -> PathBuf {
    let path = dir.join(file_name);
    fs::write(&path, contents).expect("write gcode fixture");
    path
}

/// A single-layer, fully-supported final-G-code fixture — the same minimal
/// shape as `visual_debug_gcode_renderer_tdd.rs`'s
/// `SUPPORTED_SINGLE_LAYER_GCODE`.
const SINGLE_LAYER_GCODE: &str = "\
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

/// Two layers at distinct `;Z:` heights, so a `{start, end}` range and a
/// z-only `Detail` each have a real, distinguishable target to resolve
/// against.
const TWO_LAYER_GCODE: &str = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G1 X10 Y10 E2.0
G1 X0 Y10 E3.0
G1 X0 Y0 E4.0
;LAYER_CHANGE
;Z:0.4
G1 Z0.4 F600
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E5.0 F1200
G1 X10 Y10 E6.0
G1 X0 Y10 E7.0
G1 X0 Y0 E8.0
";

fn gcode_request(
    gcode_path: PathBuf,
    layers: Vec<LayerSelector>,
    visualizations: Vec<VisualizationSpec>,
) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Gcode {
            path: Some(gcode_path),
            model: None,
        },
        layers,
        taps: vec![TapSelector::Name("final_gcode".to_string())],
        visualizations,
        resolution_scale: 1,
        gcode_line_width_mm: None,
    }
}

/// A `Model`-source request pointed at deliberately nonexistent paths —
/// mirrors `visual_debug_typed_tap_capture_tdd.rs`'s
/// `unreachable_model_request`. Used only for checks that must reject in
/// `validate_request` (Phase 1), before the model/config/modules are ever
/// touched.
fn unreachable_model_request(layers: Vec<LayerSelector>) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(PathBuf::from("/definitely/does/not/exist/model.stl")),
            config: Some(PathBuf::from("/definitely/does/not/exist/config.json")),
            module_dirs: vec![PathBuf::from("/definitely/does/not/exist/modules")],
            path: None,
        },
        layers,
        taps: vec![TapSelector::Name("Layer::Perimeters".to_string())],
        visualizations: Vec::new(),
        resolution_scale: 1,
        gcode_line_width_mm: None,
    }
}

fn manifest_at(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

/// Asserts a rejected request left no manifest and no stray file anywhere
/// under `output` (mirrors the equivalent assertion in
/// `visual_debug_gcode_renderer_tdd.rs`).
fn assert_bundle_empty(output: &Path) {
    assert!(
        !output.join("manifest.json").exists(),
        "no manifest.json may be written when the request is rejected"
    );
    assert!(
        !output.exists()
            || fs::read_dir(output)
                .expect("read output dir")
                .next()
                .is_none(),
        "no stray PNG or other file may land in the output directory on rejection"
    );
}

// ─────────────────────────────── AC-N1 ──────────────────────────────────────

#[test]
fn unknown_visualization_kind_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("bundle");

    // The model/config/module paths never need to exist: Phase 1
    // (`validate_request`) must reject before any filesystem access for the
    // source at all.
    let req = unreachable_model_request(vec![LayerSelector::Index(0)]);
    let mut req = req;
    req.visualizations = vec![VisualizationSpec::Name(
        "totally_bogus_visualization".to_string(),
    )];

    let err = run_visual_debug(req, &output, false).expect_err(
        "an unknown visualization kind must fail closed before any render or bundle write",
    );
    assert!(
        matches!(
            &err,
            VisualDebugError::Validation(ValidationError::UnknownVisualizationKind { kind })
                if kind == "totally_bogus_visualization"
        ),
        "expected UnknownVisualizationKind{{kind: \"totally_bogus_visualization\"}}, got {err:?}"
    );
    assert_bundle_empty(&output);
}

// ─────────────────────────────── AC-N2 ──────────────────────────────────────

#[test]
fn diagnostic_overlay_on_gcode_source_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = tmp.path().join("nonexistent.gcode"); // never read: Phase 1 rejects first
    let output = tmp.path().join("bundle");

    let req = gcode_request(
        gcode_path,
        vec![LayerSelector::Index(0)],
        vec![VisualizationSpec::Name("diagnostic_overlay".to_string())],
    );

    let err = run_visual_debug(req, &output, false).expect_err(
        "diagnostic_overlay against a gcode source must be rejected as a source/visualization \
         mismatch before any render or bundle write, never silently dropped",
    );
    assert!(
        matches!(
            err,
            VisualDebugError::Validation(ValidationError::DiagnosticOverlayRequiresModelSource)
        ),
        "expected DiagnosticOverlayRequiresModelSource, got {err:?}"
    );
    assert_bundle_empty(&output);
}

// ─────────────────────────────── AC-N3 ──────────────────────────────────────

#[test]
fn anonymous_name_and_malformed_range_rejected() {
    // (a) `LayerSelector::Name` has no resolution target for the `Model`
    // source — `GlobalLayer` carries `index`/`z`/flags, no name — so it
    // must be rejected in `validate_request`'s Phase 1, before the model is
    // ever touched (the bogus paths below would otherwise surface as
    // `CaptureFailed`, proving this rejection happens earlier still).
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("bundle");
    let req = unreachable_model_request(vec![LayerSelector::Name("first_layer".to_string())]);

    let err = run_visual_debug(req, &output, false)
        .expect_err("LayerSelector::Name must be rejected: layers are anonymous");
    assert!(
        matches!(
            err,
            VisualDebugError::Validation(ValidationError::AnonymousLayerSelector)
        ),
        "expected AnonymousLayerSelector, got {err:?}"
    );
    assert_bundle_empty(&output);

    // (b) A malformed `{start, end}` range object carrying an extra,
    // unrecognized field must fail to deserialize outright — never
    // silently fall through to an empty `Detail { index: None, z: None }`
    // the way an untagged enum without per-variant field enforcement would.
    let malformed = serde_json::json!({"start": 2, "end": 5, "bogus_field": true});
    let result: Result<LayerSelector, _> = serde_json::from_value(malformed);
    assert!(
        result.is_err(),
        "a malformed {{start, end, bogus_field}} range must fail to deserialize, not silently \
         become an empty Detail; got {result:?}"
    );

    // A well-formed `{start, end}` range (no extra fields) must still
    // deserialize successfully as `Range`, confirming (b) is about field
    // enforcement, not about rejecting the shape entirely.
    let well_formed = serde_json::json!({"start": 2, "end": 5});
    let parsed: LayerSelector =
        serde_json::from_value(well_formed).expect("a well-formed {start, end} range must parse");
    assert!(
        matches!(parsed, LayerSelector::Range { start: 2, end: 5 }),
        "expected LayerSelector::Range {{ start: 2, end: 5 }}, got {parsed:?}"
    );
}

// ─────────────────────────────── AC-N4 ──────────────────────────────────────

#[test]
fn range_and_zonly_selectors_resolve_or_fail_closed() {
    let tmp = TempDir::new().expect("tempdir");

    // (a) A valid `{start, end}` range covering both layers, together with
    // a z-only `Detail` that also lands on the second layer, must each
    // resolve to a real layer against the parsed `;Z:` schedule.
    let ok_gcode_path = write_gcode(tmp.path(), "two_layer_ok.gcode", TWO_LAYER_GCODE);
    let ok_output = tmp.path().join("bundle_ok");
    let ok_req = gcode_request(
        ok_gcode_path,
        vec![
            LayerSelector::Range { start: 0, end: 1 },
            LayerSelector::Detail {
                index: None,
                z: Some(0.4),
            },
        ],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
    );

    let manifest_path = run_visual_debug(ok_req, &ok_output, false)
        .expect("a valid range and a z-only detail must resolve against the parsed ;Z: schedule");
    let manifest = manifest_at(&manifest_path);
    let images = manifest["images"].as_array().expect("images array");
    let resolved_indices: BTreeSet<i64> = images
        .iter()
        .map(|img| {
            img["layer_index"]
                .as_i64()
                .expect("layer_index is an integer")
        })
        .collect();
    assert_eq!(
        resolved_indices,
        BTreeSet::from([0, 1]),
        "range{{0,1}} union z-only detail(z=0.4) must resolve to exactly layers 0 and 1; got \
         {resolved_indices:?}"
    );

    // (b) A selector matching no real layer must fail closed before any
    // bundle write — even though the source itself is perfectly valid.
    let fail_gcode_path = write_gcode(tmp.path(), "two_layer_fail.gcode", TWO_LAYER_GCODE);
    let fail_output = tmp.path().join("bundle_fail");
    let fail_req = gcode_request(
        fail_gcode_path,
        vec![LayerSelector::Range {
            start: 100,
            end: 200,
        }],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
    );

    let err = run_visual_debug(fail_req, &fail_output, false)
        .expect_err("a range matching no scheduled layer must fail closed");
    assert!(
        matches!(
            err,
            VisualDebugError::Validation(ValidationError::LayerSelectorResolvesToNoLayer { .. })
        ),
        "expected LayerSelectorResolvesToNoLayer, got {err:?}"
    );
    assert_bundle_empty(&fail_output);
}

/// Sanity check that the single-layer fixture and helper still produce a
/// successful bundle on an ordinary `Index` selector, so the two-layer/
/// range/z-only fixtures above are read as *additions*, not a replacement
/// of already-working index-based selection.
#[test]
fn plain_index_selector_still_resolves() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode_path = write_gcode(tmp.path(), "single_layer.gcode", SINGLE_LAYER_GCODE);
    let output = tmp.path().join("bundle");
    let req = gcode_request(
        gcode_path,
        vec![LayerSelector::Index(0)],
        vec![VisualizationSpec::Name("filament_lines".to_string())],
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("a plain Index(0) selector must resolve");
    let manifest = manifest_at(&manifest_path);
    assert_eq!(
        manifest["images"].as_array().expect("images array").len(),
        1
    );
}
