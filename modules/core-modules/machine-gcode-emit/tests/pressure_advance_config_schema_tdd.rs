//! P35 (ticket 42) manifest guard: `machine-gcode-emit.toml` declares the
//! static pressure-advance gate.
//!
//! Parses the manifest via the `toml` crate (packet-260/262 precedent) and
//! asserts the exact type/default/min/max the AC demands, not just section
//! presence. Canonical ground (`PrintConfig.cpp`): `enable_pressure_advance`
//! `coBools` default false; `pressure_advance` `coFloats` default 0.02,
//! max 2. The manifest default is dead for this class of key (the
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
fn enable_pressure_advance_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "enable_pressure_advance");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("bool"),
        "expected bool type for enable_pressure_advance"
    );
    assert_eq!(
        entry.get("default").and_then(Value::as_bool),
        Some(false),
        "expected canonical default false for enable_pressure_advance"
    );
}

#[test]
fn pressure_advance_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "pressure_advance");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("float"),
        "expected float type for pressure_advance"
    );
    let default = entry
        .get("default")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float default for pressure_advance"));
    assert!(
        (default - 0.02).abs() < 1e-9,
        "expected canonical default 0.02 for pressure_advance, got {default}"
    );
    let min = entry
        .get("min")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float min for pressure_advance"));
    assert!(
        (min - 0.0).abs() < f64::EPSILON,
        "expected min 0.0 for pressure_advance, got {min}"
    );
    let max = entry
        .get("max")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float max for pressure_advance"));
    assert!(
        (max - 2.0).abs() < f64::EPSILON,
        "expected canonical max 2.0 for pressure_advance, got {max}"
    );
}
