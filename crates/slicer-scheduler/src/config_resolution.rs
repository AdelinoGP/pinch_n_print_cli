//! Host-side resolver that turns user-supplied CLI config into per-object
//! [`slicer_ir::ResolvedConfig`] values. Invoked from the `Run` command and
//! the live execution-plan path; the resulting configs drive `RegionPlan.config`
//! during pipeline execution.

use std::collections::{BTreeMap, HashMap};

use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigIngestionError, ConfigIngestor,
    ConfigSchemaRegistry, ConfigScope, ExpansionContext, HostChannels, ModuleDeclaration,
    ResolutionError, ResolutionTarget, ScopeDelta, ScopedConfig,
};
use slicer_ir::{ConfigKey, ConfigValue, PaintSemantic, ResolvedConfig};

use crate::manifest::LoadedModule;

// Re-exported so `slicer_runtime::config_resolution::ConfigResolutionError` keeps
// resolving; the canonical definition lives next to `ResolvedConfig` in
// `slicer_ir::resolved_config`.
pub use slicer_ir::ConfigResolutionError;

/// A single module's declared `[min, max]` for a numeric config key. Input
/// shape for [`ConfigBoundsIndex::from_declarations`].
#[derive(Debug, Clone, PartialEq)]
pub struct BoundsDeclaration {
    /// Config key name (e.g. `"layer_height"`).
    pub key: String,
    /// Inclusive minimum declared by the module, if any.
    pub min: Option<f64>,
    /// Inclusive maximum declared by the module, if any.
    pub max: Option<f64>,
    /// Module that declared the bound (used only in diagnostics).
    pub module_id: String,
}

/// Strictest numeric bounds for a single config key, merged across every
/// module that declared the key in its manifest `[config.schema]` table.
#[derive(Debug, Clone, PartialEq)]
struct NumericBounds {
    min: Option<f64>,
    max: Option<f64>,
}

impl NumericBounds {
    fn intersect(&self, other: &NumericBounds) -> NumericBounds {
        // For min: take the larger (more restrictive). None means unbounded
        // below â€” any Some wins.
        let min = match (self.min, other.min) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        // For max: take the smaller (more restrictive).
        let max = match (self.max, other.max) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        NumericBounds { min, max }
    }
}

/// Per-key numeric bounds aggregated from every loaded module's manifest
/// schema. Built once at host startup via [`ConfigBoundsIndex::from_modules`]
/// and threaded into the resolver entry points so out-of-range CLI values are
/// rejected the same way variant TypeMismatches are.
///
/// When several modules declare the same key with different `[min, max]`
/// bounds, the strictest range wins (intersection): every module's contract
/// must hold simultaneously. If the intersection is empty (`min > max`), the
/// resulting range is retained and rejects every value â€” a `log::warn!` is
/// emitted naming the offending modules at construction time.
#[derive(Debug, Clone)]
pub struct ConfigBoundsIndex {
    bounds: HashMap<String, NumericBounds>,
    enum_values: HashMap<String, Vec<String>>,
    /// Parsed schema defaults for `percent` / `float_or_percent` fields
    /// (packet 185 / AC-6, TASK-303). Threaded into
    /// `ResolvedConfig.extensions` during unified scope resolution when the
    /// profile supplies no value for the key, so module-owned percent keys
    /// reach the live transport as `Percent` / `FloatOrPercent` values
    /// instead of vanishing at the parser.
    schema_defaults: HashMap<String, ConfigValue>,
    registry: ConfigSchemaRegistry,
}

impl Default for ConfigBoundsIndex {
    fn default() -> Self {
        Self::empty()
    }
}

impl ConfigBoundsIndex {
    /// An empty index â€” no bounds enforced. Useful for tests and call sites
    /// that have no loaded modules in scope.
    pub fn empty() -> Self {
        Self {
            bounds: HashMap::new(),
            enum_values: HashMap::new(),
            schema_defaults: HashMap::new(),
            registry: assemble_config_registry(&[]),
        }
    }

