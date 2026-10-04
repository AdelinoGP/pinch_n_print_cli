// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Slicing.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Deterministic resolution of typed configuration scope deltas.

use std::collections::BTreeMap;
use std::fmt;

use slicer_ir::slice_ir::cli_bool_spelling;
use slicer_ir::{ConfigResolutionError, ConfigValue, ResolvedConfig};

use crate::{
    canonical_config_key, expand_automatic_values, scope_denial_label, ConfigSchemaRegistry,
    ConfigScope, ExpansionContext, ExpansionError, RegistryEntry, ScopeDelta, ScopedConfig,
    CONFIG_KEY_ALIASES,
};

/// The object-level planning values needed to construct the shared Z grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedObjectLayerConfig {
    /// Stable object identifier.
    pub object_id: String,
    /// Object height in millimetres.
    pub object_height: f64,
    /// Effective ordinary layer height in millimetres.
    pub layer_height: f64,
    /// Effective first-layer height in millimetres.
    pub first_layer_height: f64,
    /// Number of support raft layers preceding model layers.
    pub support_raft_layers: u32,
    /// Explicit object-local model-layer top Zs, in millimetres.
    ///
    /// Empty means the uniform height formula applies; a non-empty list is the
    /// exact schedule produced by [`layer_top_zs`] over the object's composed
    /// layer-height profile.
    pub layer_z_tops: Vec<f64>,
}

/// Applicable typed scopes for one resolution query.
///
/// `Eq` is intentionally absent: [`Self::layer_top_z`] is an `f64`, which
/// implements only `PartialEq`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolutionTarget {
    /// Object whose object and modifier deltas apply.
    pub object_id: String,
    /// Applicable modifier identifiers in ascending priority order.
    pub modifier_ids: Vec<String>,
    /// Selected paint semantics. Resolution applies them in lexical order.
    pub paint_semantics: Vec<String>,
    /// Selected tool, when tool-specific configuration applies.
    pub tool_index: Option<u32>,
    /// Layer top print Z in millimetres, when resolution has a layer context.
    ///
    /// A layer-range delta applies only when this is `Some(z)` and the range's
    /// half-open world-Z interval `[min_z, max_z)` covers `z`. `None` means no
    /// layer context, so no layer-range delta ever applies.
    pub layer_top_z: Option<f64>,
}

/// One contiguous interval of an object's composed layer-height profile.
///
/// The segment covers `[z_start, z_end)` and supplies the layer height used
/// while the current layer top Z lies inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightProfileSegment {
    /// Inclusive lower world-Z bound in millimetres.
    pub z_start: f64,
    /// Exclusive upper world-Z bound in millimetres.
    pub z_end: f64,
    /// Layer height in millimetres applied inside this interval.
    pub height: f64,
}

/// Failure while resolving a typed configuration scope stack.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolutionError {
    /// A delta could not be applied to its typed `ResolvedConfig` field.
    Application(ConfigResolutionError),
    /// Phase-B automatic-value expansion failed.
    Expansion(ExpansionError),
    /// An object height was absent, non-finite, zero, or negative.
    InvalidObjectHeight {
        /// Object carrying the invalid height.
        object_id: String,
        /// Invalid finite value, or `None` when the value was non-finite.
        value: Option<f64>,
    },
    /// An authored key is not statable at the scope carrying its value.
    ScopeDenied {
        /// Registry key denied at `scope`.
        key: String,
        /// Scope whose delta stated the denied key.
        scope: ConfigScope,
    },
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application(error) => error.fmt(formatter),
            Self::Expansion(error) => error.fmt(formatter),
            Self::InvalidObjectHeight { object_id, value } => match value {
                Some(value) => write!(
                    formatter,
                    "object {object_id:?} height must be positive and finite, got {value}"
                ),
                None => write!(
                    formatter,
                    "object {object_id:?} height must be positive and finite"
                ),
            },
            Self::ScopeDenied { key, scope } => {
                write!(
                    formatter,
                    "config key {key:?} is denied at {} scope",
                    scope_denial_label(scope)
                )
            }
        }
    }
}

