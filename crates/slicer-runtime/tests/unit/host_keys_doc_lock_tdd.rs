//! Locks `docs/config/host-keys.toml` to the live code defaults it mirrors, so
//! the generated host-key tables in `docs/15_config_keys_reference.md`
//! (`cargo xtask gen-config-docs`) cannot drift from `FeedrateConfig::default()`
//! / `ResolvedConfig::default()`. doc 15 previously hand-listed these and had
//! drifted ~half its host defaults away from the code.
//!
//! Relocated from `crates/slicer-runtime/src/gcode_emit.rs` in packet 86 Step 3
//! when that file was deleted as part of the slicer-gcode extraction.

use slicer_ir::{FeedrateConfig, ResolvedConfig};
use std::path::{Path, PathBuf};

fn host_keys_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/config/host-keys.toml")
}

fn host_keys() -> toml::Value {
    let text =
        std::fs::read_to_string(host_keys_path()).expect("docs/config/host-keys.toml must exist");
    toml::from_str(&text).expect("host-keys.toml must parse")
}

fn doc_num(spec: &toml::Value) -> f64 {
    let d = &spec["default"];
    d.as_float()
        .or_else(|| d.as_integer().map(|i| i as f64))
        .expect("numeric default")
}

fn resolved_num(c: &ResolvedConfig, key: &str) -> Option<f64> {
    Some(match key {
        "top_shell_layers" => c.top_shell_layers as f64,
        "bottom_shell_layers" => c.bottom_shell_layers as f64,
        "gcode_xy_decimals" => c.gcode_xy_decimals as f64,
        "gcode_resolution" => c.gcode_resolution as f64,
        "infill_resolution" => c.infill_resolution as f64,
        "support_resolution" => c.support_resolution as f64,
        "min_segment_length" => c.min_segment_length as f64,
        "slice_closing_radius" => c.slice_closing_radius as f64,
        _ => return None,
    })
}

fn resolved_str<'a>(c: &'a ResolvedConfig, key: &str) -> Option<&'a str> {
    Some(match key {
        "top_fill_holder" => c.top_fill_holder.as_str(),
        "bottom_fill_holder" => c.bottom_fill_holder.as_str(),
        "bridge_fill_holder" => c.bridge_fill_holder.as_str(),
        "sparse_fill_holder" => c.sparse_fill_holder.as_str(),
        _ => return None,
    })
}

fn resolved_bool(c: &ResolvedConfig, key: &str) -> Option<bool> {
    Some(match key {
        "flat_bridge_square_closing" => c.flat_bridge_square_closing,
        _ => return None,
    })
}

