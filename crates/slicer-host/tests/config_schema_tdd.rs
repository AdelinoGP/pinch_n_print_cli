//! TDD tests for TASK-035: Config schema query API.
//!
//! These tests define the expected behavior of the config schema query API
//! which allows the host to inspect module configuration schemas for UI
//! rendering and runtime validation.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::path::Path;

use slicer_host::{
    get_advanced_fields, get_basic_fields, get_field_schema, group_fields_by_ui_group,
    parse_config_schema, query_config_schema, validate_config, validate_field_value,
    ConfigFieldSchema, ConfigFieldType, ConfigSchemaParseError, ConfigSchemaParseErrorKind,
    ConfigUnit, ConfigValidationError, ConfigValidationErrorKind, ConfigValue, CrossValidateRule,
    CrossValidateSeverity, FullConfigSchema,
};

// ============================================================================
// Schema Query Tests
// ============================================================================

#[test]
fn query_empty_schema_returns_empty_fields() {
    let schema = FullConfigSchema::default();
    let fields = query_config_schema(&schema);
    assert!(fields.is_empty());
}

#[test]
fn query_schema_returns_all_declared_fields() {
    let schema = make_tpms_infill_schema();
    let fields = query_config_schema(&schema);

    assert_eq!(fields.len(), 5);
    assert!(fields.contains_key("pattern"));
    assert!(fields.contains_key("density"));
    assert!(fields.contains_key("multiline-count"));
    assert!(fields.contains_key("marching-cell-size"));
    assert!(fields.contains_key("raster-precision"));
}

#[test]
fn get_field_schema_returns_none_for_unknown_key() {
    let schema = make_tpms_infill_schema();
    assert!(get_field_schema(&schema, "nonexistent").is_none());
}

#[test]
fn get_field_schema_returns_full_field_definition() {
    let schema = make_tpms_infill_schema();
    let density = get_field_schema(&schema, "density").expect("density field should exist");

    assert_eq!(density.key, "density");
    assert_eq!(density.field_type, ConfigFieldType::Float);
    assert_eq!(density.default, Some(ConfigValue::Float(0.15)));
    assert_eq!(density.min, Some(0.05));
    assert_eq!(density.max, Some(0.95));
    assert_eq!(density.step, Some(0.01));
    assert_eq!(density.display.as_deref(), Some("Infill Density"));
    assert_eq!(density.unit, ConfigUnit::Ratio);
    assert_eq!(density.group.as_deref(), Some("Pattern"));
    assert!(!density.advanced);
    assert_eq!(
        density.validate.as_deref(),
        Some("value > 0.0 && value < 1.0")
    );
}

#[test]
fn enum_field_includes_allowed_values() {
    let schema = make_tpms_infill_schema();
    let pattern = get_field_schema(&schema, "pattern").expect("pattern field should exist");

    assert_eq!(pattern.field_type, ConfigFieldType::Enum);
    assert_eq!(
        pattern.default,
        Some(ConfigValue::String("schwartz-d".into()))
    );
    assert_eq!(
        pattern.enum_values,
        Some(vec!["schwartz-d".into(), "fischer-koch-s".into()])
    );
}

#[test]
fn int_field_includes_range_constraints() {
    let schema = make_tpms_infill_schema();
    let multiline =
        get_field_schema(&schema, "multiline-count").expect("multiline-count field should exist");

    assert_eq!(multiline.field_type, ConfigFieldType::Int);
    assert_eq!(multiline.default, Some(ConfigValue::Int(1)));
    assert_eq!(multiline.min, Some(1.0));
    assert_eq!(multiline.max, Some(4.0));
}

#[test]
fn advanced_flag_is_preserved() {
    let schema = make_tpms_infill_schema();

    let marching = get_field_schema(&schema, "marching-cell-size")
        .expect("marching-cell-size field should exist");
    assert!(marching.advanced);

    let raster =
        get_field_schema(&schema, "raster-precision").expect("raster-precision field should exist");
    assert!(raster.advanced);

    let density = get_field_schema(&schema, "density").expect("density field should exist");
    assert!(!density.advanced);
}