    /// Build the index by walking every loaded module's config schema.
    ///
    /// Only entries whose `field_type` is numeric (`int`, `float`,
    /// `float-list`, `int-list`) and that carry at least one of `min`/`max`
    /// contribute to the index. On collision across modules, ranges are
    /// intersected.
    ///
    /// Entries carrying a parsed `percent` / `float_or_percent` schema
    /// default (`ConfigFieldEntry::parsed_default`) additionally populate the
    /// schema-default table consumed by unified scope resolution; on
    /// collision the first module's default wins.
    pub fn from_modules<'a, I>(modules: I) -> Self
    where
        I: IntoIterator<Item = &'a LoadedModule>,
    {
        let modules: Vec<&LoadedModule> = modules.into_iter().collect();
        let declarations = modules
            .iter()
            .map(|module| ModuleDeclaration {
                module_id: module.id().to_owned(),
                schema: module.config_schema().clone(),
                claim_exclusive_group: None,
            })
            .collect::<Vec<_>>();
        let registry = assemble_config_registry(&declarations);
        let mut schema_defaults: HashMap<String, ConfigValue> = HashMap::new();
        let mut enum_values: HashMap<String, Vec<String>> = HashMap::new();
        for module in &modules {
            for (key, entry) in &module.config_schema().entries {
                if let Some(default) = &entry.parsed_default {
                    schema_defaults
                        .entry(key.clone())
                        .or_insert_with(|| default.clone());
                }
                if entry.field_type == "enum" {
                    if let Some(values) = &entry.values {
                        enum_values
                            .entry(key.clone())
                            .or_insert_with(|| values.clone());
                    }
                }
            }
        }
        let bounds_declarations = modules.into_iter().flat_map(|module| {
            let module_id = module.id().to_string();
            module
                .config_schema()
                .entries
                .iter()
                .filter_map(move |(key, entry)| {
                    if !is_numeric_field_type(&entry.field_type) {
                        return None;
                    }
                    if entry.min.is_none() && entry.max.is_none() {
                        return None;
                    }
                    Some(BoundsDeclaration {
                        key: key.clone(),
                        min: entry.min,
                        max: entry.max,
                        module_id: module_id.clone(),
                    })
                })
        });
        let mut index = Self::from_declarations_with_registry(bounds_declarations, registry);
        index.schema_defaults = schema_defaults;
        index.enum_values = enum_values;
        index
    }

    /// Build the index from an explicit iterator of per-module
    /// `(key, min, max, module_id)` declarations.
    ///
    /// Used by [`ConfigBoundsIndex::from_modules`] internally and exposed for
    /// integration tests so they can construct a bounds index without
    /// fabricating full `LoadedModule` values.
    pub fn from_declarations<I>(declarations: I) -> Self
    where
        I: IntoIterator<Item = BoundsDeclaration>,
    {
        Self::from_declarations_with_registry(declarations, assemble_config_registry(&[]))
    }

    fn from_declarations_with_registry<I>(declarations: I, registry: ConfigSchemaRegistry) -> Self
    where
        I: IntoIterator<Item = BoundsDeclaration>,
    {
        let mut index: HashMap<String, NumericBounds> = HashMap::new();
        let mut contributors: HashMap<String, Vec<String>> = HashMap::new();

        for decl in declarations {
            let new_bounds = NumericBounds {
                min: decl.min,
                max: decl.max,
            };
            contributors
                .entry(decl.key.clone())
                .or_default()
                .push(decl.module_id);
            index
                .entry(decl.key)
                .and_modify(|existing| {
                    *existing = existing.intersect(&new_bounds);
                })
                .or_insert(new_bounds);
        }

        for (key, bounds) in &index {
            if let (Some(lo), Some(hi)) = (bounds.min, bounds.max) {
                if lo > hi {
                    let empty = Vec::new();
                    let modules = contributors.get(key).unwrap_or(&empty);
                    log::warn!(
                        "config bounds intersection is empty for key '{key}' (effective range [{lo}, {hi}]); every value will be rejected. Modules declaring this key: {modules:?}"
                    );
                }
            }
        }

        Self {
            bounds: index,
            enum_values: HashMap::new(),
            schema_defaults: HashMap::new(),
            registry,
        }
    }

