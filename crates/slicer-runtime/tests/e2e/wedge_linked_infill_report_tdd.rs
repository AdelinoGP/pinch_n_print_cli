//! Packet 136 — AC-3 wedge_linked_infill_report.
//!
//! Slices `resources/regression_wedge.stl` with `--report`, asserts the HTML
//! report is written, and the committed gcode-level sparse infill output is
//! linked (mean G1 moves per sparse-infill block is well above the raw
//! 2-point baseline of 1). The visual confirmation of linked paths (no
//! disjoint-segment travel storms) is recorded in the closure log; this
//! test gates the geometry signal.
//!
//! Authoritative pipe command:
//!   `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report`

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn pnp_cli_bin() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let profile = if std::env::var("PROFILE").as_deref() == Ok("release") {
        "release"
    } else {
        "debug"
    };
    PathBuf::from(manifest)
        .join("..")
        .join("..")
        .join("target")
        .join(profile)
        .join(if cfg!(windows) {
            "pnp_cli.exe"
        } else {
            "pnp_cli"
        })
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules").join("core-modules")
}

fn wedge_stl() -> PathBuf {
    repo_root().join("resources").join("regression_wedge.stl")
}

fn report_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("target")
        .join("wedge_linked_infill_report.html")
}

fn gcode_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("target")
        .join("wedge_linked_infill_report.gcode")
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

#[test]
fn wedge_linked_infill_report() {
    let bin = pnp_cli_bin();
    let model = wedge_stl();
    let modules = core_modules_dir();
    let report = report_path();
    let gcode = gcode_path();
    assert!(bin.exists(), "pnp_cli not built at {}", bin.display());
    assert!(
        model.exists(),
        "regression_wedge.stl missing at {}",
        model.display()
    );
    assert!(
        modules.exists(),
        "core-modules dir missing at {}",
        modules.display()
    );

    let _ = std::fs::remove_file(&report);
    let _ = std::fs::remove_file(&gcode);

    let proc = Command::new(&bin)
        .args(["slice", "--model"])
        .arg(&model)
        .args(["--output"])
        .arg(&gcode)
        .args(["--report"])
        .arg(&report)
        .args(["--module-dir"])
        .arg(&modules)
        .output()
        .expect("pnp_cli binary should execute");
    let stderr = String::from_utf8_lossy(&proc.stderr);
    assert!(
        proc.status.success(),
        "pnp_cli must succeed for the wedge linked-infill report run. Stderr:\n{stderr}"
    );

    // (a) HTML report exists (AC-3 "the HTML report exists").
    assert!(
        report.exists(),
        "report file not written at {}",
        report.display()
    );
    let html = std::fs::read_to_string(&report).expect("read report");
    assert!(
        html.contains("<html"),
        "report file should contain HTML markup"
    );

    // (b) Gcode was written.
    assert!(
        gcode.exists(),
        "gcode file not written at {}",
        gcode.display()
    );
    let gcode_text = std::fs::read_to_string(&gcode).expect("read gcode");

    // (c) Gcode-level linkage signal: the committed `InfillIR` post-linker
    // contains linked sparse polylines. Gcode proxy: mean G1 extrusion moves
    // per `;TYPE:Sparse infill` block. Raw 2-point rectilinear output would
    // give mean ~ 1.0; linked output gives mean >> 1.0.
    let sparse_moves = parse_sparse_infill_g1_moves(&gcode_text);
    assert!(
        sparse_moves.len() >= 2,
        "wedge slice must produce at least 2 sparse-infill blocks (got {})",
        sparse_moves.len()
    );
    // (c) Per-bucket linkage: every `;TYPE:Sparse infill` block has at
    // least 2 G1 extrusion moves. A 2-point raw path is 1 G1 move; linked
    // output chains segments, raising the count. This is the gcode proxy
    // for the IR-level `points_per_path > 2` check the spec calls for.
    //
    // IR-level verification is OUT OF SCOPE for the e2e binary: the
    // `run_slice` API returns only the gcode text, and `InfillIR` is only
    // exposed via the in-process `run_pipeline_with_instrumentation` which
    // requires custom `PipelineStageRunners` and does NOT use the real
    // core-modules WASM. The closest IR-level assertion lives in the
    // linker module's own tests (packet 133). The gcode proxy proves the
    // linker is wired in the real pipeline; per-bucket coverage catches
    // raw 2-point regression.
    for (k, moves) in sparse_moves.iter().enumerate() {
        assert!(
            *moves >= 2,
            "AC-3: sparse-infill block {k} has only {moves} G1 moves; \
             raw 2-point output would have 1. Block counts: {sparse_moves:?}"
        );
    }
}
