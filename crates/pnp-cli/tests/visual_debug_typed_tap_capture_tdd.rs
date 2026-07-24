//! Packet 158 — request-gated, typed post-stage capture at the executor
//! boundary. Exercises `pnp_cli::visual_debug::run_visual_debug`'s
//! `Model`-source path directly (as a library call, not a subprocess) so
//! error variants can be matched by type rather than parsed from stderr
//! text.
//!
//! Real model + real modules: `resources/regression_wedge.stl` sliced
//! against the full `modules/core-modules` set, with `layer_height: 1.0`
//! (the schema max) to bound the model to ~40 layers and keep the suite
//! fast. The dependency-closure executor runs the truncated per-layer stage
//! sequence only for the request's selected layers (follow-up fix:
//! layer-skip) — no tap in this packet's scope has a cross-layer
//! correctness dependency (docs/01_system_architecture.md "Tier 2 —
//! Per-Layer") — but the model/module load and slice PrePass are still
//! whole-model work, so the fixture stays bounded regardless.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugError, VisualDebugRequest,
    VisualDebugSource,
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

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// A config file with `layer_height` at the schema max (1.0mm) so the
/// ~40mm-tall regression_wedge fixture bounds to ~40 layers instead of the
/// ~200 layers the 0.2mm default would produce — the closure executor
/// always runs every layer (AC-3), so this materially bounds test runtime.
fn write_bounded_config(dir: &Path) -> PathBuf {
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height": 1.0}"#).expect("write bounded config");
    path
}

fn write_bounded_support_config(dir: &Path) -> PathBuf {
    let path = dir.join("support-config.json");
    fs::write(
        &path,
        br#"{"layer_height":1.0,"enable_support":true,"support_filament":2,"support_interface_filament":3}"#,
    )
    .expect("write bounded support config");
    path
}

fn tap(name: &str) -> TapSelector {
    TapSelector::Name(name.to_string())
}

fn model_request(taps: Vec<&str>, layers: Vec<i64>, config: PathBuf) -> VisualDebugRequest {
    model_request_for_model(wedge_path(), taps, layers, config)
}

fn model_request_for_model(
    model: PathBuf,
    taps: Vec<&str>,
    layers: Vec<i64>,
    config: PathBuf,
) -> VisualDebugRequest {
    model_request_for_model_with_selectors(
        model,
        taps,
        layers.into_iter().map(LayerSelector::Index).collect(),
        config,
    )
}

fn model_request_for_model_with_selectors(
    model: PathBuf,
    taps: Vec<&str>,
    layers: Vec<LayerSelector>,
    config: PathBuf,
) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(model),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers,
        taps: taps.into_iter().map(tap).collect(),
        visualizations: Vec::new(),
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

/// A request whose `model`/`config` paths do not exist on disk — used by
/// the negative tests that must reject before ever touching the
/// filesystem for the model or config (AC-N2, AC-N4: "pipeline closure not
/// executed").
fn unreachable_model_request(taps: Vec<&str>, layers: Vec<i64>) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(PathBuf::from("/definitely/does/not/exist/model.stl")),
            config: Some(PathBuf::from("/definitely/does/not/exist/config.json")),
            module_dirs: vec![PathBuf::from("/definitely/does/not/exist/modules")],
            path: None,
        },
        layers: layers.into_iter().map(LayerSelector::Index).collect(),
        taps: taps.into_iter().map(tap).collect(),
        visualizations: Vec::new(),
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

/// STAGE_ORDER's `Layer::*` slice, mirrored here only to assert monotonic
/// ordering of `executed_stage_ids` without depending on slicer-runtime's
/// (crate-internal) constant.
const LAYER_STAGE_ORDER: &[&str] = &[
    "Layer::PaintRegionAnnotation",
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::PathOptimization",
];

fn stage_position(stage_id: &str) -> usize {
    LAYER_STAGE_ORDER
        .iter()
        .position(|s| *s == stage_id)
        .unwrap_or_else(|| panic!("unknown per-layer stage id in test fixture: {stage_id}"))
}

// ─────────────────────────────── AC-1 ──────────────────────────────────────