    /// Validate a single `(key, value)` pair against the merged bounds.
    ///
    /// Returns:
    /// - `Ok(())` when the key has no bounds, the value's variant is not
    ///   numeric (variant mismatch is `apply_cli_key`'s job), or every
    ///   numeric component lies within the declared range.
    /// - `Err(ConfigResolutionError::OutOfRange { .. })` when a numeric scalar
    ///   or list element falls outside `[min, max]` or is NaN/non-finite
    ///   against a finite bound. For list values, the first offending element
    ///   is reported with `index: Some(i)`.
    pub fn check(&self, key: &str, value: &ConfigValue) -> Result<(), ConfigResolutionError> {
        if let (Some(values), ConfigValue::String(value)) = (self.enum_values.get(key), value) {
            if !values.contains(value) {
                return Err(ConfigResolutionError::TypeMismatch {
                    key: key.to_string(),
                    expected: "one of the manifest-declared enum values",
                    actual: format!("unsupported enum value '{value}'"),
                });
            }
        }
        let Some(bounds) = self.bounds.get(key) else {
            return Ok(());
        };
        check_value(key, value, bounds, None)
    }

    /// Iterate the parsed `percent` / `float_or_percent` schema defaults
    /// collected by [`ConfigBoundsIndex::from_modules`].
    pub fn schema_defaults(&self) -> impl Iterator<Item = (&String, &ConfigValue)> {
        self.schema_defaults.iter()
    }

    /// Registry used by the unified scope-stack resolver.
    #[must_use]
    pub fn registry(&self) -> &ConfigSchemaRegistry {
        &self.registry
    }
}

fn assemble_config_registry(declarations: &[ModuleDeclaration]) -> ConfigSchemaRegistry {
    assemble_registry(declarations, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("loaded module config schemas must reconcile: {error}"))
        .registry
}

/// Compatibility ingestion for callers that still enter through the legacy
/// flat-map resolver APIs.
///
/// Production composition roots must call the corresponding `*_scoped`
/// entry points with their retained manifest-first ingestion result, avoiding
/// this adapter and any second decoding of scope prefixes. This adapter stays
/// tolerant so legacy callers preserve warn-and-keep behavior for authored
/// shapes their declarations cannot represent. Absolute-value bounds
/// enforcement remains in the resolver loop.
fn ingest_scoped_config(
    source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<ScopedConfig, ConfigResolutionError> {
    let mut ingestor = ConfigIngestor::tolerant(&bounds.registry);
    ingestor
        .ingest_flat(source)
        .map_err(|error| map_ingestion_error(error, source))?;
    Ok(ingestor.finish().scoped)
}

fn map_ingestion_error(
    error: ConfigIngestionError,
    source: &HashMap<ConfigKey, ConfigValue>,
) -> ConfigResolutionError {
    match error {
        ConfigIngestionError::TypeMismatch {
            key,
            expected,
            authored,
        } => {
            let actual = source
                .get(&key)
                .map_or_else(|| authored.clone(), |value| format!("{value:?}"));
            ConfigResolutionError::TypeMismatch {
                key,
                expected: resolution_type_name(&expected),
                actual,
            }
        }
        ConfigIngestionError::MalformedScopeKey { wire_key, expected } => {
            ConfigResolutionError::TypeMismatch {
                key: wire_key,
                expected: malformed_scope_shape(&expected),
                actual: "malformed scoped config key".to_owned(),
            }
        }
    }
}

fn resolution_type_name(field_type: &str) -> &'static str {
    match field_type {
        "bool" => "Bool",
        "int" => "Int",
        "float" => "Float",
        "string" => "String",
        "enum" => "String",
        "percent" => "Percent",
        "float_or_percent" => "FloatOrPercent",
        "float-list" => "List<Float>",
        "int-list" => "List<Int>",
        "string-list" => "List<String>",
        _ => "registry-declared type",
    }
}

fn malformed_scope_shape(expected: &str) -> &'static str {
    match expected {
        "object_config:<object_id>:<key>" => "object_config:<object_id>:<key>",
        "paint_config:<semantic>:<key>" => "paint_config:<semantic>:<key>",
        "tool_config:<u32>:<key>" => "tool_config:<u32>:<key>",
        _ => "well-formed scoped config key",
    }
}

