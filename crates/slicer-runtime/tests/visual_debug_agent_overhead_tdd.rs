//! Packet 161, Step 10 — no-overhead closure gate for the visual-debug agent
//! (AC-7): an ordinary slice (visual debugging NOT requested) run through the
//! runtime's real entry point (`slicer_runtime::run_slice`) must never
//! allocate/serialize/render/invoke any visual-debug capture machinery, and
//! must never produce a visual-debug `manifest.json`/PNG bundle as a
//! byproduct.
//!
//! Two lines of evidence, mirroring `visual_debug_typed_tap_capture_tdd.rs`'s
//! (packet 158, `crates/pnp-cli/tests/`) own `ordinary_slice_has_no_tap_capture`
//! structural-guarantee pattern — read-only source inspection, no production
//! code touched:
//! - Structural: `run_slice` and `prepare_prepass_context`'s own function
//!   bodies (this crate's `src/run.rs`) never reference any visual-debug
//!   capture symbol (`execute_captured_stages`, `CaptureRequest`,
//!   `execute_blackboard_taps`, `execute_postpass_with_capture`, `CapturedIr`,
//!   `PostPassCapture`) — the capture path is reachable only through
//!   `pnp_cli::visual_debug::run_visual_debug`, never through the ordinary
//!   slice entry point.
//! - Behavioral: a repeated-run harness (`run_slice` invoked twice, into two
//!   independent clean output directories) still succeeds, produces non-empty
//!   G-code each time, and leaves each output directory containing ONLY the
//!   one G-code artifact this test itself writes — no `manifest.json`, no
//!   `*.png`, anywhere under the directory tree.
//!
//! Fixture: `resources/regression_wedge.stl` + `modules/core-modules`, the
//! same fixture `e2e/run_slice_api_tdd.rs` already uses to exercise
//! `run_slice` directly (the smallest existing `run_slice` fixture in this
//! crate) — no new geometry generator is authored here.

use std::fs;
use std::path::{Path, PathBuf};

use slicer_runtime::{run_slice, SliceRunOptions};

fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// Extract the body of `fn_marker` (e.g. `"pub fn run_slice("`) up to its
/// balanced closing brace, brace-matched from the first `{` after the
/// marker — mirrors `visual_debug_typed_tap_capture_tdd.rs`'s own
/// `function_body` helper (packet 158, `crates/pnp-cli/tests/`) exactly, so a
/// doc comment elsewhere in the file that merely *mentions* a forbidden
/// symbol (e.g. `PrepassContext`'s doc naming its own intended caller) can
/// never false-positive this check.
fn function_body<'a>(text: &'a str, fn_marker: &str) -> &'a str {
    let start = text
        .find(fn_marker)
        .unwrap_or_else(|| panic!("marker '{fn_marker}' not found in source"));
    let rest = &text[start..];
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

/// Every symbol that only exists to support visual-debug typed-tap capture
/// (arena taps: packet 158; Blackboard-read taps: packet 161 Steps 3-4;
/// whole-print PostPass taps: packet 161 Step 5). None of these may appear in
/// the ordinary slice entry point's own function body.
const FORBIDDEN_CAPTURE_SYMBOLS: &[&str] = &[
    "execute_captured_stages",
    "CaptureRequest",
    "execute_blackboard_taps",
    "execute_postpass_with_capture",
    "CapturedIr",
    "PostPassCapture",
];