impl std::error::Error for ResolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Application(error) => Some(error),
            Self::Expansion(error) => Some(error),
            Self::InvalidObjectHeight { .. } => None,
            Self::ScopeDenied { .. } => None,
        }
    }
}

impl From<ConfigResolutionError> for ResolutionError {
    fn from(error: ConfigResolutionError) -> Self {
        Self::Application(error)
    }
}

impl From<ExpansionError> for ResolutionError {
    fn from(error: ExpansionError) -> Self {
        Self::Expansion(error)
    }
}

fn apply_delta(
    registry: &ConfigSchemaRegistry,
    config: &mut ResolvedConfig,
    scope: &ConfigScope,
    delta: Option<&ScopeDelta>,
) -> Result<(), ResolutionError> {
    let Some(delta) = delta else {
        return Ok(());
    };

    // Ingestion deliberately retains authored spellings. Reject ambiguity
    // within a scope before canonicalization; different scopes still override.
    for (legacy, canonical) in CONFIG_KEY_ALIASES {
        if delta.values.contains_key(legacy) && delta.values.contains_key(canonical) {
            return Err(ConfigResolutionError::TypeMismatch {
                key: format!("{canonical} and {legacy}"),
                expected: "one config key",
                actual: "both config keys supplied".to_owned(),
            }
            .into());
        }
    }

    // Eligibility gate: every authored key must be statable at the scope
    // carrying it. Scope instances share their family's policy, so `scope`
    // itself is the authority rather than a caller-supplied roster. The check
    // runs over the whole delta before the first mutation, so a denied key can
    // never leave a partially applied scope behind.
    let admission = registry.admission_set(scope);
    for (key, _) in delta.iter() {
        let key = canonical_config_key(key);
        if !admission.contains(key) {
            return Err(ResolutionError::ScopeDenied {
                key: key.to_owned(),
                scope: scope.clone(),
            });
        }
    }

    for (key, value) in delta.iter() {
        let key = canonical_config_key(key);
        // Typed fields and extensions share the registry's numerical contract.
        // Validate before extraction, including before any unsigned conversion.
        validate_extension(registry, key, value)?;
        if !config.apply_cli_key(key, value)? {
            config.extensions.insert(key.to_owned(), value.clone());
        }
    }
    Ok(())
}

/// Seed every seed-set registry default into the lowest-precedence layer.
///
/// A key is seeded only when all four hold:
///
/// - its key is exact (`prefix:*` wildcards are never materialized);
/// - it has a registry default;
/// - it is not a `selector` (a selector value travels as a selector value, not
///   as a delta);
/// - its canonical identity is not a `declare_resolved_config!` field. Typed
///   fields carry their own defaults through `ResolvedConfig::default()`, and a
///   seeded extension would shadow the typed value because `to_config_map`
///   merges `extensions` last.
///   `apply_cli_key` cannot test this because `plain` rows also return
///   `Ok(false)`; membership in `ResolvedConfig::typed_field_keys()` is the
///   authority.
///
/// Seeding iterates the registry's `BTreeMap` order and runs before Phase-B
/// expansion, so a seeded percent default expands exactly like an authored
/// value.
fn seed_registry_defaults(
    registry: &ConfigSchemaRegistry,
    config: &mut ResolvedConfig,
) -> Result<(), ResolutionError> {
    for key in registry.keys() {
        if key.contains('*') {
            continue;
        }
        let Some(entry) = registry.entry(key) else {
            continue;
        };
        if entry.selector {
            continue;
        }
        // Alias declarations must not seed a second extension identity that
        // diverges from the typed canonical field after scoped resolution.
        if ResolvedConfig::typed_field_keys().contains(&canonical_config_key(key)) {
            continue;
        }
        let Some(default) = entry.default.as_ref() else {
            continue;
        };
        let value = parse_registry_default(entry, default).map_err(|()| {
            // A declaration the registry cannot render is a manifest defect;
            // surfacing it keeps registry assembly loud instead of silently
            // dropping the key from every resolved config.
            ResolutionError::Application(ConfigResolutionError::TypeMismatch {
                key: key.to_owned(),
                expected: render_type_name(&entry.field_type),
                actual: format!("unrenderable default {default:?}"),
            })
        })?;
        validate_extension(registry, key, &value)?;
        // The seed enters below every authored scope: `or_insert` keeps an
        // authored value if one already claimed the slot.
        config.extensions.entry(key.to_owned()).or_insert(value);
    }
    Ok(())
}

