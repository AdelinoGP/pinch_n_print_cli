//! Configuration schema ownership and deterministic registry assembly.

#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use slicer_ir::config_schema::ConfigFieldEntry;
use slicer_ir::feedrate::{FeedrateConfig, SPEED_DENIED_SCOPES, SPEED_KEYS, SPEED_META};
use slicer_ir::resolved_config::{
    HostConfigKey, HostKeyMeta, HostRuntimeKey, ResolvedConfig, HOST_RUNTIME_KEYS, SCOPE_PRINT,
};
use slicer_ir::{ConfigKey, ConfigValue};

pub mod ingestion;
pub mod resolution;

pub use ingestion::{
    ConfigIngestionError, ConfigIngestor, ConfigScope, IngestionOutcome, IngestionWarning,
    ScopeDelta, ScopedConfig,
};
pub use resolution::{
    query_z_grid, resolve_scope_stack, ResolutionError, ResolutionTarget, ResolvedObjectLayerConfig,
};

const HOST_PROVENANCE: &str = "host";
const SPEED_PROVENANCE: &str = "speed";
const RUNTIME_PROVENANCE: &str = "runtime";

const ALLOWED_DENIED_SCOPES: &[&str] = &[
    "global",
    "object",
    "layer_range",
    "modifier",
    "paint_semantic",
    "tool",
];

const PER_REGION_SCOPES: &[&str] = &[
    "object",
    "layer_range",
    "modifier",
    "paint_semantic",
    "tool",
];

/// A loaded module's configuration declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleDeclaration {
    /// Reverse-domain identifier of the declaring module.
    pub module_id: slicer_ir::ModuleId,
    /// Configuration schema contributed by the module.
    pub schema: slicer_ir::config_schema::ConfigSchema,
    /// Optional group identifying mutually exclusive module claims.
    pub claim_exclusive_group: Option<String>,
}

/// Owned UI metadata selected from a module declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleKeyMeta {
    /// Human-readable label for the setting.
    pub display: Option<String>,
    /// Help text for the setting.
    pub description: Option<String>,
    /// UI grouping hint.
    pub group: Option<String>,
    /// Unit displayed alongside the setting.
    pub unit: Option<String>,
    /// Search and taxonomy tags.
    pub tags: Vec<String>,
    /// Whether the setting is hidden from the ordinary settings view.
    pub advanced: bool,
}

impl ModuleKeyMeta {
    fn from_field(field: &ConfigFieldEntry) -> Self {
        Self {
            display: field.display.clone(),
            description: field.description.clone(),
            group: field.group.clone(),
            unit: field.unit.clone(),
            tags: field.tags.clone(),
            advanced: field.advanced,
        }
    }
}

/// A reconciled configuration key in the schema registry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegistryEntry {
    /// Canonical configuration key.
    pub key: String,
    /// Reconciled wire field type.
    pub field_type: String,
    /// Deterministically selected default value.
    pub default: Option<String>,
    /// Effective inclusive lower bound after intersection.
    pub min: Option<f64>,
    /// Effective inclusive upper bound after intersection.
    pub max: Option<f64>,
    /// Reconciled enum domain, when one is declared.
    pub values: Option<Vec<String>>,
    /// Union of scopes in which the key cannot be stated.
    pub denied_scopes: Vec<String>,
    /// Whether the key is usable during claim selection.
    pub selector: bool,
    /// Whether the key is omitted from the resolved config block.
    ///
    /// Reconciled any-true-wins: a key is omitted if any host row or declaring
    /// module marks it `config_block = false`. The polarity is deliberate — a
    /// derived `Default` means "emitted".
    pub omit_from_config_block: bool,
    /// Key supplying the absolute base for relative values.
    pub base_key: Option<String>,
    /// Metadata supplied by the selected host declaration, if any.
    pub host_meta: Option<HostKeyMeta>,
    /// Metadata supplied by the alphabetically first module declaration, if any.
    pub module_meta: Option<ModuleKeyMeta>,
    /// Sorted provenance labels for every declaration contributing this entry.
    pub provenance: Vec<String>,
}

/// Host-side declaration channels used to build a registry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HostChannels {
    /// Keys declared by `ResolvedConfig`.
    pub host_keys: Vec<HostConfigKey>,
    /// Keys declared by the feedrate table.
    pub speed_keys: Vec<HostConfigKey>,
    /// Const-safe keys declared by host runtime code.
    pub runtime_keys: Vec<HostRuntimeKey>,
}

