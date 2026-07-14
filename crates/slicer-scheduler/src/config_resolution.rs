//! Host-side resolver that turns user-supplied CLI config into per-object
//! [`slicer_ir::ResolvedConfig`] values. Invoked from the `Run` command and
//! the live execution-plan path; the resulting configs drive `RegionPlan.config`
//! during pipeline execution.

use std::collections::{BTreeMap, HashMap};

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
#[derive(Debug, Clone, Default)]
pub struct ConfigBoundsIndex {
    bounds: HashMap<String, NumericBounds>,
}

impl ConfigBoundsIndex {
    /// An empty index â€” no bounds enforced. Useful for tests and call sites
    /// that have no loaded modules in scope.
    pub fn empty() -> Self {
        Self {
            bounds: HashMap::new(),
        }
    }

    /// Build the index by walking every loaded module's config schema.
    ///
    /// Only entries whose `field_type` is numeric (`int`, `float`,
    /// `float-list`, `int-list`) and that carry at least one of `min`/`max`
    /// contribute to the index. On collision across modules, ranges are
    /// intersected.
    pub fn from_modules<'a, I>(modules: I) -> Self
    where
        I: IntoIterator<Item = &'a LoadedModule>,
    {
        let declarations = modules.into_iter().flat_map(|module| {
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
        Self::from_declarations(declarations)
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

        Self { bounds: index }
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
        let Some(bounds) = self.bounds.get(key) else {
            return Ok(());
        };
        check_value(key, value, bounds, None)
    }
}

fn is_numeric_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "int" | "float" | "float-list" | "int-list" | "percent" | "float_or_percent"
    )
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
        // `is_numeric_field_type` above, so a module-declared `[min, max]`
        // for a `percent` / `float_or_percent` key is enforced here against
        // the raw percent number / literal value. The percent→absolute base
        // is per-call-site and unknown at this point, so bounds cannot be
        // applied to the resolved absolute value here — only to the raw
        // number as declared in config.
        ConfigValue::Percent(p) => check_scalar(key, *p, bounds, index),
        ConfigValue::FloatOrPercent { value, .. } => check_scalar(key, *value, bounds, index),
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

/// Resolve `paint_config:<semantic>:<key>` overlays into per-semantic configs.
///
/// Mirrors [`resolve_per_object_configs`]: starts each per-semantic config from
/// `global` and applies the overlay from keys matching the
/// `paint_config:<namespace_key>:` prefix.
///
/// Returns `(map, warnings)`:
/// - `map` keyed by [`PaintSemantic`] from `present_semantics` that had at
///   least one matching override key.
/// - `warnings` for `paint_config:NAME:<key>` entries whose NAME is not in
///   `present_semantics`. The call does NOT fail; the caller forwards these.
pub fn resolve_per_paint_semantic_configs(
    global: &ResolvedConfig,
    source: &HashMap<ConfigKey, ConfigValue>,
    present_semantics: &[PaintSemantic],
    bounds: &ConfigBoundsIndex,
) -> Result<
    (
        BTreeMap<PaintSemantic, ResolvedConfig>,
        Vec<UnknownSemanticWarning>,
    ),
    ConfigResolutionError,
> {
    let mut result: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    let mut warnings: Vec<UnknownSemanticWarning> = Vec::new();

    // Collect all paint_config: keys from the source.
    const PREFIX: &str = "paint_config:";
    for (key, value) in source {
        if let Some(rest) = key.strip_prefix(PREFIX) {
            // rest is "<semantic>:<sub_key>"
            if let Some(colon_pos) = rest.find(':') {
                let semantic_name = &rest[..colon_pos];
                let sub_key = &rest[colon_pos + 1..];

                // Try to match semantic_name against a present semantic.
                let matched = present_semantics
                    .iter()
                    .find(|s| paint_semantic_namespace_key(s) == semantic_name);

                match matched {
                    Some(semantic) => {
                        // Clone semantic for map key use.
                        let sem_key = semantic.clone();
                        let entry = result
                            .entry(sem_key.clone())
                            .or_insert_with(|| global.clone());
                        // Apply this single override key to the entry.
                        let single: HashMap<String, ConfigValue> =
                            [(sub_key.to_string(), value.clone())].into();
                        let updated = apply_overlay(entry, &single, bounds)?;
                        *entry = updated;
                    }
                    None => {
                        warnings.push(UnknownSemanticWarning {
                            semantic_name: semantic_name.to_string(),
                            key: sub_key.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok((result, warnings))
}

/// Resolve a flat `HashMap<ConfigKey, ConfigValue>` (as produced by
/// [`parse_cli_config_source`]) into a global [`ResolvedConfig`].
///
/// Resolution rules
/// ----------------
/// * Keys matching declared `ResolvedConfig` fields are applied with strict
///   type checking.  A wrong variant returns
///   [`ConfigResolutionError::TypeMismatch`].
/// * Keys with the prefix `object_config:` are per-object overlays; they are
///   **not** applied here â€” see [`resolve_per_object_configs`].
/// * Keys with the prefix `object_height:` are pre-existing host-injected keys
///   consumed by other host code; they are silently skipped (not an error, not
///   routed to `extensions`).
/// * Any remaining key lands in `ResolvedConfig.extensions`.
///
/// Defaults come from [`ResolvedConfig::default()`].
pub fn resolve_global_config(
    source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<ResolvedConfig, ConfigResolutionError> {
    let mut cfg = ResolvedConfig::default();

    for (key, value) in source {
        // Skip per-object overlay keys â€” handled by resolve_per_object_configs.
        if key.starts_with("object_config:") {
            continue;
        }
        // Skip per-paint-semantic overlay keys â€” handled by resolve_per_paint_semantic_configs.
        if key.starts_with("paint_config:") {
            continue;
        }
        // Skip per-tool overlay keys — handled by resolve_per_tool_configs.
        if key.starts_with("tool_config:") {
            continue;
        }
        // Skip host-injected object_height keys.
        if key.starts_with("object_height:") {
            continue;
        }

        // Enforce numeric min/max declared in any module's manifest before
        // routing the value into a declared field or the extensions bucket.
        bounds.check(key.as_str(), value)?;

        // Dispatch into the macro-generated per-field setter. Unknown keys
        // fall through to the `extensions` overflow bucket. Single source of
        // truth lives in `slicer-ir::resolved_config`.
        if !cfg.apply_cli_key(key.as_str(), value)? {
            cfg.extensions.insert(key.clone(), value.clone());
        }
    }

    Ok(cfg)
}

/// Build per-object [`ResolvedConfig`] overlays starting from the global base.
///
/// For each `object_id` in `object_ids`:
/// 1. Clone the `global` config as the starting point.
/// 2. Apply any `object_config:<object_id>:<config_key>` entries from `source`.
///
/// The returned map is a [`BTreeMap`] (sorted by `object_id`) to ensure
/// deterministic ordering.
pub fn resolve_per_object_configs(
    global: &ResolvedConfig,
    source: &HashMap<ConfigKey, ConfigValue>,
    object_ids: &[&str],
    bounds: &ConfigBoundsIndex,
) -> Result<BTreeMap<String, ResolvedConfig>, ConfigResolutionError> {
    let mut result = BTreeMap::new();

    for &object_id in object_ids {
        // Build a per-object sub-map with only the overrides for this object.
        let prefix = format!("object_config:{object_id}:");
        let mut per_object_source: HashMap<String, ConfigValue> = HashMap::new();
        for (key, value) in source {
            if let Some(sub_key) = key.strip_prefix(&prefix) {
                per_object_source.insert(sub_key.to_string(), value.clone());
            }
        }

        // Start from the global config and apply overrides.
        let mut per_obj_cfg = global.clone();
        if !per_object_source.is_empty() {
            // Merge by running through resolve_global_config with the
            // per-object sub-map, then selectively apply non-default fields.
            // Simpler: rebuild from global + per_object_source overlay.
            per_obj_cfg = apply_overlay(global, &per_object_source, bounds)?;
        }

        result.insert(object_id.to_string(), per_obj_cfg);
    }

    Ok(result)
}

/// Build per-tool/extruder [`ResolvedConfig`] overlays starting from the global
/// base. For each `tool_config:<tool_index>:<config_key>` entry in `source`, the
/// value overrides the global base for that integer tool index.
///
/// This is a clean additive config axis enabled by the region_id↔tool split
/// (`PrintEntity.tool_index` is now a first-class selector). Precedence:
/// `global < per_object < per_paint_semantic < per_tool` — per-tool is the
/// highest-precedence override (mirroring OrcaSlicer's filament-preset overrides
/// applied last, `PrintApply.cpp`). This function builds only the `global +
/// per_tool` overlay; the per-object / per-paint overlays are composed at the
/// region-mapping site (`region_mapping.rs`), where the per-tool result is
/// applied last so a `tool_config:<idx>:<key>` wins over an object/paint value
/// on the same key.
///
/// The returned map is a [`BTreeMap`] (sorted by tool index) for deterministic
/// ordering. Entries with a non-numeric tool index are skipped.
pub fn resolve_per_tool_configs(
    global: &ResolvedConfig,
    source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<BTreeMap<u32, ResolvedConfig>, ConfigResolutionError> {
    const PREFIX: &str = "tool_config:";
    // Group override sub-keys by tool index: "tool_config:<idx>:<sub_key>".
    let mut per_tool_source: BTreeMap<u32, HashMap<String, ConfigValue>> = BTreeMap::new();
    for (key, value) in source {
        if let Some(rest) = key.strip_prefix(PREFIX) {
            if let Some(colon_pos) = rest.find(':') {
                let idx_str = &rest[..colon_pos];
                let sub_key = &rest[colon_pos + 1..];
                if let Ok(tool_index) = idx_str.parse::<u32>() {
                    per_tool_source
                        .entry(tool_index)
                        .or_default()
                        .insert(sub_key.to_string(), value.clone());
                }
            }
        }
    }

    let mut result = BTreeMap::new();
    for (tool_index, sub) in per_tool_source {
        result.insert(tool_index, apply_overlay(global, &sub, bounds)?);
    }
    Ok(result)
}

/// Validate per-object `support_layer_height_mm` settings against each
/// object's effective layer height.
///
/// Rule: `support_layer_height_mm` of `0.0` means "use the object's
/// effective layer height" (the historical default). Any non-zero value
/// must be **at least** the object's effective layer height; the printer
/// cannot extrude a support layer thinner than the nominal model layer.
///
/// "Effective layer height" is taken to be each per-object resolved
/// config's `layer_height` field. Variable-layer-height plans may refine
/// this at runtime (per-region `effective_layer_height` on `ActiveRegion`),
/// but the input-domain gate here uses the configured per-object value.
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
        if support_h > 0.0 && support_h < effective_h {
            return Err(ConfigResolutionError::SupportLayerHeightTooFine {
                object_id: object_id.clone(),
                support_layer_height_mm: support_h,
                effective_layer_height_mm: effective_h,
            });
        }
    }
    Ok(())
}

/// Apply a flat override map (already stripped of the `object_config:<id>:`
/// prefix) on top of a base [`ResolvedConfig`].
fn apply_overlay(
    base: &ResolvedConfig,
    overrides: &HashMap<String, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<ResolvedConfig, ConfigResolutionError> {
    // Merge: start from a merged source where declared-field defaults come
    // from base, then overrides win.
    // Strategy: serialise base back to a source map, then merge overrides,
    // then resolve. Alternatively, re-use resolve_global_config with a
    // combined source. We use the simpler approach: re-run
    // resolve_global_config seeded from base-as-source then override.
    //
    // Simplest correct approach: clone base, then patch each override key.
    let mut cfg = base.clone();

    for (key, value) in overrides {
        // object_config / object_height prefixes won't appear here (already
        // stripped), but skip them defensively.
        if key.starts_with("object_config:") || key.starts_with("object_height:") {
            continue;
        }

        bounds.check(key.as_str(), value)?;

        if !cfg.apply_cli_key(key.as_str(), value)? {
            cfg.extensions.insert(key.clone(), value.clone());
        }
    }

    Ok(cfg)
}