/// Render a registry `default` wire string into the `ConfigValue` its declared
/// type requires.
fn parse_registry_default(entry: &RegistryEntry, default: &str) -> Result<ConfigValue, ()> {
    let trimmed = default.trim();
    match entry.field_type.as_str() {
        "bool" => match trimmed {
            "true" => Ok(ConfigValue::Bool(true)),
            "false" => Ok(ConfigValue::Bool(false)),
            _ => Err(()),
        },
        "int" => trimmed.parse::<i64>().map(ConfigValue::Int).map_err(|_| ()),
        "float" => trimmed
            .parse::<f64>()
            .map(ConfigValue::Float)
            .map_err(|_| ()),
        "string" | "enum" => Ok(ConfigValue::String(default.to_owned())),
        "percent" => {
            let magnitude = trimmed.strip_suffix('%').unwrap_or(trimmed);
            magnitude
                .trim()
                .parse::<f64>()
                .map(ConfigValue::Percent)
                .map_err(|_| ())
        }
        "float_or_percent" => {
            if let Some(magnitude) = trimmed.strip_suffix('%') {
                magnitude
                    .trim()
                    .parse::<f64>()
                    .map(|value| ConfigValue::FloatOrPercent {
                        value,
                        is_percent: true,
                    })
                    .map_err(|_| ())
            } else {
                trimmed
                    .parse::<f64>()
                    .map(|value| ConfigValue::FloatOrPercent {
                        value,
                        is_percent: false,
                    })
                    .map_err(|_| ())
            }
        }
        "int-list" => default
            .split(',')
            .map(|item| item.trim().parse::<i64>().map(ConfigValue::Int))
            .collect::<Result<Vec<_>, _>>()
            .map(ConfigValue::List)
            .map_err(|_| ()),
        "float-list" => default
            .split(',')
            .map(|item| item.trim().parse::<f64>().map(ConfigValue::Float))
            .collect::<Result<Vec<_>, _>>()
            .map(ConfigValue::List)
            .map_err(|_| ()),
        "string-list" => Ok(ConfigValue::List(if default.is_empty() {
            Vec::new()
        } else {
            default
                .split(',')
                .map(|item| ConfigValue::String(item.to_owned()))
                .collect()
        })),
        _ => Err(()),
    }
}

fn render_type_name(field_type: &str) -> &'static str {
    match field_type {
        "bool" => "Bool",
        "int" => "Int",
        "float" => "Float",
        "string" | "enum" => "String",
        "percent" => "Percent",
        "float_or_percent" => "FloatOrPercent",
        "int-list" => "List<Int>",
        "float-list" => "List<Float>",
        "string-list" => "List<String>",
        _ => "registry-declared type",
    }
}

