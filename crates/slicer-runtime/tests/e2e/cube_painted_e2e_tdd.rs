//! DEV-044 — Paint data must reach `PaintSegmentation` on the live path.
//!
//! Fixtures: cube_4color.3mf (painted side) and 20mm_cube.obj (unpainted
//! comparator — same 20 mm cube geometry as the 3MF interior, just without
//! the four per-face paint strokes).
//!
//! These tests lock in the contract that a `.3mf` file with embedded paint
//! metadata, sliced via the live `pnp_cli` binary, produces GCode that
//! observably differs from the same model with no paint metadata. This is
//! the user-reachable surface for the `PrePass::PaintSegmentation` stage.

#![allow(missing_docs)]

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn cube_4color_3mf() -> PathBuf {
    repo_root().join("resources/cube_4color.3mf")
}

fn unpainted_cube_obj() -> PathBuf {
    repo_root().join("resources/20mm_cube.obj")
}

/// Strip volatile content (timestamps, run-ids, etc.) from emitted
/// GCode so byte-level diffs reflect real geometric / extrusion
/// divergence rather than nondeterministic header fields.
fn normalize_gcode(gcode: &str) -> String {
    gcode
        .lines()
        .filter(|line| {
            // Drop common volatile comment families. Production emitter
            // will be the authoritative source; trim here is best-effort.
            !line.starts_with("; generated_at")
                && !line.starts_with("; run_id")
                && !line.starts_with("; slicer_host_version")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Pre-flight: fixture must exist before E2E paint claims can be verified.
// ---------------------------------------------------------------------------

#[test]
fn painted_3mf_fixture_is_committed() {
    let p = cube_4color_3mf();
    assert!(
        p.exists(),
        "painted cube fixture missing at {}. \
         resources/cube_4color.3mf is the 4-color painted-cube fixture.",
        p.display()
    );
}

// ---------------------------------------------------------------------------
// E2E: paint data must reach the live pipeline.
// ---------------------------------------------------------------------------

/// The painted cube must produce GCode that differs observably from the
/// unpainted equivalent cube. If paint metadata is silently discarded by
/// `load_3mf`, both slices produce byte-identical output after normalization.
#[test]
fn painted_cube_3mf_reaches_paint_segmentation() {
    let painted = cube_4color_3mf();
    let unpainted = unpainted_cube_obj();
    assert!(
        painted.exists(),
        "painted cube 3MF fixture missing at {} — blocked by \
         the prerequisite test `painted_3mf_fixture_is_committed`",
        painted.display()
    );
    assert!(
        unpainted.exists(),
        "unpainted cube OBJ missing at {}",
        unpainted.display()
    );

    let painted_cached = crate::common::slicer_cache::cached_run(
        &painted,
        crate::common::slicer_cache::ModuleDirKind::CoreModules,
        None,
    );
    let painted_outcome = crate::common::slicer_cache::expect_outcome(&painted_cached);
    assert!(
        painted_outcome.success,
        "pnp_cli must succeed on painted cube 3MF; exit_code={:?}\nstderr:\n{}",
        painted_outcome.exit_code, painted_outcome.stderr
    );

    let unpainted_cached = crate::common::slicer_cache::cached_run(
        &unpainted,
        crate::common::slicer_cache::ModuleDirKind::CoreModules,
        None,
    );
    let unpainted_outcome = crate::common::slicer_cache::expect_outcome(&unpainted_cached);
    assert!(
        unpainted_outcome.success,
        "pnp_cli must succeed on unpainted cube OBJ; exit_code={:?}\nstderr:\n{}",
        unpainted_outcome.exit_code, unpainted_outcome.stderr
    );

    let painted_gcode = normalize_gcode(&painted_outcome.gcode);
    let unpainted_gcode = normalize_gcode(&unpainted_outcome.gcode);

    assert!(
        !painted_gcode.is_empty() && !unpainted_gcode.is_empty(),
        "both GCode outputs must be non-empty"
    );

    // The load-bearing assertion: paint MUST have an effect. If
    // paint_data is silently discarded by load_3mf (DEV-044 open
    // state), both slices produce byte-identical output.
    assert_ne!(
        painted_gcode, unpainted_gcode,
        "Packet 89: painted-cube GCode is byte-identical to unpainted-cube \
         GCode after normalization. This is the signature of paint metadata \
         being discarded by the host loader. Confirm via: rg 'paint_data: None' \
         in crates/slicer-runtime/src/model_loader.rs."
    );
}
