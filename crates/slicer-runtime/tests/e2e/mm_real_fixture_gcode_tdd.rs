//! Packet 172 - real multi-material fixtures must emit the routed tools.
//!
//! The named `multi_tool_triangle.3mf` fixture now passes and is used directly;
//! `resources/cube_4color.3mf` remains a separate painted-fixture regression.

#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::ConfigValue;
use slicer_runtime::{run_slice, SliceRunOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn tool_lines(gcode: &str) -> Vec<u32> {
    gcode
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let digits = line.strip_prefix('T')?;
            let end = digits
                .bytes()
                .position(|byte| !byte.is_ascii_digit())
                .unwrap_or(digits.len());
            if end == 0 {
                return None;
            }
            digits[..end].parse().ok()
        })
        .collect()
}

fn assert_has_t0_t1(gcode: &str, fixture: &str) {
    let tools = tool_lines(gcode);
    let distinct: HashSet<_> = tools.iter().copied().collect();

    assert!(
        tools.len() >= 2,
        "{fixture} G-code must contain at least two tool-selection lines"
    );
    assert!(
        distinct.contains(&0),
        "{fixture} G-code must contain T0; found {distinct:?}"
    );
    assert!(
        distinct.contains(&1),
        "{fixture} G-code must contain T1; found {distinct:?}"
    );
}

fn slice_fixture(model: PathBuf, config_overrides: HashMap<String, ConfigValue>) -> String {
    let root = workspace_root();
    let module_dir = root.join("modules").join("core-modules");
    assert!(model.exists(), "fixture must exist: {}", model.display());
    assert!(
        module_dir.exists(),
        "core-modules dir must exist: {}",
        module_dir.display()
    );

    let mesh = Arc::new(slicer_model_io::load_model(&model).expect("model load"));
    let outcome = run_slice(SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        progress_events: false,
        cancel_flag: None,
        config_overrides,
    })
    .expect("run_slice must succeed");

    assert!(!outcome.gcode_text.is_empty(), "G-code must be non-empty");
    outcome.gcode_text
}

#[test]
fn mm_painted_fixture_t0_t1() {
    let model = workspace_root()
        .join("crates")
        .join("slicer-runtime")
        .join("tests")
        .join("fixtures")
        .join("perimeter_parity")
        .join("multi_tool_triangle")
        .join("multi_tool_triangle.3mf");
    let gcode = slice_fixture(model, HashMap::new());
    assert_has_t0_t1(&gcode, "painted cube fixture");
}

#[test]
fn mm_support_filament_real_fixture() {
    let model = workspace_root().join("resources/bridge_support_enforcers.3mf");
    let mut overrides = HashMap::new();
    overrides.insert("enable_support".to_string(), ConfigValue::Bool(true));
    // Raw config uses Orca's 1-indexed filament convention; run_slice rebases 2 to tool 1.
    overrides.insert("support_filament".to_string(), ConfigValue::Int(2));

    let gcode = slice_fixture(model, overrides);
    assert_has_t0_t1(&gcode, "support fixture");
}