impl HostChannels {
    /// Assemble the three live host declaration channels.
    #[must_use]
    pub fn from_live() -> Self {
        let host_keys = ResolvedConfig::host_config_keys();
        let speed_defaults = FeedrateConfig::default();
        let speed_keys = SPEED_KEYS
            .iter()
            .enumerate()
            .map(|(index, &(key, field))| {
                let mut probe = speed_defaults.clone();
                let speed_field = field(&mut probe);
                let default = speed_field.default_string();
                let meta = SPEED_META
                    .get(index)
                    .and_then(|maybe_meta| *maybe_meta)
                    .unwrap_or(HostKeyMeta::NONE);
                HostConfigKey {
                    key,
                    field_type: speed_field.wire_type(),
                    scope: SCOPE_PRINT,
                    default: Some(default),
                    meta,
                    denied_scopes: SPEED_DENIED_SCOPES[index],
                }
            })
            .collect();

        Self {
            host_keys,
            speed_keys,
            runtime_keys: HOST_RUNTIME_KEYS.to_vec(),
        }
    }

    /// Construct host channels from explicitly supplied declaration parts.
    #[must_use]
    pub fn from_parts(
        host_keys: Vec<HostConfigKey>,
        speed_keys: Vec<HostConfigKey>,
        runtime_keys: Vec<HostRuntimeKey>,
    ) -> Self {
        Self {
            host_keys,
            speed_keys,
            runtime_keys,
        }
    }
}

/// Registry assembled together with non-fatal reconciliation diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyOutcome {
    /// Reconciled configuration entries.
    pub registry: ConfigSchemaRegistry,
    /// Warnings raised while reconciling declarations.
    pub warnings: Vec<RegistryWarning>,
}

/// Non-fatal diagnostic emitted while reconciling declarations.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryWarning {
    /// Mutually exclusive modules disagree on their selected defaults.
    ClaimExclusiveDivergence {
        /// Configuration key with divergent declarations.
        key: String,
        /// Alphabetically first module in the claim group.
        first_module: String,
        /// Default selected by the first module.
        first_value: String,
        /// Alphabetically later module in the claim group.
        second_module: String,
        /// Default selected by the later module.
        second_value: String,
    },
    /// Mutually exclusive modules disagree on their selected numeric bounds.
    ClaimExclusiveBoundsDivergence {
        /// Configuration key with divergent declarations.
        key: String,
        /// Alphabetically first module in the claim group.
        first_module: String,
        /// Inclusive lower bound declared by the first module.
        first_min: Option<f64>,
        /// Inclusive upper bound declared by the first module.
        first_max: Option<f64>,
        /// Alphabetically later module in the claim group.
        second_module: String,
        /// Inclusive lower bound declared by the later module.
        second_min: Option<f64>,
        /// Inclusive upper bound declared by the later module.
        second_max: Option<f64>,
    },
    /// Numeric bounds were intersected across declarations.
    BoundsIntersection {
        /// Configuration key whose bounds were intersected.
        key: String,
        /// Effective inclusive lower bound.
        effective_min: Option<f64>,
        /// Effective inclusive upper bound.
        effective_max: Option<f64>,
        /// Sorted provenance labels for the declarations.
        declarers: Vec<String>,
    },
    /// A module default is retained for documentation but cannot override a host default.
    ModuleDefaultForHostKey {
        /// Host-owned configuration key.
        key: String,
        /// Module whose default cannot become effective.
        module_id: String,
    },
}

/// A fatal error preventing construction of a valid schema registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryLoadError {
    /// Declarations disagree on the wire field type.
    TypeDisagreement {
        /// Configuration key with the disagreement.
        key: String,
        /// Sorted provenance labels for all contributors.
        declarers: Vec<String>,
    },
    /// Enum declarations disagree on their exact value domain.
    EnumDomainDisagreement {
        /// Configuration key with the disagreement.
        key: String,
        /// Sorted provenance labels for all contributors.
        declarers: Vec<String>,
    },
    /// A selector remains statable at one or more per-region scopes.
    SelectorStatablePerRegion {
        /// Selector key with an invalid eligibility policy.
        key: String,
    },
    /// A base key is not declared by any channel.
    BaseKeyMissing {
        /// Relative key requiring a base.
        key: String,
        /// Missing base key.
        base: String,
    },
    /// A base key is declared with a type that cannot supply an absolute value.
    BaseKeyNotPercentCompatible {
        /// Relative key requiring a base.
        key: String,
        /// Incompatible base key.
        base: String,
    },
    /// Base-key traversal found a dependency cycle.
    BaseKeyCycle {
        /// Relative key from which the cycle was discovered.
        key: String,
        /// First base key named by the dependent key.
        base: String,
    },
    /// A declaration names a scope outside the deny-list vocabulary.
    UnknownDeniedScope {
        /// Configuration key carrying the invalid scope.
        key: String,
        /// Unknown denied scope.
        scope: String,
    },
}

