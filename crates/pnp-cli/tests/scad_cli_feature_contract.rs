//! Feature-state contract for the command-local SCAD flags.
//!
//! This target is **ungated on purpose**: it must build and execute real
//! assertions in both the default (SCAD-off) and `--features scad` builds. The
//! feature-off branches pin that the flags do not exist and that `.scad` input
//! still reports the disabled-support diagnostic instead of a STEP parse error.
//! A whole-file `#![cfg(feature = "scad")]` gate would turn the feature-off run
//! into a green zero-test binary.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use tempfile::tempdir;

#[cfg(feature = "scad")]
fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

fn write_scad(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write SCAD fixture");
    path
}

/// Run the Cargo-selected binary for the current feature set.
fn run(args: &[&OsStr]) -> Output {
    Command::cargo_bin("pnp_cli")
        .expect("cargo-selected pnp_cli binary")
        .args(args)
        .output()
        .expect("run pnp_cli")
}

fn run_owned(argv: &[OsString]) -> Output {
    let refs: Vec<&OsStr> = argv.iter().map(OsString::as_os_str).collect();
    run(&refs)
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_exit(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: expected exit {expected}, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        stderr_text(output),
    );
}

fn assert_stderr_contains(output: &Output, needle: &str, context: &str) {
    let stderr = stderr_text(output);
    assert!(
        stderr.contains(needle),
        "{context}: stderr must contain {needle:?}\nstderr:\n{stderr}"
    );
}

#[cfg(feature = "scad")]
fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// `slice` argv for the feature-off flag-rejection check. The enabled flag
/// coverage lives in the `CE` target's `scad_cli_rejects_option_misuse`.
#[cfg(not(feature = "scad"))]
fn slice_argv(model: &Path, output: &Path, extra: &[&str]) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec![
        "slice".into(),
        "--model".into(),
        model.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
    ];
    argv.extend(extra.iter().map(OsString::from));
    argv
}