/// Validate one authored or seeded value against its registry declaration.
///
/// An undeclared key is retained untyped (the registry cannot type it); a
/// declared key must match the declared wire type and, for numeric types, its
/// declared `[min, max]` bounds. Percent-magnitude values check only the `min`
/// side, mirroring `slicer-scheduler`'s percent bounds rule: an absolute `max`
/// is expressed in the key's unit while a raw percent magnitude is only
/// meaningful once Phase B supplies its base.
fn validate_extension(
    registry: &ConfigSchemaRegistry,
    key: &str,
    value: &ConfigValue,
) -> Result<(), ResolutionError> {
    // Registry wildcard entries (`prefix:*`) govern concrete `prefix:<instance>`
    // keys; the wildcard's declaration types the instance value.
    let entry = registry.entry(key).or_else(|| {
        let (base, _) = key.rsplit_once(':')?;
        registry.entry(&format!("{base}:*"))
    });
    let Some(entry) = entry else {
        return Ok(());
    };

    // Per-filament envelope (canonical `coFloats`/`coInts`/`coStrings` wire):
    // a scalar-typed key may arrive as a `List` whose first element carries the
    // value — `extract_float_or_first`'s documented shape leniency. An empty
    // list stays a hard error (it falls through to the type arms below), and
    // list-typed declarations keep their `List` shape.
    let value = match value {
        ConfigValue::List(items) if !entry.field_type.ends_with("-list") => {
            items.first().map_or(value, |first| first)
        }
        other => other,
    };

    match entry.field_type.as_str() {
        "bool" => match value {
            ConfigValue::Bool(_) => Ok(()),
            // `extract_bool` accepts Int 0/1 as boolean; mirror that here.
            ConfigValue::Int(0 | 1) => Ok(()),
            // Canonical CLI bool spellings (canonical `normalize_cli_bool_value`
            // in `DynamicConfig::read_cli`): strict deserialization rejects
            // these at ingestion, which retains them untyped with a warning;
            // consumers apply this documented leniency.
            ConfigValue::String(text) if cli_bool_spelling(text).is_some() => Ok(()),
            other => Err(type_mismatch(key, "Bool", other)),
        },
        "int" => match value {
            ConfigValue::Int(number) => check_extension_scalar(key, *number as f64, entry, None),
            ConfigValue::Float(number) => {
                if number.is_finite() && number.fract() == 0.0 {
                    check_extension_scalar(key, *number, entry, None)
                } else {
                    Err(type_mismatch(key, "Int", value))
                }
            }
            other => Err(type_mismatch(key, "Int", other)),
        },
        "float" => match value {
            ConfigValue::Float(number) => check_extension_scalar(key, *number, entry, None),
            ConfigValue::Int(number) => check_extension_scalar(key, *number as f64, entry, None),
            // Canonical wire spellings: the nullable "nil" sentinel (canonical
            // `ConfigOptionFloatsNullable`) marks an absent value and needs no
            // bounds check; a percent-form magnitude (canonical
            // `ConfigOptionFloat::deserialize` parses the numeric prefix)
            // checks the `min` side only, mirroring `Percent` values below.
            ConfigValue::String(text) => {
                let trimmed = text.trim();
                if trimmed.eq_ignore_ascii_case("nil") {
                    Ok(())
                } else if let Some(percent) = trimmed.strip_suffix('%') {
                    match percent.trim().parse::<f64>() {
                        Ok(number) if number.is_finite() => {
                            check_extension_percent(key, number, entry)
                        }
                        _ => Err(type_mismatch(key, "Float", value)),
                    }
                } else {
                    match trimmed.parse::<f64>() {
                        Ok(number) if number.is_finite() => {
                            check_extension_scalar(key, number, entry, None)
                        }
                        _ => Err(type_mismatch(key, "Float", value)),
                    }
                }
            }
            other => Err(type_mismatch(key, "Float", other)),
        },
        "string" | "enum" => match value {
            ConfigValue::String(_) => Ok(()),
            other => Err(type_mismatch(key, "String", other)),
        },
        "percent" => match value {
            ConfigValue::Percent(number) => check_extension_percent(key, *number, entry),
            other => Err(type_mismatch(key, "Percent", other)),
        },
        "float_or_percent" => match value {
            ConfigValue::FloatOrPercent {
                value: number,
                is_percent,
            } => {
                if *is_percent {
                    check_extension_percent(key, *number, entry)
                } else {
                    check_extension_scalar(key, *number, entry, None)
                }
            }
            ConfigValue::Float(number) => check_extension_scalar(key, *number, entry, None),
            ConfigValue::Int(number) => check_extension_scalar(key, *number as f64, entry, None),
            ConfigValue::Percent(number) => check_extension_percent(key, *number, entry),
            other => Err(type_mismatch(key, "FloatOrPercent", other)),
        },
        "int-list" => match value {
            ConfigValue::List(items) => {
                for (index, item) in items.iter().enumerate() {
                    match item {
                        ConfigValue::Int(number) => {
                            check_extension_scalar(key, *number as f64, entry, Some(index))?
                        }
                        other => return Err(type_mismatch(key, "Int", other)),
                    }
                }
                Ok(())
            }
            other => Err(type_mismatch(key, "List", other)),
        },
        "float-list" => match value {
            ConfigValue::List(items) => {
                for (index, item) in items.iter().enumerate() {
                    match item {
                        ConfigValue::Float(number) => {
                            check_extension_scalar(key, *number, entry, Some(index))?
                        }
                        ConfigValue::Int(number) => {
                            check_extension_scalar(key, *number as f64, entry, Some(index))?
                        }
                        other => return Err(type_mismatch(key, "Float", other)),
                    }
                }
                Ok(())
            }
            // A bare scalar is accepted as a one-element list, matching the
            // extractors' scalar tolerance; the element index is 0.
            ConfigValue::Float(number) => check_extension_scalar(key, *number, entry, Some(0)),
            ConfigValue::Int(number) => check_extension_scalar(key, *number as f64, entry, Some(0)),
            other => Err(type_mismatch(key, "List", other)),
        },
        "string-list" => match value {
            ConfigValue::List(items) => {
                for item in items {
                    if !matches!(item, ConfigValue::String(_)) {
                        return Err(type_mismatch(key, "String", item));
                    }
                }
                Ok(())
            }
            ConfigValue::String(_) => Ok(()),
            other => Err(type_mismatch(key, "List", other)),
        },
        _ => Ok(()),
    }
}