impl fmt::Display for RegistryLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeDisagreement { key, declarers } => {
                write!(formatter, "type disagreement for {key} ({declarers:?})")
            }
            Self::EnumDomainDisagreement { key, declarers } => {
                write!(
                    formatter,
                    "enum domain disagreement for {key} ({declarers:?})"
                )
            }
            Self::SelectorStatablePerRegion { key } => {
                write!(formatter, "selector {key} is statable per region")
            }
            Self::BaseKeyMissing { key, base } => {
                write!(formatter, "base key {base} for {key} is missing")
            }
            Self::BaseKeyNotPercentCompatible { key, base } => {
                write!(
                    formatter,
                    "base key {base} for {key} is not percent-compatible"
                )
            }
            Self::BaseKeyCycle { key, base } => {
                write!(formatter, "base-key cycle for {key} through {base}")
            }
            Self::UnknownDeniedScope { key, scope } => {
                write!(formatter, "unknown denied scope {scope} for {key}")
            }
        }
    }
}

impl std::error::Error for RegistryLoadError {}

/// The `denied_scopes` vocabulary label for a typed configuration scope.
///
/// Scope instances share their family's eligibility policy: every object,
/// modifier, paint-semantic name, and tool index is judged by its family.
pub(crate) fn scope_denial_label(scope: &ConfigScope) -> &'static str {
    match scope {
        ConfigScope::Global => "global",
        ConfigScope::Object(_) => "object",
        ConfigScope::Modifier { .. } => "modifier",
        ConfigScope::PaintSemantic(_) => "paint_semantic",
        ConfigScope::Tool(_) => "tool",
    }
}

/// Deterministic registry of reconciled configuration declarations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConfigSchemaRegistry {
    entries: BTreeMap<String, RegistryEntry>,
}

impl ConfigSchemaRegistry {
    /// Return the registry keys statable at `scope`.
    ///
    /// The set is derived only from the reconciled `denied_scopes` of each
    /// entry: a scope denied by any declarer is denied for the key (ADR-0069
    /// union), and an absent denial means the key is statable at every scope.
    /// It is the only per-scope key roster the resolver consults, so it can
    /// never disagree with a hand-authored roster elsewhere.
    #[must_use]
    pub fn admission_set(&self, scope: &ConfigScope) -> BTreeSet<ConfigKey> {
        let label = scope_denial_label(scope);
        self.entries
            .values()
            .filter(|entry| !entry.denied_scopes.iter().any(|denied| denied == label))
            .map(|entry| entry.key.clone())
            .collect()
    }

    /// Iterate over registry keys in lexicographic order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Look up one reconciled registry entry.
    #[must_use]
    pub fn entry(&self, key: &str) -> Option<&RegistryEntry> {
        self.entries.get(key)
    }

    /// Return the number of reconciled keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the registry contains no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Project an effective resolved config into the resolved config block.
    ///
    /// A key of `resolved.to_config_map()` is emitted when either:
    ///
    /// - it is a registry entry without `omit_from_config_block`; or
    /// - it is a [`ResolvedConfig::typed_field_keys`] key with no registry
    ///   entry (today `infill_type`) — typed fields keep emitting because a
    ///   `plain` row binds to no config key and so has no registry entry.
    ///
    /// Retained unknown `extensions` keys are never emitted. The returned map
    /// is a `BTreeMap`, so emission order is deterministic and never depends on
    /// hash iteration.
    #[must_use]
    pub fn config_block_map(&self, resolved: &ResolvedConfig) -> BTreeMap<String, ConfigValue> {
        self.project_config_block(resolved, None)
    }

    /// Project the resolved config block over the LOADED (claim-selected)
    /// module set.
    ///
    /// [`Self::config_block_map`] spans every declaration that survived
    /// assembly, and claim dedup drops a module from dispatch only, never
    /// from the config schema (`crates/slicer-runtime/src/run.rs`), so a
    /// claim-losing module's keys stay declared and resolvable. The emitted
    /// `CONFIG_BLOCK` is the LOADED surface though (packet-06 AC-4): a key is
    /// emitted only when its [`RegistryEntry::provenance`] names a module in
    /// `live_modules` or one of the host channels. Typed fields with no
    /// registry entry are host-owned and always emit.
    #[must_use]
    pub fn config_block_map_for_modules(
        &self,
        resolved: &ResolvedConfig,
        live_modules: &BTreeSet<String>,
    ) -> BTreeMap<String, ConfigValue> {
        self.project_config_block(resolved, Some(live_modules))
    }

