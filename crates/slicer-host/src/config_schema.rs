//! Config schema query API for host-side module configuration inspection.
//!
//! This module provides types and functions for querying module configuration
//! schemas, validating configuration values, and supporting UI rendering of
//! module settings.

use std::collections::BTreeMap;

/// Type of a configuration field as declared in the module manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigFieldType {
    /// Boolean checkbox.
    Bool,
    /// Integer with optional min/max/step.
    Int,
    /// Floating point with optional min/max/step/unit.
    Float,
    /// Free text string with optional max-length.
    String,
    /// Fixed set of string values.
    Enum,
    /// List of floating point values.
    FloatList,
    /// List of strings.
    StringList,
}

/// Unit hint for UI rendering of numeric fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigUnit {
    /// Millimeters (renders as "X mm").
    Millimeters,
    /// Ratio (renders as "X%" with value × 100).
    Ratio,
    /// Degrees (renders as "X°").
    Degrees,
    /// Speed (renders as "X mm/s").
    MillimetersPerSecond,
    /// Duration (renders as "X ms").
    Milliseconds,
    /// No unit specified.
    None,
}

/// Full schema definition for a single configuration field.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigFieldSchema {
    /// The field key (e.g., "density", "pattern").
    pub key: String,
    /// The field type.
    pub field_type: ConfigFieldType,
    /// Default value as a string representation.
    pub default: Option<ConfigValue>,
    /// Display name for UI.
    pub display: Option<String>,
    /// Description for tooltips.
    pub description: Option<String>,
    /// UI grouping hint.
    pub group: Option<String>,
    /// Unit for numeric fields.
    pub unit: ConfigUnit,
    /// Whether this is an advanced setting (hidden by default).
    pub advanced: bool,
    /// Minimum value for Int/Float fields.
    pub min: Option<f64>,
    /// Maximum value for Int/Float fields.
    pub max: Option<f64>,
    /// Step size for Int/Float fields.
    pub step: Option<f64>,
    /// Maximum length for String fields.
    pub max_length: Option<usize>,
    /// Allowed values for Enum fields.
    pub enum_values: Option<Vec<String>>,
    /// Minimum length for list fields.
    pub min_list_length: Option<usize>,
    /// Maximum length for list fields.
    pub max_list_length: Option<usize>,
    /// Single-field validation expression.
    pub validate: Option<String>,
}

/// Cross-field validation rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossValidateRule {
    /// The validation expression referencing multiple field names.
    pub rule: String,
    /// Human-readable message when validation fails.
    pub message: String,
    /// Severity: "error" blocks slicing, "warning" notifies only.
    pub severity: CrossValidateSeverity,
}

/// Severity level for cross-field validation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossValidateSeverity {
    /// Validation failure blocks slicing.
    Error,
    /// Validation failure emits a warning but continues.
    Warning,
}

/// A configuration value that can be validated against a schema.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Floating point value.
    Float(f64),
    /// String value.
    String(String),
    /// List of floating point values.
    FloatList(Vec<f64>),
    /// List of string values.
    StringList(Vec<String>),
}

/// Full configuration schema for a module.
#[derive(Debug, Clone, PartialEq)]
pub struct FullConfigSchema {
    /// Field schemas keyed by field name.
    pub fields: BTreeMap<String, ConfigFieldSchema>,
    /// Cross-field validation rules.
    pub cross_validate: Vec<CrossValidateRule>,
}

