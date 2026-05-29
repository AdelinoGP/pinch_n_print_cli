//! DEV-044 — Paint data must reach `PaintSegmentation` on the live path.
//!
//! These tests are TDD-RED today. They lock in the contract that a
//! `.3mf` file with embedded paint metadata, sliced via the live
//! `pnp_cli` binary, produces GCode that observably differs from
//! the same model with no paint metadata. This is the user-reachable
//! surface for the `PrePass::PaintSegmentation` stage that the project
//! claims to have shipped via DEV-025 closure.
//!
//! Live-path evidence of the gap (2026-05-10):
//!   - `crates/slicer-runtime/src/model_loader.rs:150` unconditionally
//!     sets `ObjectMesh::paint_data = None`. `load_3mf` parses only
//!     `<vertex>` / `<triangle>` XML and silently discards every
//!     Bambu/Orca paint namespace (`custom_supports`, `paint_color`,
//!     `support_blocker`, `seam_painting`).
//!   - The CLI surface on `pnp_cli` (and on the dev `slicer-cli`)
//!     has no paint flag. There is no codepath that ever produces a
//!     non-`None` `FacetPaintData` on the live binary path.
//!
//! Closure expectation: once Packet 50 (`paint-input-3mf-ingestion`)
//! lands, `load_3mf` will parse paint metadata and `paint_data` will
//! cross the host loader into the prepass pipeline. These tests will
//! then go GREEN.
//!
//! Fixture requirement: `resources/benchy_painted.3mf` (a Benchy with
//! at least one painted facet cluster — e.g. the smokestack — and
//! valid Orca-compatible paint metadata). The fixture is NOT in the
//! repo today; Packet 50 Step 2 commits it. Until then the tests fail
//! cleanly on "fixture missing".

#![allow(missing_docs)]

mod common;

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn painted_benchy_3mf() -> PathBuf {
    repo_root().join("resources/benchy_painted.3mf")
}

fn unpainted_benchy_stl() -> PathBuf {
    repo_root().join("resources/benchy.stl")
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

/// FAILING RED — fixture is not committed until Packet 50 Step 2.
#[test]
fn painted_3mf_fixture_is_committed() {
    let p = painted_benchy_3mf();
    assert!(
        p.exists(),
        "DEV-044 RED: painted-Benchy 3MF fixture missing at {}. \
         Packet 50 Step 2 must commit a Benchy 3MF with embedded \
         Orca-compatible paint metadata on a clearly identifiable \
         facet cluster (recommended: the smokestack triangles, \
         painted with a `PaintSemantic::FuzzySkin` semantic).",
        p.display()
    );
}

// ---------------------------------------------------------------------------
// E2E: paint data must reach the live pipeline.
// ---------------------------------------------------------------------------

/// FAILING RED — paint metadata is currently discarded by `load_3mf`,
/// so the GCode for the painted 3MF will be byte-identical to the
/// GCode for the unpainted STL (modulo trivial header differences).
/// When Packet 50 + Packet 51 land, the painted slice will produce
/// observably different output (different perimeter / fill / seam
/// behavior on the painted facet cluster), and this assertion flips
/// GREEN.
#[test]
fn painted_benchy_3mf_reaches_paint_segmentation() {
    let painted = painted_benchy_3mf();
    let unpainted = unpainted_benchy_stl();
    assert!(
        painted.exists(),
        "DEV-044 RED: painted 3MF fixture missing at {} — blocked by \
         the prerequisite test `painted_3mf_fixture_is_committed`",
        painted.display()
    );
    assert!(
        unpainted.exists(),
        "unpainted Benchy STL missing at {}",
        unpainted.display()
    );

    let painted_cached = common::slicer_cache::cached_run(
        &painted,
        common::slicer_cache::ModuleDirKind::CoreModules,
        None,
    );
    let painted_outcome = common::slicer_cache::expect_outcome(&painted_cached);
    assert!(
        painted_outcome.success,
        "pnp_cli must succeed on painted 3MF; exit_code={:?}\nstderr:\n{}",
        painted_outcome.exit_code, painted_outcome.stderr
    );

    let unpainted_cached = common::slicer_cache::cached_run(
        &unpainted,
        common::slicer_cache::ModuleDirKind::CoreModules,
        None,
    );
    let unpainted_outcome = common::slicer_cache::expect_outcome(&unpainted_cached);
    assert!(
        unpainted_outcome.success,
        "pnp_cli must succeed on unpainted STL; exit_code={:?}\nstderr:\n{}",
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
        "DEV-044 RED: painted-Benchy GCode is byte-identical to \
         unpainted-Benchy GCode after normalization. This is the \
         signature of paint metadata being discarded by the host \
         loader. Confirm via: rg 'paint_data: None' in \
         crates/slicer-runtime/src/model_loader.rs. Closure: Packet 50 \
         wires load_3mf paint extraction; this assertion should then \
         pass (paint-driven divergence in regions covered by the \
         smokestack painted cluster)."
    );
}