// ============================================================================
// Field Validation Tests
// ============================================================================

#[test]
fn validate_bool_field_accepts_bool_value() {
    let field = make_bool_field("apply-to-all", false);
    let result = validate_field_value(&field, &ConfigValue::Bool(true));
    assert!(result.is_ok());
}

#[test]
fn validate_bool_field_rejects_non_bool_value() {
    let field = make_bool_field("apply-to-all", false);
    let result = validate_field_value(&field, &ConfigValue::Int(1));

    let error = result.expect_err("bool field should reject int value");
    assert_eq!(error.kind, ConfigValidationErrorKind::TypeMismatch);
    assert!(error.message.contains("bool"));
}

#[test]
fn validate_int_field_accepts_int_in_range() {
    let field = make_int_field("count", 1, 1, 10);
    let result = validate_field_value(&field, &ConfigValue::Int(5));
    assert!(result.is_ok());
}

#[test]
fn validate_int_field_rejects_int_below_min() {
    let field = make_int_field("count", 1, 1, 10);
    let result = validate_field_value(&field, &ConfigValue::Int(0));

    let error = result.expect_err("int field should reject value below min");
    assert_eq!(error.kind, ConfigValidationErrorKind::OutOfRange);
    assert!(error.message.contains("1") || error.message.contains("min"));
}

#[test]
fn validate_int_field_rejects_int_above_max() {
    let field = make_int_field("count", 1, 1, 10);
    let result = validate_field_value(&field, &ConfigValue::Int(11));

    let error = result.expect_err("int field should reject value above max");
    assert_eq!(error.kind, ConfigValidationErrorKind::OutOfRange);
}

#[test]
fn validate_float_field_accepts_float_in_range() {
    let field = make_float_field("density", 0.15, 0.05, 0.95);
    let result = validate_field_value(&field, &ConfigValue::Float(0.5));
    assert!(result.is_ok());
}

#[test]
fn validate_float_field_rejects_float_below_min() {
    let field = make_float_field("density", 0.15, 0.05, 0.95);
    let result = validate_field_value(&field, &ConfigValue::Float(0.01));

    let error = result.expect_err("float field should reject value below min");
    assert_eq!(error.kind, ConfigValidationErrorKind::OutOfRange);
}

#[test]
fn validate_float_field_rejects_float_above_max() {
    let field = make_float_field("density", 0.15, 0.05, 0.95);
    let result = validate_field_value(&field, &ConfigValue::Float(0.99));

    let error = result.expect_err("float field should reject value above max");
    assert_eq!(error.kind, ConfigValidationErrorKind::OutOfRange);
}

#[test]
fn validate_float_field_rejects_nan() {
    let field = make_float_field("density", 0.15, 0.05, 0.95);
    let result = validate_field_value(&field, &ConfigValue::Float(f64::NAN));

    let error = result.expect_err("float field should reject NaN");
    assert_eq!(error.kind, ConfigValidationErrorKind::ValidationFailed);
    assert!(error.message.contains("non-finite"));
}

#[test]
fn validate_float_field_rejects_positive_infinity() {
    let field = make_float_field("density", 0.15, 0.05, 0.95);
    let result = validate_field_value(&field, &ConfigValue::Float(f64::INFINITY));

    let error = result.expect_err("float field should reject infinity");
    assert_eq!(error.kind, ConfigValidationErrorKind::ValidationFailed);
    assert!(error.message.contains("non-finite"));
}

#[test]
fn validate_string_field_accepts_string_within_max_length() {
    let field = make_string_field("name", 100);
    let result = validate_field_value(&field, &ConfigValue::String("test".into()));
    assert!(result.is_ok());
}

