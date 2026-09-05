//! Manifest schema guard: `support_line_width` must stay declared on the tree
//! support renderer.
//!
//! P29 (ticket 36): the renderer's `from_config` reads `support_line_width`,
//! but `ConfigView::from_declared` whitelists the raw source by the manifest's
//! own schema keys — an undeclared key is filtered out and the read silently
//! falls back. This guard pins the declaration (canonical `coFloatOrPercent`,
//! default 0.0 = auto) and the exact sibling key set.

#![allow(missing_docs)]

use toml::Value;

fn manifest() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tree-support.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "tree-support.toml must be readable at {}: {error}",
            path.display()
        )
    });
    text.parse::<Value>()
        .expect("tree-support.toml must parse as TOML")
}

fn schema_entry<'a>(manifest: &'a Value, key: &str) -> &'a Value {
    manifest
        .get("config")
        .and_then(|config| config.get("schema"))
        .and_then(|schema| schema.get(key))
        .unwrap_or_else(|| panic!("tree-support.toml is missing [config.schema.{key}]"))
}

#[test]
fn support_line_width_is_declared_with_canonical_shape_and_siblings_unchanged() {
    let actual = manifest();
    let actual_schema = actual
        .get("config")
        .and_then(|config| config.get("schema"))
        .and_then(Value::as_table)
        .expect("[config.schema] must be a table");

    let mut actual_keys: Vec<_> = actual_schema.keys().map(String::as_str).collect();
    actual_keys.sort_unstable();
    assert_eq!(
        actual_keys,
        vec![
            "enable_support",
            "line_width",
            "support_base_pattern_spacing",
            "support_bottom_interface_spacing",
            "support_interface_flow",
            "support_interface_spacing",
            "support_line_width",
            "support_speed",
            "tree_support_wall_count",
        ]
    );

    // Canonical `support_line_width` (`PrintConfig.cpp`, `coFloatOrPercent`,
    // default 0.0 = auto, min 0, percent-form max 1000).
    let width = schema_entry(&actual, "support_line_width");
    assert_eq!(
        width.get("type").and_then(Value::as_str),
        Some("float_or_percent")
    );
    assert_eq!(width.get("default").and_then(Value::as_float), Some(0.0));
    assert_eq!(width.get("min").and_then(Value::as_float), Some(0.0));
    assert_eq!(width.get("max").and_then(Value::as_float), Some(1000.0));
}