#[test]
fn speeds_match_feedrate_default() {
    // Exhaustive destructuring makes this bidirectional at compile time:
    // adding a field to `FeedrateConfig` fails to compile here until it is
    // listed below and in host-keys.toml `[speeds]`.
    let FeedrateConfig {
        outer_wall_speed,
        inner_wall_speed,
        thin_wall_speed,
        top_surface_speed,
        bottom_surface_speed,
        sparse_infill_speed,
        bridge_speed,
        internal_bridge_speed,
        support_speed,
        support_interface_speed,
        gap_infill_speed,
        ironing_speed,
        skirt_speed,
        wipe_tower_speed,
        prime_tower_speed,
        travel_speed,
        travel_speed_z,
        initial_layer_speed,
        initial_layer_infill_speed,
        initial_layer_travel_speed,
        wipe_speed,
        filament_ironing_speed,
        overhang_1_4_speed,
        overhang_2_4_speed,
        overhang_3_4_speed,
        overhang_4_4_speed,
    } = FeedrateConfig::default();
    let fields: [(&str, f64); 26] = [
        ("outer_wall_speed", outer_wall_speed as f64),
        ("inner_wall_speed", inner_wall_speed as f64),
        ("thin_wall_speed", thin_wall_speed as f64),
        ("top_surface_speed", top_surface_speed as f64),
        ("bottom_surface_speed", bottom_surface_speed as f64),
        ("sparse_infill_speed", sparse_infill_speed as f64),
        ("bridge_speed", bridge_speed as f64),
        ("internal_bridge_speed", internal_bridge_speed as f64),
        ("support_speed", support_speed as f64),
        ("support_interface_speed", support_interface_speed as f64),
        ("gap_infill_speed", gap_infill_speed as f64),
        ("ironing_speed", ironing_speed as f64),
        ("skirt_speed", skirt_speed as f64),
        ("wipe_tower_speed", wipe_tower_speed as f64),
        ("prime_tower_speed", prime_tower_speed as f64),
        ("travel_speed", travel_speed as f64),
        ("travel_speed_z", travel_speed_z as f64),
        ("initial_layer_speed", initial_layer_speed as f64),
        (
            "initial_layer_infill_speed",
            initial_layer_infill_speed as f64,
        ),
        (
            "initial_layer_travel_speed",
            initial_layer_travel_speed as f64,
        ),
        ("wipe_speed", wipe_speed as f64),
        ("filament_ironing_speed", filament_ironing_speed as f64),
        ("overhang_1_4_speed", overhang_1_4_speed as f64),
        ("overhang_2_4_speed", overhang_2_4_speed as f64),
        ("overhang_3_4_speed", overhang_3_4_speed as f64),
        ("overhang_4_4_speed", overhang_4_4_speed as f64),
    ];

    let v = host_keys();
    let speeds = v["speeds"].as_table().expect("[speeds] table");

    // code -> toml: every struct field is present and matches.
    for (name, code) in fields {
        let spec = speeds
            .get(name)
            .unwrap_or_else(|| panic!("[speeds.{name}] missing from host-keys.toml"));
        let doc = doc_num(spec);
        assert!(
            (doc - code).abs() < 1e-6,
            "[speeds.{name}]: host-keys.toml={doc} != FeedrateConfig::default()={code}"
        );
    }

    // toml -> code: no extra speed keys that no struct field backs.
    let names: std::collections::HashSet<&str> = fields.iter().map(|(n, _)| *n).collect();
    for key in speeds.keys() {
        assert!(
            names.contains(key.as_str()),
            "[speeds.{key}] has no matching FeedrateConfig field"
        );
    }
}

#[test]
fn host_runtime_keys_match_constants() {
    let v = host_keys();
    let t = v["host_runtime"].as_table().expect("[host_runtime] table");
    assert_eq!(
        t["use_relative_e_distances"]["default"].as_bool().unwrap(),
        slicer_runtime::run::DEFAULT_USE_RELATIVE_E_DISTANCES,
        "host-keys.toml use_relative_e_distances != run::DEFAULT_USE_RELATIVE_E_DISTANCES"
    );
    assert_eq!(
        t["thumbnail_path"]["default"].as_str().unwrap(),
        slicer_runtime::pipeline::DEFAULT_THUMBNAIL_PATH,
        "host-keys.toml thumbnail_path != pipeline::DEFAULT_THUMBNAIL_PATH"
    );
}

#[test]
fn resolved_config_keys_match_default() {
    let v = host_keys();
    let rc = ResolvedConfig::default();
    let table = v["resolved_config"].as_table().expect("[resolved_config]");
    for (key, spec) in table {
        if let Some(expected) = spec["default"].as_str() {
            let code = resolved_str(&rc, key).unwrap_or_else(|| {
                panic!("[resolved_config.{key}] has no matching ResolvedConfig string field")
            });
            assert_eq!(
                expected, code,
                "[resolved_config.{key}]: host-keys.toml={expected:?} != default={code:?}"
            );
        } else if let Some(expected) = spec["default"].as_bool() {
            let code = resolved_bool(&rc, key).unwrap_or_else(|| {
                panic!("[resolved_config.{key}] has no matching ResolvedConfig bool field")
            });
            assert_eq!(
                expected, code,
                "[resolved_config.{key}]: host-keys.toml={expected} != default={code}"
            );
        } else {
            let doc = doc_num(spec);
            let code = resolved_num(&rc, key).unwrap_or_else(|| {
                panic!("[resolved_config.{key}] has no matching ResolvedConfig numeric field")
            });
            assert!(
                (doc - code).abs() < 1e-6,
                "[resolved_config.{key}]: host-keys.toml={doc} != default={code}"
            );
        }
    }
}