fn check_extension_scalar(
    key: &str,
    value: f64,
    entry: &RegistryEntry,
    index: Option<usize>,
) -> Result<(), ResolutionError> {
    let in_range = value.is_finite()
        && entry.min.is_none_or(|min| value >= min)
        && entry.max.is_none_or(|max| value <= max);
    if in_range {
        Ok(())
    } else {
        Err(ResolutionError::Application(
            ConfigResolutionError::OutOfRange {
                key: key.to_owned(),
                value,
                min: entry.min,
                max: entry.max,
                index,
            },
        ))
    }
}

/// Percent-magnitude bounds check: only the `min` side is meaningful before
/// Phase B supplies the base.
fn check_extension_percent(
    key: &str,
    value: f64,
    entry: &RegistryEntry,
) -> Result<(), ResolutionError> {
    if value.is_finite() && entry.min.is_none_or(|min| value >= min) {
        Ok(())
    } else {
        Err(ResolutionError::Application(
            ConfigResolutionError::OutOfRange {
                key: key.to_owned(),
                value,
                min: entry.min,
                max: entry.max,
                index: None,
            },
        ))
    }
}

fn type_mismatch(key: &str, expected: &'static str, value: &ConfigValue) -> ResolutionError {
    ResolutionError::Application(ConfigResolutionError::TypeMismatch {
        key: key.to_owned(),
        expected,
        actual: config_value_name(value).to_owned(),
    })
}

