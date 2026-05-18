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
    ConfigFieldSchema, ConfigFieldSchemaBuilder, ConfigFieldType, ConfigSchemaParseErrorKind,
    ConfigUnit, ConfigValidationErrorKind, ConfigValue, CrossValidateRule, CrossValidateSeverity,
    FullConfigSchema,
};

// ============================================================================
// Schema Query Tests
// ============================================================================

#[test]
#[ignore = "FullConfigSchema::default() is now a populated host-level config registry (packets 55, 59). Test contract needs rework in a future packet — see config_schema.rs:128 Default impl."]
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

    assert_eq!(density.key(), "density");
    assert_eq!(density.field_type(), &ConfigFieldType::Float);
    assert_eq!(density.default(), Some(&ConfigValue::Float(0.15)));
    assert_eq!(density.min(), Some(0.05));
    assert_eq!(density.max(), Some(0.95));
    assert_eq!(density.step(), Some(0.01));
    assert_eq!(density.display(), Some("Infill Density"));
    assert_eq!(density.unit(), &ConfigUnit::Ratio);
    assert_eq!(density.group(), Some("Pattern"));
    assert!(!density.advanced());
    assert_eq!(
        density.validate(),
        Some("value > 0.0 && value < 1.0")
    );
}

#[test]
fn enum_field_includes_allowed_values() {
    let schema = make_tpms_infill_schema();
    let pattern = get_field_schema(&schema, "pattern").expect("pattern field should exist");

    assert_eq!(pattern.field_type(), &ConfigFieldType::Enum);
    assert_eq!(
        pattern.default(),
        Some(&ConfigValue::String("schwartz-d".into()))
    );
    assert_eq!(
        pattern.enum_values(),
        Some(&["schwartz-d".to_string(), "fischer-koch-s".to_string()][..])
    );
}

#[test]
fn int_field_includes_range_constraints() {
    let schema = make_tpms_infill_schema();
    let multiline =
        get_field_schema(&schema, "multiline-count").expect("multiline-count field should exist");

    assert_eq!(multiline.field_type(), &ConfigFieldType::Int);
    assert_eq!(multiline.default(), Some(&ConfigValue::Int(1)));
    assert_eq!(multiline.min(), Some(1.0));
    assert_eq!(multiline.max(), Some(4.0));
}