fn is_numeric_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "int" | "float" | "float-list" | "int-list" | "percent" | "float_or_percent"
    )
}

/// Legacy config-key spellings and the canonical key each resolves to.
///
/// Entries are `(legacy, canonical)`. Supplying **both** spellings in one
/// source is rejected rather than silently resolved: with a `HashMap` source
/// there is no defined ordering between the two keys, so last-writer-wins would
/// make the resolved value depend on hash iteration order — non-deterministic
/// across runs. Rejecting is the pre-existing precedent set by
/// `first_layer_line_width`, and is applied uniformly here.
const CONFIG_KEY_ALIASES: [(&str, &str); 2] = [
    ("first_layer_line_width", "initial_layer_line_width"),
    // Renamed to the canonical OrcaSlicer spelling (`PrintConfig.cpp`'s
    // `support_threshold_angle`); the old in-tree name stays accepted so
    // existing profiles and 3MF project settings keep resolving.
    ("support_overhang_angle", "support_threshold_angle"),
];

/// Rejects any source that supplies both spellings of an aliased key.
fn reject_alias_conflicts<F>(contains: F) -> Result<(), ConfigResolutionError>
where
    F: Fn(&str) -> bool,
{
    for (legacy, canonical) in CONFIG_KEY_ALIASES {
        if contains(canonical) && contains(legacy) {
            return Err(ConfigResolutionError::TypeMismatch {
                key: format!("{canonical} and {legacy}"),
                expected: "one config key",
                actual: "both config keys supplied".into(),
            });
        }
    }
    Ok(())
}

fn canonical_config_key(key: &str) -> &str {
    for (legacy, canonical) in CONFIG_KEY_ALIASES {
        if key == legacy {
            return canonical;
        }
    }
    key
}

