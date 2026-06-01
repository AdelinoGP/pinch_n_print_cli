//! AC-3: run_slice() returns Ok(SliceOutcome) with non-empty gcode_text against benchy.stl.
//!
//! Also asserts that run.rs (the library entry point) no longer contains
//! the `_stale_build_plan` mod (formerly checked against the now-deleted
//! slicer-runtime main.rs; the guard now lives in the library).

use std::path::PathBuf;

use slicer_runtime::{run_slice, SliceRunOptions};

/// Resolve the workspace root from CARGO_MANIFEST_DIR (crates/slicer-runtime)
/// by walking two levels up.
fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

#[test]
fn run_slice_against_benchy_returns_nonempty_gcode() {
    let root = workspace_root();

    let model = root.join("resources").join("benchy.stl");
    let module_dir = root.join("modules").join("core-modules");

    assert!(
        model.exists(),
        "benchy.stl must exist at {}: run from the workspace root or ensure resources/ is present",
        model.display()
    );
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh =
        std::sync::Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));
    let opts = SliceRunOptions {
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
    };

    let outcome = run_slice(opts).expect("run_slice must succeed against benchy + core-modules");

    assert!(
        !outcome.gcode_text.is_empty(),
        "gcode_text must not be empty"
    );

    // AC-3 second clause: run.rs must not contain _stale_build_plan.
    // After the pnp-cli-unification refactor, the binary's main.rs was
    // deleted; the library entry point is now slicer-runtime/src/run.rs.
    let run_rs_path = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/run.rs";
    let run_rs = std::fs::read_to_string(&run_rs_path).expect("should be able to read src/run.rs");
    assert!(
        !run_rs.contains("_stale_build_plan"),
        "run.rs must not contain _stale_build_plan"
    );
}
