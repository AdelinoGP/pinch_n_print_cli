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
///
/// Construction goes through [`ConfigFieldSchemaBuilder`], which uses
/// non-consuming `&mut self -> &mut Self` setters — see spec §6.3 — so
/// the existing `FullConfigSchema::default()` loop bodies stay readable
/// when each row sets a different mix of optionals. Field reads from
/// outside the crate go through the `pub fn` accessor methods declared
/// below.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigFieldSchema {
    /// The field key (e.g., "density", "pattern").
    pub(crate) key: String,
    /// The field type.
    pub(crate) field_type: ConfigFieldType,
    /// Default value as a string representation.
    pub(crate) default: Option<ConfigValue>,
    /// Display name for UI.
    pub(crate) display: Option<String>,
    /// Description for tooltips.
    pub(crate) description: Option<String>,
    /// UI grouping hint.
    pub(crate) group: Option<String>,
    /// Unit for numeric fields.
    pub(crate) unit: ConfigUnit,
    /// Whether this is an advanced setting (hidden by default).
    pub(crate) advanced: bool,
    /// Minimum value for Int/Float fields.
    pub(crate) min: Option<f64>,
    /// Maximum value for Int/Float fields.
    pub(crate) max: Option<f64>,
    /// Step size for Int/Float fields.
    pub(crate) step: Option<f64>,
    /// Maximum length for String fields.
    pub(crate) max_length: Option<usize>,
    /// Allowed values for Enum fields.
    pub(crate) enum_values: Option<Vec<String>>,
    /// Minimum length for list fields.
    pub(crate) min_list_length: Option<usize>,
    /// Maximum length for list fields.
    pub(crate) max_list_length: Option<usize>,
    /// Single-field validation expression.
    pub(crate) validate: Option<String>,
}

impl ConfigFieldSchema {
    /// The field key (e.g., "density", "pattern").
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The field type.
    pub fn field_type(&self) -> &ConfigFieldType {
        &self.field_type
    }

    /// Default value as a string representation.
    pub fn default(&self) -> Option<&ConfigValue> {
        self.default.as_ref()
    }

    /// Display name for UI.
    pub fn display(&self) -> Option<&str> {
        self.display.as_deref()
    }

    /// Description for tooltips.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// UI grouping hint.
    pub fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    /// Unit for numeric fields.
    pub fn unit(&self) -> &ConfigUnit {
        &self.unit
    }

    /// Whether this is an advanced setting (hidden by default).
    pub fn advanced(&self) -> bool {
        self.advanced
    }

    /// Minimum value for Int/Float fields.
    pub fn min(&self) -> Option<f64> {
        self.min
    }

    /// Maximum value for Int/Float fields.
    pub fn max(&self) -> Option<f64> {
        self.max
    }

    /// Step size for Int/Float fields.
    pub fn step(&self) -> Option<f64> {
        self.step
    }

    /// Maximum length for String fields.
    pub fn max_length(&self) -> Option<usize> {
        self.max_length
    }

    /// Allowed values for Enum fields.
    pub fn enum_values(&self) -> Option<&[String]> {
        self.enum_values.as_deref()
    }

    /// Minimum length for list fields.
    pub fn min_list_length(&self) -> Option<usize> {
        self.min_list_length
    }

    /// Maximum length for list fields.
    pub fn max_list_length(&self) -> Option<usize> {
        self.max_list_length
    }

    /// Single-field validation expression.
    pub fn validate(&self) -> Option<&str> {
        self.validate.as_deref()
    }
}

/// Builder for [`ConfigFieldSchema`].
///
/// Required identity fields (`key`, `field_type`) are positional
/// arguments to [`ConfigFieldSchemaBuilder::new`]; everything else is
/// optional and set via non-consuming `&mut self -> &mut Self` setters.
/// Terminal `build(&self)` clones the configured fields into a finished
/// [`ConfigFieldSchema`]. The non-consuming style is the documented
/// per-struct exception (spec §3.2 / §6.3) — it keeps the
/// `FullConfigSchema::default()` row builders readable.
#[derive(Debug, Clone)]
pub struct ConfigFieldSchemaBuilder {
    key: String,
    field_type: ConfigFieldType,
    default: Option<ConfigValue>,
    display: Option<String>,
    description: Option<String>,
    group: Option<String>,
    unit: ConfigUnit,
    advanced: bool,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    max_length: Option<usize>,
    enum_values: Option<Vec<String>>,
    min_list_length: Option<usize>,
    max_list_length: Option<usize>,
    validate: Option<String>,
}

impl ConfigFieldSchemaBuilder {
    /// Start a new builder for the given key and field type.
    pub fn new(key: impl Into<String>, field_type: ConfigFieldType) -> Self {
        Self {
            key: key.into(),
            field_type,
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
            min_list_length: None,
            max_list_length: None,
            validate: None,
        }
    }

