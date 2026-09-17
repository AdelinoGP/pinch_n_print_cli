//! Registry-typed ingestion of flat and explicitly scoped configuration.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use slicer_ir::{ConfigKey, ConfigValue, ModifierId, ObjectId};

use crate::{ConfigSchemaRegistry, RegistryEntry};

/// The configuration scope receiving an authored value.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum ConfigScope {
    /// The print-wide configuration scope.
    Global,
    /// Configuration for one object.
    Object(ObjectId),
    /// Configuration for one modifier belonging to one object.
    Modifier {
        /// The containing object identifier.
        object_id: ObjectId,
        /// The modifier identifier.
        modifier_id: ModifierId,
    },
    /// Configuration associated with one paint semantic.
    PaintSemantic(String),
    /// Configuration associated with one tool index.
    Tool(u32),
}

/// Authored configuration entries for one scope.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopeDelta {
    /// Values explicitly supplied by the author.
    pub values: BTreeMap<ConfigKey, ConfigValue>,
}

impl ScopeDelta {
    /// Iterate over authored values in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&ConfigKey, &ConfigValue)> {
        self.values.iter()
    }
}

/// All authored scope deltas in deterministic scope order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopedConfig {
    /// Authored deltas keyed by their typed scope.
    pub deltas: BTreeMap<ConfigScope, ScopeDelta>,
}

impl ScopedConfig {
    /// Return the authored delta for `scope`, if one was supplied.
    #[must_use]
    pub fn delta(&self, scope: &ConfigScope) -> Option<&ScopeDelta> {
        self.deltas.get(scope)
    }

    /// Return the authored global delta, if one was supplied.
    #[must_use]
    pub fn global(&self) -> Option<&ScopeDelta> {
        self.delta(&ConfigScope::Global)
    }

    /// Iterate over scope deltas in the deterministic [`ConfigScope`] order.
    pub fn iter(&self) -> impl Iterator<Item = (&ConfigScope, &ScopeDelta)> {
        self.deltas.iter()
    }

    /// Iterate over scopes in deterministic order.
    pub fn scopes(&self) -> impl Iterator<Item = &ConfigScope> {
        self.deltas.keys()
    }
}

/// A non-fatal issue encountered while retaining an authored value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestionWarning {
    /// The key was not declared by the registry.
    UnrecognizedKey {
        /// The original flat wire key.
        wire_key: String,
        /// The canonical key retained in the delta.
        key: String,
        /// The nearest canonical registry key, when sufficiently close.
        suggestion: Option<String>,
    },
    /// A tolerant-mode diagnostic: the value was retained unchanged because the declaration cannot represent the authored shape.
    UntypedValue {
        /// The canonical key receiving the value.
        key: String,
        /// The registry field type that could not represent the value.
        declared_type: String,
        /// The authored scalar spelling or debug representation.
        authored: String,
    },
}

/// A fatal error while decoding or typing an authored value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigIngestionError {
    /// A recognized scope prefix did not have the required shape.
    MalformedScopeKey {
        /// The original flat wire key.
        wire_key: String,
        /// The required wire-key shape.
        expected: String,
    },
    /// An authored value could not be represented by the declared field type.
    TypeMismatch {
        /// The canonical key receiving the value.
        key: String,
        /// The registry field type required by the declaration.
        expected: String,
        /// The authored scalar spelling or debug representation.
        authored: String,
    },
}

impl fmt::Display for ConfigIngestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedScopeKey { wire_key, expected } => write!(
                formatter,
                "malformed scope key {wire_key:?}; expected {expected}"
            ),
            Self::TypeMismatch {
                key,
                expected,
                authored,
            } => write!(
                formatter,
                "config key {key:?}: expected {expected}, authored {authored:?}"
            ),
        }
    }
}

impl std::error::Error for ConfigIngestionError {}

/// The completed result of typed configuration ingestion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IngestionOutcome {
    /// Authored values grouped by typed scope.
    pub scoped: ScopedConfig,
    /// Authored values for registry-declared selectors in the global scope.
    pub selector_values: BTreeMap<ConfigKey, ConfigValue>,
    /// Non-fatal warnings raised while retaining authored values.
    pub warnings: Vec<IngestionWarning>,
}