    fn project_config_block(
        &self,
        resolved: &ResolvedConfig,
        live_modules: Option<&BTreeSet<String>>,
    ) -> BTreeMap<String, ConfigValue> {
        let effective = resolved.to_config_map();
        let mut block = BTreeMap::new();

        for (key, entry) in &self.entries {
            if entry.omit_from_config_block {
                continue;
            }
            if let Some(live) = live_modules {
                let contributed_by_live = entry.provenance.iter().any(|label| {
                    live.contains(label.as_str())
                        || matches!(
                            label.as_str(),
                            HOST_PROVENANCE | SPEED_PROVENANCE | RUNTIME_PROVENANCE
                        )
                });
                if !contributed_by_live {
                    continue;
                }
            }
            if let Some(value) = effective.get(key) {
                block.insert(key.clone(), value.clone());
            }
        }
        for key in ResolvedConfig::typed_field_keys() {
            if self.entries.contains_key(*key) {
                continue;
            }
            if let Some(value) = effective.get(*key) {
                block.insert((*key).to_owned(), value.clone());
            }
        }

        block
    }
}

/// Machine- and tool-specific values used while expanding automatic settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpansionContext {
    /// Nozzle diameter in millimetres, or zero when it has not been supplied.
    pub nozzle_diameter_mm: f64,
    /// Absolute base values keyed first by tool index, then by config key.
    pub tool_bases: BTreeMap<u32, BTreeMap<String, f64>>,
}

/// Failure raised while expanding a relative or automatic configuration value.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpansionError {
    /// A relative value's declared base has no merged or contextual value.
    UnknownBaseKey {
        /// Key whose value requires expansion.
        key: String,
        /// Declared base key that could not be resolved.
        base_key: String,
    },
    /// A sentinel automatic value has no corresponding source value.
    MissingAutoBase {
        /// Key whose sentinel value requires expansion.
        key: String,
        /// Automatic source key that is absent.
        base_key: String,
    },
    /// A required base is zero, negative, or non-finite.
    NonPositiveBase {
        /// Key whose value requires expansion.
        key: String,
        /// Base key containing the invalid value.
        base_key: String,
        /// Invalid base value.
        value: f64,
    },
}

impl fmt::Display for ExpansionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBaseKey { key, base_key } => {
                write!(formatter, "unknown base key {base_key} required by {key}")
            }
            Self::MissingAutoBase { key, base_key } => {
                write!(
                    formatter,
                    "automatic base key {base_key} required by {key} is missing"
                )
            }
            Self::NonPositiveBase {
                key,
                base_key,
                value,
            } => write!(
                formatter,
                "base key {base_key} required by {key} must be positive and finite, got {value}"
            ),
        }
    }
}

impl std::error::Error for ExpansionError {}

const BOTTOM_AUTO_BASES: [(&str, &str); 2] = [
    (
        "support_interface_bottom_layers",
        "support_interface_top_layers",
    ),
    (
        "support_bottom_interface_spacing",
        "support_interface_spacing",
    ),
];

struct ExpansionResolver<'a> {
    registry: &'a ConfigSchemaRegistry,
    source: BTreeMap<String, ConfigValue>,
    context: &'a ExpansionContext,
    tool_index: Option<u32>,
    expanded: BTreeMap<String, ConfigValue>,
    visiting: BTreeSet<String>,
}

