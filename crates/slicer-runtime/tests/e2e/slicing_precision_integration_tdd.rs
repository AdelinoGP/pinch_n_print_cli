//! Integration tests for packet-60: configurable slicing precision.
//!
//! Slices `resources/test_stl/ASCII/20mmbox-LF.stl` under three precision
//! configurations and asserts:
//!
//! - RES-1  (`explicit_defaults_match_omitted_keys_byte_for_byte`): passing the
//!   7 packet-60 keys explicitly at exactly their `ResolvedConfig::default()`
//!   values produces byte-identical output to passing no config keys at all —
//!   overriding a declared key with its own default is an end-to-end no-op.
//! - DET-2  (`legacy_precision_run_is_deterministic`): two runs of the legacy
//!   zero-cost configuration produce byte-identical output.
//! - INV-3  (`legacy_slice_structural_invariants`): the legacy slice is
//!   structurally sound (analytic layer count, monotone Z-set, closed outer
//!   wall loops, E monotonicity, decimal-formatting contract).
//! - AC-10  (`default_emits_fewer_lines_than_legacy`): default G1 XY line count
//!   is strictly less than legacy by ≥ 5%.
//!
//! # History: the removed byte-golden
//!
//! Through packet 234a this file carried a fourth test,
//! `legacy_zero_matches_golden`, which required byte-identity against a frozen
//! full-G-code capture (`tests/fixtures/golden/precision_legacy_20mmbox.gcode`,
//! gated by `BLESS_GOLDEN=1`). It was re-blessed six times between 2026-07-02
//! and 2026-08-24 because every geometry packet legitimately shifts wall and
//! infill coordinates; the churn drowned the signal. It was removed on
//! 2026-08-25 in favor of RES-1 + DET-2 + INV-3: contract-level checks that
//! never need re-blessing. Detection of *geometric drift* is intentionally
//! delegated to the parity/invariant suites; this file proves the packet-60
//! plumbing contracts only.
//!
//! AC-10 uses its own `sparse_fill_holder = "gyroid-infill"` config pair
//! (`AC10_DEFAULT_PRECISION_JSON` / `AC10_LEGACY_PRECISION_JSON`) rather than
//! RES-1/DET-2's plain rectilinear-holder configs — see the earlier
//! precision-fixture correction in git history
//! for why straight rectilinear lines give D-P/min-segment simplification
//! nothing to reduce.
//!
//! S6b evidence (2026-08-25): section counts changed only at Internal solid infill
//! (4 -> 3) and Bridge (0 -> 1); all other counts and the 100-value Z set are identical.
//! The changed extrusion distribution is expected from packet 234a's spacing-source fix
//! and carrier-free extra-layer feature; no unexpected diff class was observed.

#![allow(missing_docs)]

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

fn core_module_dirs() -> Vec<PathBuf> {
    crate::common::slicer_cache::module_dir_paths(
        &crate::common::slicer_cache::ModuleDirKind::CoreModules,
    )
}

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

/// The same seven packet-60 keys at exactly the values
/// `ResolvedConfig::default()` supplies when the keys are omitted (mirrored
/// from the macro table in `crates/slicer-ir/src/resolved_config.rs`).
///
/// Paired with [`EMPTY_CONFIG_JSON`] in RES-1 to prove that overriding a
/// declared key with its own default value is a no-op through resolution →
/// modules → emit.
const EXPLICIT_DEFAULT_PRECISION_JSON: &str = r#"{
  "gcode_resolution": 0.0125,
  "infill_resolution": 0.04,
  "support_resolution": 0.0375,
  "min_segment_length": 0.05,
  "gcode_xy_decimals": 3,
  "perimeter_arc_tolerance": 0.0125,
  "slice_closing_radius": 0.049
}"#;

/// No keys at all: `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`)
/// seeds `ResolvedConfig::default()` and applies zero overrides, so the run
/// rides pure defaults.
const EMPTY_CONFIG_JSON: &str = r#"{}"#;