/// Decode and validate a legacy flat source for the unified scope-stack resolver.
///
/// This adapter is the only scheduler-owned part of resolution: it preserves
/// legacy aliases, manifest bounds, and schema defaults while returning the
/// typed [`ScopedConfig`] consumed by [`slicer_config::resolve_scope_stack`].
pub fn ingest_resolution_config(
    source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<ScopedConfig, ConfigResolutionError> {
    let scoped = ingest_scoped_config(source, bounds)?;
    prepare_scoped_config(&scoped, bounds)
}

/// Resolve one complete scope stack through the canonical `slicer-config`
/// resolver, including Phase-B automatic-value expansion after all deltas have
/// merged.
pub fn resolve_config(
    source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &ConfigBoundsIndex,
    target: &ResolutionTarget,
    expansion: &ExpansionContext,
) -> Result<ResolvedConfig, ResolutionError> {
    let scoped = ingest_resolution_config(source, bounds)?;
    resolve_scope_stack(bounds.registry(), &scoped, target, expansion)
}

/// Report paint-semantic deltas that do not correspond to a semantic present
/// in the model. Resolution itself only applies semantics selected by its
/// [`ResolutionTarget`], so warning production remains a separate host concern.
pub fn unknown_paint_semantic_warnings(
    scoped: &ScopedConfig,
    present_semantics: &[PaintSemantic],
) -> Vec<UnknownSemanticWarning> {
    let mut warnings = Vec::new();
    for (scope, delta) in scoped.iter() {
        let ConfigScope::PaintSemantic(semantic_name) = scope else {
            continue;
        };
        if present_semantics
            .iter()
            .any(|semantic| paint_semantic_namespace_key(semantic) == *semantic_name)
        {
            continue;
        }
        warnings.extend(delta.iter().map(|(key, _)| UnknownSemanticWarning {
            semantic_name: semantic_name.clone(),
            key: key.clone(),
        }));
    }
    warnings
}

fn prepare_scoped_config(
    scoped: &ScopedConfig,
    bounds: &ConfigBoundsIndex,
) -> Result<ScopedConfig, ConfigResolutionError> {
    let mut prepared = ScopedConfig::default();

    for (scope, delta) in scoped.iter() {
        reject_alias_conflicts(|key| delta.values.contains_key(key))?;
        let mut values = BTreeMap::new();
        for (key, value) in delta.iter() {
            if matches!(scope, ConfigScope::Global) && key.starts_with("object_height:") {
                continue;
            }
            let resolved_key = canonical_config_key(key);
            bounds.check(resolved_key, value)?;
            values.insert(resolved_key.to_owned(), value.clone());
        }
        prepared.deltas.insert(scope.clone(), ScopeDelta { values });
    }

    let global = prepared.deltas.entry(ConfigScope::Global).or_default();
    for (key, default) in bounds.schema_defaults() {
        if global.values.contains_key(key) {
            continue;
        }
        let mut probe = ResolvedConfig::default();
        if matches!(
            probe.apply_cli_key(canonical_config_key(key), default),
            Ok(false)
        ) {
            global.values.insert(key.clone(), default.clone());
        }
    }

    Ok(prepared)
}

fn check_value(
    key: &str,
    value: &ConfigValue,
    bounds: &NumericBounds,
    index: Option<usize>,
) -> Result<(), ConfigResolutionError> {
    match value {
        ConfigValue::Int(i) => check_scalar(key, *i as f64, bounds, index),
        ConfigValue::Float(f) => check_scalar(key, *f, bounds, index),
        ConfigValue::List(elements) => {
            for (i, element) in elements.iter().enumerate() {
                check_value(key, element, bounds, Some(i))?;
            }
            Ok(())
        }
        // Non-numeric variants: variant mismatch (if any) is reported by
        // `apply_cli_key`'s TypeMismatch path; numeric bounds don't apply.
        ConfigValue::Bool(_) | ConfigValue::String(_) => Ok(()),
        // `Percent` / `FloatOrPercent` (packet 150) ARE numeric per
        // `is_numeric_field_type` above, but only their MIN side is checked
        // here — see `check_percent_scalar`. A declared `[min, max]` is
        // expressed in the key's absolute unit (e.g. mm) while a
        // percent-authored value carries a relative magnitude whose absolute
        // equivalent is only known at Phase-B expansion, which packet 04 owns.
        ConfigValue::Percent(p) => check_percent_scalar(key, *p, bounds, index),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                check_percent_scalar(key, *value, bounds, index)
            } else {
                check_scalar(key, *value, bounds, index)
            }
        }
    }
}

fn check_scalar(
    key: &str,
    value: f64,
    bounds: &NumericBounds,
    index: Option<usize>,
) -> Result<(), ConfigResolutionError> {
    let in_range = value.is_finite()
        && bounds.min.map_or(true, |lo| value >= lo)
        && bounds.max.map_or(true, |hi| value <= hi);
    if in_range {
        Ok(())
    } else {
        Err(ConfigResolutionError::OutOfRange {
            key: key.to_string(),
            value,
            min: bounds.min,
            max: bounds.max,
            index,
        })
    }
}

/// Bounds check for a percent-authored value (`Percent`, or
/// `FloatOrPercent { is_percent: true }`).
///
/// Only the **min** side is enforced against the raw magnitude: a negative
/// percent is invalid regardless of how it expands, so `min` is meaningful in
/// both the relative and absolute domains. The **max** side is deliberately
/// NOT enforced here because a declared `[min, max]` is expressed in the key's
/// absolute unit (e.g. `overhang_reverse_threshold` declares `max = 10.0` mm)
/// while a percent-authored value carries a relative magnitude whose absolute
/// equivalent (`50%` × the percent base) is only known once Phase-B expansion
/// runs — packet 04 owns that. Enforcing an absolute max against a raw
/// percentage would reject valid Orca values: `resources/cube_4color.3mf`
/// authors `overhang_reverse_threshold = "50%"`, which would compare `50.0`
/// against `max = 10.0` mm.
fn check_percent_scalar(
    key: &str,
    value: f64,
    bounds: &NumericBounds,
    index: Option<usize>,
) -> Result<(), ConfigResolutionError> {
    let satisfies_min = value.is_finite() && bounds.min.map_or(true, |lo| value >= lo);
    if satisfies_min {
        Ok(())
    } else {
        Err(ConfigResolutionError::OutOfRange {
            key: key.to_string(),
            value,
            min: bounds.min,
            max: bounds.max,
            index,
        })
    }
}