/// Transactional ingestor for registry-typed configuration values.
pub struct ConfigIngestor<'registry> {
    registry: &'registry ConfigSchemaRegistry,
    deltas: BTreeMap<ConfigScope, ScopeDelta>,
    warnings: Vec<IngestionWarning>,
    untyped_global_keys: BTreeSet<ConfigKey>,
    tolerant: bool,
}

impl<'registry> ConfigIngestor<'registry> {
    /// Start an ingestor backed by a reconciled schema registry.
    #[must_use]
    pub fn new(registry: &'registry ConfigSchemaRegistry) -> Self {
        Self::with_mode(registry, false)
    }

    /// Start an ingestor that retains declared values the schema cannot type.
    #[must_use]
    pub fn tolerant(registry: &'registry ConfigSchemaRegistry) -> Self {
        Self::with_mode(registry, true)
    }

    fn with_mode(registry: &'registry ConfigSchemaRegistry, tolerant: bool) -> Self {
        Self {
            registry,
            deltas: BTreeMap::new(),
            warnings: Vec::new(),
            untyped_global_keys: BTreeSet::new(),
            tolerant,
        }
    }

    /// Decode flat wire keys, type authored values, and retain them by scope.
    pub fn ingest_flat(
        &mut self,
        values: &HashMap<ConfigKey, ConfigValue>,
    ) -> Result<(), ConfigIngestionError> {
        let mut ordered = values.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(key, _)| *key);