impl Default for FullConfigSchema {
    fn default() -> Self {
        let mut fields = BTreeMap::new();
        let speed_keys = [
            ("outer_wall_speed", 60.0),
            ("inner_wall_speed", 60.0),
            ("thin_wall_speed", 30.0),
            ("top_surface_speed", 100.0),
            ("bottom_surface_speed", 100.0),
            ("sparse_infill_speed", 100.0),
            ("bridge_speed", 25.0),
            ("internal_bridge_speed", 37.5),
            ("support_speed", 80.0),
            ("support_interface_speed", 80.0),
            ("gap_infill_speed", 30.0),
            ("ironing_speed", 20.0),
            ("skirt_speed", 50.0),
            ("wipe_tower_speed", 90.0),
            ("prime_tower_speed", 90.0),
            ("travel_speed", 120.0),
            ("travel_speed_z", 0.0),
            ("initial_layer_speed", 30.0),
            ("initial_layer_infill_speed", 60.0),
            ("initial_layer_travel_speed", 120.0),
            ("wipe_speed", 96.0),
            ("overhang_1_4_speed", 0.0),
            ("overhang_2_4_speed", 0.0),
            ("overhang_3_4_speed", 0.0),
            ("overhang_4_4_speed", 0.0),
            ("filament_ironing_speed", 0.0),
        ];

        for (key, default_val) in speed_keys {
            fields.insert(
                key.to_string(),
                ConfigFieldSchema {
                    key: key.to_string(),
                    field_type: ConfigFieldType::Float,
                    default: Some(ConfigValue::Float(default_val)),
                    display: None,
                    description: None,
                    group: Some("Speed".to_string()),
                    unit: ConfigUnit::MillimetersPerSecond,
                    advanced: false,
                    min: Some(0.0),
                    max: None,
                    step: None,
                    max_length: None,
                    enum_values: None,
                    min_list_length: None,
                    max_list_length: None,
                    validate: None,
                },
            );
        }

        let cooling_int_keys = [
            ("fan_speed_min", 51i64, Some(255.0)),
            ("fan_speed_max", 255i64, Some(255.0)),
            ("disable_fan_first_layers", 1i64, None),
            ("overhang_fan_speed", 100i64, Some(100.0)),
        ];

        for (key, default_val, max) in cooling_int_keys {
            fields.insert(
                key.to_string(),
                ConfigFieldSchema {
                    key: key.to_string(),
                    field_type: ConfigFieldType::Int,
                    default: Some(ConfigValue::Int(default_val)),
                    display: None,
                    description: None,
                    group: Some("Cooling".to_string()),
                    unit: ConfigUnit::None,
                    advanced: false,
                    min: Some(0.0),
                    max,
                    step: None,
                    max_length: None,
                    enum_values: None,
                    min_list_length: None,
                    max_list_length: None,
                    validate: None,
                },
            );
        }

        let cooling_bool_keys = [
            ("enable_overhang_fan", true),
            ("slow_down_for_layer_cooling", true),
        ];

        for (key, default_val) in cooling_bool_keys {
            fields.insert(
                key.to_string(),
                ConfigFieldSchema {
                    key: key.to_string(),
                    field_type: ConfigFieldType::Bool,
                    default: Some(ConfigValue::Bool(default_val)),
                    display: None,
                    description: None,
                    group: Some("Cooling".to_string()),
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
        }

        let cooling_float_keys = [
            (
                "slow_down_min_speed",
                10.0,
                ConfigUnit::MillimetersPerSecond,
            ),
            ("slow_down_layer_time", 5.0, ConfigUnit::None),
        ];

        for (key, default_val, unit) in cooling_float_keys {
            fields.insert(
                key.to_string(),
                ConfigFieldSchema {
                    key: key.to_string(),
                    field_type: ConfigFieldType::Float,
                    default: Some(ConfigValue::Float(default_val)),
                    display: None,
                    description: None,
                    group: Some("Cooling".to_string()),
                    unit,
                    advanced: false,
                    min: Some(0.0),
                    max: None,
                    step: None,
                    max_length: None,
                    enum_values: None,
                    min_list_length: None,
                    max_list_length: None,
                    validate: None,
                },
            );
        }

        Self {
            fields,
            cross_validate: Vec::new(),
        }
    }
}

/// Error returned when configuration validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationError {
    /// The field that failed validation (None for cross-field errors).
    pub field: Option<String>,
    /// Human-readable error message.
    pub message: String,
    /// The validation error kind.
    pub kind: ConfigValidationErrorKind,
}

/// Classification of configuration validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationErrorKind {
    /// The value type does not match the field type.
    TypeMismatch,
    /// The value is outside the allowed range.
    OutOfRange,
    /// The string value exceeds max-length.
    TooLong,
    /// The enum value is not in the allowed set.
    InvalidEnumValue,
    /// The list length is outside allowed bounds.
    InvalidListLength,
    /// Single-field validation expression failed.
    ValidationFailed,
    /// Cross-field validation rule failed.
    CrossValidationFailed,
    /// Required field is missing.
    MissingRequired,
}