impl ExpansionResolver<'_> {
    fn expand_key(&mut self, key: &str) -> Result<Option<ConfigValue>, ExpansionError> {
        if let Some(value) = self.expanded.get(key) {
            return Ok(Some(value.clone()));
        }
        let Some(value) = self.source.get(key).cloned() else {
            return Ok(None);
        };

        // Registry construction rejects base-key cycles. The guard also keeps
        // this API total if a future registry source bypasses that validation.
        if !self.visiting.insert(key.to_owned()) {
            return Ok(Some(value));
        }

        let result = if key == "line_width" && is_plain_zero(&value) {
            ConfigValue::Float(1.125 * self.required_base(key, "nozzle_diameter")?)
        } else if key == "support_line_width" && is_plain_zero(&value) {
            ConfigValue::Float(self.required_base(key, "nozzle_diameter")?)
        } else if let Some((_, base_key)) = BOTTOM_AUTO_BASES
            .iter()
            .find(|(automatic_key, _)| *automatic_key == key && is_plain_minus_one(&value))
        {
            self.expand_key(base_key)?
                .ok_or_else(|| ExpansionError::MissingAutoBase {
                    key: key.to_owned(),
                    base_key: (*base_key).to_owned(),
                })?
        } else if let Some((percent, base_key)) = percent_magnitude(&value).and_then(|percent| {
            self.registry
                .entry(key)
                .and_then(|entry| entry.base_key.as_deref())
                .map(|base_key| (percent, base_key.to_owned()))
        }) {
            let base = self.required_base(key, &base_key)?;
            absolute_like(&value, percent / 100.0 * base)
        } else {
            value
        };

        self.visiting.remove(key);
        self.expanded.insert(key.to_owned(), result.clone());
        Ok(Some(result))
    }

    fn required_nozzle(&self, key: &str) -> Result<f64, ExpansionError> {
        validate_base(key, "nozzle_diameter", self.context.nozzle_diameter_mm)
    }

    fn required_base(&mut self, key: &str, base_key: &str) -> Result<f64, ExpansionError> {
        if let Some(value) = self
            .tool_index
            .and_then(|tool| self.context.tool_bases.get(&tool))
            .and_then(|bases| bases.get(base_key))
            .copied()
        {
            return validate_base(key, base_key, value);
        }

        if let Some(value) = self.expand_key(base_key)? {
            if let Some(value) = base_number(&value) {
                return validate_base(key, base_key, value);
            }
        } else if base_key == "nozzle_diameter" {
            return self.required_nozzle(key);
        }

        Err(ExpansionError::UnknownBaseKey {
            key: key.to_owned(),
            base_key: base_key.to_owned(),
        })
    }
}

fn percent_magnitude(value: &ConfigValue) -> Option<f64> {
    match value {
        ConfigValue::Percent(percent) => Some(*percent),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: true,
        } => Some(*value),
        _ => None,
    }
}

fn absolute_number(value: &ConfigValue) -> Option<f64> {
    match value {
        ConfigValue::Float(value) => Some(*value),
        ConfigValue::Int(value) => Some(*value as f64),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => Some(*value),
        _ => None,
    }
}

/// Base resolution's scalar view of a key's value: canonical vector wire
/// (`coFloats`/`coInts`-family — `nozzle_diameter` is `ConfigOptionFloats`,
/// `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp`) reaches scalar
/// readers as a `List` whose first element carries the value, the same
/// documented shape leniency as typed reads' `envelope`
/// (`crates/slicer-ir/src/slice_ir.rs`). Per-tool bases are handled earlier
/// via [`ExpansionContext::tool_bases`].
fn base_number(value: &ConfigValue) -> Option<f64> {
    match value {
        ConfigValue::List(items) => items.first().and_then(absolute_number),
        other => absolute_number(other),
    }
}

fn absolute_like(original: &ConfigValue, value: f64) -> ConfigValue {
    match original {
        ConfigValue::FloatOrPercent { .. } => ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        },
        _ => ConfigValue::Float(value),
    }
}

fn is_plain_zero(value: &ConfigValue) -> bool {
    matches!(value, ConfigValue::Float(value) if *value == 0.0)
        || matches!(value, ConfigValue::Int(0))
        || matches!(
            value,
            ConfigValue::FloatOrPercent {
                value,
                is_percent: false,
            } if *value == 0.0
        )
}

fn is_plain_minus_one(value: &ConfigValue) -> bool {
    matches!(value, ConfigValue::Float(value) if *value == -1.0)
        || matches!(value, ConfigValue::Int(-1))
        || matches!(
            value,
            ConfigValue::FloatOrPercent {
                value,
                is_percent: false,
            } if *value == -1.0
        )
}

fn validate_base(key: &str, base_key: &str, value: f64) -> Result<f64, ExpansionError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(ExpansionError::NonPositiveBase {
            key: key.to_owned(),
            base_key: base_key.to_owned(),
            value,
        })
    }
}