/// Resolve all applicable deltas in canonical precedence order, then run Phase B.
///
/// Precedence is global, object, the layer-range deltas covering
/// [`ResolutionTarget::layer_top_z`], modifiers in target order, paint semantics
/// in lexical order, and finally the selected tool. A delta's presence,
/// including a value equal to the default, is always an override.
///
/// Every delta is validated against the registry's admission set for its scope
/// before merge or expansion. A scope whose delta states a denied key is
/// rejected whole: `ResolvedConfig::default()` is local to this call, so no
/// partially resolved config can escape.
pub fn resolve_scope_stack(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    target: &ResolutionTarget,
    expansion: &ExpansionContext,
) -> Result<ResolvedConfig, ResolutionError> {
    let mut config = ResolvedConfig::default();

    // Global is the only scope that is not statable per region, so it is not a
    // member of the `denied_scopes` vocabulary and admits every key.
    apply_delta(registry, &mut config, &ConfigScope::Global, scoped.global())?;
    apply_delta(
        registry,
        &mut config,
        &ConfigScope::Object(target.object_id.clone()),
        scoped.delta(&ConfigScope::Object(target.object_id.clone())),
    )?;

    // Layer-range deltas sit between object and modifier. The stored order is
    // already `(min_z, max_z, source_index)` ascending, which is also
    // `range_index` order, so a plain forward scan is canonical.
    // `layer_top_z: None` means no layer context: no range ever applies.
    // Membership is `LayerConfigRange::covers`, the single authority the
    // runtime kernel shares, so the two cannot disagree.
    if let Some(layer_top_z) = target.layer_top_z {
        for range in scoped.ranges_for(&target.object_id) {
            if range.covers(layer_top_z) {
                apply_delta(registry, &mut config, &range.scope, Some(&range.delta))?;
            }
        }
    }

    for modifier_id in &target.modifier_ids {
        let scope = ConfigScope::Modifier {
            object_id: target.object_id.clone(),
            modifier_id: modifier_id.clone(),
        };
        apply_delta(registry, &mut config, &scope, scoped.delta(&scope))?;
    }

    let mut paint_semantics = target.paint_semantics.clone();
    paint_semantics.sort();
    paint_semantics.dedup();
    for semantic in paint_semantics {
        let scope = ConfigScope::PaintSemantic(semantic);
        apply_delta(registry, &mut config, &scope, scoped.delta(&scope))?;
    }

    if let Some(tool_index) = target.tool_index {
        let scope = ConfigScope::Tool(tool_index);
        apply_delta(registry, &mut config, &scope, scoped.delta(&scope))?;
    }

    seed_registry_defaults(registry, &mut config)?;

    expand_automatic_values(registry, &mut config, expansion, target.tool_index)?;
    Ok(config)
}

/// Return the zero-based range index carried by a layer-range scope.
fn layer_range_index(scope: &ConfigScope) -> u32 {
    match scope {
        ConfigScope::LayerRange { range_index, .. } => *range_index,
        _ => 0,
    }
}

/// Validate one object-level planning height before it feeds a profile.
///
/// The variant is named for the object height because that is its first and
/// most common trigger; every height read here (object height, resolved
/// `layer_height`, resolved `first_layer_height`) obeys the same
/// finite-and-positive contract.
fn validate_height(object_id: &str, height: f64) -> Result<(), ResolutionError> {
    if height.is_finite() && height > 0.0 {
        return Ok(());
    }
    Err(ResolutionError::InvalidObjectHeight {
        object_id: object_id.to_owned(),
        value: height.is_finite().then_some(height),
    })
}