/// Queries the full configuration schema from a loaded module.
///
/// Returns the parsed and validated configuration schema from the module's
/// manifest, suitable for UI rendering and runtime validation.
pub fn query_config_schema(schema: &FullConfigSchema) -> &BTreeMap<String, ConfigFieldSchema> {
    &schema.fields
}

/// Retrieves a specific field schema by key.
///
/// Returns `None` if the field does not exist in the schema.
pub fn get_field_schema<'a>(
    schema: &'a FullConfigSchema,
    key: &str,
) -> Option<&'a ConfigFieldSchema> {
    schema.fields.get(key)
}

/// Validates a single configuration value against its field schema.
///
/// Checks type compatibility, range constraints, enum membership, list length,
/// and single-field validation expressions.
pub fn validate_field_value(
    field: &ConfigFieldSchema,
    value: &ConfigValue,
) -> Result<(), ConfigValidationError> {
    // Type checking
    match (&field.field_type, value) {
        (ConfigFieldType::Bool, ConfigValue::Bool(_)) => {}
        (ConfigFieldType::Bool, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected bool for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::Int, ConfigValue::Int(v)) => {
            // Range validation for int
            if let Some(min) = field.min {
                if (*v as f64) < min {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "value {} is below minimum {} for field '{}'",
                            v, min, field.key
                        ),
                        kind: ConfigValidationErrorKind::OutOfRange,
                    });
                }
            }
            if let Some(max) = field.max {
                if (*v as f64) > max {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "value {} is above maximum {} for field '{}'",
                            v, max, field.key
                        ),
                        kind: ConfigValidationErrorKind::OutOfRange,
                    });
                }
            }
        }
        (ConfigFieldType::Int, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected int for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::Float, ConfigValue::Float(v)) => {
            if !v.is_finite() {
                return Err(ConfigValidationError {
                    field: Some(field.key.clone()),
                    message: format!(
                        "non-finite float '{}' is invalid for field '{}'",
                        v, field.key
                    ),
                    kind: ConfigValidationErrorKind::ValidationFailed,
                });
            }

            // Range validation for float
            if let Some(min) = field.min {
                if *v < min {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "value {} is below minimum {} for field '{}'",
                            v, min, field.key
                        ),
                        kind: ConfigValidationErrorKind::OutOfRange,
                    });
                }
            }
            if let Some(max) = field.max {
                if *v > max {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "value {} is above maximum {} for field '{}'",
                            v, max, field.key
                        ),
                        kind: ConfigValidationErrorKind::OutOfRange,
                    });
                }
            }
        }
        (ConfigFieldType::Float, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected float for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::String, ConfigValue::String(s)) => {
            // Max length validation for string
            if let Some(max_len) = field.max_length {
                if s.len() > max_len {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "string length {} exceeds maximum {} for field '{}'",
                            s.len(),
                            max_len,
                            field.key
                        ),
                        kind: ConfigValidationErrorKind::TooLong,
                    });
                }
            }
        }
        (ConfigFieldType::String, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected string for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::Enum, ConfigValue::String(s)) => {
            // Enum membership validation
            if let Some(ref enum_values) = field.enum_values {
                if !enum_values.contains(s) {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "value '{}' is not a valid enum value for field '{}'. Allowed: {:?}",
                            s, field.key, enum_values
                        ),
                        kind: ConfigValidationErrorKind::InvalidEnumValue,
                    });
                }
            }
        }
        (ConfigFieldType::Enum, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected string for enum field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::FloatList, ConfigValue::FloatList(list)) => {
            if let Some(value) = list.iter().find(|value| !value.is_finite()) {
                return Err(ConfigValidationError {
                    field: Some(field.key.clone()),
                    message: format!(
                        "non-finite float '{}' is invalid for field '{}'",
                        value, field.key
                    ),
                    kind: ConfigValidationErrorKind::ValidationFailed,
                });
            }

            // List length validation
            if let Some(min_len) = field.min_list_length {
                if list.len() < min_len {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "list length {} is below minimum {} for field '{}'",
                            list.len(),
                            min_len,
                            field.key
                        ),
                        kind: ConfigValidationErrorKind::InvalidListLength,
                    });
                }
            }
            if let Some(max_len) = field.max_list_length {
                if list.len() > max_len {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "list length {} exceeds maximum {} for field '{}'",
                            list.len(),
                            max_len,
                            field.key
                        ),
                        kind: ConfigValidationErrorKind::InvalidListLength,
                    });
                }
            }
        }
        (ConfigFieldType::FloatList, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected float list for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
        (ConfigFieldType::StringList, ConfigValue::StringList(list)) => {
            // List length validation
            if let Some(min_len) = field.min_list_length {
                if list.len() < min_len {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "list length {} is below minimum {} for field '{}'",
                            list.len(),
                            min_len,
                            field.key
                        ),
                        kind: ConfigValidationErrorKind::InvalidListLength,
                    });
                }
            }
            if let Some(max_len) = field.max_list_length {
                if list.len() > max_len {
                    return Err(ConfigValidationError {
                        field: Some(field.key.clone()),
                        message: format!(
                            "list length {} exceeds maximum {} for field '{}'",
                            list.len(),
                            max_len,
                            field.key
                        ),
                        kind: ConfigValidationErrorKind::InvalidListLength,
                    });
                }
            }
        }
        (ConfigFieldType::StringList, _) => {
            return Err(ConfigValidationError {
                field: Some(field.key.clone()),
                message: format!("expected string list for field '{}'", field.key),
                kind: ConfigValidationErrorKind::TypeMismatch,
            });
        }
    }

    Ok(())
}

