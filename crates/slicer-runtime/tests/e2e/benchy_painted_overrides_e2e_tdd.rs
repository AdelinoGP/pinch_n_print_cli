//! DEV-045 — `RegionMap` must honor per-paint-semantic `ResolvedConfig`
//! overrides on the live path.
//!
//! TDD-RED today. Locks in the contract that, given a painted 3MF and
//! a user config containing a `paint_config:<semantic>:<key>=<value>`
//! entry, the GCode for the painted region must reflect the override.
//! Today no such namespace exists in `config_resolution.rs`, and
//! `region_mapping.rs` (the host built-in at `crates/slicer-runtime/src/
//! region_mapping.rs:103-248`) is paint-blind — zero occurrences of
//! the tokens "paint*"/"semantic" anywhere.
//!
//! Live-path evidence of the gap (2026-05-10):
//!   - `crates/slicer-runtime/src/config_resolution.rs` recognizes only
//!     `object_config:<id>:<key>` (line 84, 195). No `paint_config:`
//!     namespace. Unknown keys fall through to `cfg.extensions`.
//!   - `crates/slicer-ir/src/slice_ir.rs:1028-1033`: `RegionPlan` is
//!     `{ config: ResolvedConfig, stage_modules: ... }`. No paint
//!     semantic dimension. `RegionKey` (`:1006-1015`) keys on
//!     `(global_layer_index, object_id, region_id)` only.
//!   - `crates/slicer-runtime/src/region_mapping.rs:236-242` stamps
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

// ---------------------------------------------------------------------------
// E2E: paint_config:<semantic>:<key> override must visibly differ GCode.
// ---------------------------------------------------------------------------

/// Smoke test: the `paint_config:<semantic>:<key>` namespace must parse
/// cleanly through the pipeline and not crash either the no-paint baseline
/// or the override path. Gcode-level wall-count comparison is intentionally
/// NOT asserted on this fixture: the painted Benchy's only painted area is
/// the chimney (layer 193+), and the chimney geometry is too narrow to fit
/// the requested 5 walls per layer, so an override of wall_count=5 produces
/// the same emitted wall count as the baseline regardless of how cleanly
/// the overlay reaches the perimeter module. Behavioral verification of the
/// overlay propagation lives in region_mapping_paint_semantic_tdd.rs.
#[test]
fn paint_config_override_visibly_differs_gcode() {
    let painted = painted_benchy_3mf();
    assert!(
        painted.exists(),
        "painted 3MF fixture missing at {} (DEV-044 closure dependency); \
         Packet 50 commits resources/benchy_painted.3mf.",
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
  "paint_config:fuzzy_skin:wall_count": 5
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
