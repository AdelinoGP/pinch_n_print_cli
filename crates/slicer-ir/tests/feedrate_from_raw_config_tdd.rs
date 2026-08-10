#![allow(missing_docs)]

//! TDD tests for `FeedrateConfig::from_raw_config`, the bridge that wires the
//! host `[speeds]` keys (Orca key names, mm/s) from the raw config source into
//! the G-code emitter's feedrate table. Before this existed the production
//! emitter used `FeedrateConfig::default()` and every F value was a pnp
//! default scaled by module speed factors.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, FeedrateConfig};

fn raw(pairs: &[(&str, ConfigValue)]) -> HashMap<String, ConfigValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn from_raw_config_overrides_known_keys_and_keeps_defaults() {
    let config = raw(&[
        ("outer_wall_speed", ConfigValue::Float(200.0)),
        ("inner_wall_speed", ConfigValue::Float(150.0)),
        ("sparse_infill_speed", ConfigValue::Float(300.0)),
        ("travel_speed", ConfigValue::Float(500.0)),
    ]);

    let fc = FeedrateConfig::from_raw_config(&config);
    assert_eq!(fc.outer_wall_speed, 200.0);
    assert_eq!(fc.inner_wall_speed, 150.0);
    assert_eq!(fc.sparse_infill_speed, 300.0);
    assert_eq!(fc.travel_speed, 500.0);
    // Absent keys keep the defaults.
    assert_eq!(fc.thin_wall_speed, 30.0);
    assert_eq!(fc.top_surface_speed, 100.0);
    assert_eq!(fc.bridge_speed, 25.0);
    assert_eq!(fc.support_interface_speed, 80.0);
    assert_eq!(fc.wipe_speed, 96.0);
}

#[test]
fn from_raw_config_accepts_lists_and_ints() {
    // Orca stores some per-filament speeds as coFloats arrays (List); the
    // generic sidecar extractor keeps them as lists, so the first element
    // must be taken.
    let config = raw(&[
        (
            "filament_ironing_speed",
            ConfigValue::List(vec![ConfigValue::Float(25.0)]),
        ),
        ("overhang_1_4_speed", ConfigValue::Int(30)),
    ]);

    let fc = FeedrateConfig::from_raw_config(&config);
    assert_eq!(fc.filament_ironing_speed, 25.0);
    assert_eq!(fc.overhang_1_4_speed, 30.0);
}

#[test]
fn from_raw_config_ignores_unresolvable_values() {
    // A bare Percent has no base here; a percent-flagged FloatOrPercent cannot
    // be resolved either — both keep the default instead of emitting garbage
    // speeds.
    let config = raw(&[
        ("outer_wall_speed", ConfigValue::Percent(50.0)),
        (
            "inner_wall_speed",
            ConfigValue::FloatOrPercent {
                value: 75.0,
                is_percent: true,
            },
        ),
        ("bridge_speed", ConfigValue::String("fast".to_string())),
        ("skirt_speed", ConfigValue::Bool(true)),
    ]);

    let fc = FeedrateConfig::from_raw_config(&config);
    assert_eq!(fc.outer_wall_speed, 60.0);
    assert_eq!(fc.inner_wall_speed, 60.0);
    assert_eq!(fc.bridge_speed, 25.0);
    assert_eq!(fc.skirt_speed, 50.0);
}

#[test]
fn from_raw_config_empty_map_is_default() {
    let fc = FeedrateConfig::from_raw_config(&HashMap::new());
    assert_eq!(
        fc.outer_wall_speed,
        FeedrateConfig::default().outer_wall_speed
    );
    assert_eq!(fc.travel_speed, FeedrateConfig::default().travel_speed);
    assert_eq!(
        fc.initial_layer_speed,
        FeedrateConfig::default().initial_layer_speed
    );
}