/// Parse the JSONL progress rows on stderr. Plain `note:` / `error:` lines are
/// expected and ignored; only well-formed JSON objects count as events.
#[cfg(feature = "scad")]
fn stderr_events(stderr: &str) -> Vec<serde_json::Value> {
    stderr
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn scad_cli_feature_routing() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(dir.path(), "routing.scad", "cube([2,2,2]);\n");
    let scad_upper = write_scad(dir.path(), "routing_upper.SCAD", "cube([2,2,2]);\n");

    #[cfg(not(feature = "scad"))]
    {
        // `.scad` must reach the SCAD adapter's disabled-support diagnostic
        // instead of leaking into STEP parsing. Exit 4 is the import parse
        // error; the destination is never created.
        let import_out = dir.path().join("off_import.stl");
        let result = run(&[
            OsStr::new("mesh"),
            OsStr::new("import"),
            OsStr::new("--input"),
            scad.as_os_str(),
            OsStr::new("--output"),
            import_out.as_os_str(),
        ]);
        assert_exit(&result, 4, "feature-off .scad import");
        assert_stderr_contains(&result, "SCAD_SUPPORT_DISABLED", "feature-off .scad import");
        assert!(
            !import_out.exists(),
            "feature-off .scad import must not create an output file"
        );

        // Extension detection is case-insensitive; `.SCAD` follows the same
        // branch.
        let upper_out = dir.path().join("off_upper.stl");
        let result = run(&[
            OsStr::new("mesh"),
            OsStr::new("import"),
            OsStr::new("--input"),
            scad_upper.as_os_str(),
            OsStr::new("--output"),
            upper_out.as_os_str(),
        ]);
        assert_exit(&result, 4, "feature-off .SCAD import");
        assert_stderr_contains(&result, "SCAD_SUPPORT_DISABLED", "feature-off .SCAD import");
        assert!(
            !upper_out.exists(),
            "feature-off .SCAD import must not create an output file"
        );

        // `slice` keeps its existing model-load failure status (1) and never
        // writes G-code.
        let slice_out = dir.path().join("off.gcode");
        let result = run(&[
            OsStr::new("slice"),
            OsStr::new("--model"),
            scad.as_os_str(),
            OsStr::new("--output"),
            slice_out.as_os_str(),
        ]);
        assert_exit(&result, 1, "feature-off .scad slice");
        assert_stderr_contains(&result, "SCAD_SUPPORT_DISABLED", "feature-off .scad slice");
        assert!(
            !slice_out.exists(),
            "feature-off .scad slice must not create G-code"
        );

        // The flags are not advertised and not accepted.
        for argv in [
            vec![OsStr::new("slice"), OsStr::new("--help")],
            vec![
                OsStr::new("mesh"),
                OsStr::new("import"),
                OsStr::new("--help"),
            ],
        ] {
            let result = run(&argv);
            assert_exit(&result, 0, "feature-off help");
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                !stdout.contains("--scad-param"),
                "feature-off help must not advertise --scad-param:\n{stdout}"
            );
            assert!(
                !stdout.contains("--scad-extrude-height"),
                "feature-off help must not advertise --scad-extrude-height:\n{stdout}"
            );
        }

        // Supplying the flag is an unknown-argument usage error (exit 2),
        // proving clap never accepts it in this feature state.
        let rejected_out = dir.path().join("off_flag.gcode");
        let result = run_owned(&slice_argv(&scad, &rejected_out, &["--scad-param", "a=1"]));
        assert_exit(&result, 2, "feature-off flag must be an unknown argument");
        let stderr = stderr_text(&result);
        assert!(
            stderr.contains("unexpected argument") || stderr.contains("--scad-param"),
            "feature-off flag rejection must name the unknown argument:\n{stderr}"
        );
        assert!(!rejected_out.exists());
    }

    #[cfg(feature = "scad")]
    {
        // The two intended commands expose both flags.
        for argv in [
            vec![OsStr::new("slice"), OsStr::new("--help")],
            vec![
                OsStr::new("mesh"),
                OsStr::new("import"),
                OsStr::new("--help"),
            ],
        ] {
            let result = run(&argv);
            assert_exit(&result, 0, "feature-on help");
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                stdout.contains("--scad-param"),
                "feature-on help must advertise --scad-param:\n{stdout}"
            );
            assert!(
                stdout.contains("--scad-extrude-height"),
                "feature-on help must advertise --scad-extrude-height:\n{stdout}"
            );
        }

        // No other command surface inherits the flags.
        for args in [
            &["mesh", "convert", "--help"][..],
            &["mesh", "repair", "--help"][..],
            &["mesh", "decimate", "--help"][..],
            &["support-preview", "--help"][..],
            &["profile", "--help"][..],
        ] {
            let argv: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
            let result = run(&argv);
            assert_exit(&result, 0, &format!("help for {args:?}"));
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                !stdout.contains("--scad-param"),
                "{} must not advertise --scad-param:\n{stdout}",
                args.join(" ")
            );
            assert!(
                !stdout.contains("--scad-extrude-height"),
                "{} must not advertise --scad-extrude-height:\n{stdout}",
                args.join(" ")
            );
        }

        // Step 3 wires the explicit evaluation into `slice`: a `.scad` input
        // now produces real G-code through the loaded module tree. Both
        // extension casings take the same detected-format branch.
        for (label, path, output_path) in [
            ("lowercase", &scad, dir.path().join("slice_lower.gcode")),
            (
                "uppercase",
                &scad_upper,
                dir.path().join("slice_upper.gcode"),
            ),
        ] {
            let argv: Vec<OsString> = vec![
                "slice".into(),
                "--model".into(),
                path.as_os_str().to_os_string(),
                "--output".into(),
                output_path.as_os_str().to_os_string(),
                "--module-dir".into(),
                module_dir().into_os_string(),
                "--no-default-module-paths".into(),
                "--no-integrated-modules".into(),
            ];
            let result = run_owned(&argv);
            let context = format!("enabled {label} slice");
            assert_exit(&result, 0, &context);
            assert!(
                result.stdout.is_empty(),
                "{context}: slice must not write G-code to stdout when --output is set:\n{}",
                String::from_utf8_lossy(&result.stdout)
            );
            let gcode = std::fs::read_to_string(&output_path)
                .unwrap_or_else(|e| panic!("{context}: read {}: {e}", output_path.display()));
            assert!(
                !gcode.is_empty(),
                "{context}: a successful slice must write non-empty G-code"
            );

            let stderr = stderr_text(&result);
            assert!(
                !stderr.contains("SCAD_OPTIONS_REQUIRED"),
                "{context}: the slice path must evaluate explicitly, not refuse the load:\n{stderr}"
            );
            assert!(
                !stderr.contains("SCAD_SUPPORT_DISABLED"),
                "{context}: the scad feature is enabled in this build:\n{stderr}"
            );

            let events = stderr_events(&stderr);
            let complete: Vec<&serde_json::Value> = events
                .iter()
                .filter(|event| event["event"] == "slice_complete")
                .collect();
            assert_eq!(
                complete.len(),
                1,
                "{context}: exactly one slice_complete event expected\nstderr:\n{stderr}"
            );
            assert_eq!(
                complete[0]["status"].as_str(),
                Some("ok"),
                "{context}: slice_complete must report an ok status: {}",
                complete[0]
            );
        }

        // Only the two intended commands evaluate scripts. `support-preview`
        // goes through the non-evaluating loader, so a `.scad` input is refused
        // with the actionable diagnostic and no artefact is written.
        let preview = dir.path().join("preview.json");
        let result = run(&[
            OsStr::new("support-preview"),
            OsStr::new("--input"),
            scad.as_os_str(),
            OsStr::new("--output"),
            preview.as_os_str(),
        ]);
        assert_exit(&result, 1, "support-preview on .scad");
        assert_stderr_contains(&result, "SCAD_OPTIONS_REQUIRED", "support-preview on .scad");
        assert!(
            !preview.exists(),
            "support-preview must not write a preview for .scad input"
        );
    }

    // Both states: import keeps its explicit `--output-format` option and
    // never grows an invented `--format` alias.
    let result = run(&[
        OsStr::new("mesh"),
        OsStr::new("import"),
        OsStr::new("--help"),
    ]);
    assert_exit(&result, 0, "import help (both states)");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("--output-format"),
        "mesh import help must keep --output-format:\n{stdout}"
    );
    assert!(
        !stdout.contains("--format"),
        "mesh import help must not gain a --format alias:\n{stdout}"
    );
}