#[test]
fn typed_tap_capture_records_selected_layer() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let req = model_request(vec!["Layer::Perimeters"], vec![0], config);

    let manifest_path =
        run_visual_debug(req, &output, false).expect("typed tap capture should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "expected exactly one typed capture entry, got {images:#?}"
    );
    let entry = &images[0];
    assert_eq!(entry["tap"], "Layer::Perimeters", "tap identity preserved");
    assert_eq!(entry["layer_index"], 0);
    assert_eq!(entry["source"], "model");
    assert_eq!(
        entry["png_path"], "",
        "no PNG required/produced by typed capture"
    );
    let payload = &entry["typed_capture"];
    assert!(
        !payload.is_null(),
        "typed capture payload must be non-empty, got null"
    );
    assert_eq!(
        payload["kind"], "Perimeter",
        "typed capture payload must carry the captured IR's own identity"
    );
    assert!(
        payload["value"].is_object(),
        "typed capture payload must carry the captured PerimeterIR value"
    );
}

#[test]
fn visual_debug_forwards_support_tool_selection() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_support_config(tmp.path());
    let output = tmp.path().join("bundle");
    let model = workspace_root()
        .join("resources")
        .join("bridge_support_enforcers.3mf");

    let req = model_request_for_model_with_selectors(
        model,
        vec!["Layer::PathOptimization"],
        vec![LayerSelector::Range { start: 0, end: 100 }],
        config,
    );

    let manifest = manifest_at(
        &run_visual_debug(req, &output, false)
            .expect("visual-debug support capture should succeed"),
    );
    let images = manifest["images"].as_array().expect("images array");
    let support_entities: Vec<&Value> = images
        .iter()
        .filter(|image| image["tap"] == "Layer::PathOptimization")
        .flat_map(|image| image["typed_capture"]["value"]["ordered_entities"].as_array())
        .flatten()
        .filter(|entity| {
            matches!(
                entity["role"].as_str(),
                Some("SupportMaterial") | Some("SupportInterface")
            )
        })
        .collect();
    assert!(
        support_entities
            .iter()
            .any(|entity| entity["role"] == "SupportMaterial" && entity["tool_index"] == 1),
        "support entities must use raw support_filament=2 rebased to tool 1; captured={support_entities:#?}"
    );
    // This real fixture emits support material but no interface entities. The
    // shared parser and synthetic entity test cover the interface selector.
}

// ─────────────────────────────── AC-2 ──────────────────────────────────────

#[test]
fn dependency_closure_stops_at_furthest_tap() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    // Two taps at different scheduler stages; "Layer::Infill" is the
    // furthest of the two in fixed stage order.
    let req = model_request(vec!["Layer::Perimeters", "Layer::Infill"], vec![0], config);

    let manifest_path =
        run_visual_debug(req, &output, false).expect("two-tap closure should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "one capture per requested tap");
    let taps: Vec<&str> = images
        .iter()
        .map(|e| e["tap"].as_str().expect("tap is a string"))
        .collect();
    assert!(taps.contains(&"Layer::Perimeters"));
    assert!(taps.contains(&"Layer::Infill"));

    let executed: Vec<String> = manifest["executed_stage_ids"]
        .as_array()
        .expect("executed_stage_ids array")
        .iter()
        .map(|v| v.as_str().expect("stage id is a string").to_string())
        .collect();
    assert!(
        !executed.is_empty(),
        "the closure must have executed at least the requested stages"
    );

    // Every prerequisite stage runs in fixed order through the furthest
    // selected tap ("Layer::Infill")...
    assert_eq!(
        executed.last().map(String::as_str),
        Some("Layer::Infill"),
        "furthest requested tap must be the last stage the closure ran; got {executed:?}"
    );
    assert!(
        executed.contains(&"Layer::Perimeters".to_string()),
        "the earlier requested tap must also be in the closure; got {executed:?}"
    );
    // ...and fixed order is monotonic in STAGE_ORDER position.
    let positions: Vec<usize> = executed.iter().map(|s| stage_position(s)).collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "closure stages must run in strictly increasing fixed scheduler order; got {executed:?}"
    );

    // ...and no stage after the furthest tap executes.
    for later in [
        "Layer::Support",
        "Layer::SupportPostProcess",
        "Layer::PathOptimization",
    ] {
        assert!(
            !executed.contains(&later.to_string()),
            "stage '{later}' is strictly after the furthest requested tap and must not run; \
             executed = {executed:?}"
        );
    }
}