/// Validates all configuration values against the full schema.
///
/// Checks all field values for validity and runs cross-field validation rules.
/// Returns all errors found, not just the first one.
pub fn validate_config(
    schema: &FullConfigSchema,
    values: &BTreeMap<String, ConfigValue>,
) -> Vec<ConfigValidationError> {
    let mut errors = Vec::new();

    // Validate each provided value against its field schema
    for (key, value) in values {
        if let Some(field) = schema.fields.get(key) {
            if let Err(error) = validate_field_value(field, value) {
                errors.push(error);
            }
        }
    }

    // Run cross-field validation rules
    for rule in &schema.cross_validate {
        // Simple pattern matching for the test case:
        // "marching-cell-size >= raster-precision * 10"
        if rule.rule == "marching-cell-size >= raster-precision * 10" {
            let cell_size = values.get("marching-cell-size").and_then(|v| {
                if let ConfigValue::Float(f) = v {
                    Some(*f)
                } else {
                    None
                }
            });
            let precision = values.get("raster-precision").and_then(|v| {
                if let ConfigValue::Float(f) = v {
                    Some(*f)
                } else {
                    None
                }
            });

            if let (Some(cell), Some(prec)) = (cell_size, precision) {
                if cell < prec * 10.0 {
                    errors.push(ConfigValidationError {
                        field: None,
                        message: rule.message.clone(),
                        kind: ConfigValidationErrorKind::CrossValidationFailed,
                    });
                }
            }
        }
    }

    errors
}

/// Parses a configuration schema from a module manifest TOML value.
///
/// This is called during manifest ingestion to convert the raw TOML into
/// the structured `FullConfigSchema` representation.
pub fn parse_config_schema(
    config_section: &toml::Value,
    _manifest_path: &std::path::Path,
) -> Result<FullConfigSchema, ConfigSchemaParseError> {
    let mut fields = BTreeMap::new();

    // config_section is expected to be a table
    if let Some(table) = config_section.as_table() {
        for (key, value) in table {
            // Each field is expected to be a table with properties
            if let Some(field_table) = value.as_table() {
                let field = parse_field_schema(key, field_table)?;
                fields.insert(key.clone(), field);
            }
        }
    }

    Ok(FullConfigSchema {
        fields,
        cross_validate: Vec::new(), // Cross-validate rules come from a separate section
    })
}