/// AC-10-only default-precision config: identical to [`LEGACY_PRECISION_JSON`]'s
/// sibling default-value set (all 7 packet-60 keys at OrcaSlicer defaults)
/// plus `sparse_fill_holder = "gyroid-infill"`.
///
/// On the plain rectilinear-holder path (straight corner-to-corner
/// lines, no curvature), D-P/min-segment simplification has nothing to
/// simplify, so `default_count == legacy_count` regardless of code
/// correctness — that regression was traced to `ea16e992` correctly fixing a
/// duplicate gyroid/lightning fill-emission bug that used to (accidentally)
/// give this fixture curved content. Forcing the gyroid holder restores
/// genuine curved geometry for AC-10 to measure simplification against,
/// without touching NEG-2's golden-backed rectilinear path.
const AC10_DEFAULT_PRECISION_JSON: &str = r#"{
  "gcode_resolution": 0.0125,
  "infill_resolution": 0.04,
  "support_resolution": 0.0375,
  "min_segment_length": 0.05,
  "gcode_xy_decimals": 3,
  "perimeter_arc_tolerance": 0.0125,
  "slice_closing_radius": 0.049,
  "sparse_fill_holder": "gyroid-infill"
}"#;

/// AC-10-only legacy-precision config: identical to [`LEGACY_PRECISION_JSON`]
/// plus `sparse_fill_holder = "gyroid-infill"`. See [`AC10_DEFAULT_PRECISION_JSON`].
const AC10_LEGACY_PRECISION_JSON: &str = r#"{
  "gcode_resolution": 0.0,
  "infill_resolution": 0.0,
  "support_resolution": 0.0,
  "min_segment_length": 0.0,
  "gcode_xy_decimals": 4,
  "perimeter_arc_tolerance": 0.0,
  "slice_closing_radius": 0.0,
  "sparse_fill_holder": "gyroid-infill"
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

