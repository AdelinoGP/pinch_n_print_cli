//! TDD tests for TASK-201 / packet 60 Step 1: 7 new precision keys on `ResolvedConfig`.

use slicer_ir::{
    resolved_config::{ResolvedConfig, ResolvedFloatOrPercent},
    ConfigValue,
};

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
        line_width: f64::from(0.4_f32),
        initial_layer_line_width: ResolvedFloatOrPercent {
            value: f64::from(0.4_f32),
            is_percent: false,
        },
        ..ResolvedConfig::default()
    };

    let map = cfg.to_config_map();
    assert_eq!(
        map.get("line_width"),
        Some(&ConfigValue::Float(f64::from(0.4_f32)))
    );
    assert_eq!(
        map.get("initial_layer_line_width"),
        Some(&ConfigValue::FloatOrPercent {
            value: f64::from(0.4_f32),
            is_percent: false,
        })
    );
    assert!(!map.contains_key("first_layer_line_width"));
}

#[test]
fn resolved_line_width_retains_f64_precision_in_config_map() {
    let authored = 0.451_234_567_890_123_f64;
    assert_ne!(authored, f64::from(authored as f32));
    let mut resolved = ResolvedConfig::default();
    assert!(resolved
        .apply_cli_key("line_width", &ConfigValue::Float(authored))
        .expect("authored line_width should bind to the typed resolver"));

    assert_eq!(
        resolved.to_config_map().get("line_width"),
        Some(&ConfigValue::Float(authored)),
        "line_width must not narrow to f32 while crossing the resolved-config map"
    );
}

/// Regression: `flat_bridge_closing_join` is a serialization-only carrier.
///
/// Commit `6db49f0f` removed the field from the middle of `ResolvedConfig`, which
/// shifted every later Postcard field and broke the frozen `RegionMapIR` 3.0.0
/// fixture (`DeserializeBadBool`). Restoring it must not resurrect the retired
/// configuration key, so the field is declared with the `carrier` row kind: it
/// participates in the struct, the default, `Eq`/`Hash` and the wire layout, but
/// not in the effective config map, the typed-key roster or CLI dispatch.
#[test]
fn flat_bridge_closing_join_is_a_serialization_only_carrier() {
    const RETIRED_KEY: &str = "flat_bridge_closing_join";
    let mut config = ResolvedConfig::default();

    assert_eq!(
        config.flat_bridge_closing_join, "miter",
        "the carrier keeps the original default so the decoded layout is unchanged"
    );
    assert!(
        !config.to_config_map().contains_key(RETIRED_KEY),
        "the retired key must not reappear in the effective config map"
    );
    assert!(
        !ResolvedConfig::typed_field_keys().contains(&RETIRED_KEY),
        "the retired key must not reappear in the typed field roster"
    );
    assert_eq!(
        config.apply_cli_key(RETIRED_KEY, &ConfigValue::String("round".into())),
        Ok(false),
        "no CLI dispatch may be rebound for a retired key"
    );

    // It is still a serialized field, so the wire layout is what the frozen
    // fixture carries.
    let bytes = postcard::to_allocvec(&config).expect("config is Postcard-encodable");
    let decoded: ResolvedConfig = postcard::from_bytes(&bytes).expect("config bytes decode");
    assert_eq!(
        decoded.flat_bridge_closing_join, "miter",
        "the carrier must round-trip through Postcard"
    );
    assert_eq!(decoded, config, "the carrier must not disturb equality");
}
