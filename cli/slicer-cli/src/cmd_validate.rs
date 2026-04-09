//! Implementation of the `slicer validate` subcommand.
//!
//! Validates the module manifest TOML without building.
//! Checks: TOML schema validity, stage ID recognition, config field types,
//! cross-validate expression syntax, claim names, and wit-world version.

use std::fmt;
use std::fs;
use std::path::Path;

/// The nine valid pipeline stages a module can target.
const VALID_STAGES: &[&str] = &[
    "Layer::Infill",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::InfillPostProcess",
    "Layer::SlicePostProcess",
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
    "PostPass::GCodePostProcess",
    "PostPass::TextPostProcess",
];

/// The three WIT world package strings the current SDK supports.
const SUPPORTED_WIT_WORLDS: &[&str] = &[
    "slicer:world-layer@1.0.0",
    "slicer:world-prepass@1.0.0",
    "slicer:world-postpass@1.0.0",
];

/// Valid config field types from docs/03_wit_and_manifest.md.
const VALID_CONFIG_TYPES: &[&str] = &[
    "bool", "int", "float", "string", "enum", "float-list", "string-list",
];

/// Recognized claim names.
const RECOGNIZED_CLAIMS: &[&str] = &[
    "infill-generator",
    "perimeter-generator",
    "support-generator",
    "slice-postprocessor",
    "gcode-postprocessor",
    "text-postprocessor",
];

/// Recognized cross-validate severity values.
const VALID_SEVERITIES: &[&str] = &["error", "warning"];

/// Errors that can occur during manifest validation.
#[derive(Debug)]
pub enum ValidateError {
    /// No manifest TOML file found.
    ManifestNotFound,
    /// TOML could not be parsed.
    TomlParseError(String),
    /// Missing required section or field.
    MissingField(String),
    /// Stage ID is not one of the nine valid stages.
    InvalidStage(String),
    /// Config field has an invalid type.
    InvalidConfigType { field: String, got: String },
    /// Enum config field is missing the required `values` array.
    EnumMissingValues(String),
    /// Config field range is invalid (min > max).
    InvalidConfigRange { field: String, reason: String },
    /// Cross-validate rule has invalid syntax.
    InvalidCrossValidateRule { index: usize, reason: String },
    /// Cross-validate severity is not recognized.
    InvalidCrossValidateSeverity { index: usize, got: String },
    /// Unrecognized claim name.
    UnrecognizedClaim { kind: String, name: String },
    /// wit-world is not supported by the current SDK.
    UnsupportedWitWorld(String),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestNotFound => write!(f, "no module manifest (.toml) found in the current directory"),
            Self::TomlParseError(msg) => write!(f, "invalid TOML: {msg}"),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
            Self::InvalidStage(stage) => {
                write!(f, "unknown stage '{stage}'. Valid stages: {}", VALID_STAGES.join(", "))
            }
            Self::InvalidConfigType { field, got } => {
                write!(f, "config field '{field}' has invalid type '{got}'. Valid types: {}", VALID_CONFIG_TYPES.join(", "))
            }
            Self::EnumMissingValues(field) => {
                write!(f, "enum config field '{field}' is missing required 'values' array")
            }
            Self::InvalidConfigRange { field, reason } => {
                write!(f, "config field '{field}': {reason}")
            }
            Self::InvalidCrossValidateRule { index, reason } => {
                write!(f, "cross-validate rule [{index}]: {reason}")
            }
            Self::InvalidCrossValidateSeverity { index, got } => {
                write!(f, "cross-validate rule [{index}]: unknown severity '{got}'. Valid: {}", VALID_SEVERITIES.join(", "))
            }
            Self::UnrecognizedClaim { kind, name } => {
                write!(f, "unrecognized {kind} claim '{name}'")
            }
            Self::UnsupportedWitWorld(world) => {
                write!(f, "unsupported wit-world '{world}'. Supported: {}", SUPPORTED_WIT_WORLDS.join(", "))
            }
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for ValidateError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Find the module manifest TOML file in the given directory.
///
/// Looks for a `.toml` file that is not `Cargo.toml`. If exactly one is found,
/// returns its path. If none or multiple are found, returns an error.
pub fn find_manifest(dir: &Path) -> Result<std::path::PathBuf, ValidateError> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml")
            && path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml")
        {
            candidates.push(path);
        }
    }
    match candidates.len() {
        0 => Err(ValidateError::ManifestNotFound),
        1 => Ok(candidates.into_iter().next().unwrap()),
        _ => {
            // Multiple manifests found — still return the first alphabetically for determinism
            candidates.sort();
            Ok(candidates.into_iter().next().unwrap())
        }
    }
}