        let mut pending = Vec::with_capacity(ordered.len());
        for (wire_key, authored) in ordered {
            let (scope, key) = decode_wire_key(wire_key)?;
            pending.push(self.prepare_entry(scope, key, wire_key, authored)?);
        }
        self.commit(pending);
        Ok(())
    }

    /// Ingest values already associated with an explicit typed scope.
    pub fn ingest_delta(
        &mut self,
        scope: ConfigScope,
        values: &HashMap<ConfigKey, ConfigValue>,
    ) -> Result<(), ConfigIngestionError> {
        let mut ordered = values.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(key, _)| *key);

        let mut pending = Vec::with_capacity(ordered.len());
        for (key, authored) in ordered {
            pending.push(self.prepare_entry(scope.clone(), key.clone(), key, authored)?);
        }
        self.commit(pending);
        Ok(())
    }

    /// Finish ingestion and return only authored values and derived selectors.
    #[must_use]
    pub fn finish(self) -> IngestionOutcome {
        let selector_values = self
            .deltas
            .get(&ConfigScope::Global)
            .map(|delta| {
                delta
                    .values
                    .iter()
                    .filter_map(|(key, value)| {
                        self.registry_entry(key)
                            .filter(|entry| {
                                entry.selector && !self.untyped_global_keys.contains(key)
                            })
                            .map(|_| (key.clone(), value.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();

        IngestionOutcome {
            scoped: ScopedConfig {
                deltas: self.deltas,
            },
            selector_values,
            warnings: self.warnings,
        }
    }

    fn prepare_entry<'value>(
        &self,
        scope: ConfigScope,
        key: ConfigKey,
        wire_key: &str,
        authored: &'value ConfigValue,
    ) -> Result<PendingEntry, ConfigIngestionError> {
        let Some(entry) = self.registry_entry(&key) else {
            return Ok(PendingEntry {
                scope,
                key: key.clone(),
                value: authored.clone(),
                warning: Some(IngestionWarning::UnrecognizedKey {
                    wire_key: wire_key.to_owned(),
                    key: key.clone(),
                    suggestion: self.suggestion(&key),
                }),
            });
        };

        let (value, warning) = match coerce_value(&key, authored, &entry.field_type) {
            Ok(value) => (value, None),
            Err(error) if self.tolerant => {
                if is_non_finite_type_mismatch(authored, &entry.field_type) {
                    return Err(error);
                }

                let value = match coerce_scalar_list(&key, authored, &entry.field_type) {
                    Ok(Some(value)) => value,
                    Ok(None) => authored.clone(),
                    Err(error) => return Err(error),
                };
                let warning = IngestionWarning::UntypedValue {
                    key: key.clone(),
                    declared_type: entry.field_type.clone(),
                    authored: authored_text(authored),
                };
                (value, Some(warning))
            }
            Err(error) => return Err(error),
        };
        Ok(PendingEntry {
            scope,
            key,
            value,
            warning,
        })
    }

    fn commit(&mut self, pending: Vec<PendingEntry>) {
        for entry in pending {
            let untyped_global = matches!(
                (&entry.scope, &entry.warning),
                (
                    ConfigScope::Global,
                    Some(IngestionWarning::UntypedValue { .. })
                )
            );
            if untyped_global {
                self.untyped_global_keys.insert(entry.key.clone());
            }
            self.deltas
                .entry(entry.scope)
                .or_default()
                .values
                .insert(entry.key, entry.value);
            if let Some(warning) = entry.warning {
                self.warnings.push(warning);
            }
        }
        self.warnings.sort_by(warning_order);
    }

    fn registry_entry(&self, key: &str) -> Option<&RegistryEntry> {
        self.registry.entry(key).or_else(|| {
            let (base, _) = key.rsplit_once(':')?;
            let wildcard = format!("{base}:*");
            self.registry.entry(&wildcard)
        })
    }

    fn suggestion(&self, key: &str) -> Option<String> {
        let mut candidates = self.registry.keys().collect::<Vec<_>>();
        candidates.sort_unstable();

        candidates
            .into_iter()
            .filter_map(|candidate| {
                let distance = levenshtein(key, candidate);
                (distance <= 2).then_some((distance, candidate))
            })
            .min_by(
                |(first_distance, first_key), (second_distance, second_key)| {
                    first_distance
                        .cmp(second_distance)
                        .then_with(|| first_key.cmp(second_key))
                },
            )
            .map(|(_, candidate)| candidate.to_owned())
    }
}

struct PendingEntry {
    scope: ConfigScope,
    key: ConfigKey,
    value: ConfigValue,
    warning: Option<IngestionWarning>,
}

fn warning_order(first: &IngestionWarning, second: &IngestionWarning) -> std::cmp::Ordering {
    match (first, second) {
        (
            IngestionWarning::UnrecognizedKey {
                wire_key: first_wire,
                key: first_key,
                suggestion: first_suggestion,
            },
            IngestionWarning::UnrecognizedKey {
                wire_key: second_wire,
                key: second_key,
                suggestion: second_suggestion,
            },
        ) => first_wire
            .cmp(second_wire)
            .then_with(|| first_key.cmp(second_key))
            .then_with(|| first_suggestion.cmp(second_suggestion)),
        (
            IngestionWarning::UntypedValue {
                key: first_key,
                declared_type: first_type,
                authored: first_authored,
            },
            IngestionWarning::UntypedValue {
                key: second_key,
                declared_type: second_type,
                authored: second_authored,
            },
        ) => first_key
            .cmp(second_key)
            .then_with(|| first_type.cmp(second_type))
            .then_with(|| first_authored.cmp(second_authored)),
        (IngestionWarning::UnrecognizedKey { .. }, IngestionWarning::UntypedValue { .. }) => {
            std::cmp::Ordering::Less
        }
        (IngestionWarning::UntypedValue { .. }, IngestionWarning::UnrecognizedKey { .. }) => {
            std::cmp::Ordering::Greater
        }
    }
}

fn decode_wire_key(wire_key: &str) -> Result<(ConfigScope, ConfigKey), ConfigIngestionError> {
    if wire_key == "object_config" || wire_key.starts_with("object_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "object_config:<object_id>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::Object(parts[1].to_owned()),
            parts[2].to_owned(),
        ));
    }

    if wire_key == "paint_config" || wire_key.starts_with("paint_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "paint_config:<semantic>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::PaintSemantic(parts[1].to_owned()),
            parts[2].to_owned(),
        ));
    }

    if wire_key == "tool_config" || wire_key.starts_with("tool_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        let tool = parts.get(1).and_then(|value| value.parse::<u32>().ok());
        if parts.len() != 3 || tool.is_none() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "tool_config:<u32>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::Tool(tool.expect("validated tool index")),
            parts[2].to_owned(),
        ));
    }

    Ok((ConfigScope::Global, wire_key.to_owned()))
}

