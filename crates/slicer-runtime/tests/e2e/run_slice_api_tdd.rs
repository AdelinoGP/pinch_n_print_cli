//! AC-3: run_slice() returns Ok(SliceOutcome) with non-empty gcode_text against regression_wedge.stl.
//!
//! Also asserts that run.rs (the library entry point) no longer contains
//! the `_stale_build_plan` mod (formerly checked against the now-deleted
//! slicer-runtime main.rs; the guard now lives in the library).

use std::path::PathBuf;

use slicer_ir::ConfigValue;
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
fn run_slice_against_wedge_returns_nonempty_gcode() {
    let root = workspace_root();

    let model = root.join("resources").join("regression_wedge.stl");
    let module_dir = root.join("modules").join("core-modules");

    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}: run from the workspace root or ensure resources/ is present",
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
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        ..Default::default()
    };

    let outcome = run_slice(opts).expect("run_slice must succeed against wedge + core-modules");

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

/// Wayfinder ticket 91 / P84: the host-export key `gcode_add_line_number`
/// (canonical `coBool` default `0`) prefixes every exported line with `N<line> `
/// at the export seam. Default `false` leaves the artifact byte-identical;
/// `true` numbers the whole artifact contiguously from 1 without disturbing any
/// line's payload.
#[test]
fn gcode_add_line_number_prefixes_every_exported_line() {
    let root = workspace_root();

    let model = root.join("resources").join("regression_wedge.stl");
    let module_dir = root.join("modules").join("core-modules");

    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}",
        model.display()
    );
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh =
        std::sync::Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));

    let has_line_number = |line: &str| {
        line.strip_prefix('N')
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_digit())
    };

    // Default: the key is absent, so no line carries an `N<line> ` prefix.
    let baseline = run_slice(SliceRunOptions {
        mesh: std::sync::Arc::clone(&mesh),
        model_label: model.to_string_lossy().into_owned(),
        module_dirs: vec![module_dir.clone()],
        no_default_module_paths: true,
        ..Default::default()
    })
    .expect("baseline run_slice must succeed against wedge + core-modules");
    assert!(
        !baseline.gcode_text.lines().any(has_line_number),
        "default output must not carry line numbers"
    );

    // Non-default `true`: every line is `N<line> `, numbered from 1, with the
    // payload otherwise untouched.
    let config_overrides = std::collections::HashMap::from([(
        "gcode_add_line_number".to_string(),
        ConfigValue::Bool(true),
    )]);
    let numbered = run_slice(SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        config_overrides,
        ..Default::default()
    })
    .expect("numbered run_slice must succeed against wedge + core-modules");

    assert!(
        !baseline.gcode_text.is_empty(),
        "the fixture must produce G-code content for this test to prove anything"
    );

    // Strip the `N<line> ` prefix from every line. Numbering must be contiguous
    // from 1 and cover the whole artifact.
    let stripped: Vec<&str> = numbered
        .gcode_text
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let prefix = format!("N{} ", index + 1);
            line.strip_prefix(&prefix)
                .unwrap_or_else(|| panic!("line {} must start with {prefix:?}, got {line:?}", index + 1))
        })
        .collect();

    // The enabled key reports itself in the CONFIG_BLOCK: it routes through the
    // config resolver's `extensions` bucket, which `to_config_map` emits.
    let is_line_number_config_line = |payload: &str| {
        payload
            .trim_start_matches(';')
            .trim_start()
            .starts_with("gcode_add_line_number")
    };
    assert!(
        stripped
            .iter()
            .any(|payload| is_line_number_config_line(payload)),
        "the enabled key must report itself in the CONFIG_BLOCK"
    );

    // Everything ahead of the CONFIG_BLOCK — header, comments, and every
    // extrusion move — must be byte-identical once the prefixes are stripped.
    // The CONFIG_BLOCK itself is allowed to differ: it is a count-bounded table
    // (the ≥96-key OrcaSlicer floor), so a newly-present key displaces one
    // padding row rather than growing the block.
    let marker = "; CONFIG_BLOCK_START";
    let unprefixed = format!("{}\n", stripped.join("\n"));
    let head_of = |text: &str| {
        let at = text
            .find(marker)
            .expect("serialized output must carry a CONFIG_BLOCK");
        text[..at].to_string()
    };
    assert_eq!(
        head_of(&unprefixed),
        head_of(&baseline.gcode_text),
        "everything before the CONFIG_BLOCK must be unchanged apart from line-number prefixes"
    );
}
