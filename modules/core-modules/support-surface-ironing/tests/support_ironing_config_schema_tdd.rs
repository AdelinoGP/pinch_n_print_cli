//! Manifest schema guard for the support-surface ironing gate.

#![allow(missing_docs)]

use toml::Value;

fn manifest() -> Value {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("support-surface-ironing.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "support-surface-ironing.toml must be readable at {}: {error}",
            path.display()
        )
    });
    text.parse::<Value>()
        .expect("support-surface-ironing.toml must parse as TOML")
}

fn schema_entry<'a>(manifest: &'a Value, key: &str) -> &'a Value {
    manifest
        .get("config")
        .and_then(|config| config.get("schema"))
        .and_then(|schema| schema.get(key))
        .unwrap_or_else(|| panic!("support-surface-ironing.toml is missing [config.schema.{key}]"))
}

#[test]
fn support_ironing_is_the_only_gate_and_sibling_schema_is_unchanged() {
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
            "line_width",
            "support_ironing",
            "support_ironing_flow",
            "support_ironing_spacing",
            "support_ironing_speed",
        ]
    );

    let gate = schema_entry(&actual, "support_ironing");
    assert_eq!(gate.get("type").and_then(Value::as_str), Some("bool"));
    assert_eq!(gate.get("default").and_then(Value::as_bool), Some(false));
    assert_eq!(
        gate.get("display").and_then(Value::as_str),
        Some("Ironing Support Interface")
    );
    assert_eq!(gate.get("group").and_then(Value::as_str), Some("Support"));
    assert!(
        actual_schema.get("ironing_enabled").is_none(),
        "the legacy support gate must not remain declared"
    );
    assert!(
        !slicer_ir::feedrate::SPEED_KEYS
            .iter()
            .any(|(key, _)| *key == "support_ironing_speed"),
        "support_ironing_speed is module-owned, not a host feedrate key"
    );

    let expected: Value = r#"
[config.schema.support_ironing_speed]
type = "float"
default = 30.0
min = 1.0
max = 300.0
display = "Ironing Speed"
group = "Support"

[config.schema.support_ironing_flow]
type = "float"
default = 0.10
min = 0.01
max = 1.0
display = "Ironing Flow Rate"
group = "Support"

[config.schema.support_ironing_spacing]
type = "float"
default = 0.1
min = 0.01
max = 1.0
display = "Ironing Spacing"
group = "Support"

[config.schema.line_width]
type = "float"
default = 0.4
min = 0.1
max = 2.0
display = "Line Width"
group = "Support"
"#
    .parse()
    .expect("expected sibling schema must parse as TOML");
    let expected_schema = expected
        .get("config")
        .and_then(|config| config.get("schema"))
        .and_then(Value::as_table)
        .expect("expected [config.schema] must be a table");

    for key in [
        "support_ironing_flow",
        "support_ironing_spacing",
        "support_ironing_speed",
        "line_width",
    ] {
        assert_eq!(
            actual_schema.get(key),
            expected_schema.get(key),
            "sibling schema changed for {key}"
        );
    }
}
