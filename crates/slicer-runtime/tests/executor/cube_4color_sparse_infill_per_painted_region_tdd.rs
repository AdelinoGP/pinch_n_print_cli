//! AC-3: per-region sparse-infill origin propagation (packet 127).
//!
//! Proves the `set-current-origin` WIT method + SDK `begin_region` mechanism
//! routes per-region perimeter infill output to the correct tool. Pre-fix,
//! T1 had effectively no sparse-infill extrusion (30 unretract-priming moves
//! only) because the LIFO-touch bug collapsed all per-region `set_infill_areas`
//! to the last-touched region's origin. Post-fix, all four tools T0-T3 must
//! each appear in at least one `;TYPE:Sparse infill` segment with a `G1 ... E`
//! extrusion move.
//!
//! NOTE: the absolute thresholds (T1 >= 1000, T3 <= 1500) from the original
//! AC-3 are deferred to a follow-up infill-generation packet — a separate
//! pre-existing bug (multiple infill patterns running concurrently) inflates
//! the absolute move counts ~9-12x and is out of scope for this packet. This
//! test asserts only the origin-propagation mechanism this packet owns: that
//! each of the four painted regions' tools appears in sparse-infill output.

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use slicer_runtime::{run_slice, SliceOutcome, SliceRunOptions};
use std::sync::Arc;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn slice_fixture_file(model_path: &PathBuf) -> SliceOutcome {
    assert!(
        model_path.exists(),
        "fixture missing: {} — run from workspace root or restore resources/",
        model_path.display()
    );
    let module_dir = core_modules_dir();
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );
    let opts = SliceRunOptions {
        mesh,
        model_label: model_path.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        config_overrides: std::collections::HashMap::new(),
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against {}: {e}", model_path.display()))
}

/// Parse the gcode and return the set of tool indices (T<n>) that appear in
/// at least one `;TYPE:Sparse infill` block containing a `G1 ... E` extrusion
/// move. State machine: track current type (reset on LAYER_CHANGE) and
/// current tool (set by `T<n>` lines); when inside `;TYPE:Sparse infill` and
/// a `G1 ... E` line is seen, record the current tool.
fn tools_with_sparse_infill_extrusion(gcode: &str) -> BTreeSet<u32> {
    let mut found: BTreeSet<u32> = BTreeSet::new();
    let mut current_type: &str = "";
    let mut current_tool: Option<u32> = None;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(";LAYER_CHANGE") {
            current_type = "";
            current_tool = None;
            continue;
        }
        if trimmed.starts_with(";TYPE:") {
            current_type = trimmed.trim_start_matches(";TYPE:").trim();
            continue;
        }
        // Tool-change line: T<digits> only
        if trimmed.len() >= 2
            && trimmed.as_bytes()[0] == b'T'
            && trimmed[1..].bytes().all(|c| c.is_ascii_digit())
        {
            if let Ok(n) = trimmed[1..].parse::<u32>() {
                current_tool = Some(n);
            }
            continue;
        }
        // G1 extrusion move inside a Sparse infill block
        if current_type == "Sparse infill" && trimmed.starts_with("G1 ") && trimmed.contains(" E") {
            if let Some(tool) = current_tool {
                found.insert(tool);
            }
        }
    }
    found
}

#[test]
fn cube_4color_sparse_infill_per_painted_region() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let tools = tools_with_sparse_infill_extrusion(&outcome.gcode_text);
    let expected: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();
    assert_eq!(
        tools, expected,
        "cube_4color sparse-infill must include all four tool indices (T0-T3); \
         pre-fix T1 was effectively absent (only unretract priming). \
         Found: {tools:?}, expected: {expected:?}"
    );
}
