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

/// Wayfinder ticket 92 / P85: the host-export key `post_process` (canonical
/// `coStrings` default `{}`) runs the configured commands against the exported
/// artifact. Each command is handed the `<output>.pp` working copy — canonical
/// `PostProcessor.cpp::run_post_process_scripts`'s File-host name — and the
/// rewritten text is what both export paths carry. Canonical applies them
/// before `gcode_add_line_number`, so a line a script appends is numbered too.
#[test]
fn post_process_scripts_rewrite_the_exported_artifact() {
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
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("post_process.gcode");

    // A command that appends `marker` to the path the runner appends as its
    // final argument, in each platform shell's spelling.
    #[cfg(windows)]
    let appending_script = |marker: &str| format!("echo {marker}>>");
    #[cfg(not(windows))]
    let appending_script = |marker: &str| format!("printf '{marker}\\n' >>");

    // Baseline: the key is absent, so the artifact carries no marker and no
    // working copy is created. `run_slice` still does not write `output` —
    // that stays the caller's job.
    let baseline = run_slice(SliceRunOptions {
        mesh: std::sync::Arc::clone(&mesh),
        model_label: model.to_string_lossy().into_owned(),
        module_dirs: vec![module_dir.clone()],
        no_default_module_paths: true,
        output_path: Some(output.clone()),
        ..Default::default()
    })
    .expect("baseline run_slice must succeed against wedge + core-modules");
    assert!(
        !baseline.gcode_text.contains("POST_PROCESSED"),
        "an absent post_process key must leave the artifact unchanged"
    );
    assert!(
        !output.exists(),
        "run_slice must not write the caller's output file"
    );

    // With the key set: the script rewrites the working copy, and the rewritten
    // text is what the artifact carries.
    let scripts_config = dir.path().join("post_process.json");
    std::fs::write(
        &scripts_config,
        serde_json::json!({ "post_process": [appending_script("POST_PROCESSED")] }).to_string(),
    )
    .expect("write the post_process config");
    let rewritten = run_slice(SliceRunOptions {
        mesh: std::sync::Arc::clone(&mesh),
        model_label: model.to_string_lossy().into_owned(),
        config_path: Some(scripts_config),
        module_dirs: vec![module_dir.clone()],
        no_default_module_paths: true,
        output_path: Some(output.clone()),
        ..Default::default()
    })
    .expect("run_slice with a post_process script must succeed");
    assert_eq!(
        rewritten.gcode_text.lines().last(),
        Some("POST_PROCESSED"),
        "the configured script's output must land in the exported artifact"
    );
    assert!(
        !dir.path().join("post_process.gcode.pp").exists(),
        "the working copy must not survive the export"
    );

    // Both export keys at once: canonical's order is scripts first, then
    // numbering, so the appended line is numbered as well.
    let both_config = dir.path().join("post_process_and_numbers.json");
    std::fs::write(
        &both_config,
        serde_json::json!({
            "post_process": [appending_script("POST_PROCESSED")],
            "gcode_add_line_number": true,
        })
        .to_string(),
    )
    .expect("write the combined config");
    let numbered = run_slice(SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        config_path: Some(both_config),
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        output_path: Some(output),
        ..Default::default()
    })
    .expect("run_slice with both export keys must succeed");
    let marker_line = numbered
        .gcode_text
        .lines()
        .last()
        .expect("the artifact must have a last line");
    assert!(
        marker_line.starts_with('N') && marker_line.ends_with("POST_PROCESSED"),
        "the script must run before the line-number rewrite, got last line {marker_line:?}"
    );
    for (index, line) in numbered.gcode_text.lines().enumerate() {
        assert!(
            line.starts_with(&format!("N{} ", index + 1)),
            "line {} must be numbered, got {line:?}",
            index + 1
        );
    }
}

/// A `post_process` value carried by *model* metadata (a 3MF's project
/// settings, ingested as `SliceRunOptions::config_overrides`) is refused: the
/// key names host commands, and a downloaded model must not be able to arm
/// them. Canonical runs the model's value — deliberate divergence, DEV-197.
#[test]
fn post_process_from_model_metadata_is_ignored() {
    let root = workspace_root();

    let model = root.join("resources").join("regression_wedge.stl");
    let module_dir = root.join("modules").join("core-modules");
    assert!(model.exists(), "regression_wedge.stl must exist");
    assert!(module_dir.exists(), "core-modules directory must exist");

    let mesh =
        std::sync::Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("from_model.gcode");

    #[cfg(windows)]
    let script = "echo FROM_MODEL>>".to_string();
    #[cfg(not(windows))]
    let script = "printf 'FROM_MODEL\\n' >>".to_string();

    let outcome = run_slice(SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        output_path: Some(output),
        config_overrides: std::collections::HashMap::from([(
            "post_process".to_string(),
            ConfigValue::List(vec![ConfigValue::String(script)]),
        )]),
        ..Default::default()
    })
    .expect("a model-supplied post_process must not fail the slice, only be ignored");

    assert!(
        !outcome.gcode_text.contains("FROM_MODEL"),
        "a model-supplied post_process must not run"
    );
}
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
