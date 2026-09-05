//! TDD tests for TASK-201 / packet 60 Step 1: 7 new precision keys on `ResolvedConfig`.

use slicer_ir::{resolved_config::ResolvedConfig, ConfigValue};

#[test]
fn new_precision_keys_have_orca_defaults() {
    let cfg = ResolvedConfig::default();
    assert_eq!(cfg.gcode_resolution, 0.0125_f32);
    assert_eq!(cfg.infill_resolution, 0.04_f32);
    assert_eq!(cfg.support_resolution, 0.0375_f32);
    assert_eq!(cfg.min_segment_length, 0.05_f32);
    assert_eq!(cfg.gcode_xy_decimals, 3_u32);
    assert_eq!(cfg.perimeter_arc_tolerance, 0.0125_f32);
    assert_eq!(cfg.slice_closing_radius, 0.049_f32);
}

#[test]
fn line_width_defaults_are_auto_sentinels() {
    let cfg = ResolvedConfig::default();
    let widths: Vec<_> = cfg
        .to_config_map()
        .into_iter()
        .filter(|(key, _)| key.ends_with("line_width"))
        .collect();

    assert!(widths.iter().any(|(key, _)| key == "line_width"));
    assert!(widths
        .iter()
        .any(|(key, _)| key == "initial_layer_line_width"));
    for (key, value) in widths {
        // `support_line_width` is canonical `coFloatOrPercent` (238a retype),
        // so its auto sentinel is the non-percent 0.0 float-or-percent shape;
        // plain-float width keys keep the bare 0.0 sentinel.
        let is_auto = matches!(value, ConfigValue::Float(v) if v == 0.0)
            || matches!(
                value,
                ConfigValue::FloatOrPercent {
                    value: v,
                    is_percent: false,
                } if v == 0.0
            );
        assert!(is_auto, "{key} should default to auto, got {value:?}");
    }
}

#[test]
fn explicit_width_round_trips_with_canonical_initial_layer_name() {
    let cfg = ResolvedConfig {
        line_width: 0.4_f32,
        initial_layer_line_width: 0.4_f32,
        ..ResolvedConfig::default()
    };

    let map = cfg.to_config_map();
    assert_eq!(
        map.get("line_width"),
        Some(&ConfigValue::Float(f64::from(0.4_f32)))
    );
    assert_eq!(
        map.get("initial_layer_line_width"),
        Some(&ConfigValue::Float(f64::from(0.4_f32)))
    );
    assert!(!map.contains_key("first_layer_line_width"));
}

/// Wayfinder ticket 114: `sparse_infill_speed` defaults to canonical 100.0 —
/// the value the modules receive via `to_config_map` (shadowing the manifold
/// declarations), which is also the host `FeedrateConfig` default — not the
/// old 50.0 whose 50/50 cancellation against the modules' private BASE_SPEED
/// made factor 1.0 coincidental.
#[test]
fn sparse_infill_speed_resolved_default_is_canonical() {
    let cfg = ResolvedConfig::default();
    assert_eq!(cfg.sparse_infill_speed, 100.0_f32);
    assert_eq!(
        cfg.to_config_map().get("sparse_infill_speed"),
        Some(&ConfigValue::Float(f64::from(100.0_f32))),
        "the module-facing config map must carry the canonical default"
    );
}