fn coerce_value(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
) -> Result<ConfigValue, ConfigIngestionError> {
    let mismatch = || ConfigIngestionError::TypeMismatch {
        key: key.to_owned(),
        expected: field_type.to_owned(),
        authored: authored_text(authored),
    };

    match field_type {
        "bool" => match authored {
            ConfigValue::Bool(value) => Ok(ConfigValue::Bool(*value)),
            ConfigValue::String(value) => parse_bool(value)
                .map(ConfigValue::Bool)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "int" => match authored {
            ConfigValue::Int(value) => Ok(ConfigValue::Int(*value)),
            ConfigValue::String(value) => value
                .trim()
                .parse::<i64>()
                .map(ConfigValue::Int)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float" => match authored {
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::Float(*value)),
            ConfigValue::Int(value) => Ok(ConfigValue::Float(*value as f64)),
            ConfigValue::String(value) => parse_finite_float(value)
                .map(ConfigValue::Float)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "string" | "enum" => match authored {
            ConfigValue::String(value) => Ok(ConfigValue::String(value.clone())),
            _ => Err(mismatch()),
        },
        "percent" => match authored {
            ConfigValue::Percent(value) => Ok(ConfigValue::Percent(*value)),
            ConfigValue::FloatOrPercent { value, .. } if value.is_finite() => {
                Ok(ConfigValue::Percent(*value))
            }
            ConfigValue::Int(value) => Ok(ConfigValue::Percent(*value as f64)),
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::Percent(*value)),
            ConfigValue::String(value) => parse_percent(value)
                .map(ConfigValue::Percent)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float_or_percent" => match authored {
            ConfigValue::FloatOrPercent { value, is_percent } => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: *is_percent,
            }),
            ConfigValue::Percent(value) if value.is_finite() => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: true,
            }),
            ConfigValue::Int(value) => Ok(ConfigValue::FloatOrPercent {
                value: *value as f64,
                is_percent: false,
            }),
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: false,
            }),
            ConfigValue::String(value) => parse_float_or_percent(value)
                .map(|(value, is_percent)| ConfigValue::FloatOrPercent { value, is_percent })
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float-list" => coerce_list(key, authored, field_type, |item| match item {
            ConfigValue::Float(value) if value.is_finite() => Some(ConfigValue::Float(*value)),
            ConfigValue::Int(value) => Some(ConfigValue::Float(*value as f64)),
            ConfigValue::String(value) => parse_finite_float(value).ok().map(ConfigValue::Float),
            _ => None,
        }),
        "int-list" => coerce_list(key, authored, field_type, |item| match item {
            ConfigValue::Int(value) => Some(ConfigValue::Int(*value)),
            ConfigValue::String(value) => value.parse::<i64>().ok().map(ConfigValue::Int),
            _ => None,
        }),
        "string-list" => coerce_list(key, authored, field_type, |item| match item {
            ConfigValue::String(value) => Some(ConfigValue::String(value.clone())),
            _ => None,
        }),
        _ => Err(mismatch()),
    }
}

fn coerce_scalar_list(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
) -> Result<Option<ConfigValue>, ConfigIngestionError> {
    let ConfigValue::List(items) = authored else {
        return Ok(None);
    };
    if field_type.ends_with("-list") {
        return Ok(None);
    }

    let mut coerced = Vec::with_capacity(items.len());
    for item in items {
        match coerce_value(key, item, field_type) {
            Ok(value) => coerced.push(value),
            Err(error) if is_non_finite_scalar_value(item, field_type) => return Err(error),
            Err(_) => return Ok(None),
        }
    }
    Ok(Some(ConfigValue::List(coerced)))
}

fn is_non_finite_type_mismatch(authored: &ConfigValue, field_type: &str) -> bool {
    match authored {
        ConfigValue::List(items) if field_type.ends_with("-list") => {
            items.iter().any(|item| match field_type {
                "float-list" => is_non_finite_scalar_value(item, "float"),
                _ => false,
            })
        }
        ConfigValue::List(items) => items
            .iter()
            .any(|item| is_non_finite_scalar_value(item, field_type)),
        _ => is_non_finite_scalar_value(authored, field_type),
    }
}