// ─────────────────────────────── AC-3 ──────────────────────────────────────

/// Follow-up fix (session reopen): a request selecting a strict subset of
/// the ~40-layer wedge's layers must retain captures ONLY for the selected
/// layers, AND the closure must never have executed the truncated per-layer
/// stage sequence for any other layer. `Layer::Perimeters` through
/// `Layer::PathOptimization` (this packet's tap scope) have no cross-layer
/// correctness dependency (docs/01_system_architecture.md "Tier 2 —
/// Per-Layer": "Each layer runs independently. Layers share no mutable
/// state."), so a non-selected layer is never required for correctness and
/// must not appear in `layer_expansions` (that field is reserved for a
/// genuine future correctness dependency, which does not exist for any tap
/// in this packet's scope today) NOR in `executed_layer_indices`.
#[test]
fn selected_layers_bound_capture_retention() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let selected = [0i64, 5i64];
    let req = model_request(vec!["Layer::Perimeters"], selected.to_vec(), config);

    let manifest_path = run_visual_debug(req, &output, false).expect("subset-layer capture");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        selected.len(),
        "manifest must record only the requested layer captures; got {images:#?}"
    );
    let captured_layers: Vec<i64> = images
        .iter()
        .map(|e| e["layer_index"].as_i64().expect("layer_index is i64"))
        .collect();
    for l in selected {
        assert!(
            captured_layers.contains(&l),
            "expected layer {l} to be captured; got {captured_layers:?}"
        );
    }

    // No cross-layer dependency exists for any tap in this closure's scope,
    // so the closure must never have expanded into an unselected layer.
    let expansions = manifest["layer_expansions"]
        .as_array()
        .expect("layer_expansions array");
    assert!(
        expansions.is_empty(),
        "no tap in this closure's scope has a genuine cross-layer correctness \
         dependency, so the closure must never execute (and record an \
         expansion for) a layer outside the request's selection; got {expansions:#?}"
    );

    // Direct proof the closure did not merely skip retention but skipped
    // EXECUTION: the executed-layer set the runtime actually ran the
    // truncated stage sequence for must be exactly the 2 selected layers,
    // not all ~40 layers in the wedge.
    let executed_layers: Vec<i64> = manifest["executed_layer_indices"]
        .as_array()
        .expect("executed_layer_indices array")
        .iter()
        .map(|v| v.as_i64().expect("layer index is i64"))
        .collect();
    assert_eq!(
        {
            let mut sorted = executed_layers.clone();
            sorted.sort_unstable();
            sorted
        },
        {
            let mut sorted = selected.to_vec();
            sorted.sort_unstable();
            sorted
        },
        "the closure must execute the truncated stage sequence for exactly the \
         selected layers and no others; got executed_layer_indices={executed_layers:?}"
    );
}

// ─────────────────────────────── AC-4 ──────────────────────────────────────

#[test]
fn typed_capture_is_deterministic() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output_a = tmp.path().join("bundle-a");
    let output_b = tmp.path().join("bundle-b");

    let req_a = model_request(
        vec!["Layer::Perimeters", "Layer::Infill"],
        vec![0, 3],
        config.clone(),
    );
    let req_b = model_request(
        vec!["Layer::Perimeters", "Layer::Infill"],
        vec![0, 3],
        config,
    );

    let manifest_a_path = run_visual_debug(req_a, &output_a, false).expect("first run");
    let manifest_b_path = run_visual_debug(req_b, &output_b, false).expect("second run");

    let manifest_a = manifest_at(&manifest_a_path);
    let manifest_b = manifest_at(&manifest_b_path);

    assert_eq!(
        manifest_a["executed_stage_ids"], manifest_b["executed_stage_ids"],
        "closure stage order must be identical across runs"
    );
    assert_eq!(
        manifest_a["images"], manifest_b["images"],
        "tap identity, layer indices, schema versions, and serialized payload \
         ordering must be identical across runs"
    );
    // Sanity: the deterministic payload is non-trivial (both taps × both layers).
    assert_eq!(manifest_a["images"].as_array().unwrap().len(), 4);
}

// ─────────────────────────────── AC-N1 ─────────────────────────────────────

