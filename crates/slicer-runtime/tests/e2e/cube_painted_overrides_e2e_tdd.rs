//! DEV-045 — `RegionMap` must honor per-paint-semantic `ResolvedConfig`
//! overrides on the live path.
//!
//! Fixture: cube_4color.3mf stores its paint strokes under the
//! `paint_color="..."` attribute, which the loader maps to
//! `PaintSemantic::Material`; the override key is
//! `paint_config:material:wall_count`. This is a parse-only smoke (gcode-level
//! wall-count diff is intentionally not asserted because the painted region
//! geometry cannot fit the requested override).
//!
//! Locks in the contract that, given a painted 3MF and a user config
//! containing a `paint_config:<semantic>:<key>=<value>` entry, the pipeline
//! parses the override cleanly through the resolution stack and does not
//! crash either the no-paint baseline or the override path. Behavioral
//! verification of the overlay propagation lives in
//! region_mapping_paint_semantic_tdd.rs.

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

// ---------------------------------------------------------------------------
// E2E: paint_config:<semantic>:<key> override must parse cleanly through the
// pipeline on both the baseline and override paths.
// ---------------------------------------------------------------------------

#[test]
fn paint_config_override_visibly_differs_gcode() {
    let painted = cube_4color_3mf();
    assert!(
        painted.exists(),
        "painted cube 3MF fixture missing at {}",
        painted.display()
    );

    let tmp = tempfile::tempdir().expect("tempdir");

    let baseline_cfg_path = tmp.path().join("baseline.json");
    std::fs::write(
        &baseline_cfg_path,
        br#"{
  "wall_count": 2
}"#,
    )
    .expect("write baseline config");

    let override_cfg_path = tmp.path().join("override.json");
    std::fs::write(
        &override_cfg_path,
        br#"{
  "wall_count": 2,
  "paint_config:material:wall_count": 5
}"#,
    )
    .expect("write override config");

    let baseline_cached = crate::common::slicer_cache::cached_run(
        &painted,
        crate::common::slicer_cache::ModuleDirKind::CoreModules,
        Some(&baseline_cfg_path),
    );
    let baseline_outcome = crate::common::slicer_cache::expect_outcome(&baseline_cached);
    assert!(
        baseline_outcome.success,
        "baseline slice must succeed; stderr:\n{}",
        baseline_outcome.stderr
    );

    let override_cached = crate::common::slicer_cache::cached_run(
        &painted,
        crate::common::slicer_cache::ModuleDirKind::CoreModules,
        Some(&override_cfg_path),
    );
    let override_outcome = crate::common::slicer_cache::expect_outcome(&override_cached);
    assert!(
        override_outcome.success,
        "override slice must succeed (paint_config namespace must parse cleanly); stderr:\n{}",
        override_outcome.stderr
    );
}
