//! P43 (ticket 50) manifest guard: `machine-gcode-emit.toml` declares the
//! manual-filament-change gate.
//!
//! Parses the manifest via the `toml` crate (packet-260/262 precedent) and
//! asserts the exact type/default the AC demands, not just section presence.
//! Canonical ground (`PrintConfig.cpp`): `manual_filament_change` `coBool`
//! default false. The manifest default is dead for this class of key (the
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
fn manual_filament_change_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "manual_filament_change");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("bool"),
        "expected bool type for manual_filament_change"
    );
    assert_eq!(
        entry.get("default").and_then(Value::as_bool),
        Some(false),
        "expected canonical default false for manual_filament_change"
    );
}
