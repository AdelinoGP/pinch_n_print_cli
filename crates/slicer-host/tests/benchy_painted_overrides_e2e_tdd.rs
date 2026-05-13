//! DEV-045 — `RegionMap` must honor per-paint-semantic `ResolvedConfig`
//! overrides on the live path.
//!
//! TDD-RED today. Locks in the contract that, given a painted 3MF and
//! a user config containing a `paint_config:<semantic>:<key>=<value>`
//! entry, the GCode for the painted region must reflect the override.
//! Today no such namespace exists in `config_resolution.rs`, and
//! `region_mapping.rs` (the host built-in at `crates/slicer-host/src/
//! region_mapping.rs:103-248`) is paint-blind — zero occurrences of
//! the tokens "paint*"/"semantic" anywhere.
//!
//! Live-path evidence of the gap (2026-05-10):
//!   - `crates/slicer-host/src/config_resolution.rs` recognizes only
//!     `object_config:<id>:<key>` (line 84, 195). No `paint_config:`
//!     namespace. Unknown keys fall through to `cfg.extensions`.
//!   - `crates/slicer-ir/src/slice_ir.rs:1028-1033`: `RegionPlan` is
//!     `{ config: ResolvedConfig, stage_modules: ... }`. No paint
//!     semantic dimension. `RegionKey` (`:1006-1015`) keys on
//!     `(global_layer_index, object_id, region_id)` only.
//!   - `crates/slicer-host/src/region_mapping.rs:236-242` stamps
//!     configs per-object; never reads `PaintRegionIR`.
//!
//! Closure expectation: Packet 51 (`paint-semantic-region-overrides`)
//! extends `ResolvedConfig` resolution, `RegionPlan` shape, and the
//! host built-in `region_mapping.rs` so that a config payload like:
//!   `{ "perimeter_count": 2, "paint_config:fuzzy_skin:perimeter_count": 5 }`
//! produces different perimeter loop counts on regions covered by the
//! `fuzzy_skin` paint semantic vs unpainted regions.
//!
//! Depends on DEV-044 closure (Packet 50) for the painted fixture.

#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Command;

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

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

