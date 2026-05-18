//! Integration tests for packet-60: configurable slicing precision.
//!
//! Slices `resources/test_stl/ASCII/20mmbox-LF.stl` twice — once with default
//! precision (all 7 packet-60 keys at their OrcaSlicer defaults) and once with
//! all 7 keys at their legacy zero-cost values — and asserts:
//!
//! - AC-10  (`default_emits_fewer_lines_than_legacy`): default G1 XY line count
//!   is strictly less than legacy by ≥ 5%.
//! - NEG-2  (`legacy_zero_matches_golden`): legacy output is byte-identical to a
//!   pre-recorded golden file at
//!   `crates/slicer-host/tests/fixtures/golden/precision_legacy_20mmbox.gcode`.
//!
//! # Golden regeneration
//!
//! If the legacy golden file needs to be (re-)recorded, run:
//!
//! ```text
//! BLESS_GOLDEN=1 cargo test -p slicer-host --test slicing_precision_integration_tdd -- legacy_zero_matches_golden --nocapture
//! ```
//!
//! Legacy mode = all 7 packet-60 keys at their legacy values (zero-cost path).
//! If this golden breaks in a future packet, either restore byte-identity for
//! legacy mode or update the golden with a documented justification.

#![allow(missing_docs)]

mod common;

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn fixture_stl() -> PathBuf {
    repo_root().join("resources/test_stl/ASCII/20mmbox-LF.stl")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/precision_legacy_20mmbox.gcode")
}

fn core_modules_dir() -> PathBuf {
    common::slicer_cache::module_dir_path(&common::slicer_cache::ModuleDirKind::CoreModules)
}

/// Default-precision config JSON: all 7 packet-60 keys at OrcaSlicer defaults.
const DEFAULT_PRECISION_JSON: &str = r#"{
  "gcode_resolution": 0.0125,
  "infill_resolution": 0.04,
  "support_resolution": 0.0375,
  "min_segment_length": 0.05,
  "gcode_xy_decimals": 3,
  "perimeter_arc_tolerance": 0.0125,
  "slice_closing_radius": 0.049
}"#;

/// Legacy zero-cost config JSON: all 7 packet-60 keys at legacy values.
const LEGACY_PRECISION_JSON: &str = r#"{
  "gcode_resolution": 0.0,
  "infill_resolution": 0.0,
  "support_resolution": 0.0,
  "min_segment_length": 0.0,
  "gcode_xy_decimals": 4,
  "perimeter_arc_tolerance": 0.0,
  "slice_closing_radius": 0.0
}"#;

/// Count G1 lines that contain both an X token and a Y token (XY move lines).
fn count_g1_xy_lines(gcode: &str) -> usize {
    gcode
        .lines()
        .filter(|l| {
            let l = l.trim();
            l.starts_with("G1")
                && l.split_whitespace().any(|t| t.starts_with('X'))
                && l.split_whitespace().any(|t| t.starts_with('Y'))
        })
        .count()
}

/// Run the slicer-host binary with a given config and return the G-code bytes.
fn run_with_config(config_json: &str) -> Vec<u8> {
    let stl = fixture_stl();
    assert!(stl.exists(), "fixture STL missing at {}", stl.display());

    let tmp = tempfile::tempdir().expect("tempdir for precision test");
    let cfg_path = tmp.path().join("precision_config.json");
    std::fs::write(&cfg_path, config_json.as_bytes()).expect("write config JSON");

    let out_path = tmp.path().join("out.gcode");
    let module_dir = core_modules_dir();

    let proc_out = common::slicer_cache::run_slicer_host_uncached(
        &stl,
        &module_dir,
        &out_path,
        Some(&cfg_path),
    );

    assert!(
        proc_out.status.success(),
        "slicer-host exited non-zero ({}); stderr:\n{}",
        proc_out.status,
        String::from_utf8_lossy(&proc_out.stderr)
    );
    assert!(
        out_path.exists(),
        "slicer-host did not write output file at {}",
        out_path.display()
    );

    std::fs::read(&out_path).expect("read output gcode")
}

// ---------------------------------------------------------------------------
// AC-10 — default G1 XY line count < legacy by ≥ 5%
// ---------------------------------------------------------------------------

