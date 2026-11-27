//! Packet 246 AC-8 — end-to-end wave-overhang bridge-fill discriminator.
//!
//! Slices `resources/A_upsidedown.obj` with `bridge_fill_holder =
//! "wave-overhangs"` and proves BOTH halves of AC-8:
//!
//! 1. **Typed capture**: the `Layer::InfillPostProcess` visual-debug tap
//!    contains at least one *contiguous* order-locked `BridgeInfill` block —
//!    i.e. a run of consecutive `solid_infill` paths sharing one
//!    `order_lock` tag.
//! 2. **G-code discriminator**: the emitted wave block runs at the configured
//!    `wave_overhang_print_speed` and extrudes the configured
//!    `wave_overhang_flow_mm3_per_mm`.
//!
//! Half 2 is the reason this test exists. Order-locked waves and the
//! rectilinear fallback **share the role `BridgeInfill`** and therefore share
//! the `;TYPE:Bridge` G-code label, so role alone cannot tell them apart.
//! Feedrate (`F`) and volumetric flow are the discriminator: waves emit at
//! `wave_overhang_print_speed * 60`, the fallback at `bridge_speed * 60`.
//!
//! ## Why `wave_overhang_anchor_depth_mm` is deliberately NOT set
//!
//! Selecting the module as `bridge_fill_holder` is the enable; waves must
//! engage out of the box, so this test exercises the DEFAULT (auto) anchor
//! depth. The original auto formula (`min(3mm, bridge_spacing *
//! (wall_loops + 1))` = 1.8 mm at nozzle 0.4 / wall_loops 3) never exceeded
//! the generator's own `anchors_size` (the same expression), `inset_anchors`
//! came out empty, seed generation failed, and every connected component fell
//! back to rectilinear bridge fill — measured on this very fixture: 48
//! `BridgeInfill` paths, ALL at `speed_factor == 1.0`. The auto depth is now
//! floored at `anchors_size + base_spacing`, so omitting the key here proves
//! the out-of-the-box path.
//!
//! Authoritative invocation:
//!   `cargo test -p slicer-runtime --test e2e -- \
//!    wave_overhang_bridge_fill_e2e_tdd::wave_overhang_bridge_fill_e2e --exact`

use pnp_cli_locator::pnp_cli_bin;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Wave print speed, mm/s. `speed_factor` = 5.0 / 25.0 = 0.2, inside the
/// emitter's representable `[0.05, 5.0]` band.
const WAVE_PRINT_SPEED_MM_S: f64 = 5.0;
/// Host bridge speed, mm/s — the fallback's feedrate, and the divisor the
/// module uses to derive `speed_factor`.
const BRIDGE_SPEED_MM_S: f64 = 25.0;
/// Configured wave volumetric flow, mm^3 per mm of path.
const WAVE_FLOW_MM3_PER_MM: f64 = 0.12;
/// Filament diameter, mm — needed to turn G-code E (filament length) back
/// into extruded volume.
const FILAMENT_DIAMETER_MM: f64 = 1.75;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn scratch_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("wave_overhang_bridge_fill_e2e");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Write the shared slicing config. `wave_overhang_anchor_depth_mm` is
/// deliberately omitted so the auto anchor depth is exercised — see the
/// module docs above.
fn write_config(path: &Path) {
    let body = serde_json::json!({
        "layer_height": 0.2,
        "initial_layer_print_height": 0.2,
        "nozzle_diameter": 0.4,
        "wall_loops": 3,
        "filament_diameter": FILAMENT_DIAMETER_MM,
        "bridge_fill_holder": "wave-overhangs",
        "bridge_speed": BRIDGE_SPEED_MM_S,
        "wave_overhang_print_speed": WAVE_PRINT_SPEED_MM_S,
        "wave_overhang_flow_mm3_per_mm": WAVE_FLOW_MM3_PER_MM,
    });
    std::fs::write(path, serde_json::to_vec(&body).expect("serialize config"))
        .expect("write config");
}

/// Run `pnp_cli visual-debug` with a `Layer::InfillPostProcess` typed tap and
/// return the parsed manifest.
fn capture_infill_manifest(config: &Path) -> Value {
    let root = repo_root();
    let scratch = scratch_dir();
    let request = scratch.join("request.json");
    let output = scratch.join("bundle");
    let _ = std::fs::remove_dir_all(&output);
    let body = serde_json::json!({
        "schema_version": "1.0.0",
        "source": {
            "kind": "model",
            "model": root.join("resources/A_upsidedown.obj"),
            "config": config,
            "module_dirs": [root.join("modules/core-modules")],
            "path": null
        },
        "layers": [{"start": 0, "end": 1000}],
        "taps": ["Layer::InfillPostProcess"],
        "visualizations": [],
        "resolution_scale": 1,
        "frame": "model"
    });
    std::fs::write(
        &request,
        serde_json::to_vec(&body).expect("serialize request"),
    )
    .expect("write visual-debug request");
    let proc = Command::new(pnp_cli_bin())
        .args(["visual-debug", "--request"])
        .arg(&request)
        .args(["--output"])
        .arg(&output)
        .output()
        .expect("pnp_cli visual-debug should execute");
    assert!(
        proc.status.success(),
        "visual-debug failed: {}",
        String::from_utf8_lossy(&proc.stderr)
    );
    serde_json::from_slice(
        &std::fs::read(output.join("manifest.json")).expect("read visual-debug manifest"),
    )
    .expect("manifest JSON")
}

