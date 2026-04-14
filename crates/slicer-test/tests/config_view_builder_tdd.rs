//! TDD tests for ConfigViewBuilder — all five ConfigValue variants.

use slicer_ir::ConfigValue;
use slicer_test::fixtures::ConfigViewBuilder;

#[test]
fn empty_builder_produces_empty_config() {
    let config = ConfigViewBuilder::new().build();
    assert!(config.fields.is_empty());
}

#[test]
fn bool_true_value() {
    let config = ConfigViewBuilder::new().bool("enabled", true).build();
    assert_eq!(config.fields.get("enabled"), Some(&ConfigValue::Bool(true)));
}

#[test]
fn bool_false_value() {
    let config = ConfigViewBuilder::new().bool("enabled", false).build();
    assert_eq!(
        config.fields.get("enabled"),
        Some(&ConfigValue::Bool(false))
    );
}

#[test]
fn int_value() {
    let config = ConfigViewBuilder::new().int("count", 42).build();
    assert_eq!(config.fields.get("count"), Some(&ConfigValue::Int(42)));
}

#[test]
fn float_value() {
    let config = ConfigViewBuilder::new().float("density", 0.15).build();
    assert_eq!(
        config.fields.get("density"),
        Some(&ConfigValue::Float(0.15))
    );
}

#[test]
fn string_value() {
    let config = ConfigViewBuilder::new().string("pattern", "grid").build();
    assert_eq!(
        config.fields.get("pattern"),
        Some(&ConfigValue::String("grid".to_string()))
    );
}

#[test]
fn list_of_floats() {
    let values = vec![ConfigValue::Float(1.0), ConfigValue::Float(2.0)];
    let config = ConfigViewBuilder::new()
        .list("speeds", values.clone())
        .build();
    assert_eq!(
        config.fields.get("speeds"),
        Some(&ConfigValue::List(values))
    );
}

#[test]
fn list_empty() {
    let config = ConfigViewBuilder::new().list("empty", vec![]).build();
    assert_eq!(config.fields.get("empty"), Some(&ConfigValue::List(vec![])));
}

#[test]
fn list_of_mixed_types() {
    let values = vec![
        ConfigValue::Int(1),
        ConfigValue::String("two".to_string()),
        ConfigValue::Bool(true),
    ];
    let config = ConfigViewBuilder::new()
        .list("mixed", values.clone())
        .build();
    assert_eq!(config.fields.get("mixed"), Some(&ConfigValue::List(values)));
}

#[test]
fn chaining_all_variants() {
    let config = ConfigViewBuilder::new()
        .bool("flag", true)
        .int("count", 5)
        .float("ratio", 0.5)
        .string("name", "test")
        .list("items", vec![ConfigValue::Int(1)])
        .build();
    assert_eq!(config.fields.len(), 5);
    assert_eq!(config.fields.get("flag"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.fields.get("count"), Some(&ConfigValue::Int(5)));
    assert_eq!(config.fields.get("ratio"), Some(&ConfigValue::Float(0.5)));
    assert_eq!(
        config.fields.get("name"),
        Some(&ConfigValue::String("test".to_string()))
    );
    assert_eq!(
        config.fields.get("items"),
        Some(&ConfigValue::List(vec![ConfigValue::Int(1)]))
    );
}

#[test]
fn overwrite_same_key() {
    let config = ConfigViewBuilder::new().int("x", 1).float("x", 2.0).build();
    // Last write wins
    assert_eq!(config.fields.get("x"), Some(&ConfigValue::Float(2.0)));
    assert_eq!(config.fields.len(), 1);
}

#[test]
fn overwrite_bool_with_string() {
    let config = ConfigViewBuilder::new()
        .bool("x", true)
        .string("x", "hello")
        .build();
    assert_eq!(
        config.fields.get("x"),
        Some(&ConfigValue::String("hello".to_string()))
    );
}
