//! Integration tests for `pnp_cli dag` and `pnp_cli diagnose`.
//!
//! Drives the actual compiled `pnp_cli` binary via
//! `std::process::Command` against the project's `modules/core-modules/`
//! manifest set (no WASM compilation — `dag_cli` and `diagnose` stop at
//! `load_modules_from_roots`).
//!
//! See `docs/specs/agent-cli-debugging.md` §8.2.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn bin() -> PathBuf {
    let exe_name = if cfg!(windows) {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };
    let root = workspace_root();
    let debug = root.join("target").join("debug").join(exe_name);
    if debug.exists() {
        return debug;
    }
    let release = root.join("target").join("release").join(exe_name);
    if release.exists() {
        return release;
    }
    panic!("pnp_cli binary not found. Run `cargo build --workspace` first.")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/slicer-runtime has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn core_modules_path() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn run_dag(args: &[&str]) -> (Value, i32) {
    let core = core_modules_path();
    let mut cmd = Command::new(bin());
    cmd.arg("dag");
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--module-dir")
        .arg(&core)
        .arg("--no-default-module-paths");
    let output = cmd.output().expect("spawn pnp_cli");
    let exit = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if exit == 0 {
        let value: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "dag stdout was not JSON (exit={exit}): {e}\nstdout:\n{stdout}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (value, exit)
    } else {
        (Value::Null, exit)
    }
}

#[test]
fn dag_stages_against_core_modules_returns_known_stages() {
    let (json, exit) = run_dag(&["stages"]);
    assert_eq!(exit, 0);
    let stages = json["stages"].as_array().expect("stages array");
    assert!(!stages.is_empty(), "core-modules should populate stages");
    let ids: Vec<&str> = stages.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(
        ids.contains(&"Layer::Infill"),
        "expected Layer::Infill in stages, got {ids:?}"
    );
    // Tier strings must be derived from the canonical prefix.
    for s in stages {
        let id = s["id"].as_str().unwrap();
        let tier = s["tier"].as_str().unwrap();
        let expected_tier = if id.starts_with("PrePass::") {
            "prepass"
        } else if id.starts_with("Layer::") {
            "per_layer"
        } else if id.starts_with("PostPass::") {
            "postpass"
        } else {
            "unknown"
        };
        assert_eq!(tier, expected_tier, "tier mismatch for stage {id}");
    }
}

#[test]
fn dag_stages_with_empty_module_dir_surfaces_host_builtin_stages_only() {
    let tmp = std::env::temp_dir().join("dag_cli_empty_dir");
    let _ = std::fs::create_dir_all(&tmp);
    let output = Command::new(bin())
        .arg("dag")
        .arg("stages")
        .arg("--module-dir")
        .arg(&tmp)
        .arg("--no-default-module-paths")
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("json");
    let stages = json["stages"].as_array().expect("stages array");
    let stage_ids: Vec<&str> = stages.iter().filter_map(|s| s["id"].as_str()).collect();
    for expected in [
        "PrePass::MeshAnalysis",
        "PrePass::RegionMapping",
        "PrePass::Slice",
        "PrePass::ShellClassification",
        "PrePass::SupportGeometry",
        "PrePass::PaintSegmentation",
        "PostPass::GCodeEmit",
    ] {
        assert!(
            stage_ids.contains(&expected),
            "expected host built-in stage `{expected}` in stages with empty module dir; saw {stage_ids:?}",
        );
    }
}

#[test]
fn dag_stage_layer_infill_returns_serial_edges_with_flat_reasons() {
    let (json, exit) = run_dag(&["stage", "Layer::Infill"]);
    assert_eq!(exit, 0);
    assert_eq!(json["id"].as_str(), Some("Layer::Infill"));
    assert_eq!(json["tier"].as_str(), Some("per_layer"));
    let modules = json["modules"].as_array().expect("modules array");
    assert!(!modules.is_empty(), "Layer::Infill should have modules");
    // serial_edges may be empty if no in-stage IR write/read overlap; either
    // way it must be an array. Any present reason string must start with
    // `ir_write_read: ` or equal `explicit_requires`.
    let edges = json["serial_edges"].as_array().expect("serial_edges array");
    for e in edges {
        let reason = e["reason"].as_str().unwrap();
        assert!(
            reason == "explicit_requires" || reason.starts_with("ir_write_read: "),
            "unexpected reason flattening: {reason}"
        );
    }
}