/// Slice the fixture to G-code and return the text.
fn slice_to_gcode(config: &Path) -> String {
    let root = repo_root();
    let out = scratch_dir().join("wave.gcode");
    let proc = Command::new(pnp_cli_bin())
        .args(["slice", "--model"])
        .arg(root.join("resources/A_upsidedown.obj"))
        .args(["--config"])
        .arg(config)
        .args(["--output"])
        .arg(&out)
        .args(["--module-dir"])
        .arg(root.join("modules/core-modules"))
        .output()
        .expect("pnp_cli slice should execute");
    assert!(
        proc.status.success(),
        "pnp_cli slice failed: {}",
        String::from_utf8_lossy(&proc.stderr)
    );
    std::fs::read_to_string(&out).expect("read emitted gcode")
}

/// Accumulated extrusion for one `(;TYPE: label, F feedrate)` bucket.
#[derive(Default, Clone, Copy)]
struct Bucket {
    /// Summed positive E delta, mm of filament.
    filament_mm: f64,
    /// Summed XY path length of the extruding moves, mm.
    length_mm: f64,
}

/// Parse the G-code into `(;TYPE: label, F)` buckets.
///
/// Own copy of the `parse_layers` shape used by
/// `calicat_internal_bridge_gating_e2e_tdd`, extended with the `b'F'` arm this
/// test needs: `;TYPE:` labels carry across lines until the next marker, E is
/// summed under M83-relative / M82-absolute semantics, and F is sticky (a move
/// without an F token keeps the last commanded feedrate).
fn parse_type_speed_buckets(gcode: &str) -> HashMap<(String, i64), Bucket> {
    let mut buckets: HashMap<(String, i64), Bucket> = HashMap::new();
    let mut current_type = String::from("(none)");
    let mut relative_e = true;
    let mut last_e = 0.0f64;
    let mut feedrate = 0.0f64;
    let mut pos = (0.0f64, 0.0f64);
    let token_value = |tok: &str| -> Option<f64> { tok[1..].parse::<f64>().ok() };
    for raw in gcode.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix(";TYPE:") {
            current_type = rest.trim().to_string();
            continue;
        }
        match line {
            "M83" => relative_e = true,
            "M82" => relative_e = false,
            _ => {}
        }
        if !(line.starts_with("G0") || line.starts_with("G1")) {
            continue;
        }
        let mut x = None;
        let mut y = None;
        let mut e = None;
        for tok in line.split_whitespace() {
            if tok.len() < 2 {
                continue;
            }
            match tok.as_bytes()[0] {
                b'X' => x = token_value(tok),
                b'Y' => y = token_value(tok),
                b'E' => e = token_value(tok),
                b'F' => {
                    if let Some(v) = token_value(tok) {
                        feedrate = v;
                    }
                }
                _ => {}
            }
        }
        let new_pos = (x.unwrap_or(pos.0), y.unwrap_or(pos.1));
        if let Some(e_val) = e {
            let delta = if relative_e { e_val } else { e_val - last_e };
            last_e = e_val;
            if delta > 0.0 {
                let dx = new_pos.0 - pos.0;
                let dy = new_pos.1 - pos.1;
                let bucket = buckets
                    .entry((current_type.clone(), (feedrate * 10.0).round() as i64))
                    .or_default();
                bucket.filament_mm += delta;
                bucket.length_mm += (dx * dx + dy * dy).sqrt();
            }
        }
        pos = new_pos;
    }
    buckets
}