/// Run the pnp_cli binary with a given config and return the G-code bytes.
fn run_with_config(config_json: &str) -> Vec<u8> {
    let stl = fixture_stl();
    assert!(stl.exists(), "fixture STL missing at {}", stl.display());

    let tmp = tempfile::tempdir().expect("tempdir for precision test");
    let cfg_path = tmp.path().join("precision_config.json");
    std::fs::write(&cfg_path, config_json.as_bytes()).expect("write config JSON");

    let out_path = tmp.path().join("out.gcode");
    let module_dirs = core_module_dirs();

    let proc_out = crate::common::slicer_cache::run_pnp_cli_uncached(
        &stl,
        &module_dirs,
        &out_path,
        Some(&cfg_path),
    );

    assert!(
        proc_out.status.success(),
        "pnp_cli exited non-zero ({}); stderr:\n{}",
        proc_out.status,
        String::from_utf8_lossy(&proc_out.stderr)
    );
    assert!(
        out_path.exists(),
        "pnp_cli did not write output file at {}",
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
///
/// Uses [`AC10_DEFAULT_PRECISION_JSON`] / [`AC10_LEGACY_PRECISION_JSON`]
/// (gyroid sparse-fill holder) rather than the plain rectilinear-holder
/// configs — straight corner-to-corner rectilinear lines have no
/// curvature for D-P/min-segment simplification to reduce, so this AC needs
/// genuinely curved content to be meaningful.
#[test]
fn default_emits_fewer_lines_than_legacy() {
    let default_bytes = run_with_config(AC10_DEFAULT_PRECISION_JSON);
    let legacy_bytes = run_with_config(AC10_LEGACY_PRECISION_JSON);

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
// RES-1 — explicit-defaults config is byte-identical to omitted keys
// ---------------------------------------------------------------------------

/// RES-1: Passing the seven packet-60 config keys explicitly at exactly their
/// `ResolvedConfig::default()` values must produce byte-identical *motion
/// output* to passing no config keys at all.
///
/// This proves that overriding a declared key with its own default value is a
/// no-op everywhere those keys act — through `resolve_global_config`
/// (`crates/slicer-scheduler/src/config_resolution.rs`), module dispatch, and
/// the emit path. All seven keys manifest only in motion lines (geometry
/// simplification/closing via the six numeric keys, coordinate formatting via
/// `gcode_xy_decimals`), so a motion-byte difference means a default drifted
/// from the mirror in [`EXPLICIT_DEFAULT_PRECISION_JSON`] or an override
/// stopped being value-transparent.
///
/// Comment lines are excluded because the trailing CONFIG_BLOCK deliberately
/// echoes invocation-supplied keys (`run_pipeline_core` overlays
/// `raw_config_source` onto the resolved map; packet 55 AC-8 requires
/// user-passed keys to appear) and pads to OrcaSlicer's minimum-key gate when
/// few are supplied — two different-but-equivalent invocations legitimately
/// produce different CONFIG_BLOCKs while emitting identical toolpaths.
#[test]
fn explicit_defaults_match_omitted_keys_byte_for_byte() {
    let explicit_bytes = run_with_config(EXPLICIT_DEFAULT_PRECISION_JSON);
    let omitted_bytes = run_with_config(EMPTY_CONFIG_JSON);

    // Non-vacuous guard: both runs must have produced real layered geometry,
    // otherwise two empty outputs would trivially "match".
    let layer_markers = |bytes: &[u8]| -> usize {
        std::str::from_utf8(bytes)
            .expect("gcode is utf-8")
            .lines()
            .filter(|l| l.trim() == ";LAYER_CHANGE")
            .count()
    };
    let explicit_layers = layer_markers(&explicit_bytes);
    assert!(
        explicit_layers >= 10,
        "RES-1 BLOCKED: explicit-defaults run has only {explicit_layers} ;LAYER_CHANGE \
         markers — the pipeline may not be emitting real geometry."
    );
    assert_eq!(
        layer_markers(&omitted_bytes),
        explicit_layers,
        "RES-1: omitted-keys run produced a different layer count"
    );

    // Compare the motion stream only (non-comment lines). See the doc comment
    // for why CONFIG_BLOCK comment lines legitimately differ between two
    // equivalent invocations.
    let motion_lines = |bytes: &[u8]| -> String {
        std::str::from_utf8(bytes)
            .expect("gcode is utf-8")
            .lines()
            .filter(|l| !l.trim_start().starts_with(';'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        motion_lines(&explicit_bytes),
        motion_lines(&omitted_bytes),
        "RES-1 FAILED: overriding the packet-60 keys with their own default values \
         changed the emitted toolpath. Either a ResolvedConfig default no longer \
         matches the mirror in EXPLICIT_DEFAULT_PRECISION_JSON, or key \
         resolution/emit is no longer value-transparent for defaults."
    );
}

// ---------------------------------------------------------------------------
// DET-2 — legacy path determinism
// ---------------------------------------------------------------------------

/// DET-2: Two identical legacy-configuration runs must produce byte-identical
/// G-code.
///
/// Replaces the frozen golden's "stability across runs" role without pinning
/// content: determinism violations surface here; geometric drift does not.
#[test]
fn legacy_precision_run_is_deterministic() {
    let a = run_with_config(LEGACY_PRECISION_JSON);
    let b = run_with_config(LEGACY_PRECISION_JSON);

    let layers = std::str::from_utf8(&a)
        .expect("gcode is utf-8")
        .lines()
        .filter(|l| l.trim() == ";LAYER_CHANGE")
        .count();
    assert!(
        layers >= 10,
        "DET-2 BLOCKED: only {layers} ;LAYER_CHANGE markers — the pipeline may not \
         be emitting real geometry."
    );

    assert_eq!(
        a, b,
        "DET-2 FAILED: two identical legacy-mode runs produced different bytes — \
         the pipeline is nondeterministic on the zero-cost precision path."
    );
}

// ---------------------------------------------------------------------------
// INV-3 — structural soundness of the legacy slice
// ---------------------------------------------------------------------------

/// One parsed layer of legacy output: z height and its extrusion runs.
struct ParsedLayer {
    z_mm: f64,
    /// Contiguous E-positive XY runs; each run is a list of (x, y) points.
    extrusion_runs: Vec<Vec<(f64, f64)>>,
}

/// Parse G-code into per-layer z + contiguous extrusion runs.
///
/// A new run starts at each `G0` travel or after a negative-E retract line;
/// points are appended while lines carry positive E and both X and Y.
fn parse_layers(gcode: &str) -> Vec<ParsedLayer> {
    let mut layers = Vec::new();
    let mut current_z: Option<f64> = None;
    let mut current_layer: Option<ParsedLayer> = None;
    let mut current_run: Vec<(f64, f64)> = Vec::new();

    for line in gcode.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(";Z:") {
            if let Ok(z) = rest.trim().parse::<f64>() {
                if let Some(layer) = current_layer.take() {
                    layers.push(layer);
                }
                current_z = Some(z);
                current_layer = Some(ParsedLayer {
                    z_mm: z,
                    extrusion_runs: Vec::new(),
                });
                current_run.clear();
            }
            continue;
        }
        if !line.starts_with('G') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let x = tokens.iter().find_map(|t| t.strip_prefix('X'));
        let y = tokens.iter().find_map(|t| t.strip_prefix('Y'));
        let e = tokens.iter().find_map(|t| t.strip_prefix('E'));
        match (
            x.and_then(|v| v.parse::<f64>().ok()),
            y.and_then(|v| v.parse::<f64>().ok()),
        ) {
            (Some(x), Some(y)) => {
                let _ = current_z; // recorded via the ;Z: comment above
                let e_val = e.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                if e_val > 0.0 {
                    current_run.push((x, y));
                } else if !current_run.is_empty() {
                    if let Some(layer) = current_layer.as_mut() {
                        layer.extrusion_runs.push(std::mem::take(&mut current_run));
                    }
                }
            }
            _ => {
                if !current_run.is_empty() {
                    if let Some(layer) = current_layer.as_mut() {
                        layer.extrusion_runs.push(std::mem::take(&mut current_run));
                    }
                }
            }
        }
    }
    if let Some(layer) = current_layer.take() {
        layers.push(layer);
    }
    layers
}

/// INV-3: The legacy slice must be structurally sound:
///
/// 1. **Analytic layer count** — exactly 100 `;LAYER_CHANGE` markers. The
///    fixture is a solid 20 mm box (`resources/test_stl/ASCII/20mmbox-LF.stl`)
///    and the default profile is 0.2 mm layers with 0.2 mm first-layer height,
///    so 100 layers is derived from fixture geometry × documented defaults, NOT
///    captured output. An intentional change to those defaults legitimately
///    edits these constants — a pointed failure unlike the old blanket
///    byte-diff.
/// 2. **Monotone Z-set** — `;Z:` strictly increasing at uniform 0.2 mm steps.
/// 3. **Closed outer walls** — every outer-wall extrusion run revisits its own
///    start point (a closed loop).
/// 4. **E monotonicity + non-degenerate extrusion** — E never decreases within
///    an Outer wall section, consecutive points are distinct, every parsed
///    layer extrudes something.
/// 5. **Decimal-formatting contract** — legacy runs at `gcode_xy_decimals = 4`,
///    so every X/Y token carries ≤ 4 decimals; the default-config run (decimals
///    = 3) carries ≤ 3. Proves the key flows through resolution to the emitter
///    on both paths.
#[test]
fn legacy_slice_structural_invariants() {
    const EXPECTED_LAYERS: usize = 100;
    const LAYER_HEIGHT_MM: f64 = 0.2;

    let legacy_bytes = run_with_config(LEGACY_PRECISION_JSON);
    let gcode = std::str::from_utf8(&legacy_bytes).expect("legacy gcode is utf-8");

    // -- 1. Analytic layer count ---------------------------------------------
    let layer_changes = gcode
        .lines()
        .filter(|l| l.trim() == ";LAYER_CHANGE")
        .count();
    assert_eq!(
        layer_changes, EXPECTED_LAYERS,
        "INV-3.1: expected {EXPECTED_LAYERS} layers (20mm box / 0.2mm default layer \
         height); got {layer_changes}. If first_layer_height/layer_height defaults \
         changed intentionally, update EXPECTED_LAYERS."
    );

    // -- 2. Monotone Z-set -----------------------------------------------------
    let layers = parse_layers(gcode);
    assert_eq!(
        layers.len(),
        EXPECTED_LAYERS,
        "INV-3.2: parsed {} ;Z: blocks, expected {EXPECTED_LAYERS}",
        layers.len()
    );
    for pair in layers.windows(2) {
        let dz = pair[1].z_mm - pair[0].z_mm;
        assert!(
            dz > 0.0,
            "INV-3.2: Z decreased from {} to {}",
            pair[0].z_mm,
            pair[1].z_mm
        );
        assert!(
            (dz - LAYER_HEIGHT_MM).abs() < 1e-6,
            "INV-3.2: non-uniform layer step {dz} between z={} and z={} \
             (expected {LAYER_HEIGHT_MM})",
            pair[0].z_mm,
            pair[1].z_mm
        );
    }

    // -- 3 & 4. Outer-wall closure + E monotonicity ----------------------------
    // Split the file into ;TYPE: sections so we only inspect Outer wall runs.
    //
    // The serializer emits a travel-to-start line (`G1 X.. Y.. F..`, no E)
    // INSIDE the `;TYPE:Outer wall` section before the loop's first extrusion.
    // That line is the loop's anchor point, so runs are seeded with the last
    // seen XY position — otherwise every run starts one vertex late and can
    // never "close".
    let mut current_type: Option<String> = None;
    let mut outer_runs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut open_run: Vec<(f64, f64)> = Vec::new();
    let mut last_pos: Option<(f64, f64)> = None;
    let mut last_e_in_section: Option<f64> = None;

    for line in gcode.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(";TYPE:") {
            if !open_run.is_empty() && current_type.as_deref() == Some("Outer wall") {
                outer_runs.push(std::mem::take(&mut open_run));
            }
            open_run.clear();
            last_e_in_section = None;
            current_type = Some(rest.to_string());
            continue;
        }
        if current_type.as_deref() != Some("Outer wall") || !line.starts_with('G') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let x = tokens.iter().find_map(|t| t.strip_prefix('X'));
        let y = tokens.iter().find_map(|t| t.strip_prefix('Y'));
        let e = tokens.iter().find_map(|t| t.strip_prefix('E'));
        match (
            x.and_then(|v| v.parse::<f64>().ok()),
            y.and_then(|v| v.parse::<f64>().ok()),
        ) {
            (Some(x), Some(y)) => {
                let e_val = e.and_then(|v| v.parse::<f64>().ok());
                if e_val.is_none() || e_val == Some(0.0) {
                    // Travel / positioning move inside the section: it is the
                    // loop's anchor. Close any open run and record position.
                    if !open_run.is_empty() {
                        outer_runs.push(std::mem::take(&mut open_run));
                    }
                    last_pos = Some((x, y));
                } else if let Some(e) = e_val {
                    if let Some(last) = last_e_in_section {
                        // The E field prints at 5 decimals; geometrically
                        // equal segments can round one to two quanta apart
                        // when the underlying f32 lengths differ in the last
                        // bits (the aligned-seam projection inserts its
                        // vertex via f32 ops — ticket 102 made that path the
                        // live runtime default; measured delta here is
                        // exactly 2 quanta, 0.73152 -> 0.73151). Two printed
                        // quanta (1e-5) of slack keeps INV-3.4 aimed at real
                        // E regressions (G92 resets, retractions emitted
                        // mid-loop) rather than formatter half-ulp jitter.
                        assert!(
                            e >= last - 1e-5,
                            "INV-3.4: E went backwards inside an Outer wall section \
                             ({last} -> {e})"
                        );
                    }
                    last_e_in_section = Some(e);
                    if open_run.is_empty() {
                        if let Some(anchor) = last_pos {
                            open_run.push(anchor);
                        }
                    }
                    open_run.push((x, y));
                }
            }
            _ => {
                // Non-XY G-line (retract/unretract): closes the current run.
                if !open_run.is_empty() {
                    outer_runs.push(std::mem::take(&mut open_run));
                }
            }
        }
    }
    if !open_run.is_empty() {
        outer_runs.push(open_run);
    }

    assert!(
        !outer_runs.is_empty(),
        "INV-3.3: no Outer wall extrusion runs found — section sentinels may have \
         moved; check the serializer's ;TYPE emission."
    );

    // Loop closure: each run must revisit its own start point later in itself
    // (within one nozzle diameter). The perimeter loop returns to where it began.
    const CLOSURE_TOL_MM: f64 = 0.45; // > max legacy wall spacing; < feature scale
    for (i, run) in outer_runs.iter().enumerate() {
        assert!(
            run.len() >= 4,
            "INV-3.3: outer-wall run {i} has only {} points — too short to be a loop",
            run.len()
        );
        let (sx, sy) = run[0];
        let closes = run[1..]
            .iter()
            .any(|&(x, y)| ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() <= CLOSURE_TOL_MM);
        assert!(
            closes,
            "INV-3.3: outer-wall run {i} ({} points) never returns to its start \
             ({sx},{sy}) within {CLOSURE_TOL_MM}mm — the loop is broken.",
            run.len()
        );

        // INV-3.4 (geometry-side): consecutive points must be distinct.
        for w in run.windows(2) {
            assert!(
                (w[0].0 - w[1].0).abs() > 1e-9 || (w[0].1 - w[1].1).abs() > 1e-9,
                "INV-3.4: duplicate consecutive point in outer-wall run {i}"
            );
        }
    }

    // Per-layer positive extrusion exists.
    for layer in &layers {
        let total_e_points: usize = layer.extrusion_runs.iter().map(|r| r.len()).sum();
        assert!(
            total_e_points > 0,
            "INV-3.4: layer at z={} extrudes nothing",
            layer.z_mm
        );
    }

    // -- 5. Decimal-formatting contract ----------------------------------------
    let xy_decimals =
        |token: &str| -> usize { token.split_once('.').map_or(0, |(_, frac)| frac.len()) };
    let check_decimals = |gcode_text: &str, max_decimals: usize, label: &str| {
        for line in gcode_text.lines().filter(|l| {
            let l = l.trim();
            l.starts_with("G1") && l.split_whitespace().any(|t| t.starts_with('X'))
        }) {
            for token in line.split_whitespace() {
                if token.starts_with('X') || token.starts_with('Y') {
                    assert!(
                        xy_decimals(token) <= max_decimals,
                        "INV-3.5: {label} run emitted `{token}` with >{max_decimals} decimals"
                    );
                }
            }
        }
    };
    check_decimals(gcode, 4, "legacy (gcode_xy_decimals=4)");
    let default_bytes = run_with_config(EXPLICIT_DEFAULT_PRECISION_JSON);
    let default_gcode = std::str::from_utf8(&default_bytes).expect("default gcode is utf-8");
    check_decimals(default_gcode, 3, "default (gcode_xy_decimals=3)");
}