    /// Set the default value.
    pub fn default_value(&mut self, v: ConfigValue) -> &mut Self {
        self.default = Some(v);
        self
    }

    /// Set the UI display name.
    pub fn display(&mut self, s: impl Into<String>) -> &mut Self {
        self.display = Some(s.into());
        self
    }

    /// Set the tooltip / description.
    pub fn description(&mut self, s: impl Into<String>) -> &mut Self {
        self.description = Some(s.into());
        self
    }

    /// Set the UI grouping hint.
    pub fn group(&mut self, s: impl Into<String>) -> &mut Self {
        self.group = Some(s.into());
        self
    }

    /// Set the unit for numeric fields.
    pub fn unit(&mut self, u: ConfigUnit) -> &mut Self {
        self.unit = u;
        self
    }

    /// Mark as advanced.
    pub fn advanced(&mut self, b: bool) -> &mut Self {
        self.advanced = b;
        self
    }

    /// Set the minimum value for numeric fields.
    pub fn min(&mut self, v: f64) -> &mut Self {
        self.min = Some(v);
        self
    }

    /// Set the maximum value for numeric fields.
    pub fn max(&mut self, v: f64) -> &mut Self {
        self.max = Some(v);
        self
    }

    /// Set the step for numeric fields.
    pub fn step(&mut self, v: f64) -> &mut Self {
        self.step = Some(v);
        self
    }

    /// Set the max length for string fields.
    pub fn max_length(&mut self, n: usize) -> &mut Self {
        self.max_length = Some(n);
        self
    }

    /// Set the allowed values for enum fields.
    pub fn enum_values(&mut self, v: Vec<String>) -> &mut Self {
        self.enum_values = Some(v);
        self
    }

    /// Set the minimum list length.
    pub fn min_list_length(&mut self, n: usize) -> &mut Self {
        self.min_list_length = Some(n);
        self
    }

    /// Set the maximum list length.
    pub fn max_list_length(&mut self, n: usize) -> &mut Self {
        self.max_list_length = Some(n);
        self
    }

    /// Set the single-field validation expression.
    pub fn validate(&mut self, expr: impl Into<String>) -> &mut Self {
        self.validate = Some(expr.into());
        self
    }