/// Longest run of consecutive paths in `paths` that share one non-null
/// `order_lock` tag, restricted to `BridgeInfill`. Returns `(tag, run_len)`.
fn longest_locked_bridge_run(paths: &[Value]) -> Option<(u64, usize)> {
    let mut best: Option<(u64, usize)> = None;
    let mut i = 0usize;
    while i < paths.len() {
        let tag = paths[i]["order_lock"].as_u64();
        let is_bridge = paths[i]["role"].as_str() == Some("BridgeInfill");
        if let (Some(tag), true) = (tag, is_bridge) {
            let mut j = i;
            while j < paths.len()
                && paths[j]["order_lock"].as_u64() == Some(tag)
                && paths[j]["role"].as_str() == Some("BridgeInfill")
            {
                j += 1;
            }
            if best.is_none_or(|(_, len)| j - i > len) {
                best = Some((tag, j - i));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    best
}

#[test]
fn wave_overhang_bridge_fill_e2e() {
    let root = repo_root();
    let model = root.join("resources/A_upsidedown.obj");
    assert!(model.exists(), "A_upsidedown fixture missing at {model:?}");
    let config = scratch_dir().join("config.json");
    write_config(&config);

    // ---- Half 1: contiguous order-locked BridgeInfill block ---------------
    let manifest = capture_infill_manifest(&config);
    let mut runs: Vec<(f64, u64, usize)> = Vec::new();
    let mut bridge_paths = 0usize;
    for image in manifest["images"].as_array().expect("manifest images") {
        let capture = &image["typed_capture"];
        if capture["kind"].as_str() != Some("Infill") {
            continue;
        }
        let z = image["layer_z"].as_f64().unwrap_or(f64::NAN);
        for region in capture["value"]["regions"]
            .as_array()
            .expect("InfillIR regions")
        {
            let solid = region["solid_infill"]
                .as_array()
                .expect("InfillRegion solid_infill");
            bridge_paths += solid
                .iter()
                .filter(|p| p["role"].as_str() == Some("BridgeInfill"))
                .count();
            if let Some((tag, len)) = longest_locked_bridge_run(solid) {
                runs.push((z, tag, len));
            }
        }
    }
    assert!(
        bridge_paths > 0,
        "no BridgeInfill paths captured at Layer::InfillPostProcess"
    );
    assert!(
        runs.iter().any(|&(_, _, len)| len >= 2),
        "AC-8(1): expected at least one CONTIGUOUS order-locked BridgeInfill \
         block (>= 2 consecutive paths sharing one order_lock tag) in the \
         Layer::InfillPostProcess typed capture; saw {bridge_paths} \
         BridgeInfill paths and locked runs {runs:?}"
    );

    // ---- Half 2: speed + volume discriminator in the emitted G-code -------
    let gcode = slice_to_gcode(&config);
    let buckets = parse_type_speed_buckets(&gcode);

    let wave_f = (WAVE_PRINT_SPEED_MM_S * 60.0 * 10.0).round() as i64;
    let fallback_f = (BRIDGE_SPEED_MM_S * 60.0 * 10.0).round() as i64;

    let wave = buckets
        .get(&(String::from("Bridge"), wave_f))
        .copied()
        .unwrap_or_else(|| {
            let seen: Vec<_> = buckets
                .keys()
                .filter(|(t, _)| t == "Bridge")
                .map(|(_, f)| *f as f64 / 10.0)
                .collect();
            panic!(
                "AC-8(2): no `;TYPE:Bridge` extrusion at the wave feedrate \
                 F{} (= wave_overhang_print_speed {WAVE_PRINT_SPEED_MM_S} \
                 mm/s x 60). Bridge feedrates seen: {seen:?}",
                wave_f as f64 / 10.0
            )
        });
    assert!(
        wave.length_mm > 1.0,
        "AC-8(2): wave bucket has negligible path length ({} mm)",
        wave.length_mm
    );

    // The wave block must be distinguishable from the rectilinear fallback,
    // which shares the role and therefore the `;TYPE:Bridge` label but runs at
    // `bridge_speed`. Assert the feedrates are genuinely different buckets.
    assert_ne!(
        wave_f, fallback_f,
        "test misconfigured: wave and fallback feedrates coincide"
    );

    // E is filament length; convert back to extruded volume.
    let filament_area = std::f64::consts::PI * (FILAMENT_DIAMETER_MM / 2.0).powi(2);
    let wave_mm3_per_mm = wave.filament_mm * filament_area / wave.length_mm;
    assert!(
        (wave_mm3_per_mm - WAVE_FLOW_MM3_PER_MM).abs() <= WAVE_FLOW_MM3_PER_MM * 0.02,
        "AC-8(2): wave extruded volume {wave_mm3_per_mm:.5} mm^3/mm does not \
         match configured wave_overhang_flow_mm3_per_mm \
         {WAVE_FLOW_MM3_PER_MM} (+/-2%); E={} mm over L={} mm",
        wave.filament_mm,
        wave.length_mm
    );

    // The fallback must still be present and must NOT carry the wave flow —
    // otherwise the "discriminator" would be vacuous.
    if let Some(fallback) = buckets
        .get(&(String::from("Bridge"), fallback_f))
        .copied()
        .filter(|b| b.length_mm > 1.0)
    {
        let fallback_mm3_per_mm = fallback.filament_mm * filament_area / fallback.length_mm;
        assert!(
            (fallback_mm3_per_mm - WAVE_FLOW_MM3_PER_MM).abs() > WAVE_FLOW_MM3_PER_MM * 0.02,
            "AC-8(2): rectilinear fallback flow {fallback_mm3_per_mm:.5} mm^3/mm \
             is indistinguishable from the wave flow {WAVE_FLOW_MM3_PER_MM}; \
             the speed/volume discriminator would be vacuous"
        );
    }
}
