//! Packet 136 — AC-N1 no_linker_module_degraded_raw_output.
//!
//! Slices `resources/regression_wedge.stl` with a module set that EXCLUDES
//! `infill-linker`. The slice must complete without error and the committed
//! gcode-level sparse infill output must be the raw disjoint form (mean
//! points per `;TYPE:Sparse infill` path is at the raw two-point baseline of
//! 2, NOT the linked output of ~ 4.9). This pins ADR-0025's
//! degraded-not-failed trade-off at the integration level: a missing linker
//! is a degraded output, not a hard failure.
//!
//! Authoritative pipe command:
//!   `cargo test -p slicer-runtime --test integration -- no_linker_module_degraded_raw_output`

use pnp_cli_locator::pnp_cli_bin;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_root() -> PathBuf {
    repo_root().join("modules").join("core-modules")
}

fn wedge_stl() -> PathBuf {
    repo_root().join("resources").join("regression_wedge.stl")
}

fn gcode_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("target")
        .join("no_linker_module_degraded.gcode")
}

/// Count extruding moves per contiguous sparse-infill path.
///
/// A path is a maximal run of extruding moves (`G1` with `E` and an `X`/`Y`
/// coordinate) uninterrupted by travel (`G0`, or `G1` without `E`). A
/// retraction or unretraction (`G1` with `E` but no `X`/`Y`) does not break a
/// run. A path's point count is its G1-move count plus one, which is the unit
/// AC-N1 states directly ("mean points-per-path <= 2").
fn parse_sparse_infill_path_g1_moves(gcode: &str) -> Vec<u32> {
    let mut paths: Vec<u32> = Vec::new();
    let mut in_sparse = false;
    let mut in_path = false;
    let mut current: u32 = 0;
    for raw in gcode.lines() {
        let line = raw.trim();
        if line == ";TYPE:Sparse infill" {
            in_sparse = true;
            in_path = false;
            current = 0;
            continue;
        }
        if line.starts_with(";TYPE:") {
            if in_sparse && in_path {
                paths.push(current);
            }
            in_sparse = false;
            in_path = false;
            current = 0;
            continue;
        }
        if !in_sparse {
            continue;
        }
        let is_g1 = line.starts_with("G1 ");
        let has_e = is_g1 && line.contains('E');
        let has_xy = has_e && (line.contains('X') || line.contains('Y'));
        if has_xy {
            if !in_path {
                in_path = true;
                current = 0;
            }
            current += 1;
        } else if in_path && (line.starts_with("G0 ") || (is_g1 && !has_e)) {
            paths.push(current);
            in_path = false;
            current = 0;
        }
    }
    if in_sparse && in_path {
        paths.push(current);
    }
    paths
}

#[test]
fn no_linker_module_degraded_raw_output() {
    let bin = pnp_cli_bin();
    let model = wedge_stl();
    let modules_root = core_modules_root();
    let gcode = gcode_path();
    assert!(bin.exists(), "pnp_cli not built at {}", bin.display());
    assert!(
        model.exists(),
        "regression_wedge.stl missing at {}",
        model.display()
    );
    assert!(
        modules_root.exists(),
        "core-modules dir missing at {}",
        modules_root.display()
    );

    let _ = std::fs::remove_file(&gcode);

    // Build the no-linker module set: every core-module dir EXCEPT
    // `infill-linker`. The slice must still complete; the linker is not a
    // hard requirement (ADR-0025's degraded-not-failed trade-off).
    let mut cmd = Command::new(&bin);
    cmd.args(["slice", "--model"])
        .arg(&model)
        .args(["--output"])
        .arg(&gcode)
        // `--module-dir` controls external discovery only. Disable the
        // integrated tier as well so this scenario truly excludes the linker.
        .arg("--no-integrated-modules");
    let entries = std::fs::read_dir(&modules_root).expect("read core-modules dir");
    let mut any = false;
    for e in entries {
        let e = e.expect("entry");
        let name = e.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "infill-linker" {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            cmd.args(["--module-dir"]).arg(&p);
            any = true;
        }
    }
    assert!(any, "no core-modules found at {}", modules_root.display());

    let proc = cmd.output().expect("pnp_cli binary should execute");
    let stderr = String::from_utf8_lossy(&proc.stderr);

    // The slice must complete without error (ADR-0025: degraded, not failed).
    assert!(
        proc.status.success(),
        "pnp_cli must succeed even with the linker module-dir excluded. \
         Stderr tail:\n{}",
        stderr
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // And the gcode must be written, with sparse infill output that is the
    // raw disjoint two-point form, not the linked form.
    assert!(gcode.exists(), "gcode not written at {}", gcode.display());
    let gcode_text = std::fs::read_to_string(&gcode).expect("read gcode");

    // The degraded form is a disjoint two-point path (one G1 move); the linked
    // form joins those points into longer paths. Measured on this tree
    // (2026-09-24, fresh guests, `--no-integrated-modules`):
    //   without linker: 6123 paths, mean G1 moves per path = 1.000 -> 2.00 points
    //   with linker:    2515 paths, mean G1 moves per path = 3.900 -> 4.90 points
    // The metric is the AC's own unit ("mean points-per-path"); a path's point
    // count is its extruding G1-move count plus one. Threshold 2.5 sits 25%
    // above the measured degraded mean and 49% below the measured linked mean.
    // The previous block-based proxy (mean G1 per `;TYPE:Sparse infill` block)
    // went stale when packets 233/234/235 reshaped the wedge's sparse-infill
    // islands: it measured 30.93 without the linker against a 28.0 threshold,
    // even though no path in that output exceeds two points.
    let paths = parse_sparse_infill_path_g1_moves(&gcode_text);
    assert!(
        paths.len() >= 2,
        "no-linker wedge slice must still produce at least 2 sparse-infill paths (got {})",
        paths.len()
    );
    let mean_points_per_path = (paths.iter().sum::<u32>() as f32) / (paths.len() as f32) + 1.0;
    assert!(
        mean_points_per_path <= 2.5,
        "AC-N1: without the linker, mean points per sparse-infill path should be at the raw \
         two-point baseline (<= 2.5); got {mean_points_per_path:.2}. If this is high, the linker \
         is wired even though its module-dir was excluded. Path count: {}",
        paths.len()
    );
}
