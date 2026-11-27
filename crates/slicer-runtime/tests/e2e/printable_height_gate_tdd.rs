//! Wayfinder map ticket 26 (P19): the `printable_height` build-volume gate is
//! live in the real slice path.
//!
//! `validate_printable_height` (`crates/slicer-model-io/src/loader.rs`) is unit
//! tested in `crates/slicer-model-io/tests/printable_height_tdd.rs`. These tests
//! prove the *wiring*: that `run_slice` reads the resolved `printable_height`
//! and rejects an object taller than it, and that the default value does not
//! reject a fixture that slices today.
//!
//! Canonical rejects the same condition in `Print::validate` (`Print.cpp`).
//!
//! Verification: `cargo test -p slicer-runtime --test e2e printable_height`

use std::io::Write;
use std::path::PathBuf;

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

fn wedge_options(root: &PathBuf, config_path: Option<PathBuf>) -> SliceRunOptions {
    let model = root.join("resources").join("regression_wedge.stl");
    let mesh =
        std::sync::Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));
    SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        module_dirs: vec![root.join("modules").join("core-modules")],
        no_default_module_paths: true,
        config_path,
        ..Default::default()
    }
}

/// The 250.0 mm default must not reject a fixture that slices today. This is the
/// regression guard on the deviation from canonical's 100.0 mm default: had the
/// port adopted 100.0, this fixture's slice outcome would depend on its height.
#[test]
fn default_printable_height_does_not_reject_the_wedge() {
    let root = workspace_root();
    let outcome = run_slice(wedge_options(&root, None))
        .expect("the wedge must still slice under the default printable_height");
    assert!(
        !outcome.gcode_text.is_empty(),
        "gcode_text must not be empty"
    );
}

/// A `printable_height` below the object's world-space Z maximum must fail the
/// slice with the stable `EXCEEDS_PRINTABLE_HEIGHT` code — the behaviour change
/// at a non-default value that makes this key live rather than declared.
#[test]
fn printable_height_below_the_object_rejects_the_slice() {
    let root = workspace_root();

    let dir = tempfile::tempdir().expect("tempdir must be creatable");
    let config_path = dir.path().join("low_printable_height.json");
    let mut f = std::fs::File::create(&config_path).expect("config file must be creatable");
    // 1.0 mm is below any real fixture's height, so this asserts the gate fires
    // rather than asserting a particular fixture dimension.
    write!(f, r#"{{ "printable_height": 1.0 }}"#).expect("config must be writable");
    drop(f);

    let err = run_slice(wedge_options(&root, Some(config_path)))
        .expect_err("a 1.0mm build volume must reject the wedge");
    let msg = err.to_string();
    assert!(
        msg.contains("EXCEEDS_PRINTABLE_HEIGHT"),
        "error must carry the stable code, got: {msg}"
    );
}
