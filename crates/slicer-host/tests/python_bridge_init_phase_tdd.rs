#![allow(missing_docs)]

//! Gap-bridging tests for the Python bridge failure phases documented in
//! docs/05_module_sdk.md §"Python Bridge (TextPostProcess tier)":
//!
//!   "Failures surface as `PostpassError::FatalModule` wrapping
//!    `PythonBridgeError { phase, message }` with phases `MissingScript`,
//!    `ConfigEncoding`, `Init`, `ScriptError`, `OutputEncoding`."
//!
//! The existing `python_bridge_tdd.rs` covers:
//!   - MissingScript  (nonexistent script path)
//!   - ScriptError    (entry raises Python exception)
//!   - OutputEncoding (entry returns non-str)
//!
//! The `Init` phase — raised whenever the interpreter can instantiate but
//! the user script cannot be loaded or the declared entry cannot be
//! resolved — has no coverage. These tests close that gap so that any
//! regression that silently reclassifies such failures (for example as
//! `ScriptError` or `MissingScript`) is caught.

use std::collections::HashMap;
use std::io::Write;

use slicer_host::{PythonBinding, PythonBridge, PythonBridgePhase};
use slicer_ir::ConfigView;

fn write_file(path: &std::path::Path, body: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

fn empty_config() -> ConfigView {
    ConfigView::from_map(HashMap::new())
}

// ──────────────────────────────────────────────────────────────────────────
// Test 1 — entry function missing from an otherwise valid script
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn python_bridge_missing_entry_function_reports_init_phase() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("no_entry.py");
    // Valid module, but `process_gcode` is never defined.
    write_file(
        &script,
        r#"
def other_name(text, config):
    return text
"#,
    );

    let bridge = PythonBridge::default();
    let binding = PythonBinding {
        script_path: script,
        entry: "process_gcode".to_string(),
    };
    let err = bridge
        .run_text(
            &binding,
            &empty_config(),
            "G28\n",
            &"com.example.py-no-entry".to_string(),
            &"PostPass::TextPostProcess".to_string(),
        )
        .expect_err("must fail when the declared entry function is missing");

    assert_eq!(
        err.phase,
        PythonBridgePhase::Init,
        "docs/05 §'Python Bridge' requires entry-resolution failures to be \
         classified as `Init`, not `ScriptError` or `MissingScript`. Got \
         phase={:?}, message={}",
        err.phase,
        err.message
    );
    assert!(
        err.message.contains("process_gcode") || err.message.contains("entry"),
        "Init diagnostic must mention the missing entry name or field for \
         actionable debugging; got: {}",
        err.message
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 2 — script with import-time syntax error is a ScriptError, not Init
// ──────────────────────────────────────────────────────────────────────────
//
// Counter-example pinning the boundary between phases: a script that fails
// to execute at import time (syntax error, raised exception during top
// level) is a `ScriptError` — *not* `Init`. The Init phase is reserved for
// failures the user cannot directly attribute to their script's own code.

#[test]
fn python_bridge_syntax_error_in_user_script_reports_script_error_phase() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("broken.py");
    // `def` without function body is a SyntaxError at exec_module time.
    write_file(&script, "def process_gcode(text, config:\n");

    let bridge = PythonBridge::default();
    let binding = PythonBinding {
        script_path: script,
        entry: "process_gcode".to_string(),
    };
    let err = bridge
        .run_text(
            &binding,
            &empty_config(),
            "G28\n",
            &"com.example.py-syntax".to_string(),
            &"PostPass::TextPostProcess".to_string(),
        )
        .expect_err("must fail when the user script has a syntax error");

    assert_eq!(
        err.phase,
        PythonBridgePhase::ScriptError,
        "user-source syntax errors must be classified as ScriptError so the \
         operator knows to look at their own .py file. Got phase={:?}, \
         message={}",
        err.phase,
        err.message
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 3 — diagnostics always carry module_id + stage_id for routing
// ──────────────────────────────────────────────────────────────────────────
//
// docs/05 implies every Python bridge failure is structured enough to be
// routed back to the owning module in the progress-event stream. Lock that
// contract down so a future refactor cannot drop the fields.

#[test]
fn python_bridge_error_diagnostics_preserve_module_and_stage_identifiers() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("no_entry.py");
    write_file(&script, "pass\n");

    let bridge = PythonBridge::default();
    let binding = PythonBinding {
        script_path: script,
        entry: "process_gcode".to_string(),
    };
    let module_id = "com.example.py-id-check".to_string();
    let stage_id = "PostPass::TextPostProcess".to_string();

    let err = bridge
        .run_text(&binding, &empty_config(), "", &module_id, &stage_id)
        .expect_err("entry missing from empty script");

    assert_eq!(
        err.module_id, module_id,
        "module_id must round-trip for routing"
    );
    assert_eq!(
        err.stage_id, stage_id,
        "stage_id must round-trip for routing"
    );
    assert_eq!(err.phase, PythonBridgePhase::Init);
}