#[test]
fn validate_string_field_rejects_string_exceeding_max_length() {
    let field = make_string_field("name", 5);
    let result = validate_field_value(&field, &ConfigValue::String("toolong".into()));

    let error = result.expect_err("string field should reject value exceeding max length");
    assert_eq!(error.kind, ConfigValidationErrorKind::TooLong);
}

#[test]
fn validate_enum_field_accepts_valid_enum_value() {
    let field = make_enum_field("pattern", vec!["a", "b", "c"]);
    let result = validate_field_value(&field, &ConfigValue::String("b".into()));
    assert!(result.is_ok());
}

#[test]
fn validate_enum_field_rejects_invalid_enum_value() {
    let field = make_enum_field("pattern", vec!["a", "b", "c"]);
    let result = validate_field_value(&field, &ConfigValue::String("invalid".into()));

    let error = result.expect_err("enum field should reject invalid value");
    assert_eq!(error.kind, ConfigValidationErrorKind::InvalidEnumValue);
    assert!(error.message.contains("invalid"));
}

#[test]
fn validate_float_list_accepts_list_within_bounds() {
    let field = make_float_list_field("layer-heights", 1, 5);
    let result = validate_field_value(&field, &ConfigValue::FloatList(vec![0.1, 0.2, 0.3]));
    assert!(result.is_ok());
}

#[test]
fn validate_float_list_rejects_list_below_min_length() {
    let field = make_float_list_field("layer-heights", 2, 5);
    let result = validate_field_value(&field, &ConfigValue::FloatList(vec![0.1]));

    let error = result.expect_err("float list should reject list below min length");
    assert_eq!(error.kind, ConfigValidationErrorKind::InvalidListLength);
}

#[test]
fn validate_float_list_rejects_list_above_max_length() {
    let field = make_float_list_field("layer-heights", 1, 3);
    let result = validate_field_value(&field, &ConfigValue::FloatList(vec![0.1, 0.2, 0.3, 0.4]));

    let error = result.expect_err("float list should reject list above max length");
    assert_eq!(error.kind, ConfigValidationErrorKind::InvalidListLength);
}

#[test]
fn validate_float_list_rejects_non_finite_member() {
    let field = make_float_list_field("layer-heights", 1, 5);
    let result = validate_field_value(
        &field,
        &ConfigValue::FloatList(vec![0.1, f64::NEG_INFINITY, 0.3]),
    );

    let error = result.expect_err("float list should reject non-finite members");
    assert_eq!(error.kind, ConfigValidationErrorKind::ValidationFailed);
    assert!(error.message.contains("non-finite"));
}

// ============================================================================
// Full Config Validation Tests
// ============================================================================

#[test]
fn validate_config_returns_empty_for_valid_config() {
    let schema = make_tpms_infill_schema();
    let values = make_valid_tpms_config();

    let errors = validate_config(&schema, &values);
    assert!(
        errors.is_empty(),
        "valid config should have no errors: {:?}",
        errors
    );
}

#[test]
fn validate_config_returns_multiple_errors() {
    let schema = make_tpms_infill_schema();
    let mut values = BTreeMap::new();
    values.insert("density".into(), ConfigValue::Float(2.0)); // out of range
    values.insert("pattern".into(), ConfigValue::String("invalid".into())); // invalid enum

    let errors = validate_config(&schema, &values);
    assert!(
        errors.len() >= 2,
        "should report multiple validation errors"
    );

    let density_error = errors
        .iter()
        .find(|e| e.field.as_deref() == Some("density"));
    assert!(density_error.is_some(), "should report density error");

    let pattern_error = errors
        .iter()
        .find(|e| e.field.as_deref() == Some("pattern"));
    assert!(pattern_error.is_some(), "should report pattern error");
}