/// Expand relative and sentinel automatic values in a fully merged config.
///
/// Expansion is transactional: all replacements are resolved against the
/// original merged snapshot and committed only after every required base has
/// been found and validated.
pub fn expand_automatic_values(
    registry: &ConfigSchemaRegistry,
    config: &mut ResolvedConfig,
    context: &ExpansionContext,
    tool_index: Option<u32>,
) -> Result<(), ExpansionError> {
    let source = config.to_config_map().into_iter().collect();
    let mut resolver = ExpansionResolver {
        registry,
        source,
        context,
        tool_index,
        expanded: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };

    let keys: Vec<String> = resolver.source.keys().cloned().collect();
    for key in keys {
        resolver.expand_key(&key)?;
    }

    let mut candidate = config.clone();
    for (key, expanded) in resolver.expanded {
        if resolver.source.get(&key) == Some(&expanded) {
            continue;
        }
        if let std::collections::btree_map::Entry::Occupied(mut entry) =
            candidate.extensions.entry(key.clone())
        {
            entry.insert(expanded);
            continue;
        }
        match candidate.apply_cli_key(&key, &expanded) {
            Ok(true) if key == "support_line_width" => {
                // The typed field retains float-or-percent provenance when it
                // is flattened. Shadow it with the normalized literal that
                // module ConfigViews must receive after expansion.
                candidate.extensions.insert(key, expanded);
            }
            Ok(true) => {}
            Ok(false) | Err(_) => {
                candidate.extensions.insert(key, expanded);
            }
        }
    }
    *config = candidate;
    Ok(())
}

#[derive(Debug, Clone)]
struct Declaration {
    provenance: String,
    module_id: Option<String>,
    claim_exclusive_group: Option<String>,
    field_type: String,
    default: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    values: Option<Vec<String>>,
    denied_scopes: Vec<String>,
    selector: bool,
    omit_from_config_block: bool,
    base_key: Option<String>,
    host_meta: Option<HostKeyMeta>,
    module_meta: Option<ModuleKeyMeta>,
    is_host: bool,
}

fn host_declaration(row: &HostConfigKey, provenance: &str) -> Declaration {
    let field_type = row.meta.wire_type.unwrap_or(row.field_type);
    let values = if field_type == "enum" || !row.meta.values.is_empty() {
        Some(
            row.meta
                .values
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        )
    } else {
        None
    };

    Declaration {
        provenance: provenance.to_owned(),
        module_id: None,
        claim_exclusive_group: None,
        field_type: field_type.to_owned(),
        default: row.default.clone(),
        min: row.meta.min,
        max: row.meta.max,
        values,
        denied_scopes: row
            .denied_scopes
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect(),
        selector: false,
        omit_from_config_block: row.meta.omit_from_config_block,
        base_key: None,
        host_meta: Some(row.meta),
        module_meta: None,
        is_host: true,
    }
}

fn runtime_declaration(row: &HostRuntimeKey) -> Declaration {
    let field_type = row.meta.wire_type.unwrap_or(row.field_type);
    let values = if field_type == "enum" || !row.meta.values.is_empty() {
        Some(
            row.meta
                .values
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        )
    } else {
        None
    };

    Declaration {
        provenance: RUNTIME_PROVENANCE.to_owned(),
        module_id: None,
        claim_exclusive_group: None,
        field_type: field_type.to_owned(),
        default: row.default.map(|value| value.to_owned()),
        min: row.meta.min,
        max: row.meta.max,
        values,
        denied_scopes: row
            .denied_scopes
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect(),
        selector: row.selector,
        omit_from_config_block: row.meta.omit_from_config_block,
        base_key: None,
        host_meta: Some(row.meta),
        module_meta: None,
        is_host: true,
    }
}

fn module_declaration(module: &ModuleDeclaration, field: &ConfigFieldEntry) -> Declaration {
    Declaration {
        provenance: module.module_id.clone(),
        module_id: Some(module.module_id.clone()),
        claim_exclusive_group: module.claim_exclusive_group.clone(),
        field_type: field.field_type.clone(),
        default: field.default.clone(),
        min: field.min,
        max: field.max,
        values: field.values.clone(),
        denied_scopes: field.denied_scopes.clone(),
        selector: field.selector,
        omit_from_config_block: field.omit_from_config_block,
        base_key: field.base_key.clone(),
        host_meta: None,
        module_meta: Some(ModuleKeyMeta::from_field(field)),
        is_host: false,
    }
}

fn add_declaration(
    declarations: &mut BTreeMap<String, Vec<Declaration>>,
    key: &str,
    declaration: Declaration,
) {
    declarations
        .entry(key.to_owned())
        .or_default()
        .push(declaration);
}