/// Diagnostic returned when a `paint_config:<semantic>:<key>` entry references
/// a semantic not present in the model. Non-fatal; forwarded to the progress
/// event sink by the host caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSemanticWarning {
    /// The unrecognised semantic name from the config key.
    pub semantic_name: String,
    /// The config sub-key (after the semantic part).
    pub key: String,
}

impl UnknownSemanticWarning {
    /// Returns `true` if `pattern` is contained in either `semantic_name` or `key`.
    pub fn contains(&self, pattern: &str) -> bool {
        self.semantic_name.contains(pattern) || self.key.contains(pattern)
    }
}

/// Map a [`PaintSemantic`] to the snake_case string used in the
/// `paint_config:<semantic>:<key>` namespace.
///
/// Built-in variants serialize as `material`/`fuzzy_skin`/`support_enforcer`/
/// `support_blocker`; `Custom(s)` serializes as the raw `s`.
pub fn paint_semantic_namespace_key(s: &PaintSemantic) -> String {
    match s {
        PaintSemantic::Material => "material".to_string(),
        PaintSemantic::FuzzySkin => "fuzzy_skin".to_string(),
        PaintSemantic::SupportEnforcer => "support_enforcer".to_string(),
        PaintSemantic::SupportBlocker => "support_blocker".to_string(),
        PaintSemantic::Custom(name) => name.clone(),
    }
}

/// Validate per-object `support_layer_height_mm` settings against each
/// object's effective layer height.
///
/// Rule: `support_layer_height_mm` of `0.0` means "use the object's
/// effective layer height" (the historical default). When support rows must
/// stay grid-exact, any non-zero value must be **at least** the object's
/// effective layer height; the printer cannot extrude a support layer
/// thinner than the nominal model layer.
///
/// "Effective layer height" is taken to be each per-object resolved
/// config's `layer_height` field. Variable-layer-height plans may refine
/// this at runtime (per-region `effective_layer_height` on `ActiveRegion`),
/// but the input-domain gate here uses the configured per-object value.
///
/// Packet 239c: when the module key `independent_support_layer_height` is
/// true, support rows leave the object grid (canonical `Slicing.cpp` snaps
/// the support gaps to multiples of the object `layer_height` **only when
/// the flag is FALSE**, and `bottom_contact_layer` derives free-floating
/// `print_z` from the support pitch when it is TRUE), so a support pitch
/// finer than the object layer height is legal and the too-fine rejection
/// is skipped. The key is declared on both `*-support-planner` manifests
/// with `default = true` (canonical `PrintConfig.cpp` `init_fff_params`,
/// `coBool`, default true), so a resolved config carrying module schema
/// defaults holds it in `extensions`. A bare `ResolvedConfig` without
/// module-manifest defaults keeps the historical grid-exact invariant —
/// absent means the declaration surface was never consulted.
///
/// Returns `Ok(())` when every object's setting is compatible, or the
/// first offending [`ConfigResolutionError::SupportLayerHeightTooFine`]
/// otherwise.
pub fn validate_support_layer_heights(
    per_object_configs: &BTreeMap<String, ResolvedConfig>,
) -> Result<(), ConfigResolutionError> {
    for (object_id, cfg) in per_object_configs {
        let support_h = cfg.support_layer_height_mm;
        // `cfg.layer_height` is `f64` (parity with OrcaSlicer's `coordf_t`
        // layer-Z computation); cast to `f32` for the support-thinness check,
        // which compares display values, not Z-formula inputs.
        let effective_h = cfg.layer_height as f32;
        let independent = match cfg.extensions.get("independent_support_layer_height") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };
        if !independent && support_h > 0.0 && support_h < effective_h {
            return Err(ConfigResolutionError::SupportLayerHeightTooFine {
                object_id: object_id.clone(),
                support_layer_height_mm: support_h,
                effective_layer_height_mm: effective_h,
            });
        }
    }
    Ok(())
}
