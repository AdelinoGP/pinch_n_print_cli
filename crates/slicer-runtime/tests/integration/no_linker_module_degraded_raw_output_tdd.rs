//! Packet 136 — AC-N1 no_linker_module_degraded_raw_output.
//!
//! Slices `resources/regression_wedge.stl` with a module set that EXCLUDES
//! `infill-linker`. The slice must complete without error and the committed
//! gcode-level sparse infill output must be the raw disjoint form (mean G1
//! moves per `;TYPE:Sparse infill` block is at the raw baseline of ~ 1, NOT
//! the linked output of >> 1). This pins ADR-0025's degraded-not-failed
//! trade-off at the integration level: a missing linker is a degraded
//! output, not a hard failure.
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

fn parse_sparse_infill_g1_moves(gcode: &str) -> Vec<u32> {
    let mut sparse_moves: Vec<u32> = Vec::new();
    let mut in_sparse = false;
    let mut current: u32 = 0;
    for raw in gcode.lines() {
        let line = raw.trim();
        if line == ";TYPE:Sparse infill" {
            if in_sparse {
                sparse_moves.push(current);
            }
            in_sparse = true;
            current = 0;
        } else if line.starts_with(";TYPE:") {
            if in_sparse {
                sparse_moves.push(current);
            }
            in_sparse = false;
            current = 0;
        } else if in_sparse && line.starts_with("G1 ") && line.contains('E') {
            current += 1;
        }
    }
    if in_sparse {
        sparse_moves.push(current);
    }
    sparse_moves
}

/// Split each `;TYPE:Sparse infill` block into the individual extrusion paths
/// it contains and return each path's G1-move count.
///
/// A block is a run of commands under one `;TYPE:` header and may hold many
/// separate paths; a travel (`G0`, or a `G1` with no `E`) ends one path and
/// starts the next. An N-point path emits N-1 extruding G1 moves, so a raw
/// disjoint 2-point line contributes `1`.
///
/// This is the quantity AC-N1 is actually about ("mean points-per-path"),
/// unlike `parse_sparse_infill_g1_moves`, which measures blocks.
fn parse_sparse_infill_path_moves(gcode: &str) -> Vec<u32> {
    let mut paths: Vec<u32> = Vec::new();
    let mut in_sparse = false;
    let mut run: u32 = 0;
    for raw in gcode.lines() {
        let line = raw.trim();
        if line == ";TYPE:Sparse infill" {
            if run > 0 {
                paths.push(run);
            }
            in_sparse = true;
            run = 0;
        } else if line.starts_with(";TYPE:") {
            if run > 0 {
                paths.push(run);
            }
            in_sparse = false;
            run = 0;
        } else if in_sparse {
            if line.starts_with("G1 ") && line.contains('E') {
                run += 1;
            } else if line.starts_with("G0") || line.starts_with("G1 ") {
                if run > 0 {
                    paths.push(run);
                }
                run = 0;
            }
        }
    }
    if run > 0 {
        paths.push(run);
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
    // raw disjoint form (mean G1 moves per block ≈ 1, NOT >> 1).
    assert!(gcode.exists(), "gcode not written at {}", gcode.display());
    let gcode_text = std::fs::read_to_string(&gcode).expect("read gcode");

    let sparse_moves = parse_sparse_infill_g1_moves(&gcode_text);
    assert!(
        sparse_moves.len() >= 2,
        "no-linker wedge slice must still produce at least 2 sparse-infill blocks (got {})",
        sparse_moves.len()
    );
    // AC-N1's discriminator, measured directly on path shape.
    //
    // The AC's claim is about the shape of each sparse path: without the linker
    // the output is the raw disjoint form, "mean points-per-path <= 2" — an
    // N-point path is N-1 G1 moves, so a 2-point line is 1 move.
    //
    // This assertion used to proxy that with G1-moves-per-*block*, and that
    // proxy drifted: 4.68 (authored) -> 11.36 (packet 233 D11/F7 constant-
    // direction infill) -> 21.48 (packets 234/235 bridge gating) -> 28.05
    // (today), against a threshold walked 6.0 -> 12.0 -> 28.0. It drifted
    // because a block is a run under one `;TYPE:` header and holds MANY paths,
    // so it tracks how paths are grouped and ordered, not how long they are —
    // and grouping changes on every packet that reshapes infill islands. At
    // 28.05 vs a linked 40.34 it had eroded to 1.44x separation and went red
    // without any linking having occurred.
    //
    // Measured on this tree, 2026-09-04, both arms of the same wedge slice
    // (repeated runs are bit-identical):
    //
    //   without linker: 5294 paths, mean 1.070 G1 moves/path, median 1, max 3
    //   with linker:    2454 paths, mean 3.271 G1 moves/path, median 3, max 8
    //
    // So the degraded output IS raw disjoint — 1.070 is the AC's "<= 2 points
    // per path" — and the direct metric separates the arms by 3.1x instead of
    // 1.44x. Threshold 2.0 sits 1.9x above the raw baseline and 1.6x below the
    // linked one, and is stable under infill-island reshaping because it does
    // not depend on how paths are grouped into blocks.
    let path_moves = parse_sparse_infill_path_moves(&gcode_text);
    assert!(
        path_moves.len() >= 2,
        "no-linker wedge slice must still produce at least 2 sparse-infill paths (got {})",
        path_moves.len()
    );
    let mean_per_path = (path_moves.iter().sum::<u32>() as f32) / (path_moves.len() as f32);
    let median = {
        let mut sorted = path_moves.clone();
        sorted.sort_unstable();
        sorted[sorted.len() / 2]
    };
    assert!(
        mean_per_path < 2.0,
        "AC-N1: without the linker, sparse infill must be the raw disjoint form          (mean G1 moves per path < 2.0, i.e. <= 2 points per path); got {mean_per_path:.3}          over {} paths (median {median}). If this is high, the linker is wired even though          its module-dir was excluded — the linked baseline is 3.271.",
        path_moves.len()
    );
}