/// Extract the body of `fn_marker` (e.g. `"pub fn run_slice("`) up to the
/// next top-level `fn ` — a best-effort single-function slice used to prove
/// an ordinary pipeline entry point does not call into the typed-tap-capture
/// API, without false-positiving on doc comments elsewhere in the file
/// (e.g. `PrepassContext`'s doc comment legitimately names
/// `execute_captured_stages` as the intended caller of the context it
/// builds).
fn function_body<'a>(text: &'a str, fn_marker: &str) -> &'a str {
    let start = text
        .find(fn_marker)
        .unwrap_or_else(|| panic!("marker '{fn_marker}' not found in source"));
    let rest = &text[start..];
    // Brace-match from the function signature's opening `{` to its closing
    // `}` — robust against whatever doc comments / items follow (unlike a
    // "next `pub fn`" heuristic, which false-positives when a doc comment
    // between this function and the next item mentions a forbidden symbol,
    // e.g. `PrepassContext`'s doc naming `execute_captured_stages` as its
    // intended caller).
    let open = rest
        .find('{')
        .unwrap_or_else(|| panic!("no function body found for marker '{fn_marker}'"));
    let mut depth = 0i32;
    for (i, ch) in rest[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[..open + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces while scanning function body for marker '{fn_marker}'");
}

#[test]
fn ordinary_slice_has_no_tap_capture() {
    // Structural guarantee: ordinary slicing's real entry points
    // (`run_slice`, and every `pipeline::run_pipeline*` orchestrator) never
    // call the typed-tap-capture API, so ordinary `pnp_cli slice` pays no
    // capture allocation/serialization/registration cost and produces no
    // visual-debug manifest — the capture path is reachable only through
    // `pnp_cli::visual_debug::run_visual_debug`'s `Model` source (which
    // calls the *separate* `prepare_prepass_context` + `execute_captured_stages`
    // pair, never touched by `run_slice`/`run_pipeline*`).
    let runtime_src = workspace_root()
        .join("crates")
        .join("slicer-runtime")
        .join("src");
    let forbidden = ["execute_captured_stages", "CaptureRequest"];

    let run_rs = fs::read_to_string(runtime_src.join("run.rs")).expect("read run.rs");
    let run_slice_body = function_body(&run_rs, "pub fn run_slice(");
    for symbol in forbidden {
        assert!(
            !run_slice_body.contains(symbol),
            "run_slice must never reference typed-tap-capture symbol '{symbol}'"
        );
    }

    let pipeline_rs =
        fs::read_to_string(runtime_src.join("pipeline.rs")).expect("read pipeline.rs");
    for marker in [
        "pub fn run_pipeline(",
        "pub fn run_pipeline_with_events(",
        "pub fn run_pipeline_with_raw_config(",
        "pub fn run_pipeline_with_instrumentation(",
        "fn run_pipeline_core(",
    ] {
        let body = function_body(&pipeline_rs, marker);
        for symbol in forbidden {
            assert!(
                !body.contains(symbol),
                "{marker} must never reference typed-tap-capture symbol '{symbol}'"
            );
        }
    }

    let layer_executor_rs =
        fs::read_to_string(runtime_src.join("layer_executor.rs")).expect("read layer_executor.rs");
    let single_layer_body = function_body(&layer_executor_rs, "fn execute_single_layer_inner(");
    assert!(
        !single_layer_body.contains("execute_captured_stages("),
        "execute_single_layer_inner (the ordinary per-layer entry point) \
         must never call execute_captured_stages"
    );

    // Behavioral corroboration: an ordinary slice of the same fixture still
    // succeeds and produces a real, non-empty G-code artifact — the
    // structural guarantee above did not come at the cost of breaking
    // ordinary slicing.
    let tmp = TempDir::new().expect("tempdir");
    let gcode = tmp.path().join("out.gcode");
    let output = assert_cmd::Command::cargo_bin("pnp_cli")
        .expect("pnp_cli binary")
        .arg("slice")
        .arg("--model")
        .arg(wedge_path())
        .arg("--module-dir")
        .arg(module_dir())
        .arg("--no-default-module-paths")
        .arg("--output")
        .arg(&gcode)
        .output()
        .expect("spawn pnp_cli slice");
    assert!(
        output.status.success(),
        "ordinary slice must still succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(&gcode).expect("gcode file must exist");
    assert!(!text.is_empty(), "ordinary slice must still produce G-code");
}

// ─────────────────────────────── AC-N2 ─────────────────────────────────────

#[test]
fn unknown_tap_is_rejected_without_success() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("bundle");

    // Model/config/module_dirs point at paths that do not exist — tap
    // validation must reject before any of them are touched.
    let req = unreachable_model_request(vec!["totally_bogus_tap"], vec![0]);

    let err =
        run_visual_debug(req, &output, false).expect_err("an unsupported tap must not succeed");
    match err {
        VisualDebugError::UnsupportedTap(tap) => {
            assert_eq!(
                tap, "totally_bogus_tap",
                "error must name the offending tap"
            )
        }
        other => panic!("expected VisualDebugError::UnsupportedTap, got {other:?}"),
    }
    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written on validation failure"
    );
}