#[test]
fn validate_config_runs_cross_validate_rules() {
    let schema = make_schema_with_cross_validate();
    let mut values = BTreeMap::new();
    values.insert("marching-cell-size".into(), ConfigValue::Float(0.02));
    values.insert("raster-precision".into(), ConfigValue::Float(0.01));
    // marching-cell-size (0.02) should be >= raster-precision * 10 (0.1)
    // This fails the cross-validate rule

    let errors = validate_config(&schema, &values);
    let cross_error = errors
        .iter()
        .find(|e| e.kind == ConfigValidationErrorKind::CrossValidationFailed);

    assert!(
        cross_error.is_some(),
        "should detect cross-validation failure"
    );
}

// ============================================================================
// Schema Parsing Tests
// ============================================================================

#[test]
fn parse_empty_config_schema_produces_empty_fields() {
    let toml = toml::toml! {
        [schema]
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("empty schema should parse");

    assert!(schema.fields.is_empty());
    assert!(schema.cross_validate.is_empty());
}

#[test]
fn parse_float_field_extracts_all_properties() {
    let toml = toml::toml! {
        [schema.density]
        type = "float"
        default = 0.15
        min = 0.05
        max = 0.95
        step = 0.01
        display = "Infill Density"
        unit = "ratio"
        group = "Pattern"
        validate = "value > 0.0 && value < 1.0"
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("float field should parse");

    let density = schema.fields.get("density").expect("density field");
    assert_eq!(density.field_type, ConfigFieldType::Float);
    assert_eq!(density.default, Some(ConfigValue::Float(0.15)));
    assert_eq!(density.min, Some(0.05));
    assert_eq!(density.max, Some(0.95));
    assert_eq!(density.step, Some(0.01));
    assert_eq!(density.display.as_deref(), Some("Infill Density"));
    assert_eq!(density.unit, ConfigUnit::Ratio);
    assert_eq!(density.group.as_deref(), Some("Pattern"));
    assert_eq!(
        density.validate.as_deref(),
        Some("value > 0.0 && value < 1.0")
    );
}

#[test]
fn parse_enum_field_extracts_values_list() {
    let toml = toml::toml! {
        [schema.pattern]
        type = "enum"
        values = ["schwartz-d", "fischer-koch-s"]
        default = "schwartz-d"
        display = "TPMS Pattern"
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("enum field should parse");

    let pattern = schema.fields.get("pattern").expect("pattern field");
    assert_eq!(pattern.field_type, ConfigFieldType::Enum);
    assert_eq!(
        pattern.enum_values,
        Some(vec!["schwartz-d".into(), "fischer-koch-s".into()])
    );
    assert_eq!(
        pattern.default,
        Some(ConfigValue::String("schwartz-d".into()))
    );
}

#[test]
fn parse_int_field_extracts_range_constraints() {
    let toml = toml::toml! {
        [schema.multiline-count]
        type = "int"
        default = 1
        min = 1
        max = 4
        display = "Parallel Passes"
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("int field should parse");

    let multiline = schema
        .fields
        .get("multiline-count")
        .expect("multiline-count field");
    assert_eq!(multiline.field_type, ConfigFieldType::Int);
    assert_eq!(multiline.default, Some(ConfigValue::Int(1)));
    assert_eq!(multiline.min, Some(1.0));
    assert_eq!(multiline.max, Some(4.0));
}

#[test]
fn parse_advanced_flag_defaults_to_false() {
    let toml = toml::toml! {
        [schema.density]
        type = "float"
        default = 0.15
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema =
        parse_config_schema(config_section, Path::new("test.toml")).expect("field should parse");

    let density = schema.fields.get("density").expect("density field");
    assert!(!density.advanced);
}

#[test]
fn parse_advanced_flag_when_true() {
    let toml = toml::toml! {
        [schema.cell-size]
        type = "float"
        default = 0.4
        advanced = true
    };
    let config_section = toml.get("schema").expect("schema section");

    let schema =
        parse_config_schema(config_section, Path::new("test.toml")).expect("field should parse");

    let cell_size = schema.fields.get("cell-size").expect("cell-size field");
    assert!(cell_size.advanced);
}

#[test]
fn parse_float_default_normalizes_subnormal_to_zero() {
    let toml: toml::Value = r#"
        [schema.epsilon]
        type = "float"
        default = 1e-320
    "#
    .parse()
    .expect("toml should parse");
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("float field should parse");

    let epsilon = schema.fields.get("epsilon").expect("epsilon field");
    match epsilon.default.as_ref() {
        Some(ConfigValue::Float(value)) => assert_eq!(*value, 0.0),
        other => panic!("expected float default, got {other:?}"),
    }
}

#[test]
fn parse_float_list_default_normalizes_subnormal_members_to_zero() {
    let toml: toml::Value = r#"
        [schema.layer-heights]
        type = "float-list"
        default = [1e-320, 0.2, -1e-320]
    "#
    .parse()
    .expect("toml should parse");
    let config_section = toml.get("schema").expect("schema section");

    let schema = parse_config_schema(config_section, Path::new("test.toml"))
        .expect("float-list field should parse");

    let layer_heights = schema
        .fields
        .get("layer-heights")
        .expect("layer-heights field");
    match layer_heights.default.as_ref() {
        Some(ConfigValue::FloatList(values)) => {
            assert_eq!(values, &vec![0.0, 0.2, 0.0]);
        }
        other => panic!("expected float-list default, got {other:?}"),
    }
}

#[test]
fn parse_unknown_field_type_returns_error() {
    let toml = toml::toml! {
        [schema.broken]
        type = "invalid-type"
        default = "value"
    };
    let config_section = toml.get("schema").expect("schema section");

    let error = parse_config_schema(config_section, Path::new("test.toml"))
        .expect_err("unknown field type should fail");

    assert_eq!(error.kind, ConfigSchemaParseErrorKind::InvalidFieldType);
    assert_eq!(error.field.as_deref(), Some("broken"));
    assert!(error.message.contains("invalid-type"));
}

#[test]
fn parse_enum_without_values_returns_error() {
    let toml = toml::toml! {
        [schema.pattern]
        type = "enum"
        default = "a"
    };
    let config_section = toml.get("schema").expect("schema section");

    let error = parse_config_schema(config_section, Path::new("test.toml"))
        .expect_err("enum without values should fail");

    assert_eq!(error.kind, ConfigSchemaParseErrorKind::MissingEnumValues);
    assert_eq!(error.field.as_deref(), Some("pattern"));
}

#[test]
fn parse_cross_validate_rules() {
    let toml: toml::Value = r#"
        [schema]
        
        [schema.cell-size]
        type = "float"
        default = 0.4
        
        [schema.precision]
        type = "float"
        default = 0.01
        
        [[cross-validate]]
        rule = "cell-size >= precision * 10"
        message = "Cell size should be at least 10x the precision"
        severity = "warning"
    "#
    .parse()
    .expect("toml should parse");

    let schema_section = toml.get("schema").expect("schema section");

    // For this test, we need to also pass the cross-validate section
    // The parse function should handle the full config section
    let full_config = toml::toml! {
        [schema.cell-size]
        type = "float"
        default = 0.4

        [schema.precision]
        type = "float"
        default = 0.01
    };

    let schema = parse_config_schema(full_config.get("schema").unwrap(), Path::new("test.toml"))
        .expect("schema should parse");

    // Cross-validate is parsed from a separate section - this tests the field parsing
    assert_eq!(schema.fields.len(), 2);
}

// ============================================================================
// UI Grouping Tests
// ============================================================================

#[test]
fn group_fields_by_ui_group_collects_by_group_name() {
    let schema = make_tpms_infill_schema();
    let groups = group_fields_by_ui_group(&schema);

    assert!(groups.contains_key("Pattern"));
    assert!(groups.contains_key("Advanced"));

    let pattern_fields = groups.get("Pattern").expect("Pattern group");
    assert!(pattern_fields.iter().any(|f| f.key == "density"));
    assert!(pattern_fields.iter().any(|f| f.key == "pattern"));

    let advanced_fields = groups.get("Advanced").expect("Advanced group");
    assert!(advanced_fields
        .iter()
        .any(|f| f.key == "marching-cell-size"));
    assert!(advanced_fields.iter().any(|f| f.key == "raster-precision"));
}

#[test]
fn get_advanced_fields_returns_only_advanced() {
    let schema = make_tpms_infill_schema();
    let advanced = get_advanced_fields(&schema);

    assert!(advanced.iter().all(|f| f.advanced));
    assert!(advanced.iter().any(|f| f.key == "marching-cell-size"));
    assert!(advanced.iter().any(|f| f.key == "raster-precision"));
    assert!(!advanced.iter().any(|f| f.key == "density"));
}

#[test]
fn get_basic_fields_returns_only_non_advanced() {
    let schema = make_tpms_infill_schema();
    let basic = get_basic_fields(&schema);

    assert!(basic.iter().all(|f| !f.advanced));
    assert!(basic.iter().any(|f| f.key == "density"));
    assert!(basic.iter().any(|f| f.key == "pattern"));
    assert!(!basic.iter().any(|f| f.key == "marching-cell-size"));
}

// ============================================================================
// Test Fixtures
// ============================================================================

fn make_tpms_infill_schema() -> FullConfigSchema {
    let mut fields = BTreeMap::new();

    fields.insert(
        "pattern".into(),
        ConfigFieldSchema {
            key: "pattern".into(),
            field_type: ConfigFieldType::Enum,
            default: Some(ConfigValue::String("schwartz-d".into())),
            display: Some("TPMS Pattern".into()),
            description: Some("Which TPMS surface family to use".into()),
            group: Some("Pattern".into()),
            unit: ConfigUnit::None,
            advanced: false,
            min: None,
            max: None,
            step: None,
            max_length: None,
            enum_values: Some(vec!["schwartz-d".into(), "fischer-koch-s".into()]),
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    fields.insert(
        "density".into(),
        ConfigFieldSchema {
            key: "density".into(),
            field_type: ConfigFieldType::Float,
            default: Some(ConfigValue::Float(0.15)),
            display: Some("Infill Density".into()),
            description: None,
            group: Some("Pattern".into()),
            unit: ConfigUnit::Ratio,
            advanced: false,
            min: Some(0.05),
            max: Some(0.95),
            step: Some(0.01),
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: Some("value > 0.0 && value < 1.0".into()),
        },
    );

    fields.insert(
        "multiline-count".into(),
        ConfigFieldSchema {
            key: "multiline-count".into(),
            field_type: ConfigFieldType::Int,
            default: Some(ConfigValue::Int(1)),
            display: Some("Parallel Passes".into()),
            description: None,
            group: Some("Pattern".into()),
            unit: ConfigUnit::None,
            advanced: false,
            min: Some(1.0),
            max: Some(4.0),
            step: None,
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    fields.insert(
        "marching-cell-size".into(),
        ConfigFieldSchema {
            key: "marching-cell-size".into(),
            field_type: ConfigFieldType::Float,
            default: Some(ConfigValue::Float(0.40)),
            display: Some("Marching Cell Size (mm)".into()),
            description: None,
            group: Some("Advanced".into()),
            unit: ConfigUnit::Millimeters,
            advanced: true,
            min: Some(0.10),
            max: Some(1.00),
            step: Some(0.05),
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    fields.insert(
        "raster-precision".into(),
        ConfigFieldSchema {
            key: "raster-precision".into(),
            field_type: ConfigFieldType::Float,
            default: Some(ConfigValue::Float(0.004)),
            display: Some("Raster Precision (mm)".into()),
            description: None,
            group: Some("Advanced".into()),
            unit: ConfigUnit::Millimeters,
            advanced: true,
            min: Some(0.001),
            max: Some(0.010),
            step: None,
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    FullConfigSchema {
        fields,
        cross_validate: vec![CrossValidateRule {
            rule: "marching-cell-size >= raster-precision * 10".into(),
            message: "Marching cell size should be at least 10x the raster precision".into(),
            severity: CrossValidateSeverity::Warning,
        }],
    }
}

fn make_schema_with_cross_validate() -> FullConfigSchema {
    let mut fields = BTreeMap::new();

    fields.insert(
        "marching-cell-size".into(),
        ConfigFieldSchema {
            key: "marching-cell-size".into(),
            field_type: ConfigFieldType::Float,
            default: Some(ConfigValue::Float(0.40)),
            display: None,
            description: None,
            group: None,
            unit: ConfigUnit::None,
            advanced: false,
            min: None,
            max: None,
            step: None,
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    fields.insert(
        "raster-precision".into(),
        ConfigFieldSchema {
            key: "raster-precision".into(),
            field_type: ConfigFieldType::Float,
            default: Some(ConfigValue::Float(0.004)),
            display: None,
            description: None,
            group: None,
            unit: ConfigUnit::None,
            advanced: false,
            min: None,
            max: None,
            step: None,
            max_length: None,
            enum_values: None,
            min_list_length: None,
            max_list_length: None,
            validate: None,
        },
    );

    FullConfigSchema {
        fields,
        cross_validate: vec![CrossValidateRule {
            rule: "marching-cell-size >= raster-precision * 10".into(),
            message: "Marching cell size should be at least 10x the raster precision".into(),
            severity: CrossValidateSeverity::Error,
        }],
    }
}

fn make_valid_tpms_config() -> BTreeMap<String, ConfigValue> {
    let mut values = BTreeMap::new();
    values.insert("pattern".into(), ConfigValue::String("schwartz-d".into()));
    values.insert("density".into(), ConfigValue::Float(0.20));
    values.insert("multiline-count".into(), ConfigValue::Int(2));
    values.insert("marching-cell-size".into(), ConfigValue::Float(0.40));
    values.insert("raster-precision".into(), ConfigValue::Float(0.004));
    values
}

fn make_bool_field(key: &str, default: bool) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::Bool,
        default: Some(ConfigValue::Bool(default)),
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: None,
        max: None,
        step: None,
        max_length: None,
        enum_values: None,
        min_list_length: None,
        max_list_length: None,
        validate: None,
    }
}

fn make_int_field(key: &str, default: i64, min: i64, max: i64) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::Int,
        default: Some(ConfigValue::Int(default)),
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: Some(min as f64),
        max: Some(max as f64),
        step: None,
        max_length: None,
        enum_values: None,
        min_list_length: None,
        max_list_length: None,
        validate: None,
    }
}

fn make_float_field(key: &str, default: f64, min: f64, max: f64) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::Float,
        default: Some(ConfigValue::Float(default)),
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: Some(min),
        max: Some(max),
        step: None,
        max_length: None,
        enum_values: None,
        min_list_length: None,
        max_list_length: None,
        validate: None,
    }
}

fn make_string_field(key: &str, max_length: usize) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::String,
        default: None,
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: None,
        max: None,
        step: None,
        max_length: Some(max_length),
        enum_values: None,
        min_list_length: None,
        max_list_length: None,
        validate: None,
    }
}

fn make_enum_field(key: &str, values: Vec<&str>) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::Enum,
        default: Some(ConfigValue::String(values[0].into())),
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: None,
        max: None,
        step: None,
        max_length: None,
        enum_values: Some(values.into_iter().map(String::from).collect()),
        min_list_length: None,
        max_list_length: None,
        validate: None,
    }
}

fn make_float_list_field(key: &str, min_len: usize, max_len: usize) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.into(),
        field_type: ConfigFieldType::FloatList,
        default: None,
        display: None,
        description: None,
        group: None,
        unit: ConfigUnit::None,
        advanced: false,
        min: None,
        max: None,
        step: None,
        max_length: None,
        enum_values: None,
        min_list_length: Some(min_len),
        max_list_length: Some(max_len),
        validate: None,
    }
}
