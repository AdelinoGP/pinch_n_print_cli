//! Shared config-schema carriers for module declarations.

use std::collections::BTreeMap;

use crate::slice_ir::ConfigValue;

/// Helper for serde skip_serializing_if on bool.
fn is_false(b: &bool) -> bool {
    !*b
}

/// A single config field entry parsed from a module manifest `[config.schema]`
/// table entry.
///
/// Mirrors the fields defined in `docs/03_wit_and_manifest.md` § Config Field
/// Types Reference. The `type` field is required; all others are optional and
/// serialize as `null` when absent.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct ConfigFieldEntry {
    /// Field type string — must be one of: `"bool"`, `"int"`, `"float"`,
    /// `"string"`, `"enum"`, `"float-list"`, `"string-list"`.
    pub field_type: String,
    /// Default value as a string representation.
    pub default: Option<String>,
    /// Parsed `default` for `"percent"` / `"float_or_percent"` field types,
    /// retained from `parse_percent_default` rather than discarded
    /// (packet 185 / DEV-100). `None` for every other field type. Skipped in
    /// serialization so the config-schema wire shape is unchanged.
    #[serde(skip)]
    pub parsed_default: Option<ConfigValue>,
    /// Minimum for int/float fields.
    pub min: Option<f64>,
    /// Maximum for int/float fields.
    pub max: Option<f64>,
    /// Step for int/float fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// UI display name.
    pub display: Option<String>,
    /// UI tooltip / description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// UI grouping hint.
    pub group: Option<String>,
    /// Unit hint (`"mm"`, `"ratio"`, `"degrees"`, `"mm/s"`, `"ms"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Whether this is an advanced setting (hidden by default).
    #[serde(skip_serializing_if = "is_false")]
    pub advanced: bool,
    /// Allowed values for `"enum"` fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// Max length for `"string"` fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Min list length for list fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_list_length: Option<usize>,
    /// Max list length for list fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_list_length: Option<usize>,
    /// UI taxonomy tags for sub-tab filtering and search. Free-form strings;
    /// see `docs/03_wit_and_manifest.md` for conventions. Empty by default.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this field is a selector.
    #[serde(default)]
    pub selector: bool,
    /// Base config key used to resolve relative values.
    #[serde(default)]
    pub base_key: Option<String>,
    /// Scopes in which this field is not statable.
    #[serde(default)]
    pub denied_scopes: Vec<String>,
}

/// Full config schema for a module, holding all field entries.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct ConfigSchema {
    /// Parsed field entries keyed by field name.
    pub entries: BTreeMap<String, ConfigFieldEntry>,
}
