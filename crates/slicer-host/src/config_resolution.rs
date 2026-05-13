//! Host-side resolver that turns user-supplied CLI config into per-object
//! [`slicer_ir::ResolvedConfig`] values.
//!
//! # Disambiguation
//!
//! - **`config_schema.rs`** describes per-module manifest field shapes
//!   (author-time, declarative). It drives [`bind_module_config_view`] and the
//!   `config-schema` CLI subcommand.
//!
//! - **`config_resolution.rs`** (this module) resolves user-supplied CLI input
//!   into per-object `slicer_ir::ResolvedConfig` (invocation-time, imperative).
//!   The resulting configs drive `RegionPlan.config` during pipeline execution.

use std::collections::{BTreeMap, HashMap};

use slicer_ir::{ConfigKey, ConfigValue, PaintSemantic, ResolvedConfig};

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
                        let updated = apply_overlay(entry, &single)?;
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

/// Errors produced during config resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigResolutionError {
    /// A declared `ResolvedConfig` field received a value of the wrong variant.
    TypeMismatch {
        /// The config key that had the wrong type.
        key: String,
        /// The variant name expected (e.g. `"Int"`, `"Float"`, `"Bool"`).
        expected: &'static str,
        /// The variant name that was actually supplied.
        actual: String,
    },
}

