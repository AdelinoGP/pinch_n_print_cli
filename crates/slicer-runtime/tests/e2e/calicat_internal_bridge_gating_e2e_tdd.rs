//! Packet 234a — AC-5 calicat internal-bridge gating regression surface.
//!
//! Slices `resources/calicat.stl` twice and asserts:
//! 1. **Byte-identity** of the two outputs (determinism of the new
//!    ShellClassification internal-bridge pass).
//! 2. **Matched-profile label pin**: the fresh matched-oracle slice has no
//!    `;TYPE:Internal Bridge` sections; bridge geometry is currently emitted
//!    under other role labels (see DEV-153).
//! 3. **External-row guard** (packet-235 regression): at the layer nearest
//!    Z≈3.2 the `;TYPE:Bridge` row keeps a dominant direction within
//!    [85°, 95°] (baseline after packet 235: 90.0° over 74 segments /
//!    324.6 mm).
//!
//! Parsing semantics: `;Z:` headers key the layers; `;TYPE:` labels carry
//! across layer changes until the next marker; E values are summed with
//! M83-relative-E semantics (the serializer defaults to relative-E mode and
//! emits `M83`; `M82` switches to absolute tracking).
//!
//! Authoritative pipe command (`packet.spec.md` AC-6):
//!   `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_gating_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`

use pnp_cli_locator::pnp_cli_bin;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules").join("core-modules")
}

fn calicat_stl() -> PathBuf {
    repo_root().join("resources").join("calicat.stl")
}

fn gcode_path(tag: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("calicat_{tag}.gcode"))
}

/// One parsed G-code layer: total relative-E extrusion per `;TYPE:` label,
/// plus the extruding segments per label (direction vectors in mm).
#[derive(Default)]
struct Layer {
    z: f32,
    /// label → summed relative-E (mm of filament)
    extrusion: HashMap<String, f64>,
    /// label → extruding segment direction deltas (dx, dy) in mm
    segments: HashMap<String, Vec<(f64, f64)>>,
}

/// Parse layers out of gcode: `;Z:` keys layers, `;TYPE:` carries across
/// layer changes until the next marker, E sums use M83-relative-E semantics
/// (with an `M82` fallback to absolute tracking for robustness).
fn parse_layers(gcode: &str) -> Vec<Layer> {
    let mut layers: Vec<Layer> = Vec::new();
    let mut current_type = String::from("(none)");
    let mut relative_e = true;
    let mut last_e = 0.0f64;
    let mut pos = (0.0f64, 0.0f64);
    let token_value = |tok: &str| -> Option<f64> { tok[1..].parse::<f64>().ok() };
    for raw in gcode.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix(";Z:") {
            let z = rest.trim().parse::<f32>().unwrap_or(f32::NAN);
            if !layers
                .last()
                .is_some_and(|layer| (layer.z - z).abs() < 1e-6)
            {
                layers.push(Layer {
                    z,
                    ..Layer::default()
                });
            }
            continue;
        }
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
                _ => {}
            }
        }
        let new_pos = (x.unwrap_or(pos.0), y.unwrap_or(pos.1));
        if let Some(e_val) = e {
            let delta = if relative_e { e_val } else { e_val - last_e };
            last_e = e_val;
            if delta > 0.0 {
                let layer = layers.last_mut().expect("E move before any ;Z: header");
                *layer.extrusion.entry(current_type.clone()).or_insert(0.0) += delta;
                let dx = new_pos.0 - pos.0;
                let dy = new_pos.1 - pos.1;
                layer
                    .segments
                    .entry(current_type.clone())
                    .or_default()
                    .push((dx, dy));
            }
        }
        pos = new_pos;
    }
    layers
}

/// Length-weighted dominant direction angle in [0°, 180°), via the
/// doubled-angle circular mean (robust across the 0°/180° wraparound).
fn dominant_angle_deg(segments: &[(f64, f64)]) -> Option<(f64, usize, f64)> {
    let (mut sum_sin, mut sum_cos, mut total_len) = (0.0f64, 0.0f64, 0.0f64);
    let mut count = 0usize;
    for &(dx, dy) in segments {
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            continue;
        }
        let theta = dy.atan2(dx).to_degrees().rem_euclid(180.0);
        let rad2 = theta.to_radians() * 2.0;
        sum_sin += len * rad2.sin();
        sum_cos += len * rad2.cos();
        total_len += len;
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let dom = (0.5 * sum_sin.atan2(sum_cos).to_degrees()).rem_euclid(180.0);
    Some((dom, count, total_len))
}