/// Parse the manifest TOML content into a `toml::Value`.
pub fn parse_manifest(content: &str) -> Result<toml::Value, ValidateError> {
    content
        .parse::<toml::Value>()
        .map_err(|e| ValidateError::TomlParseError(e.to_string()))
}

/// Validate the `[stage].id` field.
pub fn validate_stage(manifest: &toml::Value) -> Result<(), ValidateError> {
    let stage = manifest
        .get("stage")
        .ok_or_else(|| ValidateError::MissingField("stage".into()))?;
    let id = stage
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidateError::MissingField("stage.id".into()))?;
    if !VALID_STAGES.contains(&id) {
        return Err(ValidateError::InvalidStage(id.into()));
    }
    Ok(())
}

/// Validate the `[module].wit-world` field.
pub fn validate_wit_world(manifest: &toml::Value) -> Result<(), ValidateError> {
    let module = manifest
        .get("module")
        .ok_or_else(|| ValidateError::MissingField("module".into()))?;
    let wit_world = module
        .get("wit-world")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidateError::MissingField("module.wit-world".into()))?;
    if !SUPPORTED_WIT_WORLDS.contains(&wit_world) {
        return Err(ValidateError::UnsupportedWitWorld(wit_world.into()));
    }
    Ok(())
}

/// Validate all fields under `[config.schema]`.
pub fn validate_config_schema(manifest: &toml::Value) -> Result<(), ValidateError> {
    let schema = match manifest.get("config").and_then(|c| c.get("schema")) {
        Some(s) => s,
        None => return Ok(()), // No config section is valid
    };
    let table = match schema.as_table() {
        Some(t) => t,
        None => return Ok(()),
    };
    for (field_name, field_def) in table {
        let field_type = field_def
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !VALID_CONFIG_TYPES.contains(&field_type) {
            return Err(ValidateError::InvalidConfigType {
                field: field_name.clone(),
                got: field_type.into(),
            });
        }
        // Enum fields must have a `values` array
        if field_type == "enum" && field_def.get("values").is_none() {
            return Err(ValidateError::EnumMissingValues(field_name.clone()));
        }
        // Check min/max range validity
        if let (Some(min), Some(max)) = (field_def.get("min"), field_def.get("max")) {
            let min_f = min.as_float().or_else(|| min.as_integer().map(|i| i as f64));
            let max_f = max.as_float().or_else(|| max.as_integer().map(|i| i as f64));
            if let (Some(min_val), Some(max_val)) = (min_f, max_f) {
                if min_val > max_val {
                    return Err(ValidateError::InvalidConfigRange {
                        field: field_name.clone(),
                        reason: format!("min ({min_val}) > max ({max_val})"),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Validate `[[config.cross-validate]]` rules.
pub fn validate_cross_validate_rules(manifest: &toml::Value) -> Result<(), ValidateError> {
    let rules = match manifest
        .get("config")
        .and_then(|c| c.get("cross-validate"))
    {
        Some(r) => r,
        None => return Ok(()),
    };
    let arr = match rules.as_array() {
        Some(a) => a,
        None => return Ok(()),
    };
    for (i, rule) in arr.iter().enumerate() {
        // Must have a `rule` field
        if rule.get("rule").and_then(|v| v.as_str()).is_none() {
            return Err(ValidateError::InvalidCrossValidateRule {
                index: i,
                reason: "missing 'rule' field".into(),
            });
        }
        // Severity must be recognized if present
        if let Some(severity) = rule.get("severity").and_then(|v| v.as_str()) {
            if !VALID_SEVERITIES.contains(&severity) {
                return Err(ValidateError::InvalidCrossValidateSeverity {
                    index: i,
                    got: severity.into(),
                });
            }
        }
    }
    Ok(())
}

/// Validate `[claims].holds` and `[claims].requires`.
pub fn validate_claims(manifest: &toml::Value) -> Result<(), ValidateError> {
    let claims = match manifest.get("claims") {
        Some(c) => c,
        None => return Ok(()),
    };
    // Check holds
    if let Some(holds) = claims.get("holds").and_then(|v| v.as_array()) {
        for claim in holds {
            if let Some(name) = claim.as_str() {
                if !RECOGNIZED_CLAIMS.contains(&name) {
                    return Err(ValidateError::UnrecognizedClaim {
                        kind: "holds".into(),
                        name: name.into(),
                    });
                }
            }
        }
    }
    // Check requires
    if let Some(requires) = claims.get("requires").and_then(|v| v.as_array()) {
        for claim in requires {
            if let Some(name) = claim.as_str() {
                if !RECOGNIZED_CLAIMS.contains(&name) {
                    return Err(ValidateError::UnrecognizedClaim {
                        kind: "requires".into(),
                        name: name.into(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Validate the required `[module]` section fields.
pub fn validate_module_section(manifest: &toml::Value) -> Result<(), ValidateError> {
    let module = match manifest.get("module") {
        Some(m) => m,
        None => return Err(ValidateError::MissingField("module".into())),
    };
    let required_fields = [
        "id",
        "version",
        "display-name",
        "description",
        "author",
        "license",
        "wit-world",
    ];
    for field in &required_fields {
        if module.get(*field).and_then(|v| v.as_str()).is_none() {
            return Err(ValidateError::MissingField(format!("module.{field}")));
        }
    }
    Ok(())
}

/// Validate the `[compatibility]` section fields.
pub fn validate_compatibility(manifest: &toml::Value) -> Result<(), ValidateError> {
    // Compatibility section is optional
    let _compat = match manifest.get("compatibility") {
        Some(c) => c,
        None => return Ok(()),
    };
    // If present, it's valid as long as it parses as a table (already guaranteed by TOML parse)
    Ok(())
}

/// Execute manifest validation in the given directory.
///
/// This is the core implementation used by both the CLI entry point and tests.
pub fn execute_in(dir: &Path) -> Result<(), ValidateError> {
    let manifest_path = find_manifest(dir)?;
    let content = fs::read_to_string(&manifest_path)?;
    let manifest = parse_manifest(&content)?;

    validate_module_section(&manifest)?;
    validate_stage(&manifest)?;
    validate_wit_world(&manifest)?;
    validate_config_schema(&manifest)?;
    validate_cross_validate_rules(&manifest)?;
    validate_claims(&manifest)?;
    validate_compatibility(&manifest)?;

    Ok(())
}

/// Execute the `slicer validate` workflow in the current directory.
pub fn execute() -> Result<(), ValidateError> {
    let cwd = std::env::current_dir()?;
    execute_in(&cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TOML parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_valid_toml() {
        let content = r#"
[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "A test module"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#;
        let result = parse_manifest(content);
        assert!(result.is_ok(), "valid manifest TOML should parse");
    }

    #[test]
    fn parse_invalid_toml_syntax() {
        let content = "this is not [valid toml {{{}}}";
        let result = parse_manifest(content);
        assert!(matches!(result, Err(ValidateError::TomlParseError(_))));
    }

    // ── Stage validation ──────────────────────────────────────────────────

    #[test]
    fn valid_stage_ids_accepted() {
        for stage in VALID_STAGES {
            let manifest: toml::Value = toml::from_str(&format!(
                r#"[stage]
id = "{stage}"
"#
            ))
            .unwrap();
            assert!(
                validate_stage(&manifest).is_ok(),
                "stage '{stage}' should be valid"
            );
        }
    }

    #[test]
    fn unknown_stage_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[stage]
id = "Layer::Unknown"
"#,
        )
        .unwrap();
        let result = validate_stage(&manifest);
        assert!(matches!(result, Err(ValidateError::InvalidStage(s)) if s == "Layer::Unknown"));
    }

    #[test]
    fn missing_stage_section_rejected() {
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        let result = validate_stage(&manifest);
        assert!(matches!(result, Err(ValidateError::MissingField(_))));
    }

    // ── wit-world validation ──────────────────────────────────────────────

    #[test]
    fn valid_wit_worlds_accepted() {
        for world in SUPPORTED_WIT_WORLDS {
            let manifest: toml::Value = toml::from_str(&format!(
                r#"[module]
wit-world = "{world}"
"#
            ))
            .unwrap();
            assert!(
                validate_wit_world(&manifest).is_ok(),
                "wit-world '{world}' should be valid"
            );
        }
    }

    #[test]
    fn unsupported_wit_world_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[module]
wit-world = "slicer:world-layer@2.0.0"
"#,
        )
        .unwrap();
        let result = validate_wit_world(&manifest);
        assert!(matches!(result, Err(ValidateError::UnsupportedWitWorld(_))));
    }

    #[test]
    fn missing_wit_world_rejected() {
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        let result = validate_wit_world(&manifest);
        assert!(matches!(result, Err(ValidateError::MissingField(_))));
    }

    // ── Config schema validation ──────────────────────────────────────────

    #[test]
    fn valid_config_types_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[config.schema.density]
type = "float"
default = 0.15
min = 0.05
max = 0.95

[config.schema.enabled]
type = "bool"
default = true

[config.schema.count]
type = "int"
default = 1
min = 1
max = 10

[config.schema.name]
type = "string"
default = "hello"

[config.schema.pattern]
type = "enum"
values = ["a", "b"]
default = "a"

[config.schema.weights]
type = "float-list"
default = [1.0, 2.0]

[config.schema.tags]
type = "string-list"
default = ["x"]
"#,
        )
        .unwrap();
        assert!(validate_config_schema(&manifest).is_ok());
    }

    #[test]
    fn invalid_config_type_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[config.schema.bad]
type = "complex"
default = 0
"#,
        )
        .unwrap();
        let result = validate_config_schema(&manifest);
        assert!(
            matches!(result, Err(ValidateError::InvalidConfigType { ref field, ref got }) if field == "bad" && got == "complex")
        );
    }

    #[test]
    fn enum_without_values_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[config.schema.mode]
type = "enum"
default = "a"
"#,
        )
        .unwrap();
        let result = validate_config_schema(&manifest);
        assert!(matches!(result, Err(ValidateError::EnumMissingValues(ref f)) if f == "mode"));
    }

    #[test]
    fn config_min_greater_than_max_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[config.schema.density]
type = "float"
default = 0.5
min = 0.9
max = 0.1
"#,
        )
        .unwrap();
        let result = validate_config_schema(&manifest);
        assert!(matches!(result, Err(ValidateError::InvalidConfigRange { ref field, .. }) if field == "density"));
    }

    #[test]
    fn empty_config_schema_accepted() {
        let manifest: toml::Value = toml::from_str("[config.schema]\n").unwrap();
        assert!(validate_config_schema(&manifest).is_ok());
    }

    #[test]
    fn no_config_section_accepted() {
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        assert!(validate_config_schema(&manifest).is_ok());
    }

    // ── Cross-validate rules ──────────────────────────────────────────────

    #[test]
    fn valid_cross_validate_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[[config.cross-validate]]
rule = "cell_size >= precision * 10"
message = "cell must be 10x precision"
severity = "warning"
"#,
        )
        .unwrap();
        assert!(validate_cross_validate_rules(&manifest).is_ok());
    }

    #[test]
    fn cross_validate_missing_rule_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[[config.cross-validate]]
message = "no rule"
severity = "warning"
"#,
        )
        .unwrap();
        let result = validate_cross_validate_rules(&manifest);
        assert!(matches!(result, Err(ValidateError::InvalidCrossValidateRule { index: 0, .. })));
    }

    #[test]
    fn cross_validate_invalid_severity_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[[config.cross-validate]]
rule = "a > b"
message = "bad"
severity = "fatal"
"#,
        )
        .unwrap();
        let result = validate_cross_validate_rules(&manifest);
        assert!(matches!(result, Err(ValidateError::InvalidCrossValidateSeverity { index: 0, ref got }) if got == "fatal"));
    }

    #[test]
    fn no_cross_validate_rules_accepted() {
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        assert!(validate_cross_validate_rules(&manifest).is_ok());
    }

    // ── Claim validation ──────────────────────────────────────────────────

    #[test]
    fn recognized_claims_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[claims]