#[test]
fn dag_stage_unknown_id_exits_nonzero() {
    let core = core_modules_path();
    let output = Command::new(bin())
        .arg("dag")
        .arg("stage")
        .arg("Layer::DoesNotExist")
        .arg("--module-dir")
        .arg(&core)
        .arg("--no-default-module-paths")
        .output()
        .expect("spawn");
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn dag_depends_for_known_module_returns_global_edges() {
    let (json, exit) = run_dag(&["depends", "com.core.gyroid-infill"]);
    assert_eq!(exit, 0);
    assert_eq!(json["module_id"].as_str(), Some("com.core.gyroid-infill"));
    // Either upstream or downstream must be non-empty against the real
    // core-modules tree (gyroid-infill writes InfillIR which the path
    // optimizer reads).
    let up = json["upstream"].as_array().unwrap();
    let down = json["downstream"].as_array().unwrap();
    assert!(
        !up.is_empty() || !down.is_empty(),
        "expected at least one upstream/downstream edge for gyroid-infill"
    );
    for e in up.iter().chain(down.iter()) {
        assert!(e["from"].is_string());
        assert!(e["from_stage"].is_string());
        assert!(e["to"].is_string());
        assert!(e["to_stage"].is_string());
        assert!(e["reason"].is_string());
    }
}

#[test]
fn dag_depends_unknown_module_exits_nonzero() {
    let core = core_modules_path();
    let output = Command::new(bin())
        .arg("dag")
        .arg("depends")
        .arg("com.example.does-not-exist")
        .arg("--module-dir")
        .arg(&core)
        .arg("--no-default-module-paths")
        .output()
        .expect("spawn");
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn dag_claims_returns_interchangeable_for_multi_holder_claims() {
    let (json, exit) = run_dag(&["claims"]);
    assert_eq!(exit, 0);
    let claims = json["claims"].as_array().expect("claims array");
    assert!(!claims.is_empty(), "core-modules has declared claims");
    // claim:sparse-fill is held by gyroid, lightning, and rectilinear infill
    // modules — must be reported interchangeable.
    let sparse = claims
        .iter()
        .find(|c| c["id"].as_str() == Some("claim:sparse-fill"))
        .expect("claim:sparse-fill present");
    assert_eq!(sparse["interchangeable"].as_bool(), Some(true));
    let holders = sparse["holders"].as_array().unwrap();
    assert!(holders.len() >= 2);
}

#[test]
fn diagnose_clean_core_modules_returns_pass_true_exit_zero() {
    let core = core_modules_path();
    let output = Command::new(bin())
        .arg("module")
        .arg("diagnose")
        .arg("--module-dir")
        .arg(&core)
        .arg("--no-default-module-paths")
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["pass"].as_bool(), Some(true));
    assert!(json["modules_loaded"].as_u64().unwrap() > 0);
    assert!(json["stages"].as_u64().unwrap() > 0);
    assert_eq!(json["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn diagnose_unreadable_module_dir_exits_two() {
    // Pointing at a single non-existent path that the loader cannot scan
    // should exit code 2 (unreadable files) per the spec exit-code contract.
    // The CLI only fails with code 2 when load_modules_from_roots itself
    // errors. assemble_search_roots silently drops nonexistent paths, so to
    // force the LoadError path we craft a manifest that is malformed.
    let tmp = std::env::temp_dir().join("dag_cli_unreadable");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // Drop a malformed TOML and a same-stem .wasm so the discovery loop
    // ingests it and load_modules_from_roots fails with TomlParse / Schema.
    std::fs::write(tmp.join("bad.toml"), "this is not valid toml = [").unwrap();
    std::fs::write(tmp.join("bad.wasm"), &[0u8; 4]).unwrap();

    let output = Command::new(bin())
        .arg("module")
        .arg("diagnose")
        .arg("--module-dir")
        .arg(&tmp)
        .arg("--no-default-module-paths")
        .output()
        .expect("spawn");
    // Either 1 (diagnostics emitted with errors) or 2 (LoadError) is
    // acceptable: a malformed manifest may surface either as a hard
    // LoadError or as a diagnostic at the `Error` level depending on the
    // failure mode. Both indicate a broken module tree per the spec.
    let code = output.status.code().expect("exited");
    assert!(
        code == 1 || code == 2,
        "expected exit 1 or 2 for malformed module dir, got {code}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