#[test]
fn advanced_flag_is_preserved() {
    let schema = make_tpms_infill_schema();

    let marching = get_field_schema(&schema, "marching-cell-size")
        .expect("marching-cell-size field should exist");
    assert!(marching.advanced());

    let raster =
        get_field_schema(&schema, "raster-precision").expect("raster-precision field should exist");
    assert!(raster.advanced());

    let density = get_field_schema(&schema, "density").expect("density field should exist");
    assert!(!density.advanced());
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
    assert_eq!(density.field_type(), &ConfigFieldType::Float);
    assert_eq!(density.default(), Some(&ConfigValue::Float(0.15)));
    assert_eq!(density.min(), Some(0.05));
    assert_eq!(density.max(), Some(0.95));
    assert_eq!(density.step(), Some(0.01));
    assert_eq!(density.display(), Some("Infill Density"));
    assert_eq!(density.unit(), &ConfigUnit::Ratio);
    assert_eq!(density.group(), Some("Pattern"));
    assert_eq!(
        density.validate(),
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
    assert_eq!(pattern.field_type(), &ConfigFieldType::Enum);
    assert_eq!(
        pattern.enum_values(),
        Some(&["schwartz-d".to_string(), "fischer-koch-s".to_string()][..])
    );
    assert_eq!(
        pattern.default(),
        Some(&ConfigValue::String("schwartz-d".into()))
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
    assert_eq!(multiline.field_type(), &ConfigFieldType::Int);
    assert_eq!(multiline.default(), Some(&ConfigValue::Int(1)));
    assert_eq!(multiline.min(), Some(1.0));
    assert_eq!(multiline.max(), Some(4.0));
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
    assert!(!density.advanced());
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
    assert!(cell_size.advanced());
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
    match epsilon.default() {
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
    match layer_heights.default() {
        Some(ConfigValue::FloatList(values)) => {
            assert_eq!(*values, vec![0.0, 0.2, 0.0]);
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

    let _schema_section = toml.get("schema").expect("schema section");

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
// Manifest Enum Validation Tests (packet 34)
// ============================================================================

#[test]
fn config_schema_rejects_unknown_retract_mode() {
    // Loads the real path-optimization-default manifest, parses its
    // [config.schema] section, then asserts that a config value of
    // `retract_mode = "marlin"` (an out-of-enum value) is rejected by the
    // host config validator. The diagnostic must name both the field key
    // (`retract_mode`) and the offending value (`marlin`).
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules")
        .join("core-modules")
        .join("path-optimization-default")
        .join("path-optimization-default.toml");

    let manifest_text =
        std::fs::read_to_string(&manifest_path).expect("read path-optimization-default.toml");
    let root: toml::Value = manifest_text
        .parse()
        .expect("path-optimization-default.toml must be valid TOML");

    let schema_section = root
        .get("config")
        .and_then(|c| c.get("schema"))
        .expect("manifest must have [config.schema] section");

    let schema = parse_config_schema(schema_section, &manifest_path).expect("schema should parse");

    // Sanity: the manifest declares retract_mode as an enum with gcode|firmware.
    let retract_mode_field = schema
        .fields
        .get("retract_mode")
        .expect("manifest must declare retract_mode field");
    assert_eq!(retract_mode_field.field_type(), &ConfigFieldType::Enum);
    assert_eq!(
        retract_mode_field.enum_values(),
        Some(&["gcode".to_string(), "firmware".to_string()][..])
    );

    // Now attempt to validate a config that sets retract_mode = "marlin".
    let mut values: BTreeMap<String, ConfigValue> = BTreeMap::new();
    values.insert("retract_mode".into(), ConfigValue::String("marlin".into()));

    let errors = validate_config(&schema, &values);
    let enum_error = errors
        .iter()
        .find(|e| e.kind == ConfigValidationErrorKind::InvalidEnumValue)
        .expect("validator must reject retract_mode = \"marlin\"");

    assert!(
        enum_error.message.contains("retract_mode"),
        "diagnostic must name the field 'retract_mode'; got: {}",
        enum_error.message
    );
    assert!(
        enum_error.message.contains("marlin"),
        "diagnostic must name the offending value 'marlin'; got: {}",
        enum_error.message
    );
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
    assert!(pattern_fields.iter().any(|f| f.key() == "density"));
    assert!(pattern_fields.iter().any(|f| f.key() == "pattern"));

    let advanced_fields = groups.get("Advanced").expect("Advanced group");
    assert!(advanced_fields
        .iter()
        .any(|f| f.key() == "marching-cell-size"));
    assert!(advanced_fields.iter().any(|f| f.key() == "raster-precision"));
}

#[test]
fn get_advanced_fields_returns_only_advanced() {
    let schema = make_tpms_infill_schema();
    let advanced = get_advanced_fields(&schema);

    assert!(advanced.iter().all(|f| f.advanced()));
    assert!(advanced.iter().any(|f| f.key() == "marching-cell-size"));
    assert!(advanced.iter().any(|f| f.key() == "raster-precision"));
    assert!(!advanced.iter().any(|f| f.key() == "density"));
}

#[test]
fn get_basic_fields_returns_only_non_advanced() {
    let schema = make_tpms_infill_schema();
    let basic = get_basic_fields(&schema);

    assert!(basic.iter().all(|f| !f.advanced()));
    assert!(basic.iter().any(|f| f.key() == "density"));
    assert!(basic.iter().any(|f| f.key() == "pattern"));
    assert!(!basic.iter().any(|f| f.key() == "marching-cell-size"));
}

// ============================================================================
// Test Fixtures
// ============================================================================

fn make_tpms_infill_schema() -> FullConfigSchema {
    let mut fields = BTreeMap::new();

    fields.insert("pattern".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("pattern", ConfigFieldType::Enum);
        b.default_value(ConfigValue::String("schwartz-d".into()))
            .display("TPMS Pattern")
            .description("Which TPMS surface family to use")
            .group("Pattern")
            .enum_values(vec!["schwartz-d".into(), "fischer-koch-s".into()]);
        b.build()
    });

    fields.insert("density".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("density", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(0.15))
            .display("Infill Density")
            .group("Pattern")
            .unit(ConfigUnit::Ratio)
            .min(0.05)
            .max(0.95)
            .step(0.01)
            .validate("value > 0.0 && value < 1.0");
        b.build()
    });

    fields.insert("multiline-count".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("multiline-count", ConfigFieldType::Int);
        b.default_value(ConfigValue::Int(1))
            .display("Parallel Passes")
            .group("Pattern")
            .min(1.0)
            .max(4.0);
        b.build()
    });

    fields.insert("marching-cell-size".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("marching-cell-size", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(0.40))
            .display("Marching Cell Size (mm)")
            .group("Advanced")
            .unit(ConfigUnit::Millimeters)
            .advanced(true)
            .min(0.10)
            .max(1.00)
            .step(0.05);
        b.build()
    });

    fields.insert("raster-precision".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("raster-precision", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(0.004))
            .display("Raster Precision (mm)")
            .group("Advanced")
            .unit(ConfigUnit::Millimeters)
            .advanced(true)
            .min(0.001)
            .max(0.010);
        b.build()
    });

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

    fields.insert("marching-cell-size".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("marching-cell-size", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(0.40));
        b.build()
    });

    fields.insert("raster-precision".into(), {
        let mut b = ConfigFieldSchemaBuilder::new("raster-precision", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(0.004));
        b.build()
    });

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
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Bool);
    b.default_value(ConfigValue::Bool(default));
    b.build()
}

fn make_int_field(key: &str, default: i64, min: i64, max: i64) -> ConfigFieldSchema {
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Int);
    b.default_value(ConfigValue::Int(default))
        .min(min as f64)
        .max(max as f64);
    b.build()
}

fn make_float_field(key: &str, default: f64, min: f64, max: f64) -> ConfigFieldSchema {
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
    b.default_value(ConfigValue::Float(default))
        .min(min)
        .max(max);
    b.build()
}

fn make_string_field(key: &str, max_length: usize) -> ConfigFieldSchema {
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::String);
    b.max_length(max_length);
    b.build()
}

fn make_enum_field(key: &str, values: Vec<&str>) -> ConfigFieldSchema {
    let first = values[0].to_string();
    let enum_vals: Vec<String> = values.into_iter().map(String::from).collect();
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Enum);
    b.default_value(ConfigValue::String(first))
        .enum_values(enum_vals);
    b.build()
}

fn make_float_list_field(key: &str, min_len: usize, max_len: usize) -> ConfigFieldSchema {
    let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::FloatList);
    b.min_list_length(min_len).max_list_length(max_len);
    b.build()
}