fn run_slicer_host_with_config(
    model: &Path,
    module_dir: &Path,
    output: &Path,
    config: &Path,
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_slicer-host");
    let dummy_module = model;
    Command::new(bin)
        .args([
            "run",
            "--module",
            dummy_module.to_str().unwrap(),
            "--model",
            model.to_str().unwrap(),
            "--module-dir",
            module_dir.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .expect("slicer-host binary should execute")
}

/// Count OrcaSlicer-style perimeter loop markers in a GCode file
/// limited to a Z-band.
///
/// CORRECTED (packet 51): The original marker literals (`;TYPE:Perimeter`,
/// `;TYPE:OuterWall`, `;TYPE:Wall Outer`) did not match the actual emitter
/// output in `crates/slicer-host/src/gcode_emit.rs`, which produces
/// `;TYPE:Outer wall` and `;TYPE:Inner wall` (Orca-style). Both are counted
/// so that a perimeter_count bump from 2 → 5 produces a clear delta via the
/// extra inner wall passes.
///
/// The Z-band was also corrected: the painted Benchy smokestack sits at
/// Z ≈ 40–48 mm (not 50–72 mm as originally written). See
/// `resources/benchy_painted.README.md` lines 21-22.
///
/// The Z-band is selected by walking `;Z:<f>` (or `G1 Z<f>`) markers and
/// counting only loop markers between min_z (inclusive) and max_z (exclusive).
fn count_perimeter_markers_in_z_band(gcode: &str, min_z_mm: f32, max_z_mm: f32) -> usize {
    let mut in_band = false;
    let mut count = 0usize;
    for line in gcode.lines() {
        if let Some(z_str) = line.strip_prefix(";Z:") {
            if let Ok(z) = z_str.trim().parse::<f32>() {
                in_band = z >= min_z_mm && z < max_z_mm;
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix("G1 Z") {
            if let Some(z_tok) = rest.split_whitespace().next() {
                if let Ok(z) = z_tok.parse::<f32>() {
                    in_band = z >= min_z_mm && z < max_z_mm;
                    continue;
                }
            }
        }
        if !in_band {
            continue;
        }
        if line.contains(";TYPE:Outer wall") || line.contains(";TYPE:Inner wall") {
            count += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// E2E: paint_config:<semantic>:<key> override must visibly differ GCode.
// ---------------------------------------------------------------------------

/// FAILING RED — DEV-045 closure is gated on Packet 51 landing the
/// `paint_config:<semantic>:<key>` namespace AND on Packet 50 landing
/// the painted Benchy fixture (DEV-044). Both must close before this
/// test reaches GREEN.
#[test]
fn paint_config_override_visibly_differs_gcode() {
    let painted = painted_benchy_3mf();
    assert!(
        painted.exists(),
        "DEV-045 RED: painted 3MF fixture missing at {} (DEV-044 not yet closed). \
         Closure dependency: Packet 50 commits resources/benchy_painted.3mf.",
        painted.display()
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    let modules = core_modules_dir();

    // Baseline config: globally 2 perimeters.
    let baseline_cfg_path = tmp.path().join("baseline.json");
    std::fs::write(
        &baseline_cfg_path,
        br#"{
  "wall_count": 2
}"#,
    )
    .expect("write baseline config");

    // Override config: globally 2 perimeters, but 5 perimeters for any
    // region carrying the `fuzzy_skin` paint semantic.
    let override_cfg_path = tmp.path().join("override.json");
    std::fs::write(
        &override_cfg_path,
        br#"{
  "wall_count": 2,
  "paint_config:fuzzy_skin:wall_count": 5
}"#,
    )
    .expect("write override config");

    let baseline_out = tmp.path().join("baseline.gcode");
    let override_out = tmp.path().join("override.gcode");

    let s1 = run_slicer_host_with_config(&painted, &modules, &baseline_out, &baseline_cfg_path);
    assert!(
        s1.status.success(),
        "baseline slice must succeed; stderr:\n{}",
        String::from_utf8_lossy(&s1.stderr)
    );
    let s2 = run_slicer_host_with_config(&painted, &modules, &override_out, &override_cfg_path);
    assert!(
        s2.status.success(),
        "override slice must succeed (paint_config namespace must parse cleanly); stderr:\n{}",
        String::from_utf8_lossy(&s2.stderr)
    );

    let baseline_gcode = std::fs::read_to_string(&baseline_out).expect("read baseline gcode");
    let override_gcode = std::fs::read_to_string(&override_out).expect("read override gcode");

    // Z-band corrected (packet 51):
    //   (a) The painted Benchy 3MF geometry occupies Z=[−24, +24] in model
    //       space; after slicing (Z >= 0), the live slice range is [0, 24].
    //       The previous value of (40.0, 48.0) was dead space — no layers
    //       are emitted there — so baseline_loops and override_loops were
    //       both 0 and assert_ne! always failed.
    //   (b) The 3MF <build>/<item> transform carries a +24 mm Z-translation
    //       that the current model_loader.rs does NOT apply.  That is a
    //       pre-existing deviation in the 3MF loader and is OUT OF SCOPE for
    //       Packet 51 (this packet does not own model_loader.rs).  It should
    //       be filed as a follow-up deviation against Packet 50b-rev or as
    //       its own dedicated packet.
    let (z_lo, z_hi) = (0.2_f32, 24.0_f32);
    let baseline_loops = count_perimeter_markers_in_z_band(&baseline_gcode, z_lo, z_hi);
    let override_loops = count_perimeter_markers_in_z_band(&override_gcode, z_lo, z_hi);

    // Today both numbers are equal (paint_config is silently dropped
    // into `cfg.extensions` and never consulted by region_mapping).
    // After Packet 51 closure, the override band carries 5 perimeters
    // per layer where the baseline carries 2.
    assert_ne!(
        baseline_loops, override_loops,
        "DEV-045 RED: paint_config:fuzzy_skin:perimeter_count had no observable effect on GCode \
         (baseline_loops={baseline_loops}, override_loops={override_loops} in Z=[{z_lo}, {z_hi}]). \
         Closure: Packet 51 must (1) recognize `paint_config:<semantic>:<key>` in \
         crates/slicer-host/src/config_resolution.rs, (2) extend RegionPlan with \
         paint_overrides, (3) make region_mapping.rs read PaintRegionIR and stamp \
         per-semantic overrides, (4) make Layer-tier modules honor paint_overrides."
    );
    assert!(
        override_loops > baseline_loops,
        "DEV-045 RED: paint_config override must INCREASE perimeter loop count on painted band, \
         got baseline={baseline_loops}, override={override_loops}."
    );
}