impl std::fmt::Display for ConfigResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch {
                key,
                expected,
                actual,
            } => write!(
                f,
                "config key '{key}': expected {expected} value, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ConfigResolutionError {}

/// Return a short variant-name string for a [`ConfigValue`] (used in error
/// messages).
fn variant_name(v: &ConfigValue) -> String {
    match v {
        ConfigValue::Bool(_) => "Bool".to_string(),
        ConfigValue::Int(_) => "Int".to_string(),
        ConfigValue::Float(_) => "Float".to_string(),
        ConfigValue::String(_) => "String".to_string(),
        ConfigValue::List(_) => "List".to_string(),
    }
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
///   **not** applied here — see [`resolve_per_object_configs`].
/// * Keys with the prefix `object_height:` are pre-existing host-injected keys
///   consumed by other host code; they are silently skipped (not an error, not
///   routed to `extensions`).
/// * Any remaining key lands in `ResolvedConfig.extensions`.
///
/// Defaults come from [`ResolvedConfig::default()`].
pub fn resolve_global_config(
    source: &HashMap<ConfigKey, ConfigValue>,
) -> Result<ResolvedConfig, ConfigResolutionError> {
    let mut cfg = ResolvedConfig::default();

    for (key, value) in source {
        // Skip per-object overlay keys — handled by resolve_per_object_configs.
        if key.starts_with("object_config:") {
            continue;
        }
        // Skip per-paint-semantic overlay keys — handled by resolve_per_paint_semantic_configs.
        if key.starts_with("paint_config:") {
            continue;
        }
        // Skip host-injected object_height keys.
        if key.starts_with("object_height:") {
            continue;
        }

        match key.as_str() {
            // --- Geometry ---
            "layer_height" => {
                cfg.layer_height = extract_float(key, value)?;
            }
            "line_width" => {
                cfg.line_width = extract_float(key, value)?;
            }
            "first_layer_height" => {
                cfg.first_layer_height = extract_float(key, value)?;
            }
            "first_layer_line_width" => {
                cfg.first_layer_line_width = extract_float(key, value)?;
            }

            // --- Walls ---
            "wall_count" => {
                cfg.wall_count = extract_int_as_u32(key, value)?;
            }
            "outer_wall_speed" => {
                cfg.outer_wall_speed = extract_float(key, value)?;
            }
            "inner_wall_speed" => {
                cfg.inner_wall_speed = extract_float(key, value)?;
            }
            "arachne_min_feature_size" => {
                cfg.arachne_min_feature_size = Some(extract_float(key, value)?);
            }

            // --- Infill ---
            "infill_density" => {
                cfg.infill_density = extract_float(key, value)?;
            }
            "infill_angle" => {
                cfg.infill_angle = extract_float(key, value)?;
            }
            "infill_speed" => {
                cfg.infill_speed = extract_float(key, value)?;
            }
            "solid_infill_speed" => {
                cfg.solid_infill_speed = extract_float(key, value)?;
            }
            "top_shell_layers" => {
                cfg.top_shell_layers = extract_int_as_u32(key, value)?;
            }
            "bottom_shell_layers" => {
                cfg.bottom_shell_layers = extract_int_as_u32(key, value)?;
            }

            // --- Support ---
            "support_enabled" => {
                cfg.support_enabled = extract_bool(key, value)?;
            }
            "support_overhang_angle" => {
                cfg.support_overhang_angle = extract_float(key, value)?;
            }

            // --- Non-planar ---
            "nonplanar_max_angle_deg" => {
                cfg.nonplanar_max_angle_deg = Some(extract_float(key, value)?);
            }
            "nonplanar_shell_count" => {
                cfg.nonplanar_shell_count = Some(extract_int_as_u32(key, value)?);
            }
            "nonplanar_amplitude" => {
                cfg.nonplanar_amplitude = Some(extract_float(key, value)?);
            }

            // --- Smoothificator ---
            "smoothificator_target_height" => {
                cfg.smoothificator_target_height = Some(extract_float(key, value)?);
            }
            "smoothificator_adaptive" => {
                cfg.smoothificator_adaptive = Some(extract_bool(key, value)?);
            }

            // Unknown key → extensions overflow bucket.
            _ => {
                cfg.extensions.insert(key.clone(), value.clone());
            }
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
            per_obj_cfg = apply_overlay(global, &per_object_source)?;
        }

        result.insert(object_id.to_string(), per_obj_cfg);
    }

    Ok(result)
}

/// Apply a flat override map (already stripped of the `object_config:<id>:`
/// prefix) on top of a base [`ResolvedConfig`].
fn apply_overlay(
    base: &ResolvedConfig,
    overrides: &HashMap<String, ConfigValue>,
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

        match key.as_str() {
            "layer_height" => cfg.layer_height = extract_float(key, value)?,
            "line_width" => cfg.line_width = extract_float(key, value)?,
            "first_layer_height" => cfg.first_layer_height = extract_float(key, value)?,
            "first_layer_line_width" => {
                cfg.first_layer_line_width = extract_float(key, value)?;
            }
            "wall_count" => cfg.wall_count = extract_int_as_u32(key, value)?,
            "outer_wall_speed" => cfg.outer_wall_speed = extract_float(key, value)?,
            "inner_wall_speed" => cfg.inner_wall_speed = extract_float(key, value)?,
            "arachne_min_feature_size" => {
                cfg.arachne_min_feature_size = Some(extract_float(key, value)?);
            }
            "infill_density" => cfg.infill_density = extract_float(key, value)?,
            "infill_angle" => cfg.infill_angle = extract_float(key, value)?,
            "infill_speed" => cfg.infill_speed = extract_float(key, value)?,
            "solid_infill_speed" => cfg.solid_infill_speed = extract_float(key, value)?,
            "top_shell_layers" => cfg.top_shell_layers = extract_int_as_u32(key, value)?,
            "bottom_shell_layers" => cfg.bottom_shell_layers = extract_int_as_u32(key, value)?,
            "support_enabled" => cfg.support_enabled = extract_bool(key, value)?,
            "support_overhang_angle" => {
                cfg.support_overhang_angle = extract_float(key, value)?;
            }
            "nonplanar_max_angle_deg" => {
                cfg.nonplanar_max_angle_deg = Some(extract_float(key, value)?);
            }
            "nonplanar_shell_count" => {
                cfg.nonplanar_shell_count = Some(extract_int_as_u32(key, value)?);
            }
            "nonplanar_amplitude" => {
                cfg.nonplanar_amplitude = Some(extract_float(key, value)?);
            }
            "smoothificator_target_height" => {
                cfg.smoothificator_target_height = Some(extract_float(key, value)?);
            }
            "smoothificator_adaptive" => {
                cfg.smoothificator_adaptive = Some(extract_bool(key, value)?);
            }
            _ => {
                cfg.extensions.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(cfg)
}

// --- Type extraction helpers ------------------------------------------------

fn extract_float(key: &str, value: &ConfigValue) -> Result<f32, ConfigResolutionError> {
    match value {
        ConfigValue::Float(f) => Ok(*f as f32),
        ConfigValue::Int(i) => Ok(*i as f32),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Float",
            actual: variant_name(other),
        }),
    }
}

fn extract_int_as_u32(key: &str, value: &ConfigValue) -> Result<u32, ConfigResolutionError> {
    match value {
        ConfigValue::Int(i) => Ok(*i as u32),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Int",
            actual: variant_name(other),
        }),
    }
}

fn extract_bool(key: &str, value: &ConfigValue) -> Result<bool, ConfigResolutionError> {
    match value {
        ConfigValue::Bool(b) => Ok(*b),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Bool",
            actual: variant_name(other),
        }),
    }
}