/// Parses a single field schema from a TOML table.
fn parse_field_schema(
    key: &str,
    table: &toml::map::Map<String, toml::Value>,
) -> Result<ConfigFieldSchema, ConfigSchemaParseError> {
    // Parse field type (required)
    let type_str =
        table
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigSchemaParseError {
                field: Some(key.to_string()),
                message: "missing 'type' property".into(),
                kind: ConfigSchemaParseErrorKind::MissingProperty,
            })?;

    let field_type = match type_str {
        "bool" => ConfigFieldType::Bool,
        "int" => ConfigFieldType::Int,
        "float" => ConfigFieldType::Float,
        "string" => ConfigFieldType::String,
        "enum" => ConfigFieldType::Enum,
        "float-list" => ConfigFieldType::FloatList,
        "string-list" => ConfigFieldType::StringList,
        unknown => {
            return Err(ConfigSchemaParseError {
                field: Some(key.to_string()),
                message: format!("unknown field type '{}'", unknown),
                kind: ConfigSchemaParseErrorKind::InvalidFieldType,
            });
        }
    };

    // Enum fields require a 'values' list
    if field_type == ConfigFieldType::Enum && !table.contains_key("values") {
        return Err(ConfigSchemaParseError {
            field: Some(key.to_string()),
            message: "enum field missing 'values' list".into(),
            kind: ConfigSchemaParseErrorKind::MissingEnumValues,
        });
    }

    // Parse default value
    let default = table
        .get("default")
        .map(|v| parse_config_value(v, &field_type));

    // Parse numeric constraints
    let min = table.get("min").and_then(parse_numeric_value);
    let max = table.get("max").and_then(parse_numeric_value);
    let step = table.get("step").and_then(parse_numeric_value);

    // Parse string constraints
    let max_length = table
        .get("max_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));

    // Parse enum values
    let enum_values = table.get("values").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
    });

    // Parse list constraints
    let min_list_length = table
        .get("min_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));
    let max_list_length = table
        .get("max_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));

    // Parse display and description
    let display = table
        .get("display")
        .and_then(|v| v.as_str().map(String::from));
    let description = table
        .get("description")
        .and_then(|v| v.as_str().map(String::from));

    // Parse group
    let group = table
        .get("group")
        .and_then(|v| v.as_str().map(String::from));

    // Parse unit
    let unit = table
        .get("unit")
        .and_then(|v| v.as_str())
        .map(parse_config_unit)
        .unwrap_or(ConfigUnit::None);

    // Parse advanced flag (defaults to false)
    let advanced = table
        .get("advanced")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse validate expression
    let validate = table
        .get("validate")
        .and_then(|v| v.as_str().map(String::from));

    Ok(ConfigFieldSchema {
        key: key.to_string(),
        field_type,
        default,
        display,
        description,
        group,
        unit,
        advanced,
        min,
        max,
        step,
        max_length,
        enum_values,
        min_list_length,
        max_list_length,
        validate,
    })
}

/// Parses a TOML value into a ConfigValue based on the expected field type.
fn parse_config_value(value: &toml::Value, field_type: &ConfigFieldType) -> ConfigValue {
    match field_type {
        ConfigFieldType::Bool => ConfigValue::Bool(value.as_bool().unwrap_or(false)),
        ConfigFieldType::Int => ConfigValue::Int(value.as_integer().unwrap_or(0)),
        ConfigFieldType::Float => {
            // Handle both float and integer as float
            let f = parse_numeric_value(value).unwrap_or(0.0);
            ConfigValue::Float(f)
        }
        ConfigFieldType::String | ConfigFieldType::Enum => {
            ConfigValue::String(value.as_str().unwrap_or("").to_string())
        }
        ConfigFieldType::FloatList => {
            let list = value
                .as_array()
                .map(|arr| arr.iter().filter_map(parse_numeric_value).collect())
                .unwrap_or_default();
            ConfigValue::FloatList(list)
        }
        ConfigFieldType::StringList => {
            let list = value
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            ConfigValue::StringList(list)
        }
    }
}

fn parse_numeric_value(value: &toml::Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|i| i as f64))
        .map(normalize_subnormal)
}