fn sorted_provenance(declarations: &[Declaration]) -> Vec<String> {
    declarations
        .iter()
        .map(|declaration| declaration.provenance.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_denied_scopes(
    key: &str,
    declarations: &[Declaration],
) -> Result<(), RegistryLoadError> {
    for declaration in declarations {
        for scope in &declaration.denied_scopes {
            if !ALLOWED_DENIED_SCOPES.contains(&scope.as_str()) {
                return Err(RegistryLoadError::UnknownDeniedScope {
                    key: key.to_owned(),
                    scope: scope.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_selectors(key: &str, declarations: &[Declaration]) -> Result<(), RegistryLoadError> {
    for declaration in declarations {
        if declaration.selector
            && PER_REGION_SCOPES.iter().any(|scope| {
                !declaration
                    .denied_scopes
                    .iter()
                    .any(|denied| denied == scope)
            })
        {
            return Err(RegistryLoadError::SelectorStatablePerRegion {
                key: key.to_owned(),
            });
        }
    }
    Ok(())
}

fn effective_bound<I>(values: I, maximum: bool) -> Option<f64>
where
    I: IntoIterator<Item = Option<f64>>,
{
    values.into_iter().flatten().fold(None, |current, value| {
        Some(match current {
            None => value,
            Some(previous) if maximum => previous.max(value),
            Some(previous) => previous.min(value),
        })
    })
}

fn selected_base_key(
    declarations: &[Declaration],
    module_declarations: &[&Declaration],
) -> Option<String> {
    declarations
        .iter()
        .find(|declaration| declaration.is_host)
        .and_then(|declaration| declaration.base_key.clone())
        .or_else(|| {
            module_declarations
                .iter()
                .find_map(|declaration| declaration.base_key.clone())
        })
}

fn selected_values(
    declarations: &[Declaration],
    module_declarations: &[&Declaration],
) -> Option<Vec<String>> {
    declarations
        .iter()
        .find(|declaration| declaration.is_host)
        .and_then(|declaration| declaration.values.clone())
        .or_else(|| {
            module_declarations
                .iter()
                .find_map(|declaration| declaration.values.clone())
        })
}

fn validate_base_keys(entries: &BTreeMap<String, RegistryEntry>) -> Result<(), RegistryLoadError> {
    for (key, entry) in entries {
        let Some(base) = entry.base_key.as_ref() else {
            continue;
        };
        let Some(base_entry) = entries.get(base) else {
            return Err(RegistryLoadError::BaseKeyMissing {
                key: key.clone(),
                base: base.clone(),
            });
        };
        if !matches!(base_entry.field_type.as_str(), "float" | "float_or_percent") {
            return Err(RegistryLoadError::BaseKeyNotPercentCompatible {
                key: key.clone(),
                base: base.clone(),
            });
        }
    }

    for (key, entry) in entries {
        let Some(initial_base) = entry.base_key.as_ref() else {
            continue;
        };
        let mut visited = BTreeSet::from([key.clone()]);
        let mut current = initial_base.as_str();
        loop {
            if !visited.insert(current.to_owned()) {
                return Err(RegistryLoadError::BaseKeyCycle {
                    key: key.clone(),
                    base: initial_base.clone(),
                });
            }
            let Some(next) = entries
                .get(current)
                .and_then(|base_entry| base_entry.base_key.as_deref())
            else {
                break;
            };
            current = next;
        }
    }

    Ok(())
}

fn claim_exclusive_warnings(
    key: &str,
    modules: &[&Declaration],
    warnings: &mut Vec<RegistryWarning>,
) {
    for (first_index, first) in modules.iter().enumerate() {
        for second in modules.iter().skip(first_index + 1) {
            if first.claim_exclusive_group.is_none()
                || first.claim_exclusive_group != second.claim_exclusive_group
            {
                continue;
            }
            let first_module = first
                .module_id
                .clone()
                .unwrap_or_else(|| first.provenance.clone());
            let second_module = second
                .module_id
                .clone()
                .unwrap_or_else(|| second.provenance.clone());

            if let (Some(first_value), Some(second_value)) =
                (first.default.as_ref(), second.default.as_ref())
            {
                if first_value != second_value {
                    warnings.push(RegistryWarning::ClaimExclusiveDivergence {
                        key: key.to_owned(),
                        first_module: first_module.clone(),
                        first_value: first_value.clone(),
                        second_module: second_module.clone(),
                        second_value: second_value.clone(),
                    });
                }
            }

            if first.min != second.min || first.max != second.max {
                warnings.push(RegistryWarning::ClaimExclusiveBoundsDivergence {
                    key: key.to_owned(),
                    first_module,
                    first_min: first.min,
                    first_max: first.max,
                    second_module,
                    second_min: second.min,
                    second_max: second.max,
                });
            }
        }
    }
}

/// Assemble the deterministic registry from all host and module channels.
///
/// Host declarations are considered before module declarations. Module
/// declarations are sorted by module id before defaults, metadata, warnings,
/// and provenance are selected, so caller or filesystem order cannot change
/// the result.
pub fn assemble_registry(
    modules: &[ModuleDeclaration],
    host: &HostChannels,
) -> Result<AssemblyOutcome, RegistryLoadError> {
    let mut declarations: BTreeMap<String, Vec<Declaration>> = BTreeMap::new();

    for row in &host.host_keys {
        add_declaration(
            &mut declarations,
            row.key,
            host_declaration(row, HOST_PROVENANCE),
        );
    }
    for row in &host.speed_keys {
        add_declaration(
            &mut declarations,
            row.key,
            host_declaration(row, SPEED_PROVENANCE),
        );
    }
    for row in &host.runtime_keys {
        add_declaration(&mut declarations, row.key, runtime_declaration(row));
    }

    let mut sorted_modules: Vec<&ModuleDeclaration> = modules.iter().collect();
    sorted_modules.sort_by(|first, second| first.module_id.cmp(&second.module_id));
    for module in sorted_modules {
        for (key, field) in &module.schema.entries {
            add_declaration(&mut declarations, key, module_declaration(module, field));
        }
    }

    let mut entries = BTreeMap::new();
    let mut warnings = Vec::new();

    for (key, key_declarations) in declarations {
        validate_denied_scopes(&key, &key_declarations)?;
        validate_selectors(&key, &key_declarations)?;

        let Some(first_declaration) = key_declarations.first() else {
            continue;
        };
        let provenance = sorted_provenance(&key_declarations);
        if key_declarations
            .iter()
            .any(|declaration| declaration.field_type != first_declaration.field_type)
        {
            return Err(RegistryLoadError::TypeDisagreement {
                key,
                declarers: provenance,
            });
        }

        if first_declaration.field_type == "enum" {
            let expected_values = first_declaration.values.as_ref();
            if key_declarations
                .iter()
                .any(|declaration| declaration.values.as_ref() != expected_values)
            {
                return Err(RegistryLoadError::EnumDomainDisagreement {
                    key,
                    declarers: provenance,
                });
            }
        }

        let module_declarations: Vec<&Declaration> = key_declarations
            .iter()
            .filter(|declaration| !declaration.is_host)
            .collect();
        let host_declaration = key_declarations
            .iter()
            .find(|declaration| declaration.is_host);

        let effective_min = effective_bound(
            key_declarations.iter().map(|declaration| declaration.min),
            true,
        );
        let effective_max = effective_bound(
            key_declarations.iter().map(|declaration| declaration.max),
            false,
        );
        if key_declarations
            .iter()
            .filter(|declaration| declaration.min.is_some() || declaration.max.is_some())
            .count()
            > 1
        {
            warnings.push(RegistryWarning::BoundsIntersection {
                key: key.clone(),
                effective_min,
                effective_max,
                declarers: provenance.clone(),
            });
        }

        if host_declaration.is_some() {
            for module in &module_declarations {
                if module.default.is_some() {
                    warnings.push(RegistryWarning::ModuleDefaultForHostKey {
                        key: key.clone(),
                        module_id: module
                            .module_id
                            .clone()
                            .unwrap_or_else(|| module.provenance.clone()),
                    });
                }
            }
        }
        claim_exclusive_warnings(&key, &module_declarations, &mut warnings);

        let default = match host_declaration {
            Some(declaration) => declaration.default.clone(),
            None => module_declarations
                .first()
                .and_then(|declaration| declaration.default.clone()),
        };
        let host_meta = host_declaration.and_then(|declaration| declaration.host_meta);
        let module_meta = module_declarations
            .first()
            .and_then(|declaration| declaration.module_meta.clone());
        let denied_scopes = ALLOWED_DENIED_SCOPES
            .iter()
            .filter(|scope| {
                key_declarations.iter().any(|declaration| {
                    declaration
                        .denied_scopes
                        .iter()
                        .any(|denied| denied == *scope)
                })
            })
            .map(|scope| (*scope).to_owned())
            .collect();

        entries.insert(
            key.clone(),
            RegistryEntry {
                key,
                field_type: first_declaration.field_type.clone(),
                default,
                min: effective_min,
                max: effective_max,
                values: selected_values(&key_declarations, &module_declarations),
                denied_scopes,
                selector: key_declarations
                    .iter()
                    .any(|declaration| declaration.selector),
                omit_from_config_block: key_declarations
                    .iter()
                    .any(|declaration| declaration.omit_from_config_block),
                base_key: selected_base_key(&key_declarations, &module_declarations),
                host_meta,
                module_meta,
                provenance,
            },
        );
    }

    validate_base_keys(&entries)?;

    Ok(AssemblyOutcome {
        registry: ConfigSchemaRegistry { entries },
        warnings,
    })
}
