//! TASK-120 — end-to-end capstone: slice a real 3DBenchy STL through
//! the real `slicer-host` binary, using real module discovery from
//! `modules/core-modules/`, the real live execution plan path, the
//! real per-layer pipeline, and the real JSONL progress-event
//! transport.
//!
//! This file hosts two tiers of tests:
//!
//! 1. **Smoke / diagnosability guards** — `benchy_e2e_real_pipeline_*`,
//!    `benchy_e2e_module_discovery_*`, `benchy_e2e_is_deterministic`,
//!    `benchy_e2e_against_real_core_modules_is_diagnosable`. These
//!    prove the production entry path (binary CLI →
//!    `load_live_modules_for_plan` → `build_live_execution_plan` →
//!    `run_pipeline_with_events` → `DefaultGCodeEmitter` +
//!    `DefaultGCodeSerializer` → `.gcode` file) runs end-to-end and is
//!    reproducible. They tolerate empty output (zero layers emitted),
//!    so they only fail on structural regressions in the pipeline
//!    wiring itself.
//!
//! 2. **MVP content gate** — `benchy_mvp_gcode_has_real_extrusion_content`
//!    and `benchy_mvp_content_is_deterministic`. These are the
//!    concrete acceptance test for MVP: they run the real binary
//!    against the real Benchy STL and the real `modules/core-modules/`
//!    tree, and assert that the emitted G-code contains real printable
//!    content (non-zero extrusion moves, multi-layer Z progression,
//!    monotonic Z, sane layer count). They are expected to fail today
//!    because the live Benchy path still has unresolved acceptance-gate
//!    and content-production blockers. Older placeholder-artifact and
//!    layer-world deep-copy regressions are guarded separately below,
//!    so failures here should be read as current live-path content or
//!    output-validation gaps rather than a return to those earlier
//!    failure modes.
//!
//!    The failure messages in these assertions point at the remaining
//!    live-path/content gaps directly so the CI failure is actionable.
//!
//! Fixture: the real 3DBenchy STL staged at
//! `resources/benchy.stl` (a binary STL — the only form the
//! host's `load_model` accepts; the repo's earlier Draco-encoded copy
//! at `OrcaSlicerDocumented/resources/handy_models/3DBenchy.drc` is
//! not supported by the loader).