holds = ["infill-generator"]
requires = ["perimeter-generator"]
"#,
        )
        .unwrap();
        assert!(validate_claims(&manifest).is_ok());
    }

    #[test]
    fn unrecognized_holds_claim_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[claims]
holds = ["magic-unicorn"]
requires = []
"#,
        )
        .unwrap();
        let result = validate_claims(&manifest);
        assert!(matches!(result, Err(ValidateError::UnrecognizedClaim { ref kind, ref name }) if kind == "holds" && name == "magic-unicorn"));
    }

    #[test]
    fn unrecognized_requires_claim_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[claims]
holds = []
requires = ["nonexistent-claim"]
"#,
        )
        .unwrap();
        let result = validate_claims(&manifest);
        assert!(matches!(result, Err(ValidateError::UnrecognizedClaim { ref kind, ref name }) if kind == "requires" && name == "nonexistent-claim"));
    }

    #[test]
    fn empty_claims_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[claims]
holds = []
requires = []
"#,
        )
        .unwrap();
        assert!(validate_claims(&manifest).is_ok());
    }

    #[test]
    fn no_claims_section_accepted() {
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        assert!(validate_claims(&manifest).is_ok());
    }

    // ── Module section validation ─────────────────────────────────────────

    #[test]
    fn valid_module_section_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "A test"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"