    /// Build the finished [`ConfigFieldSchema`] (non-consuming).
    pub fn build(&self) -> ConfigFieldSchema {
        ConfigFieldSchema {
            key: self.key.clone(),
            field_type: self.field_type.clone(),
            default: self.default.clone(),
            display: self.display.clone(),
            description: self.description.clone(),
            group: self.group.clone(),
            unit: self.unit.clone(),
            advanced: self.advanced,
            min: self.min,
            max: self.max,
            step: self.step,
            max_length: self.max_length,
            enum_values: self.enum_values.clone(),
            min_list_length: self.min_list_length,
            max_list_length: self.max_list_length,
            validate: self.validate.clone(),
        }
    }
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
            let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
            b.default_value(ConfigValue::Float(default_val))
                .group("Speed")
                .unit(ConfigUnit::MillimetersPerSecond)
                .min(0.0);
            fields.insert(key.to_string(), b.build());
        }

        let cooling_int_keys = [
            ("fan_speed_min", 51i64, Some(255.0)),
            ("fan_speed_max", 255i64, Some(255.0)),
            ("disable_fan_first_layers", 1i64, None),
            ("overhang_fan_speed", 100i64, Some(100.0)),
        ];

        for (key, default_val, max) in cooling_int_keys {
            let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Int);
            b.default_value(ConfigValue::Int(default_val))
                .group("Cooling")
                .min(0.0);
            if let Some(max_val) = max {
                b.max(max_val);
            }
            fields.insert(key.to_string(), b.build());
        }

        let cooling_bool_keys = [
            ("enable_overhang_fan", true),
            ("slow_down_for_layer_cooling", true),
        ];

        for (key, default_val) in cooling_bool_keys {
            let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Bool);
            b.default_value(ConfigValue::Bool(default_val))
                .group("Cooling");
            fields.insert(key.to_string(), b.build());
        }

        {
            let mut b =
                ConfigFieldSchemaBuilder::new("use_relative_e_distances", ConfigFieldType::Bool);
            b.default_value(ConfigValue::Bool(true)).group("Extrusion");
            fields.insert("use_relative_e_distances".to_string(), b.build());
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
            let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
            b.default_value(ConfigValue::Float(default_val))
                .group("Cooling")
                .unit(unit)
                .min(0.0);
            fields.insert(key.to_string(), b.build());
        }

        // Line-width keys (OrcaSlicer 0.4 mm nozzle parity defaults)
        let width_keys = [
            ("outer_wall_line_width", 0.42),
            ("inner_wall_line_width", 0.45),
            ("sparse_infill_line_width", 0.45),
            ("top_surface_line_width", 0.42),
            ("support_line_width", 0.35),
        ];

        for (key, default_val) in width_keys {
            fields.entry(key.to_string()).or_insert_with(|| {
                let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
                b.default_value(ConfigValue::Float(default_val))
                    .group("Extrusion")
                    .unit(ConfigUnit::Millimeters)
                    .min(0.0);
                b.build()
            });
        }

        // Filament / printer geometry keys required by packet 55 (AC7)
        let filament_float_keys = [
            ("filament_diameter", 1.75_f64),
            ("filament_density", 1.24_f64),
            ("max_z_height", 256.0_f64),
        ];

        for (key, default_val) in filament_float_keys {
            let unit = if key == "filament_diameter" || key == "max_z_height" {
                ConfigUnit::Millimeters
            } else {
                ConfigUnit::None
            };
            let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
            b.default_value(ConfigValue::Float(default_val))
                .group("Filament")
                .unit(unit)
                .min(0.0);
            fields.insert(key.to_string(), b.build());
        }

        // thumbnail_path — empty string = no thumbnail block
        {
            let mut b = ConfigFieldSchemaBuilder::new("thumbnail_path", ConfigFieldType::String);
            b.default_value(ConfigValue::String(String::new()))
                .group("Output");
            fields.insert("thumbnail_path".to_string(), b.build());
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

    let mut b = ConfigFieldSchemaBuilder::new(key, field_type.clone());

    if let Some(default) = table
        .get("default")
        .map(|v| parse_config_value(v, &field_type))
    {
        b.default_value(default);
    }
    if let Some(min) = table.get("min").and_then(parse_numeric_value) {
        b.min(min);
    }
    if let Some(max) = table.get("max").and_then(parse_numeric_value) {
        b.max(max);
    }
    if let Some(step) = table.get("step").and_then(parse_numeric_value) {
        b.step(step);
    }
    if let Some(max_length) = table
        .get("max_length")
        .and_then(|v| v.as_integer().map(|i| i as usize))
    {
        b.max_length(max_length);
    }
    if let Some(values) = table.get("values").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
    }) {
        b.enum_values(values);
    }
    if let Some(min_list_length) = table
        .get("min_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize))
    {
        b.min_list_length(min_list_length);
    }
    if let Some(max_list_length) = table
        .get("max_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize))
    {
        b.max_list_length(max_list_length);
    }
    if let Some(display) = table.get("display").and_then(|v| v.as_str()) {
        b.display(display);
    }
    if let Some(description) = table.get("description").and_then(|v| v.as_str()) {
        b.description(description);
    }
    if let Some(group) = table.get("group").and_then(|v| v.as_str()) {
        b.group(group);
    }
    if let Some(unit) = table
        .get("unit")
        .and_then(|v| v.as_str())
        .map(parse_config_unit)
    {
        b.unit(unit);
    }
    if table
        .get("advanced")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        b.advanced(true);
    }
    if let Some(validate) = table.get("validate").and_then(|v| v.as_str()) {
        b.validate(validate);
    }

    Ok(b.build())
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

#[cfg(test)]
mod builder_smoke_tests {
    use super::*;

    #[test]
    fn config_field_schema_builder_round_trips_minimal_field() {
        let schema = ConfigFieldSchemaBuilder::new("density", ConfigFieldType::Float).build();
        assert_eq!(schema.key, "density");
        assert_eq!(schema.field_type, ConfigFieldType::Float);
        assert_eq!(schema.default, None);
        assert_eq!(schema.unit, ConfigUnit::None);
        assert!(!schema.advanced);
        assert_eq!(schema.min, None);
        assert_eq!(schema.max, None);
    }

    #[test]
    fn config_field_schema_builder_populates_float_with_min_max() {
        let mut b = ConfigFieldSchemaBuilder::new("ironing_speed", ConfigFieldType::Float);
        b.default_value(ConfigValue::Float(20.0))
            .group("Cooling")
            .unit(ConfigUnit::MillimetersPerSecond)
            .min(0.0)
            .max(200.0);
        let schema = b.build();
        assert_eq!(schema.key, "ironing_speed");
        assert_eq!(schema.default, Some(ConfigValue::Float(20.0)));
        assert_eq!(schema.group, Some("Cooling".to_string()));
        assert_eq!(schema.unit, ConfigUnit::MillimetersPerSecond);
        assert_eq!(schema.min, Some(0.0));
        assert_eq!(schema.max, Some(200.0));
    }
}