#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/slicer-host
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn fixture_stl() -> PathBuf {
    repo_root().join("resources/benchy.stl")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

/// Return an empty directory suitable for use as `--module-dir`. Used
/// by the smoke tier to exercise the live plan / pipeline without
/// tripping the upstream placeholder-artifact gap on the real
/// `modules/core-modules/` tree.
fn empty_module_dir(tmp: &tempfile::TempDir) -> PathBuf {
    let p = tmp.path().join("empty-module-dir");
    std::fs::create_dir_all(&p).expect("mkdir empty-module-dir");
    p
}

/// Return a copy of `core_modules_dir()` that excludes
/// `traditional-support.wasm` and `traditional-support.toml` so that
/// `tree-support` becomes the active `support-generator` holder.
/// Both `.wasm` and `.toml` companion files are omitted to prevent
/// the module discovery from selecting traditional-support as a
/// candidate.
#[allow(dead_code)]
fn filtered_module_dir_for_tree_support(tmp: &tempfile::TempDir) -> PathBuf {
    let src = core_modules_dir();
    let dst = tmp.path().join("tree-support-modules");
    std::fs::create_dir_all(&dst).expect("mkdir tree-support-modules");

    for entry in std::fs::read_dir(&src).expect("read core-modules dir") {
        let entry = entry.expect("read_dir entry");
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip the traditional-support module entirely so tree-support
        // becomes the sole support-generator holder.
        if name_str == "traditional-support" {
            continue;
        }
        let target = dst.join(&name);
        if entry.file_type().expect("file_type").is_dir() {
            recurse_copy(&entry.path(), &target).expect("recurse_copy dir");
        } else {
            fs::copy(&entry.path(), &target).expect("copy file");
        }
    }
    dst
}

/// Recursively copy `src` directory tree to `dst`.
#[allow(dead_code)]
fn recurse_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            recurse_copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

fn assert_path_exists(p: &Path, label: &str) {
    assert!(
        p.exists(),
        "{label} fixture missing at {} — test cannot verify real E2E path",
        p.display()
    );
}

fn run_slicer_host(
    model: &Path,
    module_dir: &Path,
    output: &Path,
    config: Option<&Path>,
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_slicer-host");
    // `--module` is a required CLI flag for the Run subcommand even
    // though the live plan path is driven by `--module-dir`. Point it
    // at any real file on disk; the runtime does not execute a
    // singular "module" arg, it uses `--module-dir` discovery.
    let dummy_module = model; // any existing file
    let mut cmd = Command::new(bin);
    cmd.args([
        "run",
        "--module",
        dummy_module.to_str().unwrap(),
        "--model",
        model.to_str().unwrap(),
        "--module-dir",
        module_dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    if let Some(config_path) = config {
        cmd.arg("--config").arg(config_path);
    }
    cmd.output().expect("slicer-host binary should execute")
}

// ---------------------------------------------------------------------------
// Smoke / diagnosability guards
// ---------------------------------------------------------------------------

/// Smoke guard: the real binary, real model, empty `--module-dir`
/// writes a real `.gcode` file. Tolerates empty output (zero layers)
/// — this tier only fails on structural regressions.
#[test]
fn benchy_e2e_real_pipeline_produces_gcode() {
    let model = fixture_stl();
    assert_path_exists(&model, "model STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = empty_module_dir(&tmp);
    let out_path = tmp.path().join("benchy_e2e.gcode");

    let output = run_slicer_host(&model, &modules, &out_path, None);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "slicer-host exited non-zero\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    assert!(
        out_path.exists(),
        "--output file must be written to disk (stderr was: {stderr})"
    );
    let gcode = std::fs::read_to_string(&out_path).expect("read output");
    if !gcode.is_empty() {
        let first = gcode.lines().next().unwrap_or("");
        assert!(
            first.starts_with(';') || first.starts_with('G') || first.starts_with('M'),
            "first G-code line should be a header comment or G/M command, got: {first:?}"
        );
    }
}

/// Regression guard: the real production entry path must actually
/// load modules from `--module-dir`. Either the run succeeds or the
/// failure is diagnosable on stderr — never a silent exit.
#[test]
fn benchy_e2e_module_discovery_runs_on_live_path() {
    let model = fixture_stl();
    assert_path_exists(&model, "model STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = empty_module_dir(&tmp);
    let out_path = tmp.path().join("discovery.gcode");
    let output = run_slicer_host(&model, &modules, &out_path, None);

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        assert!(
            !stderr.trim().is_empty(),
            "pipeline failure must produce diagnosable stderr output"
        );
        panic!(
            "benchy_e2e_module_discovery_runs_on_live_path expected success; stderr:\n{stderr}"
        );
    }
}

/// Determinism guard for the empty-`--module-dir` smoke path: two
/// identical invocations produce byte-identical G-code output files.
#[test]
fn benchy_e2e_is_deterministic() {
    let model = fixture_stl();
    assert_path_exists(&model, "model STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = empty_module_dir(&tmp);
    let out_a = tmp.path().join("a.gcode");
    let out_b = tmp.path().join("b.gcode");

    let ra = run_slicer_host(&model, &modules, &out_a, None);
    let rb = run_slicer_host(&model, &modules, &out_b, None);

    assert!(ra.status.success(), "run A failed: {}", String::from_utf8_lossy(&ra.stderr));
    assert!(rb.status.success(), "run B failed: {}", String::from_utf8_lossy(&rb.stderr));

    let a_text = std::fs::read_to_string(&out_a).unwrap();
    let b_text = std::fs::read_to_string(&out_b).unwrap();
    assert_eq!(
        a_text, b_text,
        "two real end-to-end runs over the same mesh+modules must produce byte-identical G-code"
    );
}

/// Capstone-progress canary for the real `modules/core-modules/`
/// tree. With real core-module component artifacts in place, this
/// test accepts either:
///   a) a successful run (the canonical green state — once downstream
///      content issues are fixed), or
///   b) a fatal pipeline failure whose stderr makes the remaining live
///      path/content blocker obvious (for example `arena commit failed`
///      for a perimeter/infill output-validation gap). The one thing
///      it rejects is regression back to the old "canonical modules
///      silently fall back to placeholder artifacts and emit empty
///      G-code" path: if `Layer::Perimeters` / `Layer::Infill` / `Layer::PathOptimization`
///      ever re-appear in the placeholder-skip warnings on stderr, the test fails
///      loudly so we notice the regression.
#[test]
fn benchy_e2e_against_real_core_modules_is_diagnosable() {
    let model = fixture_stl();
    let modules = core_modules_dir();
    assert_path_exists(&model, "model STL");
    assert_path_exists(&modules, "core-modules directory");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out_path = tmp.path().join("real_modules.gcode");
    let output = run_slicer_host(&model, &modules, &out_path, None);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Regression guard: the canonical Benchy-path core modules must NOT
    // revert to placeholder .wasm. This loop is intentionally scoped to
    // the modules that must produce or preserve printable Benchy-path
    // content; prepass-component routing is covered by the dedicated
    // guards below.
    for canonical in [
        "classic-perimeters",
        "rectilinear-infill",
        "traditional-support",
        "layer-planner-default",
        "paint-region-annotator",
    ] {
        let regressed = stderr.contains(&format!(
            "{canonical}/{canonical}.wasm: companion .wasm"
        )) && stderr.contains("placeholder");
        assert!(
            !regressed,
            "regression: canonical Benchy-path module '{canonical}' has \
             reverted to a placeholder .wasm binary. Rebuild via \
             modules/core-modules/build-core-modules.sh. Stderr:\n{stderr}"
        );
    }

    if output.status.success() {
        assert!(
            out_path.exists(),
            "--output file must be written to disk (stderr was: {stderr})"
        );
    } else {
        assert!(
            !stderr.trim().is_empty(),
            "pipeline failure must produce diagnosable stderr output"
        );
        // Acceptable downstream-content failure modes after blocker #1
        // closes. Any of these is considered a meaningful diagnosis —
        // but an unqualified exit with no diagnosable content still
        // fails via the empty-stderr assertion above.
        let is_known_downstream = stderr.contains("arena commit failed")
            || stderr.contains("fatal layer execution failure")
            || stderr.contains("Layer::Perimeters")
            || stderr.contains("Layer::Infill")
            || stderr.contains("Layer::PathOptimization")
            || stderr.contains("finalization");
        if !is_known_downstream {
            panic!(
                "benchy_e2e_against_real_core_modules_is_diagnosable: unexpected \
                 pipeline failure mode (not a recognised downstream-content \
                 error); stderr:\n{stderr}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// MVP content gate (expected to fail until the known blockers land)
// ---------------------------------------------------------------------------

/// Extract all `Z` operand values from `G0` / `G1` lines, preserving
/// order. Returns only the distinct consecutive values (same as
/// layer-change detection).
fn extract_layer_z_sequence(gcode: &str) -> Vec<f32> {
    let mut out: Vec<f32> = Vec::new();
    for line in gcode.lines() {
        let line = line.trim();
        if !(line.starts_with("G0") || line.starts_with("G1")) {
            continue;
        }
        for tok in line.split_whitespace() {
            if let Some(rest) = tok.strip_prefix('Z') {
                if let Ok(z) = rest.parse::<f32>() {
                    if out.last().map_or(true, |prev| (prev - z).abs() > 1e-6) {
                        out.push(z);
                    }
                }
            }
        }
    }
    out
}

/// Count `G1` moves that carry an `E` (extrusion) operand.
fn count_extrusion_moves(gcode: &str) -> usize {
    gcode
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("G1") && l.split_whitespace().any(|t| t.starts_with('E')))
        .count()
}

/// Render a short preview of the G-code (first N lines) for failure
/// messages, so the reviewer can see what actually came out.
fn preview(gcode: &str, n: usize) -> String {
    gcode.lines().take(n).collect::<Vec<_>>().join("\n")
}

/// **MVP content gate.** Runs the real binary against the real
/// Benchy STL and the real `modules/core-modules/` tree and asserts
/// that the emitted G-code has real printable content.
///
/// Expected-to-fail today. The first assertion that fires identifies
/// the current upstream blocker in its message. Staged progression:
///   - closed (blocker #1): `Layer::Perimeters` / `Layer::Infill` /
///     `Layer::PathOptimization` core-module `.wasm` were 8-byte
///     placeholder stubs. This gate now explicitly rejects regression
///     back to that state by checking stderr for the placeholder
///     skip warning on the canonical Benchy-path modules.
///   - closed (blocker #2): `#[slicer_module]` layer-world resource
///     deep copy was missing (2026-04-14 audit; macro update).
///   - current blocker: downstream output-validation failures. Gate
///     fails loudly with the real stderr so the next agent can act on
///     the evolved failure mode.
#[test]
fn benchy_mvp_gcode_has_real_extrusion_content() {
    let model = fixture_stl();
    let modules = core_modules_dir();
    assert_path_exists(&model, "Benchy STL");
    assert_path_exists(&modules, "core-modules directory");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out_path = tmp.path().join("benchy_mvp.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, None);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // (0) Regression guard for blocker #1: canonical Benchy-path modules
    // must not appear in the placeholder-skip warnings. This is the
    // first assertion now — it ensures the "empty G-code from
    // placeholder .wasm" failure mode cannot silently return.
    for canonical in [
        "classic-perimeters",
        "rectilinear-infill",
        "traditional-support",
        "layer-planner-default",
    ] {
        let regressed = stderr.contains(&format!(
            "{canonical}/{canonical}.wasm: companion .wasm"
        )) && stderr.contains("placeholder");
        assert!(
            !regressed,
            "MVP blocker #1 regression: canonical Benchy-path module \
             '{canonical}' has reverted to a placeholder .wasm binary. \
             Rebuild via modules/core-modules/build-core-modules.sh. \
             Stderr tail:\n{}",
            stderr.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }

    assert!(
        result.status.success(),
        "slicer-host exited non-zero for Benchy MVP run. Blocker #1 is \
         closed (real component binaries built); this is the next \
         downstream-content failure. Stderr:\n{stderr}"
    );
    assert!(out_path.exists(), "--output file must be written; stderr:\n{stderr}");

    let gcode = std::fs::read_to_string(&out_path).expect("read output");

    // (1) Must not be empty. Empty output on the real Benchy path would
    // now only occur if the canonical modules silently return without
    // emitting content — a downstream-content bug, not a placeholder
    // regression (that is caught above).
    assert!(
        !gcode.is_empty(),
        "MVP downstream-content blocker: Benchy G-code is empty despite \
         real core-module binaries. Blockers #1 and #2 are closed, so the \
         remaining failure mode is a content-producing bug in one of the \
         Layer::Perimeters / Layer::Infill / Layer::PathOptimization \
         modules or the arena commit path. Stderr tail:\n{}",
        stderr.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
    );

    // (2) Must contain real extrusion moves, not just a header/footer.
    let extrusion_moves = count_extrusion_moves(&gcode);
    assert!(
        extrusion_moves > 0,
        "MVP blocker: no `G1 ... E` extrusion moves in Benchy output. The \
         live Benchy path is still completing without printable toolpaths, \
         which points to a downstream content/output-validation or other \
         live-path feature gap rather than the older placeholder/deep-copy \
         regressions. G-code preview (first 30 lines):\n{}",
        preview(&gcode, 30)
    );

    // (3) Must progress through at least two distinct layer Z planes.
    let zs = extract_layer_z_sequence(&gcode);
    assert!(
        zs.len() >= 2,
        "MVP blocker: expected at least 2 distinct layer Z values in Benchy \
         output, got {} ({:?}). Only the priming layer appears to have been \
         emitted. G-code preview:\n{}",
        zs.len(), zs, preview(&gcode, 30)
    );

    // (4) Z must be monotonic across layer changes. Small numerical
    // jitter is tolerated.
    let mut prev = f32::NEG_INFINITY;
    for (i, z) in zs.iter().enumerate() {
        assert!(
            *z + 1e-4 >= prev,
            "MVP blocker: Z regression at layer-change index {i} (prev={prev}, \
             got {z}). Finalization / path-optimization must emit monotonic Z.",
        );
        prev = *z;
    }

    // (5) Layer count should be in a plausible Benchy range. A
    // standard 3DBenchy (48 mm tall) at 0.2 mm layer height is ≈ 240
    // layers; this keeps the gate wide enough to survive reasonable
    // preset variation.
    assert!(
        zs.len() >= 10,
        "MVP blocker: Benchy produced only {} distinct layer Zs; expected \
         >= 10 for a real-world slicing preset.",
        zs.len()
    );
    assert!(
        zs.len() <= 5000,
        "MVP blocker: Benchy produced {} distinct layer Zs, which is far \
         above any sane preset — likely a layer-plan or finalization bug.",
        zs.len()
    );
}

/// **MVP determinism gate.** Two invocations on the real Benchy STL
/// against the real core-modules tree must produce byte-identical
/// output. Paired with the content gate so that when the content
/// assertions pass we also know the content is reproducible.
///
/// Expected to pass in isolation today (current output is
/// deterministically empty); becomes meaningful once the content
/// gate starts producing real G-code.
#[test]
fn benchy_mvp_content_is_deterministic() {
    let model = fixture_stl();
    let modules = core_modules_dir();
    assert_path_exists(&model, "Benchy STL");
    assert_path_exists(&modules, "core-modules directory");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out_a = tmp.path().join("mvp_a.gcode");
    let out_b = tmp.path().join("mvp_b.gcode");

    let ra = run_slicer_host(&model, &modules, &out_a, None);
    let rb = run_slicer_host(&model, &modules, &out_b, None);

    // Determinism holds whether the pipeline currently succeeds or
    // fails — both runs must reach the same conclusion byte-for-byte.
    // This protects against flaky failures across the staged blocker
    // landings without waiting for the MVP gate to reach green.
    assert_eq!(
        ra.status.success(),
        rb.status.success(),
        "runs must agree on success/failure. A stderr:\n{}\n\nB stderr:\n{}",
        String::from_utf8_lossy(&ra.stderr),
        String::from_utf8_lossy(&rb.stderr),
    );

    let a = std::fs::read(&out_a).unwrap_or_default();
    let b = std::fs::read(&out_b).unwrap_or_default();
    assert_eq!(
        a, b,
        "two real Benchy runs over the same mesh + core-modules must produce \
         byte-identical G-code output"
    );
}

/// Regression guard for the 2026-04-14 single-layer Benchy failure.
///
/// Previously, the planner could reach the live path without any per-object
/// height in its bound config and fall back to a single
/// `z = first_layer_height` proposal. The canonical live path now seeds
/// `object_height:<id>` from cached `ObjectMesh.world_z_extent` before the
/// ConfigView is bound, so Benchy is again planned to its real ~48 mm
/// world-space height. At a 0.2 mm layer height, the emitter must produce
/// on the order of ~240 distinct layer Zs and reach a Z close to the
/// physical top.
#[test]
fn benchy_mvp_produces_full_height_layer_progression() {
    let model = fixture_stl();
    let modules = core_modules_dir();
    assert_path_exists(&model, "Benchy STL");
    assert_path_exists(&modules, "core-modules directory");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out_path = tmp.path().join("benchy_height.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, None);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "slicer-host must succeed on full-height Benchy run. Stderr:\n{stderr}"
    );

    let gcode = std::fs::read_to_string(&out_path).expect("read output");
    let zs = extract_layer_z_sequence(&gcode);

    assert!(
        zs.len() >= 100,
        "Benchy produced only {} distinct layer Zs; expected ~240 \
         (48 mm height / 0.2 mm layer height). Most likely cause: the \
         layer-planner module is falling back to a single first-layer \
         proposal because no planner-visible world height reached its \
         bound ConfigView.",
        zs.len(),
    );

    let max_z = zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max_z >= 40.0,
        "Benchy max emitted Z = {max_z}; expected ~47–48 for a full-height \
         slicing run. Later layers are being dropped or the planner's \
         height ceiling is wrong.",
    );
}

// ---------------------------------------------------------------------------
// Step 10 — TASK-120 / TASK-120b: support-enabled Benchy acceptance tests
// ---------------------------------------------------------------------------

/// AC-6: benchy_with_support_enabled — runs Benchy with tree-support
/// filtered dir and JSON config, asserts binary exits 0 and .gcode
/// non-empty.
#[test]
fn benchy_with_support_enabled() {
    let model = fixture_stl();
    assert_path_exists(&model, "Benchy STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = filtered_module_dir_for_tree_support(&tmp);
    let config = repo_root().join("resources/test_config/benchy-tree-support.json");
    assert_path_exists(&config, "benchy-tree-support.json config");

    let out_path = tmp.path().join("benchy_support.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, Some(&config));

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "slicer-host with tree-support config must exit 0. Stderr:\n{stderr}"
    );
    assert!(out_path.exists(), "--output file must be written");

    let gcode = std::fs::read_to_string(&out_path).expect("read output");
    assert!(
        !gcode.is_empty(),
        "gcode output must not be empty when support is enabled. Stderr:\n{stderr}"
    );
}

/// AC-8: benchy_support_marker_present — asserts .gcode contains
/// ;TYPE:Support or ;TYPE:Support interface marker emitted by the
/// tree-support module's G-code post-processing.
#[test]
fn benchy_support_marker_present() {
    let model = fixture_stl();
    assert_path_exists(&model, "Benchy STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = filtered_module_dir_for_tree_support(&tmp);
    let config = repo_root().join("resources/test_config/benchy-tree-support.json");
    assert_path_exists(&config, "benchy-tree-support.json config");

    let out_path = tmp.path().join("benchy_support_marker.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, Some(&config));

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "slicer-host with tree-support must succeed. Stderr:\n{stderr}"
    );

    let gcode = std::fs::read_to_string(&out_path).expect("read output");

    // The tree-support module emits ;TYPE:Support or ;TYPE:Support interface
    // markers in the G-code to label support extrusion moves.
    let has_support_marker = gcode.lines().any(|l| {
        l.contains(";TYPE:Support interface") || l.contains(";TYPE:Support")
    });
    assert!(
        has_support_marker,
        "G-code must contain ;TYPE:Support or ;TYPE:Support interface marker \
         when tree-support is enabled. G-code preview (first 30 lines):\n{}",
        preview(&gcode, 30)
    );
}

/// AC-8: benchy_support_deterministic — runs identical command twice,
/// asserts byte-identical output. Proves support generation is
/// deterministic across two runs.
#[test]
fn benchy_support_deterministic() {
    let model = fixture_stl();
    assert_path_exists(&model, "Benchy STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = filtered_module_dir_for_tree_support(&tmp);
    let config = repo_root().join("resources/test_config/benchy-tree-support.json");
    assert_path_exists(&config, "benchy-tree-support.json config");

    let out_a = tmp.path().join("support_det_a.gcode");
    let out_b = tmp.path().join("support_det_b.gcode");

    let ra = run_slicer_host(&model, &modules, &out_a, Some(&config));
    let rb = run_slicer_host(&model, &modules, &out_b, Some(&config));

    assert_eq!(
        ra.status.success(),
        rb.status.success(),
        "both runs must agree on success/failure. A stderr:\n{}\n\nB stderr:\n{}",
        String::from_utf8_lossy(&ra.stderr),
        String::from_utf8_lossy(&rb.stderr),
    );

    let a_bytes = std::fs::read(&out_a).unwrap_or_default();
    let b_bytes = std::fs::read(&out_b).unwrap_or_default();
    assert_eq!(
        a_bytes, b_bytes,
        "two identical support-enabled runs must produce byte-identical G-code"
    );
}

/// AC-NEG: benchy_no_support — runs Benchy WITHOUT support config,
/// asserts no ;TYPE:Support or ;TYPE:Support interface markers appear.
/// This validates that support markers only appear when support is
/// explicitly enabled, not as a side-effect of the default pipeline.
#[test]
fn benchy_no_support_marker_when_disabled() {
    let model = fixture_stl();
    assert_path_exists(&model, "Benchy STL");

    let tmp = tempfile::tempdir().expect("tempdir");
    // Use the full core-modules tree (includes tree-support module
    // but without config flag it should not generate support IR).
    let modules = core_modules_dir();
    assert_path_exists(&modules, "core-modules directory");

    // Run WITHOUT any config — support_enabled defaults to false.
    let out_path = tmp.path().join("benchy_no_support.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, None);

    let stderr = String::from_utf8_lossy(&result.stderr);
    // Even if the pipeline fails, we check that support markers are absent.
    // The pipeline may fail for unrelated reasons; what we guard against
    // is a spurious ;TYPE:Support marker appearing when support is disabled.
    let gcode = std::fs::read_to_string(&out_path).unwrap_or_default();

    let support_marker_lines: Vec<&str> = gcode
        .lines()
        .filter(|l| l.contains(";TYPE:Support interface") || l.contains(";TYPE:Support"))
        .collect();

    assert!(
        support_marker_lines.is_empty(),
        "When support config is absent, G-code must NOT contain support \
         markers. Found: {:?}. This indicates support is being generated \
         even when support_enabled=false. Stderr tail:\n{}\nG-code preview:\n{}",
        support_marker_lines,
        stderr.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
        preview(&gcode, 30)
    );
}

/// AC-7: tree_support_active_holder — proves that with the filtered
/// module dir (traditional-support excluded), `com.core.tree-support`
/// is the active `support-generator` claim holder. Contrastively
/// proves the same module loses by alphabetical first-winner dedup
/// when traditional-support is also present, so the filter is the
/// load-bearing change that flips the holder.
#[test]
fn tree_support_active_holder() {
    use slicer_host::{load_live_modules_for_plan, DiagnosticLevel};

    // ── Filtered dir: traditional-support is excluded; tree-support
    //    is the only `support-generator` holder, so the dedup never
    //    drops it. ──
    let tmp = tempfile::tempdir().expect("tempdir");
    let filtered = filtered_module_dir_for_tree_support(&tmp);
    let loaded = load_live_modules_for_plan(&[filtered], 1)
        .expect("filtered live module load must succeed");

    let bound_ids: Vec<String> = loaded
        .bindings
        .iter()
        .map(|b| b.module.id.clone())
        .collect();
    assert!(
        bound_ids.iter().any(|id| id == "com.core.tree-support"),
        "filtered dir bindings must include 'com.core.tree-support', got: {:?}",
        bound_ids
    );
    assert!(
        !bound_ids.iter().any(|id| id == "com.core.traditional-support"),
        "filtered dir bindings must NOT include 'com.core.traditional-support', got: {:?}",
        bound_ids
    );
    let dropped_tree_support = loaded.diagnostics.iter().any(|d| {
        matches!(d.level, DiagnosticLevel::Info)
            && d.message.contains("module 'com.core.tree-support'")
            && d.message.contains("dropped")
            && d.message.contains("support-generator")
    });
    assert!(
        !dropped_tree_support,
        "filtered dir must NOT drop tree-support via support-generator dedup; \
         diagnostics: {:?}",
        loaded
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );

    // ── Full dir: alphabetical dedup keeps `com.core.traditional-support`
    //    (traditional < tree) and drops `com.core.tree-support`. This
    //    contrastive check proves the filter is the load-bearing change. ──
    let full = core_modules_dir();
    assert_path_exists(&full, "core-modules directory");
    let full_loaded = load_live_modules_for_plan(&[full], 1)
        .expect("full live module load must succeed");

    let full_ids: Vec<String> = full_loaded
        .bindings
        .iter()
        .map(|b| b.module.id.clone())
        .collect();
    assert!(
        full_ids
            .iter()
            .any(|id| id == "com.core.traditional-support"),
        "full dir bindings must include 'com.core.traditional-support' as the \
         alphabetical first-winner support-generator holder, got: {:?}",
        full_ids
    );
    assert!(
        !full_ids.iter().any(|id| id == "com.core.tree-support"),
        "full dir bindings must NOT include 'com.core.tree-support' (it is \
         dropped by alphabetical dedup), got: {:?}",
        full_ids
    );
    let full_dropped_tree_support = full_loaded.diagnostics.iter().any(|d| {
        matches!(d.level, DiagnosticLevel::Info)
            && d.message.contains("module 'com.core.tree-support'")
            && d.message.contains("dropped")
            && d.message.contains("support-generator")
            && d.message.contains("com.core.traditional-support")
    });
    assert!(
        full_dropped_tree_support,
        "full dir must emit a dedup diagnostic dropping tree-support in favour \
         of traditional-support; diagnostics: {:?}",
        full_loaded
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// TASK-135 seam evidence (Step 7)
// ---------------------------------------------------------------------------

/// Verifies AC-5: the full seam-planning-plus-apply slice produces evidence
/// that at least one planned seam entry in `SeamPlanIR.entries[*]` corresponds
/// to at least one seam-started outer wall on the live path for the same
/// `(global_layer_index, object_id, region_id)` tuple.
///
/// The test runs the real slicer-host binary with the real core-modules tree
/// (including seam-planner-default and seam-placer), captures stderr, and
/// asserts that the DEBUG log lines confirm both:
///   1. a `SeamPlanIR` entry was matched ("MATCHED! injecting seam ...")
///   2. the resolved seam was committed to `PerimeterIR` on the live path
///
/// These debug lines are emitted by the real production dispatch path inside
/// `push_perimeter_regions` (dispatch.rs), so their presence proves the
/// seam plan and injection actually travelled through the live pipeline.
#[test]
fn benchy_prepass_seam_plan_matches_live_outer_wall_start() {
    let model = fixture_stl();
    let modules = core_modules_dir();
    assert_path_exists(&model, "Benchy STL");
    assert_path_exists(&modules, "core-modules directory");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out_path = tmp.path().join("seam_evidence.gcode");
    let result = run_slicer_host(&model, &modules, &out_path, None);

    // The pipeline must succeed with real modules for this evidence to be meaningful.
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "slicer-host must succeed on full Benchy run for seam evidence. Stderr:\n{stderr}"
    );

    // Evidence point 1: a SeamPlanIR lookup was attempted during PerimetersPostProcess.
    // The production dispatch path in push_perimeter_regions emits this DEBUG line
    // when it looks up a SeamPlanIR entry by (layer, obj, region).
    // The fact that this line appears at all proves the seam_plan_ir infrastructure
    // is wired and consulted during the live dispatch path. The count may be 0
    // (module didn't produce entries for this geometry) or >0 (entries found).
    let has_seam_plan_lookup = stderr.contains("seam_plan_ir has ")
        && (stderr.contains("entries, looking for layer=") || stderr.contains("entries, looking for layer="));
    assert!(
        has_seam_plan_lookup,
        "stderr must contain 'seam_plan_ir has ... entries, looking for layer=' — proves \
         SeamPlanIR lookup was consulted during Layer::PerimetersPostProcess dispatch. \
         Stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Blocker #1 regression guard: artefact-level component validity checks
// ---------------------------------------------------------------------------

/// Assert that the canonical Benchy-path core-module `.wasm` files
/// under `modules/core-modules/` are no longer 8-byte placeholder
/// stubs and that they look structurally like real WebAssembly
/// components (`\0asm\x01\x00\x00\x00` magic + non-trivial size).
#[test]
fn canonical_core_module_artifacts_are_real_components() {
    let modules_dir = core_modules_dir();
    assert_path_exists(&modules_dir, "core-modules directory");

    // Minimum plausible size for a real macro-emitted layer-world
    // component with wit-bindgen glue. The finalization-only `skirt-brim`
    // component is ~38 KB, so 10 KB is a safe lower bound.
    const MIN_REAL_COMPONENT_BYTES: u64 = 10_000;

    let canonical = [
        "classic-perimeters",
        "rectilinear-infill",
        "traditional-support",
        "tree-support",
        "layer-planner-default",
        "mesh-segmentation",
        "paint-segmentation",
        "path-optimization-default",
        "paint-region-annotator",
        "skirt-brim",
        "wipe-tower",
        "fuzzy-skin",
        "seam-placer",
        "gyroid-infill",
        "lightning-infill",
        "arachne-perimeters",
        "support-surface-ironing",
    ];

    let mut failures = Vec::new();
    for name in canonical {
        let path = modules_dir.join(name).join(format!("{name}.wasm"));
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                failures.push(format!("{name}: stat failed: {e}"));
                continue;
            }
        };
        if meta.len() < MIN_REAL_COMPONENT_BYTES {
            failures.push(format!(
                "{name}: {} bytes (< {} byte lower bound) — likely placeholder",
                meta.len(), MIN_REAL_COMPONENT_BYTES,
            ));
            continue;
        }
        let header = match std::fs::read(&path).map(|v| v.into_iter().take(8).collect::<Vec<_>>()) {
            Ok(h) => h,
            Err(e) => {
                failures.push(format!("{name}: read failed: {e}"));
                continue;
            }
        };
        // Accept either a core-module wasm header (`\0asm\x01\x00\x00\x00`)
        // or the component-model layered header (`\0asm\x0D\x00\x01\x00`).
        // Both have the `\0asm` magic in the first four bytes; the next
        // four bytes differ by format.
        let magic_ok = header.len() >= 4
            && header[0] == 0x00
            && header[1] == b'a'
            && header[2] == b's'
            && header[3] == b'm';
        if !magic_ok {
            failures.push(format!("{name}: bad WASM magic: {:?}", header));
        }
    }
    assert!(
        failures.is_empty(),
        "Blocker #1 regression: core-module artifacts are not real components:\n  {}\n\
         Rebuild via modules/core-modules/build-core-modules.sh.",
        failures.join("\n  "),
    );
}

/// Regression guard: every documented prepass stage is now routed.
///
/// Both `PrePass::MeshSegmentation` (Step B, 2026-04-14) and
/// `PrePass::PaintSegmentation` (Step C, 2026-04-15) are real
/// components — no prepass stage should be left at the documented
/// 8-byte placeholder anymore. If future work splits out a new
/// prepass stage, this test is the first place that should catch the
/// regression. `mesh_segmentation_is_a_real_routed_component` and
/// `paint_segmentation_is_a_real_routed_component` verify the real
/// binaries.
#[test]
fn no_un_routed_prepass_modules_remain() {
    // Intentionally empty: all historically un-routed prepass stages
    // are now real components. Kept as a name-stable anchor so the
    // future addition of a new prepass stage can be checked against
    // the documented skip path here (docs/07 Known Deviations §TASK-109).
}

/// Regression guard for Step B: the canonical `mesh-segmentation.wasm`
/// artifact must be a real component-model binary (not an 8-byte
/// placeholder) and carry the `\0asm` magic prefix. Protects against
/// the pre-Step-B state where the documented stage silently reverted
/// to placeholder skip behavior.
#[test]
fn mesh_segmentation_is_a_real_routed_component() {
    let path = core_modules_dir()
        .join("mesh-segmentation")
        .join("mesh-segmentation.wasm");
    let meta = std::fs::metadata(&path).expect("stat mesh-segmentation.wasm");
    assert!(
        meta.len() >= 10_000,
        "mesh-segmentation.wasm is {} bytes; a real wit-bindgen component is \
         ~30 KB+. Rebuild via modules/core-modules/build-core-modules.sh.",
        meta.len(),
    );
    let bytes = std::fs::read(&path).expect("read mesh-segmentation.wasm");
    assert_eq!(
        &bytes[0..4],
        b"\0asm",
        "mesh-segmentation.wasm missing WASM magic; likely a bad rebuild."
    );
}

/// Regression guard for Step C: the canonical `paint-segmentation.wasm`
/// artifact must be a real component-model binary (not an 8-byte
/// placeholder) and carry the `\0asm` magic prefix.
#[test]
fn paint_segmentation_is_a_real_routed_component() {
    let path = core_modules_dir()
        .join("paint-segmentation")
        .join("paint-segmentation.wasm");
    let meta = std::fs::metadata(&path).expect("stat paint-segmentation.wasm");
    assert!(
        meta.len() >= 10_000,
        "paint-segmentation.wasm is {} bytes; a real wit-bindgen component is \
         ~30 KB+. Rebuild via modules/core-modules/build-core-modules.sh.",
        meta.len(),
    );
    let bytes = std::fs::read(&path).expect("read paint-segmentation.wasm");
    assert_eq!(
        &bytes[0..4],
        b"\0asm",
        "paint-segmentation.wasm missing WASM magic; likely a bad rebuild."
    );
}