/// Compose one object's contiguous layer-height profile over `[0, object_height)`.
///
/// The composed profile follows canonical `layer_height_profile_from_ranges`
/// (`Slicing.cpp`): the fixed first-layer interval `[0, first_layer_height)` is
/// retained first; ranges then iterate in ascending `(min_z, max_z,
/// range_index)` order, so an earlier-starting range keeps an overlap and trims
/// a later range's low edge to the last retained high. Uncovered gaps — and the
/// tail up to `object_height` — use the resolved base `layer_height`. Adjacent
/// equal-height segments are coalesced, so the result is the minimal exact
/// cover of the profile.
///
/// Only `layer_height` values are read; other range keys are precedence inputs
/// for [`resolve_scope_stack`], not profile geometry.
///
/// # Errors
///
/// Returns [`ResolutionError::InvalidObjectHeight`] when `object_height` is
/// absent, non-finite, zero, or negative, and likewise when the object's
/// resolved `layer_height` or `first_layer_height` is non-finite or
/// non-positive. Resolution failures of the base scope stack propagate
/// unchanged.
pub fn query_layer_height_profile(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    object_id: &str,
    object_height: f64,
    expansion: &ExpansionContext,
) -> Result<Vec<HeightProfileSegment>, ResolutionError> {
    validate_height(object_id, object_height)?;

    // Resolve the object's base scalars through the production scope stack so
    // the profile can never disagree with `ResolvedObjectLayerConfig`.
    let config = resolve_scope_stack(
        registry,
        scoped,
        &ResolutionTarget {
            object_id: object_id.to_owned(),
            ..ResolutionTarget::default()
        },
        expansion,
    )?;
    validate_height(object_id, config.layer_height)?;
    validate_height(object_id, config.first_layer_height)?;

    // Retained coverage: the synthetic first-layer interval first, then each
    // range's surviving portion. `coverage_high` is the end of the last
    // retained interval.
    let mut retained = vec![HeightProfileSegment {
        z_start: 0.0,
        z_end: config.first_layer_height,
        height: config.first_layer_height,
    }];
    let mut coverage_high = config.first_layer_height;

    let mut ranges: Vec<(f64, f64, u32, f64)> = scoped
        .ranges_for(&object_id.to_owned())
        .iter()
        .filter_map(|range| match range.delta.values.get("layer_height") {
            Some(ConfigValue::Float(height)) if height.is_finite() && *height > 0.0 => Some((
                range.min_z,
                range.max_z,
                layer_range_index(&range.scope),
                *height,
            )),
            _ => None,
        })
        .collect();
    ranges.sort_by(|first, second| {
        first
            .0
            .total_cmp(&second.0)
            .then_with(|| first.1.total_cmp(&second.1))
            .then_with(|| first.2.cmp(&second.2))
    });

    for (min_z, max_z, _, height) in ranges {
        // A range fully inside retained coverage contributes nothing; this is
        // also what stops a range at `0.0` from duplicating the synthetic
        // first-layer interval.
        if max_z <= coverage_high {
            continue;
        }
        // The low edge is trimmed to the last retained high, so an
        // earlier-starting range keeps the overlap.
        retained.push(HeightProfileSegment {
            z_start: min_z.max(coverage_high),
            z_end: max_z,
            height,
        });
        coverage_high = max_z;
    }

    Ok(fill_profile_gaps(
        retained,
        coverage_high,
        object_height,
        config.layer_height,
    ))
}

/// Fill profile gaps with the base layer height and coalesce equal neighbours.
///
/// The retained segments are strictly ascending and non-overlapping; the
/// result covers `[0, max(object_height, coverage_high))` because a range may
/// extend past the object height and its retained interval is kept whole.
fn fill_profile_gaps(
    retained: Vec<HeightProfileSegment>,
    coverage_high: f64,
    object_height: f64,
    base_height: f64,
) -> Vec<HeightProfileSegment> {
    let target = object_height.max(coverage_high);
    let mut profile: Vec<HeightProfileSegment> = Vec::with_capacity(retained.len() + 1);
    let mut cursor = 0.0;

    for segment in retained {
        if segment.z_start > cursor {
            push_profile_segment(&mut profile, cursor, segment.z_start, base_height);
        }
        push_profile_segment(&mut profile, segment.z_start, segment.z_end, segment.height);
        cursor = segment.z_end;
    }
    if cursor < target {
        push_profile_segment(&mut profile, cursor, target, base_height);
    }
    profile
}

fn push_profile_segment(
    profile: &mut Vec<HeightProfileSegment>,
    z_start: f64,
    z_end: f64,
    height: f64,
) {
    if z_end <= z_start {
        return;
    }
    if let Some(last) = profile.last_mut() {
        if last.height == height && last.z_end == z_start {
            last.z_end = z_end;
            return;
        }
    }
    profile.push(HeightProfileSegment {
        z_start,
        z_end,
        height,
    });
}