#[test]
fn ordinary_slice_has_no_visual_debug_overhead() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // ---- Structural guarantee: the ordinary entry points never reference ----
    // any visual-debug capture symbol.
    let run_rs = fs::read_to_string(src_dir.join("run.rs")).expect("read src/run.rs");
    for marker in ["pub fn run_slice(", "pub fn prepare_prepass_context("] {
        let body = function_body(&run_rs, marker);
        for symbol in FORBIDDEN_CAPTURE_SYMBOLS {
            assert!(
                !body.contains(symbol),
                "{marker} must never reference visual-debug capture symbol '{symbol}'; \
                 the ordinary slice path must pay no capture allocation/serialization/ \
                 rendering/process-invocation cost"
            );
        }
    }

    let pipeline_rs =
        fs::read_to_string(src_dir.join("pipeline.rs")).expect("read src/pipeline.rs");
    for marker in [
        "pub fn run_pipeline(",
        "pub fn run_pipeline_with_events(",
        "pub fn run_pipeline_with_raw_config(",
        "pub fn run_pipeline_with_instrumentation(",
        "fn run_pipeline_core(",
    ] {
        let body = function_body(&pipeline_rs, marker);
        for symbol in FORBIDDEN_CAPTURE_SYMBOLS {
            assert!(
                !body.contains(symbol),
                "{marker} must never reference visual-debug capture symbol '{symbol}'"
            );
        }
    }

    let layer_executor_rs =
        fs::read_to_string(src_dir.join("layer_executor.rs")).expect("read src/layer_executor.rs");
    let single_layer_body = function_body(&layer_executor_rs, "fn execute_single_layer_inner(");
    assert!(
        !single_layer_body.contains("execute_captured_stages("),
        "execute_single_layer_inner (the ordinary per-layer entry point) must never \
         call execute_captured_stages"
    );

    let postpass_rs =
        fs::read_to_string(src_dir.join("postpass.rs")).expect("read src/postpass.rs");
    let execute_postpass_body = function_body(&postpass_rs, "pub fn execute_postpass(");
    assert!(
        !execute_postpass_body.contains("execute_postpass_with_capture")
            && !execute_postpass_body.contains("PostPassCapture"),
        "the ordinary postpass entry point (execute_postpass) must never route through \
         the capture-sink variant execute_postpass_with_capture/PostPassCapture"
    );

    // ---- Behavioral corroboration: a repeated-run harness of the real ----
    // runtime slice entry point still succeeds, and each run's clean output
    // directory ends up containing ONLY the one G-code artifact this test
    // itself writes — no manifest.json, no *.png, anywhere in the tree.
    let model = wedge_path();
    let modules = module_dir();
    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}",
        model.display()
    );
    assert!(
        modules.exists(),
        "core-modules must exist at {}",
        modules.display()
    );

    let mesh =
        std::sync::Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));

    for run_index in 0..2 {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let output_dir = tmp.path().join(format!("run-{run_index}"));
        fs::create_dir_all(&output_dir).expect("clean output dir");
        let gcode_path = output_dir.join("out.gcode");

        let opts = SliceRunOptions {
            mesh: std::sync::Arc::clone(&mesh),
            model_label: model.to_string_lossy().into_owned(),
            config_path: None,
            output_path: Some(gcode_path.clone()),
            module_dirs: vec![modules.clone()],
            no_default_module_paths: true,
            thumbnail: None,
            report: None,
            report_verbose: false,
            instrument_stderr: false,
            config_overrides: std::collections::HashMap::new(),
        };

        let outcome = run_slice(opts).unwrap_or_else(|e| {
            panic!("ordinary slice (run {run_index}) must succeed with no visual debugging requested: {e}")
        });
        assert!(
            !outcome.gcode_text.is_empty(),
            "ordinary slice (run {run_index}) must still produce non-empty G-code"
        );

        // `run_slice` itself never writes to `output_path` (that is the
        // caller's job — see `SliceRunOptions::output_path`'s doc comment);
        // this test writes it explicitly so the "opt-out" assertion below has
        // a well-defined single expected artifact to check against.
        fs::write(&gcode_path, &outcome.gcode_text).expect("write gcode artifact");

        assert_no_visual_debug_bundle_artifacts(&output_dir, run_index);
    }
}

/// Walk `dir` recursively and assert no `manifest.json` or `*.png` file
/// exists anywhere in the tree — the two artifact kinds a visual-debug
/// bundle (`pnp_cli::visual_debug::run_visual_debug`) writes and the
/// ordinary slice path must never produce.
fn assert_no_visual_debug_bundle_artifacts(dir: &Path, run_index: usize) {
    fn walk(dir: &Path, run_index: usize) {
        for entry in fs::read_dir(dir).expect("read output dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                walk(&path, run_index);
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            assert_ne!(
                file_name,
                "manifest.json",
                "ordinary slice (run {run_index}) must never produce a visual-debug \
                 manifest.json; found at {}",
                path.display()
            );
            assert!(
                !file_name.to_lowercase().ends_with(".png"),
                "ordinary slice (run {run_index}) must never produce a visual-debug PNG; \
                 found at {}",
                path.display()
            );
        }
    }
    walk(dir, run_index);
}