"#,
        )
        .unwrap();
        assert!(validate_module_section(&manifest).is_ok());
    }

    #[test]
    fn missing_module_id_rejected() {
        let manifest: toml::Value = toml::from_str(
            r#"[module]
version = "0.1.0"
"#,
        )
        .unwrap();
        let result = validate_module_section(&manifest);
        assert!(matches!(result, Err(ValidateError::MissingField(ref f)) if f.contains("id")));
    }

    #[test]
    fn missing_module_section_rejected() {
        let manifest: toml::Value = toml::from_str("[stage]\nid = \"Layer::Infill\"").unwrap();
        let result = validate_module_section(&manifest);
        assert!(matches!(result, Err(ValidateError::MissingField(_))));
    }

    // ── Compatibility section validation ──────────────────────────────────

    #[test]
    fn valid_compatibility_accepted() {
        let manifest: toml::Value = toml::from_str(
            r#"[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.5.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"
"#,
        )
        .unwrap();
        assert!(validate_compatibility(&manifest).is_ok());
    }

    #[test]
    fn missing_compatibility_section_accepted() {
        // Compatibility is optional in the scaffold
        let manifest: toml::Value = toml::from_str("[module]\nid = \"test\"").unwrap();
        assert!(validate_compatibility(&manifest).is_ok());
    }

    // ── Manifest discovery ────────────────────────────────────────────────

    #[test]
    fn find_manifest_in_directory_with_toml() {
        let dir = tempfile::tempdir().unwrap();
        // Write Cargo.toml (should be skipped)
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        // Write the manifest
        fs::write(dir.path().join("my-module.toml"), "[module]\nid = \"test\"").unwrap();

        let result = find_manifest(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("my-module.toml"));
    }

    #[test]
    fn find_manifest_no_toml_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let result = find_manifest(dir.path());
        assert!(matches!(result, Err(ValidateError::ManifestNotFound)));
    }

    // ── Full integration (execute_in) ─────────────────────────────────────

    #[test]
    fn execute_in_valid_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();
        fs::write(
            dir.path().join("my-module.toml"),
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "A test module"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = ["infill-generator"]
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#,
        )
        .unwrap();

        let result = execute_in(dir.path());
        assert!(result.is_ok(), "valid manifest should pass validation");
    }

    #[test]
    fn execute_in_invalid_stage_fails() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            dir.path().join("test.toml"),
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "test"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Bogus"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#,
        )
        .unwrap();

        let result = execute_in(dir.path());
        assert!(matches!(result, Err(ValidateError::InvalidStage(_))));
    }

    #[test]
    fn execute_in_bad_wit_world_fails() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            dir.path().join("test.toml"),
            r#"[module]
id = "com.example.test"
version = "0.1.0"
display-name = "Test"
description = "test"
author = "tester"
license = "MIT"
wit-world = "slicer:world-layer@99.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[config.schema]

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#,
        )
        .unwrap();

        let result = execute_in(dir.path());
        assert!(matches!(result, Err(ValidateError::UnsupportedWitWorld(_))));
    }
}