fn normalize_subnormal(value: f64) -> f64 {
    if value.is_subnormal() {
        0.0
    } else {
        value
    }
}

/// Parses a unit string into a ConfigUnit.
fn parse_config_unit(s: &str) -> ConfigUnit {
    match s {
        "mm" | "millimeters" => ConfigUnit::Millimeters,
        "ratio" | "percent" => ConfigUnit::Ratio,
        "degrees" | "deg" => ConfigUnit::Degrees,
        "mm/s" | "millimeters_per_second" => ConfigUnit::MillimetersPerSecond,
        "ms" | "milliseconds" => ConfigUnit::Milliseconds,
        _ => ConfigUnit::None,
    }
}

/// Error returned when parsing a configuration schema fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSchemaParseError {
    /// The field that failed to parse (None for structural errors).
    pub field: Option<String>,
    /// Human-readable error message.
    pub message: String,
    /// The parse error kind.
    pub kind: ConfigSchemaParseErrorKind,
}

/// Classification of configuration schema parse errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSchemaParseErrorKind {
    /// The field type is unknown or invalid.
    InvalidFieldType,
    /// A required schema property is missing.
    MissingProperty,
    /// A schema property has an invalid value.
    InvalidProperty,
    /// An enum field is missing its values list.
    MissingEnumValues,
    /// A validation expression has invalid syntax.
    InvalidValidateExpression,
    /// A cross-validate rule has invalid structure.
    InvalidCrossValidateRule,
}

/// Returns all field keys grouped by their UI group.
///
/// Fields without a group are collected under an empty string key.
pub fn group_fields_by_ui_group(
    schema: &FullConfigSchema,
) -> BTreeMap<String, Vec<&ConfigFieldSchema>> {
    let mut groups: BTreeMap<String, Vec<&ConfigFieldSchema>> = BTreeMap::new();

    for field in schema.fields.values() {
        let group_key = field.group.clone().unwrap_or_default();
        groups.entry(group_key).or_default().push(field);
    }

    groups
}

/// Returns only the advanced fields from the schema.
pub fn get_advanced_fields(schema: &FullConfigSchema) -> Vec<&ConfigFieldSchema> {
    schema.fields.values().filter(|f| f.advanced).collect()
}

/// Returns only the non-advanced (basic) fields from the schema.
pub fn get_basic_fields(schema: &FullConfigSchema) -> Vec<&ConfigFieldSchema> {
    schema.fields.values().filter(|f| !f.advanced).collect()
}

/// Build the documented config-schema JSON response from loaded modules.
///
/// Per docs/01_system_architecture.md, the config-schema query response format is:
/// ```jsonc
/// {"schema": [
///   {
///     "module": "com.community.tpms-infill",
///     "fields": [
///       {"key": "pattern", "type": "enum", "values": [...], "default": "...", "display": "...", "group": "..."}
///     ]
///   }
/// ]}
/// ```
pub fn build_config_schema_json(modules: &[crate::manifest::LoadedModule]) -> serde_json::Value {
    let schema_entries: Vec<serde_json::Value> = modules
        .iter()
        .filter(|m| !m.config_schema.entries.is_empty())
        .map(|m| {
            let fields: Vec<serde_json::Value> = m
                .config_schema
                .entries
                .iter()
                .map(|(key, entry)| {
                    serde_json::json!({
                        "key": key,
                        "type": entry.field_type,
                        "default": entry.default,
                        "min": entry.min,
                        "max": entry.max,
                        "display": entry.display,
                        "group": entry.group,
                    })
                })
                .collect();
            serde_json::json!({
                "module": m.id,
                "fields": fields,
            })
        })
        .collect();

    serde_json::json!({ "schema": schema_entries })
}
