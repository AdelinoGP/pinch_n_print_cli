//! AC-5 / AC-6 / AC-7 — Phase 5 (`cut_segmented_layers`) integration gates.
//!
//! These slice `cube_4color.3mf` with non-default MMU config injected via a JSON
//! config file and assert the geometric effect of Phase 5:
//!   - AC-5: `mmu_segmented_region_max_width` erodes painted variant regions, so
//!     the geometry changes versus the default (un-eroded) slice.
//!   - AC-6: `mmu_segmented_region_interlocking_depth` erodes EVEN layers only,
//!     so it differs both from the default AND from a uniform width erosion of
//!     every layer (the even/odd alternation in the kernel).
//!   - AC-7: `mmu_segmented_region_interlocking_beam = true` skips Phase 5 at the
//!     driver, so the geometry is byte-identical to the default despite non-zero
//!     width/depth.
//!
//! Comparisons use MOTION lines only (`G0`/`G1`): setting a config key adds it to
//! the gcode CONFIG_BLOCK header, which would change a whole-file hash even when
//! the toolpath is identical (AC-7), so the header is intentionally excluded.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_runtime::{run_slice, SliceRunOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// Slice `cube_4color.3mf` with an optional JSON config override; return gcode.
/// `name` disambiguates the temporary config file per call.
fn slice_cube(name: &str, config_json: Option<&str>) -> String {
    let model = cube_4color_path();
    assert!(model.exists(), "fixture missing: {}", model.display());

    let config_path = config_json.map(|json| {
        let p = std::env::temp_dir().join(format!("p96_phase5_{name}.json"));
        std::fs::write(&p, json).expect("write temp config");
        p
    });

    let mesh = Arc::new(
        slicer_model_io::load_model(&model)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model.display())),
    );
    let opts = SliceRunOptions {
        mesh,
        model_label: "cube_4color".to_string(),
        config_path,
        output_path: None,
        module_dirs: vec![core_modules_dir()],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        progress_events: false,
        cancel_flag: None,
        config_overrides: std::collections::HashMap::new(),
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed: {e}"))
        .gcode_text
}

/// Toolpath motion lines only (`G0`/`G1`), excluding header/config/comments.
fn motion(gcode: &str) -> Vec<String> {
    gcode
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("G0 ") || l.starts_with("G1 "))
        .map(|l| l.to_string())
        .collect()
}

/// AC-5 — width limiting erodes painted variant regions (geometry changes).
#[test]
fn cube_4color_phase5_width_limit_bands() {
    let default_g = slice_cube("ac5_default", None);
    let width_g = slice_cube(
        "ac5_width2",
        Some(r#"{"mmu_segmented_region_max_width": 2.0}"#),
    );

    assert_ne!(
        motion(&default_g),
        motion(&width_g),
        "mmu_segmented_region_max_width=2.0 must change the sliced toolpath: \
         Phase 5 erodes each painted variant region by the configured width."
    );
}

/// AC-6 — interlocking depth erodes EVEN layers only (alternation), so it differs
/// from both the default slice AND a uniform all-layer width erosion.
#[test]
fn cube_4color_phase5_interlocking_alternates() {
    let default_g = slice_cube("ac6_default", None);
    let depth_g = slice_cube(
        "ac6_depth05",
        Some(r#"{"mmu_segmented_region_interlocking_depth": 0.5}"#),
    );
    let width_g = slice_cube(
        "ac6_width05",
        Some(r#"{"mmu_segmented_region_max_width": 0.5}"#),
    );

    assert_ne!(
        motion(&default_g),
        motion(&depth_g),
        "interlocking_depth=0.5 must change the toolpath (Phase 5 active)."
    );
    assert_ne!(
        motion(&depth_g),
        motion(&width_g),
        "interlocking_depth (even layers only) must differ from uniform width \
         (all layers) — proving the even/odd alternation in cut_segmented_layers."
    );
}

/// AC-7 — `interlocking_beam = true` skips Phase 5 at the driver: geometry is
/// byte-identical to the default despite non-zero width/depth.
#[test]
fn cube_4color_phase5_interlocking_beam_skips_phase5() {
    let default_g = slice_cube("ac7_default", None);
    let beam_g = slice_cube(
        "ac7_beam",
        Some(
            r#"{"mmu_segmented_region_max_width": 2.0, "mmu_segmented_region_interlocking_depth": 0.5, "mmu_segmented_region_interlocking_beam": true}"#,
        ),
    );

    assert_eq!(
        motion(&default_g),
        motion(&beam_g),
        "interlocking_beam=true must skip Phase 5 at the driver, so the toolpath \
         is identical to the default slice even with non-zero width/depth \
         (OrcaSlicer parity, MultiMaterialSegmentation.cpp:2452)."
    );
}
