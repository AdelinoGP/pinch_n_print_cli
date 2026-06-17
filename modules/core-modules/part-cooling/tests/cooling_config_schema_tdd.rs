//! Manifest schema guard for part-cooling (relocated from slicer-runtime when
//! the host was decoupled from module crates).
//!
//! Asserts that `part-cooling.toml` declares the eight cooling config keys with
//! the expected types, defaults, and `Cooling` group. Parses the manifest TOML
//! directly (the host's `load_module_from_paths` loader is not a module
//! dependency), so this test is owned by the module that owns the manifest.

#![allow(missing_docs)]

use toml::Value;

fn manifest() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("part-cooling.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "part-cooling.toml must be readable at {}: {e}",
            path.display()
        )
    });
    text.parse::<Value>()
        .expect("part-cooling.toml must parse as TOML")
}

fn schema_entry<'a>(manifest: &'a Value, key: &str) -> &'a Value {
    manifest
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get(key))
        .unwrap_or_else(|| panic!("part-cooling.toml is missing [config.schema.{key}]"))
}

fn assert_group_cooling(entry: &Value, key: &str) {
    assert_eq!(
        entry.get("group").and_then(Value::as_str),
        Some("Cooling"),
        "expected Cooling group for {key}"
    );
}

#[test]
fn int_keys_declared_with_defaults() {
    let m = manifest();
    for (key, default) in [
        ("fan_speed_min", 51),
        ("fan_speed_max", 255),
        ("disable_fan_first_layers", 1),
        ("overhang_fan_speed", 100),
    ] {
        let entry = schema_entry(&m, key);
        assert_eq!(
            entry.get("type").and_then(Value::as_str),
            Some("int"),
            "expected int type for {key}"
        );
        assert_eq!(
            entry.get("default").and_then(Value::as_integer),
            Some(default),
            "incorrect default for {key}"
        );
        assert_group_cooling(entry, key);
    }
}

#[test]
fn bool_keys_declared_with_defaults() {
    let m = manifest();
    for (key, default) in [
        ("enable_overhang_fan", true),
        ("slow_down_for_layer_cooling", true),
    ] {
        let entry = schema_entry(&m, key);
        assert_eq!(
            entry.get("type").and_then(Value::as_str),
            Some("bool"),
            "expected bool type for {key}"
        );
        assert_eq!(
            entry.get("default").and_then(Value::as_bool),
            Some(default),
            "incorrect default for {key}"
        );
        assert_group_cooling(entry, key);
    }
}

#[test]
fn float_keys_declared_with_defaults() {
    let m = manifest();
    for (key, default) in [
        ("slow_down_min_speed", 10.0_f64),
        ("slow_down_layer_time", 5.0),
    ] {
        let entry = schema_entry(&m, key);
        assert_eq!(
            entry.get("type").and_then(Value::as_str),
            Some("float"),
            "expected float type for {key}"
        );
        let parsed = entry
            .get("default")
            .and_then(Value::as_float)
            .unwrap_or_else(|| panic!("missing/non-float default for {key}"));
        assert!(
            (parsed - default).abs() < f64::EPSILON,
            "incorrect default for {key}: expected {default}, got {parsed}"
        );
        assert_group_cooling(entry, key);
    }
}