fn is_non_finite_scalar_value(authored: &ConfigValue, field_type: &str) -> bool {
    match (field_type, authored) {
        ("float", ConfigValue::Float(value)) => !value.is_finite(),
        ("float", ConfigValue::String(value)) => is_non_finite_float_text(value),
        ("percent", ConfigValue::FloatOrPercent { value, .. }) => !value.is_finite(),
        ("percent", ConfigValue::Float(value)) => !value.is_finite(),
        ("percent", ConfigValue::String(value)) => is_non_finite_percent_text(value),
        ("float_or_percent", ConfigValue::Percent(value)) => !value.is_finite(),
        ("float_or_percent", ConfigValue::Float(value)) => !value.is_finite(),
        ("float_or_percent", ConfigValue::String(value)) => {
            is_non_finite_float_or_percent_text(value)
        }
        _ => false,
    }
}

fn is_non_finite_float_text(value: &str) -> bool {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .is_some_and(|value| !value.is_finite())
}

fn is_non_finite_percent_text(value: &str) -> bool {
    value
        .trim()
        .strip_suffix('%')
        .is_some_and(is_non_finite_float_text)
}

fn is_non_finite_float_or_percent_text(value: &str) -> bool {
    value
        .trim()
        .strip_suffix('%')
        .map_or_else(|| is_non_finite_float_text(value), is_non_finite_float_text)
}

fn coerce_list<F>(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
    mut convert: F,
) -> Result<ConfigValue, ConfigIngestionError>
where
    F: FnMut(&ConfigValue) -> Option<ConfigValue>,
{
    let ConfigValue::List(items) = authored else {
        return Err(ConfigIngestionError::TypeMismatch {
            key: key.to_owned(),
            expected: field_type.to_owned(),
            authored: authored_text(authored),
        });
    };

    items
        .iter()
        .map(&mut convert)
        .collect::<Option<Vec<_>>>()
        .map(ConfigValue::List)
        .ok_or_else(|| ConfigIngestionError::TypeMismatch {
            key: key.to_owned(),
            expected: field_type.to_owned(),
            authored: authored_text(authored),
        })
}

fn parse_finite_float(value: &str) -> Result<f64, ()> {
    let value = value.trim().parse::<f64>().map_err(|_| ())?;
    value.is_finite().then_some(value).ok_or(())
}

fn parse_bool(value: &str) -> Result<bool, ()> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true") || trimmed == "1" {
        return Ok(true);
    }
    if trimmed.eq_ignore_ascii_case("false") || trimmed == "0" {
        return Ok(false);
    }
    Err(())
}

fn parse_percent(value: &str) -> Result<f64, ()> {
    let trimmed = value.trim();
    let number = trimmed.strip_suffix('%').ok_or(())?;
    parse_finite_float(number)
}

fn parse_float_or_percent(value: &str) -> Result<(f64, bool), ()> {
    let trimmed = value.trim();
    if let Some(number) = trimmed.strip_suffix('%') {
        return Ok((parse_finite_float(number)?, true));
    }
    Ok((parse_finite_float(trimmed)?, false))
}

fn authored_text(value: &ConfigValue) -> String {
    match value {
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Int(value) => value.to_string(),
        ConfigValue::Float(value) => value.to_string(),
        ConfigValue::String(value) => value.clone(),
        ConfigValue::List(value) => format!("{value:?}"),
        ConfigValue::Percent(value) => format!("{value}%"),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                format!("{value}%")
            } else {
                value.to_string()
            }
        }
    }
}

fn levenshtein(first: &str, second: &str) -> usize {
    let second_chars = second.chars().collect::<Vec<_>>();
    let mut previous = (0..=second_chars.len()).collect::<Vec<_>>();
    for (first_index, first_char) in first.chars().enumerate() {
        let mut current = vec![first_index + 1; second_chars.len() + 1];
        for (second_index, second_char) in second_chars.iter().enumerate() {
            current[second_index + 1] = if first_char == *second_char {
                previous[second_index]
            } else {
                1 + previous[second_index]
                    .min(previous[second_index + 1])
                    .min(current[second_index])
            };
        }
        previous = current;
    }
    previous[second_chars.len()]
}