// ─────────────────────────────── AC-N3 ─────────────────────────────────────

#[test]
fn unavailable_tap_source_fails_without_partial_success() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    // All supported arena stages are bound in the current core-module set.
    // With enable_support=false (the bounded fixture's default), Layer::Support
    // has no committed SupportIR, so its source remains unavailable.
    let req = model_request(vec!["Layer::Support"], vec![0], config);

    let err = run_visual_debug(req, &output, false)
        .expect_err("a tap whose source is never committed must not succeed");
    match err {
        VisualDebugError::CaptureFailed(message) => {
            assert!(
                message.contains("Layer::Support"),
                "error must identify the unavailable tap; got: {message}"
            );
            assert!(
                message.to_lowercase().contains("unavailable"),
                "error must describe the source as unavailable; got: {message}"
            );
        }
        other => panic!("expected VisualDebugError::CaptureFailed, got {other:?}"),
    }
    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when a tap source is unavailable"
    );
}

// ─────────────────────────────── AC-N4 ─────────────────────────────────────

#[test]
fn tap_without_applicable_layer_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("bundle");

    // No layers selected at all, and model/config/module_dirs point at
    // paths that do not exist — layer-applicability validation must reject
    // before the pipeline closure (or even the model load) ever runs.
    let req = unreachable_model_request(vec!["Layer::Perimeters"], vec![]);

    let err = run_visual_debug(req, &output, false)
        .expect_err("a tap with no applicable layer must not succeed");
    assert!(
        matches!(err, VisualDebugError::NoApplicableLayer),
        "expected VisualDebugError::NoApplicableLayer, got {err:?}"
    );
    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when no layer applies"
    );
}

// ─────────────────────────── regression (Fix 1) ─────────────────────────────

/// Regression for packet 158 review finding: `run_visual_debug` used to wipe
/// an existing `--overwrite`d `output_dir` before tap/layer validation ran,
/// so a rejected request (e.g. an unknown tap) silently deleted an existing
/// bundle's manifest and left the directory empty instead of failing without
/// touching it. Reproduced by pre-populating `bundle/manifest.json`, then
/// requesting `--overwrite` with an unknown tap: the old manifest must
/// survive (or, equivalently, the directory must never end up empty).
#[test]
fn overwrite_with_unsupported_tap_preserves_existing_bundle() {
    let tmp = TempDir::new().expect("tempdir");
    let output = tmp.path().join("bundle");
    fs::create_dir(&output).expect("pre-existing bundle dir");
    let sentinel = br#"{"test_marker":"preserve-me"}"#;
    fs::write(output.join("manifest.json"), sentinel).expect("pre-existing manifest");

    // Model/config/module_dirs point at paths that do not exist — the tap
    // rejection must happen before any of them are touched, and before the
    // pre-existing bundle is wiped.
    let req = unreachable_model_request(vec!["totally_bogus_tap"], vec![0]);

    let err = run_visual_debug(req, &output, true)
        .expect_err("an unsupported tap must not succeed even with --overwrite");
    assert!(
        matches!(err, VisualDebugError::UnsupportedTap(_)),
        "expected VisualDebugError::UnsupportedTap, got {err:?}"
    );

    let manifest_path = output.join("manifest.json");
    assert!(
        manifest_path.exists(),
        "the pre-existing bundle's manifest.json must survive a rejected \
         --overwrite request; the directory must never be left empty"
    );
    assert_eq!(
        fs::read(&manifest_path).expect("read manifest.json"),
        sentinel,
        "the pre-existing manifest content must be untouched, not partially \
         replaced or truncated"
    );
}