/// AC-10: Slicing with default precision (D-P + min-segment active) emits
/// strictly fewer G1 XY lines than slicing with legacy zero-cost config, by
/// at least 5%.
///
/// This proves that the seven packet-60 config keys actually drive simplification
/// through the emit path when set to their OrcaSlicer defaults.
#[test]
fn default_emits_fewer_lines_than_legacy() {
    let default_bytes = run_with_config(DEFAULT_PRECISION_JSON);
    let legacy_bytes = run_with_config(LEGACY_PRECISION_JSON);

    let default_gcode = std::str::from_utf8(&default_bytes).expect("default gcode is utf-8");
    let legacy_gcode = std::str::from_utf8(&legacy_bytes).expect("legacy gcode is utf-8");

    let default_count = count_g1_xy_lines(default_gcode);
    let legacy_count = count_g1_xy_lines(legacy_gcode);

    // If the pipeline emits no XY moves at all (e.g. empty-module-dir stub path
    // with no geometry), both counts will be 0 and the test would trivially
    // "pass" but prove nothing. Guard against that degenerate case.
    assert!(
        legacy_count >= 10,
        "AC-10 BLOCKED: legacy G-code has only {legacy_count} G1 XY lines — \
         the pipeline may not be emitting real geometry. \
         Check that the 20mmbox STL slices correctly and emits perimeter moves. \
         Legacy gcode preview:\n{}",
        legacy_gcode.lines().take(30).collect::<Vec<_>>().join("\n")
    );

    // AC-10: default_count ≤ floor(legacy_count * 0.95)
    let threshold = (legacy_count as f64 * 0.95) as usize;
    assert!(
        default_count <= threshold,
        "AC-10 FAILED: default G1 XY count ({default_count}) is not ≥ 5% less than \
         legacy count ({legacy_count}, threshold ≤ {threshold}). \
         The D-P simplification or min-segment filter is not reducing the polyline \
         through the emit path for the default-precision config."
    );
}

// ---------------------------------------------------------------------------
// NEG-2 — legacy output is byte-identical to golden
// ---------------------------------------------------------------------------

/// NEG-2: Legacy-mode output must be byte-identical to the pre-recorded golden.
///
/// This proves zero-impact of the packet-60 precision keys on the legacy
/// (zero-cost) configuration path — any byte difference indicates a regression
/// in the legacy path.
///
/// To record the golden for the first time (or re-record after a justified change):
/// ```text
/// BLESS_GOLDEN=1 cargo test -p slicer-host --test slicing_precision_integration_tdd -- legacy_zero_matches_golden --nocapture
/// ```
#[test]
fn legacy_zero_matches_golden() {
    let legacy_bytes = run_with_config(LEGACY_PRECISION_JSON);

    let golden = golden_path();
    let bless = std::env::var("BLESS_GOLDEN")
        .map(|v| v == "1")
        .unwrap_or(false);

    if bless {
        // Bless mode: write the golden once, then assert it exists.
        if let Some(parent) = golden.parent() {
            std::fs::create_dir_all(parent).expect("create golden dir");
        }
        std::fs::write(&golden, &legacy_bytes).expect("write golden file");
        println!(
            "NEG-2 BLESSED: golden written to {}  ({} bytes)",
            golden.display(),
            legacy_bytes.len()
        );
        return;
    }

    assert!(
        golden.exists(),
        "NEG-2 BLOCKED: golden file missing at {}. \
         Run with BLESS_GOLDEN=1 to record it:\n  \
         BLESS_GOLDEN=1 cargo test -p slicer-host --test slicing_precision_integration_tdd \
         -- legacy_zero_matches_golden --nocapture",
        golden.display()
    );

    let golden_bytes = std::fs::read(&golden).expect("read golden file");

    assert_eq!(
        legacy_bytes,
        golden_bytes,
        "NEG-2 FAILED: legacy-mode G-code is not byte-identical to the golden at {}. \
         The legacy (zero-cost) path has changed. \
         If this is intentional, re-bless with BLESS_GOLDEN=1 and document the justification.",
        golden.display()
    );
}