/// Evaluate an object-local layer-top schedule from a composed profile.
///
/// The walk starts at Z=0 and repeatedly adds the height of the segment
/// containing a probe just **above** the current top (`z + 1e-9`), pushing each
/// new top while it is at most `object_height` plus the same tolerance.
///
/// Probing above the top (rather than testing the top itself against
/// `z_start <= z < z_end`) is what makes the walk cross a range boundary
/// exactly once. Canonical `generate_object_layers` (`Slicing.cpp`) selects a
/// layer's height from a probe inside the layer for the same reason: `0.7 + 0.1`
/// accumulates to `0.7999999999999999`, one ULP below the `[0.4, 0.8)` end, so a
/// half-open test on the top re-selects the exhausted 0.1 mm segment and the
/// object's final layer is silently dropped. Probing past the boundary advances
/// to the next segment, whose height carries the schedule to the object top.
///
/// A probe outside every segment stops the walk defensively. The returned Zs
/// are object-local: no raft offset is applied here.
#[must_use]
pub fn layer_top_zs(segments: &[HeightProfileSegment], object_height: f64) -> Vec<f64> {
    const TOP_TOLERANCE: f64 = 1.0e-9;

    let mut tops = Vec::new();
    let mut z = 0.0;

    while let Some(segment) = segments.iter().find(|segment| {
        let probe = z + TOP_TOLERANCE;
        segment.z_start <= probe && probe < segment.z_end
    }) {
        // Profile construction never emits a non-positive height; a hand-built
        // list must not be able to hang the caller. `NaN <= 0.0` is false, so
        // the `is_nan` arm is required for a non-terminating guard.
        if segment.height.is_nan() || segment.height <= 0.0 {
            break;
        }
        z += segment.height;
        if z <= object_height + 1.0e-9 {
            tops.push(z);
        } else {
            break;
        }
    }

    tops
}

/// Resolve object planning inputs in object-id order for shared Z-grid construction.
///
/// All object heights are validated before any scope stack is resolved, so an
/// invalid height rejects the query atomically. Each record carries the
/// explicit object-local layer-top schedule derived from the object's composed
/// layer-height profile alongside the base scalars.
pub fn query_z_grid(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    object_heights: &BTreeMap<String, f64>,
    expansion: &ExpansionContext,
) -> Result<Vec<ResolvedObjectLayerConfig>, ResolutionError> {
    for (object_id, object_height) in object_heights {
        if !object_height.is_finite() || *object_height <= 0.0 {
            return Err(ResolutionError::InvalidObjectHeight {
                object_id: object_id.clone(),
                value: object_height.is_finite().then_some(*object_height),
            });
        }
    }

    object_heights
        .iter()
        .map(|(object_id, object_height)| {
            let config = resolve_scope_stack(
                registry,
                scoped,
                &ResolutionTarget {
                    object_id: object_id.clone(),
                    ..ResolutionTarget::default()
                },
                expansion,
            )?;
            let support_raft_layers = support_raft_layers(&config)?;
            let profile =
                query_layer_height_profile(registry, scoped, object_id, *object_height, expansion)?;

            Ok(ResolvedObjectLayerConfig {
                object_id: object_id.clone(),
                object_height: *object_height,
                layer_height: config.layer_height,
                first_layer_height: config.first_layer_height,
                support_raft_layers,
                layer_z_tops: layer_top_zs(&profile, *object_height),
            })
        })
        .collect()
}

fn support_raft_layers(config: &ResolvedConfig) -> Result<u32, ResolutionError> {
    let Some(value) = config.extensions.get("support_raft_layers") else {
        return Ok(0);
    };

    match value {
        ConfigValue::Int(value) => u32::try_from(*value).map_err(|_| {
            ConfigResolutionError::OutOfRange {
                key: "support_raft_layers".to_owned(),
                value: *value as f64,
                min: Some(0.0),
                max: Some(f64::from(u32::MAX)),
                index: None,
            }
            .into()
        }),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: "support_raft_layers".to_owned(),
            expected: "Int",
            actual: config_value_name(other).to_owned(),
        }
        .into()),
    }
}

fn config_value_name(value: &ConfigValue) -> &'static str {
    match value {
        ConfigValue::Bool(_) => "Bool",
        ConfigValue::Int(_) => "Int",
        ConfigValue::Float(_) => "Float",
        ConfigValue::String(_) => "String",
        ConfigValue::Percent(_) => "Percent",
        ConfigValue::FloatOrPercent { .. } => "FloatOrPercent",
        ConfigValue::List(_) => "List",
    }
}