fn slice_once(
    bin: &std::path::Path,
    model: &std::path::Path,
    config: &std::path::Path,
    output: &std::path::Path,
) {
    let proc = Command::new(bin)
        .args(["slice", "--model"])
        .arg(model)
        .args(["--config"])
        .arg(config)
        .args(["--output"])
        .arg(output)
        .args(["--module-dir"])
        .arg(core_modules_dir())
        .output()
        .expect("pnp_cli binary should execute");
    let stderr = String::from_utf8_lossy(&proc.stderr);
    assert!(
        proc.status.success(),
        "pnp_cli must succeed for calicat ({output:?}). Stderr:\n{stderr}"
    );
}

#[test]
fn calicat_internal_bridge_gating_e2e_tdd() {
    let bin = pnp_cli_bin();
    let model = calicat_stl();
    assert!(bin.exists(), "pnp_cli not built at {}", bin.display());
    assert!(model.exists(), "calicat.stl missing at {}", model.display());

    let out_a = gcode_path("a");
    let out_b = gcode_path("b");
    let config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("calicat_internal_bridge_matched_config.json");
    std::fs::write(
        &config,
        br#"{"layer_height":0.2,"first_layer_height":0.25,"nozzle_diameter":0.5,"line_width":0.525,"bridge_flow":0.95,"internal_bridge_flow":0.95,"infill_density":0.25,"sparse_infill_density":25.0,"top_shell_layers":3,"bottom_shell_layers":3,"enable_support":true,"dont_filter_internal_bridges":false,"thick_bridges":false,"thick_internal_bridges":false}"#,
    )
    .expect("write matched-oracle config");
    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);

    slice_once(&bin, &model, &config, &out_a);
    slice_once(&bin, &model, &config, &out_b);

    // (1) Determinism: byte-identical double slice.
    let bytes_a = std::fs::read(&out_a).expect("read calicat_a.gcode");
    let bytes_b = std::fs::read(&out_b).expect("read calicat_b.gcode");
    assert_eq!(
        bytes_a, bytes_b,
        "AC-5: two slices of calicat must be byte-identical"
    );

    let gcode = String::from_utf8(bytes_a).expect("gcode utf8");
    let layers = parse_layers(&gcode);
    assert!(!layers.is_empty(), "no ;Z:-keyed layers parsed from gcode");

    const INTERNAL_BRIDGE: &str = "Internal Bridge";
    const BRIDGE: &str = "Bridge";

    // (2) Matched-profile measurement: no Internal Bridge role labels appear.
    let ib_layers: Vec<&Layer> = layers
        .iter()
        .filter(|layer| layer.extrusion.get(INTERNAL_BRIDGE).copied().unwrap_or(0.0) > 0.0)
        .collect();
    let zs: Vec<f32> = ib_layers.iter().map(|layer| layer.z).collect();
    println!("internal-bridge layers={} (z={zs:?})", ib_layers.len());
    assert!(
        ib_layers.is_empty(),
        "AC-6: matched-profile Internal-Bridge-labelled layers = {}, expected 0 (zs={zs:?})",
        ib_layers.len()
    );

    // Informational: combined bridge-labelled extrusion (Bridge + Internal
    // Bridge), for comparison against the canonical reference (~950.56 mm).
    let combined: f64 = layers
        .iter()
        .map(|layer| {
            layer.extrusion.get(INTERNAL_BRIDGE).copied().unwrap_or(0.0)
                + layer.extrusion.get(BRIDGE).copied().unwrap_or(0.0)
        })
        .sum();
    println!("combined bridge-labelled extrusion = {combined:.2} mm");

    // (3) External-row guard at Z≈3.2: dominant angle within [85°, 95°].
    let (ext_layer, ext_z) = layers
        .iter()
        .filter(|layer| layer.segments.get(BRIDGE).is_some_and(|s| !s.is_empty()))
        .map(|layer| (layer, layer.z))
        .min_by(|a, b| (a.1 - 3.2).abs().total_cmp(&(b.1 - 3.2).abs()))
        .expect("at least one layer");
    assert!(
        (ext_z - 3.2).abs() <= 0.25,
        "no calicat layer near Z=3.2 (closest: {ext_z})"
    );
    let ext_segments = ext_layer.segments.get(BRIDGE).cloned().unwrap_or_default();
    let (dominant, seg_count, seg_len) = dominant_angle_deg(&ext_segments).unwrap_or_else(|| {
        panic!(
            "AC-5: no ;TYPE:Bridge segments at Z≈3.2 (z={ext_z}); external bridge \
             row vanished — packet-235 regression."
        )
    });
    println!(
        "external Bridge @ Z={ext_z}: dominant angle={dominant:.1}° over {seg_count} segs / {seg_len:.1} mm"
    );
    assert!(
        (85.0..=95.0).contains(&dominant),
        "AC-5: external Bridge dominant angle at Z≈3.2 is {dominant:.1}°, expected \
         within [85°, 95°] (packet-235 baseline: 90.0° / 74 segs / 324.6 mm)"
    );
}
