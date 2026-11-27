//! P40 (ticket 47) manifest guard: `machine-gcode-emit.toml` declares the
//! flush temp/speed placeholder pair.
//!
//! Parses the manifest via the `toml` crate (packet-260/262 precedent) and
//! asserts the exact type/default/min/max the AC demands, not just section
//! presence. Canonical ground (`PrintConfig.cpp`): `filament_flush_temp`
//! `coInts` nullable default 0, min 0, max `max_temp` (1500);
//! `filament_flush_volumetric_speed` `coFloats` nullable default 0, min 0,
//! max 200. The manifest default is dead for this class of key (the
//! `ResolvedConfig` macro default is what modules receive — map Notes), but
//! it must still read canonical or the deviation gate fires.

#![allow(missing_docs)]

use toml::Value;

fn manifest() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("machine-gcode-emit.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "machine-gcode-emit.toml must be readable at {}: {e}",
            path.display()
        )
    });
    text.parse::<Value>()
        .expect("machine-gcode-emit.toml must parse as TOML")
}

fn schema_entry<'a>(manifest: &'a Value, key: &str) -> &'a Value {
    manifest
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get(key))
        .unwrap_or_else(|| panic!("machine-gcode-emit.toml is missing [config.schema.{key}]"))
}

#[test]
fn filament_flush_temp_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "filament_flush_temp");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("int"),
        "expected int type for filament_flush_temp"
    );
    assert_eq!(
        entry.get("default").and_then(Value::as_integer),
        Some(0),
        "expected canonical default 0 for filament_flush_temp"
    );
    assert_eq!(
        entry.get("min").and_then(Value::as_integer),
        Some(0),
        "expected canonical min 0 for filament_flush_temp"
    );
    assert_eq!(
        entry.get("max").and_then(Value::as_integer),
        Some(1500),
        "expected canonical max_temp 1500 for filament_flush_temp"
    );
}

#[test]
fn filament_flush_volumetric_speed_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "filament_flush_volumetric_speed");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("float"),
        "expected float type for filament_flush_volumetric_speed"
    );
    let default = entry
        .get("default")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float default for filament_flush_volumetric_speed"));
    assert!(
        (default - 0.0).abs() < f64::EPSILON,
        "expected canonical default 0.0 for filament_flush_volumetric_speed, got {default}"
    );
    let min = entry
        .get("min")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float min for filament_flush_volumetric_speed"));
    assert!(
        (min - 0.0).abs() < f64::EPSILON,
        "expected min 0.0 for filament_flush_volumetric_speed, got {min}"
    );
    let max = entry
        .get("max")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float max for filament_flush_volumetric_speed"));
    assert!(
        (max - 200.0).abs() < f64::EPSILON,
        "expected canonical max 200.0 for filament_flush_volumetric_speed, got {max}"
    );
}
