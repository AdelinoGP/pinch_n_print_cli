//! Single-source-of-truth declaration of [`ResolvedConfig`].
//!
//! Every declared field is described exactly once via the
//! [`declare_resolved_config!`] DSL near the bottom of this file. The macro
//! expands a single declaration list into:
//!
//! - the `pub struct ResolvedConfig { ... }` definition with all fields,
//! - `impl Default for ResolvedConfig` with all initializers,
//! - `impl ResolvedConfig { pub fn apply_cli_key(...) -> Result<bool, _> }`
//!   that dispatches a `(ConfigKey, ConfigValue)` pair onto the matching field
//!   with strict per-variant type checking.
//!
//! Adding a new field requires editing one line in the macro invocation. The
//! host-side resolver in `slicer-runtime::config_resolution` is a thin loop over
//! `apply_cli_key`.

use std::collections::{BTreeMap, HashMap};

use crate::slice_ir::{ConfigValue, InfillType, SupportType, WallGenerator};

impl ResolvedConfig {
    /// Flattens this config into a `HashMap<key, ConfigValue>` of effective
    /// slicer settings.
    ///
    /// Single source of truth for two consumers that previously kept divergent
    /// copies (gcode `CONFIG_BLOCK` emission and the per-region `ConfigView`
    /// handed to layer-tier modules). `Option`-typed fields that are `None` are
    /// omitted; enum fields are emitted as their `Debug` string; module-supplied
    /// `extensions` keys are merged through unchanged. Consumers that must
    /// restrict visibility (e.g. the per-module config view) filter this map to
    /// their declared keys.
    #[must_use]
    pub fn to_config_map(&self) -> HashMap<String, ConfigValue> {
        let mut m: HashMap<String, ConfigValue> = HashMap::new();
        m.insert("layer_height".into(), ConfigValue::Float(self.layer_height));
        m.insert(
            "line_width".into(),
            ConfigValue::Float(f64::from(self.line_width)),
        );
        m.insert(
            "first_layer_height".into(),
            ConfigValue::Float(self.first_layer_height),
        );
        m.insert(
            "first_layer_line_width".into(),
            ConfigValue::Float(f64::from(self.first_layer_line_width)),
        );
        m.insert(
            "wall_count".into(),
            ConfigValue::Int(i64::from(self.wall_count)),
        );
        m.insert(
            "outer_wall_speed".into(),
            ConfigValue::Float(f64::from(self.outer_wall_speed)),
        );
        m.insert(
            "inner_wall_speed".into(),
            ConfigValue::Float(f64::from(self.inner_wall_speed)),
        );
        m.insert(
            "wall_generator".into(),
            ConfigValue::String(format!("{:?}", self.wall_generator)),
        );
        if let Some(v) = self.arachne_min_feature_size {
            m.insert(
                "arachne_min_feature_size".into(),
                ConfigValue::Float(f64::from(v)),
            );
        }
        m.insert(
            "infill_type".into(),
            ConfigValue::String(format!("{:?}", self.infill_type)),
        );
        m.insert(
            "infill_density".into(),
            ConfigValue::Float(f64::from(self.infill_density)),
        );
        m.insert(
            "infill_overlap".into(),
            ConfigValue::Float(f64::from(self.infill_overlap)),
        );
        m.insert(
            "infill_angle".into(),
            ConfigValue::Float(f64::from(self.infill_angle)),
        );
        m.insert(
            "infill_speed".into(),
            ConfigValue::Float(f64::from(self.infill_speed)),
        );
        m.insert(
            "solid_infill_speed".into(),
            ConfigValue::Float(f64::from(self.solid_infill_speed)),
        );
        m.insert(
            "top_shell_layers".into(),
            ConfigValue::Int(i64::from(self.top_shell_layers)),
        );
        m.insert(
            "bottom_shell_layers".into(),
            ConfigValue::Int(i64::from(self.bottom_shell_layers)),
        );
        m.insert(
            "top_fill_holder".into(),
            ConfigValue::String(self.top_fill_holder.clone()),
        );
        m.insert(
            "bottom_fill_holder".into(),
            ConfigValue::String(self.bottom_fill_holder.clone()),
        );
        m.insert(
            "bridge_fill_holder".into(),
            ConfigValue::String(self.bridge_fill_holder.clone()),
        );
        m.insert(
            "sparse_fill_holder".into(),
            ConfigValue::String(self.sparse_fill_holder.clone()),
        );
        m.insert(
            "enable_support".into(),
            ConfigValue::Bool(self.support_enabled),
        );
        m.insert(
            "support_type".into(),
            ConfigValue::String(format!("{:?}", self.support_type)),
        );
        m.insert(
            "support_overhang_angle".into(),
            ConfigValue::Float(f64::from(self.support_overhang_angle)),
        );
        if let Some(v) = self.nonplanar_max_angle_deg {
            m.insert(
                "nonplanar_max_angle_deg".into(),
                ConfigValue::Float(f64::from(v)),
            );
        }
        if let Some(v) = self.nonplanar_shell_count {
            m.insert(
                "nonplanar_shell_count".into(),
                ConfigValue::Int(i64::from(v)),
            );
        }
        if let Some(v) = self.nonplanar_amplitude {
            m.insert(
                "nonplanar_amplitude".into(),
                ConfigValue::Float(f64::from(v)),
            );
        }
        if let Some(v) = self.smoothificator_target_height {
            m.insert(
                "smoothificator_target_height".into(),
                ConfigValue::Float(f64::from(v)),
            );
        }
        if let Some(v) = self.smoothificator_adaptive {
            m.insert("smoothificator_adaptive".into(), ConfigValue::Bool(v));
        }
        // Machine kinematic limits + filament density (Option fields: absent → key omitted,
        // so unset configs leave CONFIG_BLOCK bytes unchanged).
        for (key, v) in [
            (
                "machine_max_acceleration_extruding",
                self.machine_max_acceleration_extruding,
            ),
            (
                "machine_max_acceleration_travel",
                self.machine_max_acceleration_travel,
            ),
            ("machine_max_speed_x", self.machine_max_speed_x),
            ("machine_max_speed_y", self.machine_max_speed_y),
            ("machine_max_speed_z", self.machine_max_speed_z),
            ("machine_max_speed_e", self.machine_max_speed_e),
            ("machine_max_jerk_x", self.machine_max_jerk_x),
            ("machine_max_jerk_y", self.machine_max_jerk_y),
            ("machine_max_jerk_z", self.machine_max_jerk_z),
            ("machine_max_jerk_e", self.machine_max_jerk_e),
        ] {
            if let Some(v) = v {
                m.insert(key.into(), ConfigValue::Float(f64::from(v)));
            }
        }
        // `filament_density` is Orca `coFloats` — one entry per filament — so
        // the whole vector is emitted, matching what canonical's
        // `append_full_config` writes. The gcode serializer renders a numeric
        // list comma-separated (`ConfigOptionFloats::serialize`), unlike the
        // semicolon form used for `coStrings` such as `filament_colour`.
        if !self.filament_density.is_empty() {
            m.insert(
                "filament_density".into(),
                ConfigValue::List(
                    self.filament_density
                        .iter()
                        .map(|d| ConfigValue::Float(*d))
                        .collect(),
                ),
            );
        }
        // mmu_segmented_region_{max_width,interlocking_depth,interlocking_beam} intentionally
        // omitted — P96 AC-8: emitting these keys would change g-code CONFIG_BLOCK bytes for all
        // prints, breaking byte-identicality vs baseline.
        // Merge extension keys (module-contributed, already in ConfigValue form).
        for (k, v) in &self.extensions {
            m.insert(k.clone(), v.clone());
        }
        m
    }
}

impl ResolvedConfig {
    /// Filament density in g/cm³ for `tool_index`, or `None` when unconfigured.
    ///
    /// Mirrors canonical `Extruder::filament_density`, which reads
    /// `filament_density.get_at(m_id)` — the value is per filament, not global.
    /// Canonical pads short lists (`GCodeProcessor::process_used_filament`
    /// extends with `DEFAULT_FILAMENT_DENSITY`); this port falls back to the
    /// first entry instead, so a single-filament config still prices every
    /// tool rather than silently substituting a hard-coded constant.
    #[must_use]
    pub fn filament_density_for(&self, tool_index: u32) -> Option<f64> {
        self.filament_density
            .get(tool_index as usize)
            .or_else(|| self.filament_density.first())
            .copied()
    }
}

// ── Error and extractor primitives ─────────────────────────────────────────

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
    /// A numeric value fell outside the `[min, max]` range declared in the
    /// module manifest schema. Also raised for NaN/Inf in numeric fields.
    OutOfRange {
        /// The config key that received the offending value.
        key: String,
        /// The numeric value (coerced to `f64` for reporting).
        value: f64,
        /// Inclusive minimum from the manifest, if declared.
        min: Option<f64>,
        /// Inclusive maximum from the manifest, if declared.
        max: Option<f64>,
        /// Index of the offending element when `value` is a list element.
        index: Option<usize>,
    },
    /// Per-object `support_layer_height_mm` is non-zero but less than the
    /// object's effective layer height. The printer cannot extrude a
    /// support layer thinner than the nominal model layer.
    SupportLayerHeightTooFine {
        /// The object whose support config is invalid.
        object_id: String,
        /// The configured support_layer_height_mm (mm).
        support_layer_height_mm: f32,
        /// The object's effective layer height in mm (its
        /// `layer_height` field after per-object override resolution).
        effective_layer_height_mm: f32,
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
            Self::OutOfRange {
                key,
                value,
                min,
                max,
                index,
            } => {
                let range = match (min, max) {
                    (Some(lo), Some(hi)) => format!("[{lo}, {hi}]"),
                    (Some(lo), None) => format!("[{lo}, +inf)"),
                    (None, Some(hi)) => format!("(-inf, {hi}]"),
                    (None, None) => "(finite)".to_string(),
                };
                match index {
                    Some(i) => write!(
                        f,
                        "config key '{key}'[{i}]: value {value} outside allowed range {range}"
                    ),
                    None => write!(
                        f,
                        "config key '{key}': value {value} outside allowed range {range}"
                    ),
                }
            }
            Self::SupportLayerHeightTooFine {
                object_id,
                support_layer_height_mm,
                effective_layer_height_mm,
            } => write!(
                f,
                "object '{object_id}': support_layer_height_mm = {support_layer_height_mm} mm \
                 is below the object's effective layer height ({effective_layer_height_mm} mm); \
                 the printer cannot extrude a support layer thinner than the model layer"
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
        ConfigValue::Percent(_) => "Percent".to_string(),
        ConfigValue::FloatOrPercent { .. } => "FloatOrPercent".to_string(),
    }
}

/// Coerce a `Bool` that originated as an untyped `"0"`/`"1"` string.
///
/// OrcaSlicer serialises *every* `project_settings.config` value as a string,
/// so `"1"` is ambiguous between `enable_support` (a boolean) and `wall_loops`
/// (an integer). The 3MF loader's `coerce_string_to_config_value` has no
/// schema to disambiguate and guesses `Bool` for `"0"`/`"1"`, which meant any
/// numeric key that happened to hold 0 or 1 aborted config resolution with a
/// `TypeMismatch` — e.g. `mmu_segmented_region_interlocking_depth = "0"`,
/// which crashed every slice of `resources/cube_4color.3mf`.
///
/// The ambiguity is irreducible at load time, so it is resolved at the point
/// of consumption instead: the declaring field knows its own type, and treats
/// a `"0"`/`"1"`-derived `Bool` as the number it stands for.
fn bool_as_number(value: &ConfigValue) -> Option<i64> {
    match value {
        ConfigValue::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// Extract an `f32` from a `Float`/`Int` `ConfigValue`. Used by the
/// [`declare_resolved_config!`] macro expansion.
///
/// Also accepts a `Bool` as 0/1 — see [`bool_as_number`].
#[doc(hidden)]
pub fn extract_float(key: &str, value: &ConfigValue) -> Result<f32, ConfigResolutionError> {
    match value {
        ConfigValue::Float(f) => Ok(*f as f32),
        ConfigValue::Int(i) => Ok(*i as f32),
        other => bool_as_number(other).map(|n| n as f32).ok_or_else(|| {
            ConfigResolutionError::TypeMismatch {
                key: key.to_string(),
                expected: "Float",
                actual: variant_name(other),
            }
        }),
    }
}

/// Extract an `f64` from a `Float` or `Int` `ConfigValue`.
///
/// Used by config fields whose value feeds the layer-Z formula
/// (`layer_height`, `first_layer_height`). These must round-trip through
/// the WIT boundary as `ConfigValue::Float(f64)` *without* an intermediate
/// `f32` narrowing: the f32 bit pattern of `0.2` is `0.20000000298...`,
/// which — when multiplied by `n` in f64 — drifts onto an adjacent f32 at
/// ~every 10th layer (e.g. n=93 yields `f32(18.80000028) = 18.80000114`
/// instead of the STL's `f32(18.8) = 18.79999924`). Keeping the value in
/// `f64` end-to-end mirrors OrcaSlicer's `coordf_t` (`double`) layer-Z
/// computation in `Slicing.cpp:807-867` (`generate_object_layers`); the
/// only `f32` cast happens at the WIT `layer-proposal.z: f32` boundary,
/// equivalent to OrcaSlicer's `float(print_z)` at `slice_facet`'s
/// `slice_z` parameter (`TriangleMeshSlicer.cpp:158`).
/// Also accepts a `Bool` as 0/1 — see [`bool_as_number`].
pub fn extract_f64(key: &str, value: &ConfigValue) -> Result<f64, ConfigResolutionError> {
    match value {
        ConfigValue::Float(f) => Ok(*f),
        ConfigValue::Int(i) => Ok(*i as f64),
        other => bool_as_number(other).map(|n| n as f64).ok_or_else(|| {
            ConfigResolutionError::TypeMismatch {
                key: key.to_string(),
                expected: "Float",
                actual: variant_name(other),
            }
        }),
    }
}

/// Extract a `u32` from an `Int` `ConfigValue`.
///
/// Also accepts a `Bool` as 0/1 — see [`bool_as_number`].
#[doc(hidden)]
pub fn extract_int_as_u32(key: &str, value: &ConfigValue) -> Result<u32, ConfigResolutionError> {
    match value {
        ConfigValue::Int(i) => Ok(*i as u32),
        other => bool_as_number(other).map(|n| n as u32).ok_or_else(|| {
            ConfigResolutionError::TypeMismatch {
                key: key.to_string(),
                expected: "Int",
                actual: variant_name(other),
            }
        }),
    }
}

/// Extract a `bool` from a `Bool` `ConfigValue`.
///
/// The mirror of [`bool_as_number`]: an `Int` 0/1 is accepted as a boolean, so
/// a genuinely-boolean key still resolves if the loader's `"0"`/`"1"` guess
/// ever changes, or if a hand-written JSON config spells the flag numerically.
#[doc(hidden)]
pub fn extract_bool(key: &str, value: &ConfigValue) -> Result<bool, ConfigResolutionError> {
    match value {
        ConfigValue::Bool(b) => Ok(*b),
        ConfigValue::Int(0) => Ok(false),
        ConfigValue::Int(1) => Ok(true),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Bool",
            actual: variant_name(other),
        }),
    }
}

/// Extract a `String` from a `String` `ConfigValue`.
#[doc(hidden)]
pub fn extract_string(key: &str, value: &ConfigValue) -> Result<String, ConfigResolutionError> {
    match value {
        ConfigValue::String(s) => Ok(s.clone()),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "String",
            actual: variant_name(other),
        }),
    }
}

/// Extract a `Vec<f64>` from a `List(Vec<ConfigValue::Float>)` `ConfigValue`.
///
/// Each element must be a `Float` (or `Int`, coerced to `f64`). Returns
/// `TypeMismatch` if the outer value is not a `List`, or if any element
/// is neither `Float` nor `Int`.
#[doc(hidden)]
pub fn extract_float_list(
    key: &str,
    value: &ConfigValue,
) -> Result<Vec<f64>, ConfigResolutionError> {
    fn element(key: &str, v: &ConfigValue) -> Result<f64, ConfigResolutionError> {
        match v {
            ConfigValue::Float(f) => Ok(*f),
            ConfigValue::Int(n) => Ok(*n as f64),
            // Orca serialises list entries as strings (`["1.24","1.24"]`), and
            // a `"0"`/`"1"` entry reaches us as `Bool` for the same reason
            // `extract_float` tolerates one — see `bool_as_number`.
            ConfigValue::String(s) => {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| ConfigResolutionError::TypeMismatch {
                        key: key.to_string(),
                        expected: "Float",
                        actual: "String".to_string(),
                    })
            }
            other => bool_as_number(other).map(|n| n as f64).ok_or_else(|| {
                ConfigResolutionError::TypeMismatch {
                    key: key.to_string(),
                    expected: "Float",
                    actual: variant_name(other),
                }
            }),
        }
    }

    match value {
        ConfigValue::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| element(&format!("{key}[{i}]"), v))
            .collect(),
        // A bare scalar is accepted as a one-element list so hand-written
        // CLI/JSON configs (and single-filament setups) keep working.
        other => element(key, other).map(|v| vec![v]),
    }
}

/// Extract a single `f32` from a scalar, or from the first entry of an Orca
/// `coFloats` list.
///
/// Used by keys OrcaSlicer declares as `coFloats` but this port models as one
/// scalar: the `machine_max_*` kinematic limits and `filament_diameter`.
///
/// OrcaSlicer types these options `coFloats`, and the entries are machine *time
/// modes* — `[normal, stealth]` — not per-extruder values (see the `AxisDefault`
/// table in canonical `PrintConfig::PrintConfig`, whose second entry is the
/// silent variant). Every canonical consumer that wants one scalar reads index
/// 0: `GCode::print_machine_envelope` and `Print`'s motion-ability check both
/// take the front element, and `GCodeProcessor`'s limit getters index by
/// `ETimeMode`, whose `Normal` discriminant is 0. We therefore take index 0 and
/// ignore any trailing modes.
///
/// Values reach us as `List` because a real project's `project_settings.config`
/// stores them as JSON arrays of *strings* (e.g. `["9","9"]`), so list elements
/// are coerced from `Float`, `Int`, or a numeric `String`. A scalar is accepted
/// unchanged so hand-written CLI/JSON configs keep working.
///
/// Unlike canonical — which silently substitutes `0.0` and then falls back to a
/// built-in default — an unparseable or empty value is still a hard error here.
/// The permissiveness is only about accepting the shape Orca actually writes,
/// not about tolerating malformed input.
#[doc(hidden)]
pub fn extract_float_or_first(key: &str, value: &ConfigValue) -> Result<f32, ConfigResolutionError> {
    fn scalar(key: &str, value: &ConfigValue) -> Result<f32, ConfigResolutionError> {
        match value {
            ConfigValue::Float(f) => Ok(*f as f32),
            ConfigValue::Int(i) => Ok(*i as f32),
            ConfigValue::String(s) => {
                s.trim()
                    .parse::<f32>()
                    .map_err(|_| ConfigResolutionError::TypeMismatch {
                        key: key.to_string(),
                        expected: "Float",
                        actual: "String".to_string(),
                    })
            }
            other => Err(ConfigResolutionError::TypeMismatch {
                key: key.to_string(),
                expected: "Float",
                actual: variant_name(other),
            }),
        }
    }

    match value {
        // Index 0 is canonical "normal" mode; trailing entries are stealth-mode
        // variants this port does not model.
        ConfigValue::List(items) => match items.first() {
            Some(first) => scalar(&format!("{key}[0]"), first),
            None => Err(ConfigResolutionError::TypeMismatch {
                key: key.to_string(),
                expected: "non-empty List",
                actual: "empty List".to_string(),
            }),
        },
        other => scalar(key, other),
    }
}

// ── Declarative DSL ────────────────────────────────────────────────────────

/// Declare every `ResolvedConfig` field in one place. Each line is one of:
///
/// - `plain <field>: <Ty> = <default>;` — struct field + Default only; the
///   field is NOT bound to any CLI key (current behavior preserved for
///   enums and per-region override targets).
/// - `cli "<cli_key>" <field>: <Ty> = <default> => <extractor>;` — struct
///   field + Default + an `apply_cli_key` match arm that calls
///   `<extractor>(key, value)?` and assigns the result to the field.
/// - `cli_opt "<cli_key>" <field>: Option<T> = <default> => <extractor>;` —
///   same as `cli`, except the extracted `T` is wrapped in `Some(...)` before
///   assignment. Default is typically `None`.
///
/// The macro always appends an `extensions: HashMap<String, ConfigValue>`
/// field initialised to `HashMap::new()`. The catch-all `_ => Ok(false)` arm
/// in `apply_cli_key` signals "unknown key, route to extensions" to the
/// host-side resolver.
#[macro_export]
macro_rules! declare_resolved_config {
    ( $($t:tt)* ) => {
        // The trailing three idents are the parameter names threaded through
        // the recursion. They become metavariables in every `__drc!` arm,
        // giving the per-field match arms a shared hygiene context with the
        // `apply_cli_key` body emitted by the terminal arm. (We cannot
        // reference `self` directly across separate macro arms — see the
        // `let __drc_cfg = ...` binding in the terminal arm.)
        $crate::__drc!(@parse
            fields:   { }
            defaults: { }
            cli_arms: { }
            cfg:      __drc_cfg
            key:      __drc_key
            value:    __drc_value
            input:    { $($t)* }
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __drc {
    // Done parsing — emit struct, Default, and apply_cli_key.
    (@parse
        fields:   { $($sf:tt)* }
        defaults: { $($df:tt)* }
        cli_arms: { $($arm:tt)* }
        cfg:      $cfg:ident
        key:      $key:ident
        value:    $value:ident
        input:    { }
    ) => {
        /// Fully merged config produced by the host resolver and consumed by
        /// every per-region/per-object planning stage.
        ///
        /// Field set is declared via [`declare_resolved_config!`]; see that
        /// macro and the invocation in `crates/slicer-ir/src/resolved_config.rs`
        /// for the single source of truth.
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        pub struct ResolvedConfig {
            $($sf)*
            /// Overflow bucket for unknown module configs.
            pub extensions: ::std::collections::BTreeMap<String, $crate::ConfigValue>,
        }

        impl ::core::default::Default for ResolvedConfig {
            fn default() -> Self {
                Self {
                    $($df)*
                    extensions: ::std::collections::BTreeMap::new(),
                }
            }
        }

        impl ResolvedConfig {
            /// Apply a single CLI-source `(key, value)` pair to the matching
            /// declared field.
            ///
            /// Returns:
            /// - `Ok(true)` if `key` matched a CLI-bound declared field and
            ///   the field was updated.
            /// - `Ok(false)` if `key` does not name a CLI-bound declared
            ///   field; the caller is expected to route the value to
            ///   [`ResolvedConfig::extensions`].
            /// - `Err(ConfigResolutionError::TypeMismatch { .. })` if the
            ///   value's `ConfigValue` variant did not match the declared
            ///   field type.
            pub fn apply_cli_key(
                &mut self,
                key: &str,
                value: &$crate::ConfigValue,
            ) -> ::core::result::Result<bool, $crate::ConfigResolutionError> {
                // Hygiene-safe re-binds: each per-field arm emitted by a
                // separate `__drc!` arm references these via metavariables
                // ($cfg, $key, $value), so they share a single binding site.
                let $cfg: &mut ResolvedConfig = self;
                let $key: &str = key;
                let $value: &$crate::ConfigValue = value;
                match $key {
                    $($arm)*
                    _ => ::core::result::Result::Ok(false),
                }
            }
        }
    };

    // `plain <field>: <ty> = <default>;` — struct + default, no CLI dispatch.
    (@parse
        fields:   { $($sf:tt)* }
        defaults: { $($df:tt)* }
        cli_arms: { $($arm:tt)* }
        cfg:      $cfg:ident
        key:      $key:ident
        value:    $value:ident
        input: {
            $(#[$m:meta])*
            plain $field:ident : $ty:ty = $default:expr ;
            $($rest:tt)*
        }
    ) => {
        $crate::__drc!(@parse
            fields: {
                $($sf)*
                $(#[$m])*
                pub $field: $ty,
            }
            defaults: {
                $($df)*
                $field: $default,
            }
            cli_arms: { $($arm)* }
            cfg:      $cfg
            key:      $key
            value:    $value
            input: { $($rest)* }
        );
    };

    // `cli "<key>" <field>: <ty> = <default> => <extractor>;`
    // — required CLI-bound field; extractor returns T directly.
    (@parse
        fields:   { $($sf:tt)* }
        defaults: { $($df:tt)* }
        cli_arms: { $($arm:tt)* }
        cfg:      $cfg:ident
        key:      $key:ident
        value:    $value:ident
        input: {
            $(#[$m:meta])*
            cli $cli_key:literal $field:ident : $ty:ty = $default:expr => $extractor:ident ;
            $($rest:tt)*
        }
    ) => {
        $crate::__drc!(@parse
            fields: {
                $($sf)*
                $(#[$m])*
                pub $field: $ty,
            }
            defaults: {
                $($df)*
                $field: $default,
            }
            cli_arms: {
                $($arm)*
                $cli_key => {
                    $cfg.$field = $crate::resolved_config::$extractor($key, $value)?;
                    ::core::result::Result::Ok(true)
                }
            }
            cfg:      $cfg
            key:      $key
            value:    $value
            input: { $($rest)* }
        );
    };

    // `cli_opt "<key>" <field>: Option<T> = <default> => <extractor>;`
    // — optional CLI-bound field; extracted T is wrapped in `Some(...)`.
    (@parse
        fields:   { $($sf:tt)* }
        defaults: { $($df:tt)* }
        cli_arms: { $($arm:tt)* }
        cfg:      $cfg:ident
        key:      $key:ident
        value:    $value:ident
        input: {
            $(#[$m:meta])*
            cli_opt $cli_key:literal $field:ident : $ty:ty = $default:expr => $extractor:ident ;
            $($rest:tt)*
        }
    ) => {
        $crate::__drc!(@parse
            fields: {
                $($sf)*
                $(#[$m])*
                pub $field: $ty,
            }
            defaults: {
                $($df)*
                $field: $default,
            }
            cli_arms: {
                $($arm)*
                $cli_key => {
                    $cfg.$field = ::core::option::Option::Some(
                        $crate::resolved_config::$extractor($key, $value)?
                    );
                    ::core::result::Result::Ok(true)
                }
            }
            cfg:      $cfg
            key:      $key
            value:    $value
            input: { $($rest)* }
        );
    };
}

// ── Field declarations: the single source of truth ─────────────────────────

declare_resolved_config! {
    // Geometry
    /// Layer height in millimeters.
    ///
    /// Stored as `f64` (not `f32`) so the layer-Z formula `first_layer_height
    /// + n * layer_height` computes in untainted `f64` — matching OrcaSlicer's
    /// `coordf_t` (`double`) `print_z += height` loop in
    /// `Slicing.cpp:859`. The f32 bit pattern of `0.2` is
    /// `0.20000000298...`; widening that back to `f64` and multiplying by
    /// `n` drifts onto an adjacent `f32` at ~every 10th layer, missing STL
    /// vertices stored as `f32(mm_value)` and breaking `classify_vertex`'s
    /// exact `f32 ==` plane test. See `extract_f64` for the full rationale.
    cli "layer_height"           layer_height: f64 = 0.2 => extract_f64;
    /// Line width in millimeters.
    cli "line_width"             line_width: f32 = 0.4 => extract_float;
    /// First layer height in millimeters. `f64` for the same reason as
    /// `layer_height` — feeds the layer-Z formula and must not be re-tainted
    /// by an `f32` round-trip. See `layer_height` and `extract_f64`.
    cli "first_layer_height"     first_layer_height: f64 = 0.2 => extract_f64;
    /// First layer line width in millimeters.
    cli "first_layer_line_width" first_layer_line_width: f32 = 0.4 => extract_float;
    /// Filament diameter in millimeters. Used by the G-code emitter to convert
    /// extruded volume (width × height × length) into filament length (E).
    /// Filament diameter in mm. Orca declares this `coFloats` (one entry per
    /// filament); this is the default-config scalar, taken from entry 0.
    /// Per-tool diameters reach the estimator through the per-tool config map
    /// built in `slicer-gcode`'s emit path, not from this field.
    cli "filament_diameter"      filament_diameter: f32 = 1.75 => extract_float_or_first;

    // Walls
    /// Number of walls (perimeters).
    cli "wall_count"             wall_count: u32 = 2 => extract_int_as_u32;
    /// Outer wall speed in mm/s.
    cli "outer_wall_speed"       outer_wall_speed: f32 = 50.0 => extract_float;
    /// Inner wall speed in mm/s.
    cli "inner_wall_speed"       inner_wall_speed: f32 = 50.0 => extract_float;
    /// Wall generator algorithm. Not CLI-bound today.
    plain                        wall_generator: WallGenerator = WallGenerator::Classic;
    /// Minimum feature size for Arachne (optional).
    cli_opt "arachne_min_feature_size" arachne_min_feature_size: Option<f32> = None => extract_float;

    // Infill
    /// Infill type. Not CLI-bound today.
    plain                        infill_type: InfillType = InfillType::Grid;
    /// Infill density (0.0 to 1.0).
    cli "infill_density"         infill_density: f32 = 0.2 => extract_float;
    /// Infill angle in degrees.
    cli "infill_angle"           infill_angle: f32 = 45.0 => extract_float;
    /// Infill speed in mm/s.
    cli "infill_speed"           infill_speed: f32 = 50.0 => extract_float;
    /// Solid infill speed in mm/s.
    cli "solid_infill_speed"     solid_infill_speed: f32 = 50.0 => extract_float;
    /// Number of top shell layers.
    cli "top_shell_layers"       top_shell_layers: u32 = 3 => extract_int_as_u32;
    /// Number of bottom shell layers.
    cli "bottom_shell_layers"    bottom_shell_layers: u32 = 3 => extract_int_as_u32;

    // Fill-role holders (packet 37). Per-region overrides flow through
    // `RegionMapIR.entries[*].config`, not CLI keys.
    /// Module ID holding `claim:top-fill`.
    cli "top_fill_holder"          top_fill_holder: String = String::from("rectilinear-infill") => extract_string;
    /// Module ID holding `claim:bottom-fill`.
    cli "bottom_fill_holder"       bottom_fill_holder: String = String::from("rectilinear-infill") => extract_string;
    /// Module ID holding `claim:bridge-fill`.
    cli "bridge_fill_holder"       bridge_fill_holder: String = String::from("rectilinear-infill") => extract_string;
    /// Module ID holding `claim:sparse-fill`.
    cli "sparse_fill_holder"       sparse_fill_holder: String = String::from("rectilinear-infill") => extract_string;

    // Precision / resolution
    /// G-code path resolution in mm (OrcaSlicer: gcode_resolution).
    cli "gcode_resolution"       gcode_resolution: f32 = 0.0125 => extract_float;
    /// Infill path resolution in mm (OrcaSlicer: infill_anchor_max).
    cli "infill_resolution"      infill_resolution: f32 = 0.04 => extract_float;
    /// Infill overlap with perimeters in mm (OrcaSlicer: infill_overlap). The
    /// infill path extends past the perimeter boundary by this much so the
    /// linker's boundary anchoring can re-attach. The linker reads this key
    /// from the per-region resolved config.
    cli "infill_overlap"         infill_overlap: f32 = 0.45 => extract_float;
    /// Support path resolution in mm (OrcaSlicer: support_resolution).
    cli "support_resolution"     support_resolution: f32 = 0.0375 => extract_float;
    /// Minimum segment length in mm (OrcaSlicer: min_length_factor).
    cli "min_segment_length"     min_segment_length: f32 = 0.05 => extract_float;
    /// Number of decimal places for G-code XY coordinates.
    cli "gcode_xy_decimals"      gcode_xy_decimals: u32 = 3 => extract_int_as_u32;
    /// Arc tolerance for perimeter arcs in mm (OrcaSlicer: arc_fitting_tolerance).
    cli "perimeter_arc_tolerance" perimeter_arc_tolerance: f32 = 0.0125 => extract_float;
    /// Slice closing radius in mm (OrcaSlicer: slice_closing_radius).
    cli "slice_closing_radius"   slice_closing_radius: f32 = 0.049 => extract_float;
    /// Morphological-closing join type used by the flat-bridge enclosure
    /// discriminator in `PrePass::Slice`. One of `"miter"` (default),
    /// `"square"`, or `"round"` (case-insensitive; unknown values fall back to
    /// `miter`). The discriminator is a boolean "does a gap ≤ 2·R re-fill?"
    /// test, so corner roundness never changes the verdict:
    /// - `miter` matches OrcaSlicer's `closing` default (`jtMiter`,
    ///   `ClipperUtils.hpp`) and is cheap (one bevel point per corner).
    /// - `square` is an equally cheap alternative.
    /// - `round` reproduces pre-optimisation flat-bridge detection
    ///   bit-for-bit, but tessellates every corner into an arc and made the
    ///   closing ~92% of `PrePass::Slice` on high-vertex cross-sections.
    cli "flat_bridge_closing_join" flat_bridge_closing_join: String = String::from("miter") => extract_string;

     // Support
     /// Whether support is enabled.
     cli "enable_support"         support_enabled: bool = false => extract_bool;
     /// Whether to suppress M73 progress commands.
     cli "disable_m73"             disable_m73: bool = false => extract_bool;
     /// Support generation type. Not CLI-bound today.
    plain                        support_type: SupportType = SupportType::Traditional;
    /// Support overhang angle threshold in degrees.
    cli "support_overhang_angle" support_overhang_angle: f32 = 45.0 => extract_float;
    /// Support layer height in millimeters. `0.0` means "use the object's
    /// effective layer height". Non-zero values must be at least the
    /// object's effective layer height (printers cannot extrude a layer
    /// thinner than the nominal model layer). Validated per-object in
    /// `slicer_runtime::config_schema`.
    cli "support_layer_height_mm" support_layer_height_mm: f32 = 0.0 => extract_float;

    // Non-planar (module-contributed)
    /// Maximum non-planar angle in degrees (optional).
    cli_opt "nonplanar_max_angle_deg"  nonplanar_max_angle_deg: Option<f32> = None => extract_float;
    /// Number of non-planar shells (optional).
    cli_opt "nonplanar_shell_count"    nonplanar_shell_count: Option<u32> = None => extract_int_as_u32;
    /// Non-planar amplitude in millimeters (optional).
    cli_opt "nonplanar_amplitude"      nonplanar_amplitude: Option<f32> = None => extract_float;

    // Smoothificator (module-contributed)
    /// Smoothificator target height in millimeters (optional).
    cli_opt "smoothificator_target_height" smoothificator_target_height: Option<f32> = None => extract_float;
    /// Smoothificator adaptive mode (optional).
    cli_opt "smoothificator_adaptive"      smoothificator_adaptive: Option<bool> = None => extract_bool;

    // Printer bed / tool-change (module-contributed)
    /// Printer bed polygon as [x0, y0, x1, y1, ...] in mm.
    /// Default: 250 × 250 mm square.
    cli "bed_shape" bed_shape: Vec<f64> = vec![0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0] => extract_float_list;
    /// Retract length in mm before tool change.
    cli "retract_length" retract_length: f32 = 2.0 => extract_float;
    /// Whether the wipe tower is enabled for multi-material purge.
    /// Default false matches single-material shipping behavior.
    cli "wipe_tower_enabled" wipe_tower_enabled: bool = false => extract_bool;

    // MMU segmented region (Phase 5 — width limiting / interlocking)
    /// Maximum width of MMU segmented regions in mm. `0.0` means no limit.
    cli "mmu_segmented_region_max_width" mmu_segmented_region_max_width: f32 = 0.0 => extract_float;
    /// Interlocking depth for MMU segmented regions in mm. `0.0` means no interlocking.
    cli "mmu_segmented_region_interlocking_depth" mmu_segmented_region_interlocking_depth: f32 = 0.0 => extract_float;
    /// When true, Phase 5 width-limiting is skipped entirely (OrcaSlicer
    /// interlocking-beam parity). Default `false` matches single-material behaviour.
    cli "mmu_segmented_region_interlocking_beam" mmu_segmented_region_interlocking_beam: bool = false => extract_bool;

    // Machine kinematic limits (time estimator; optional — absent keys stay None)
    /// Maximum acceleration while extruding, in mm/s² (optional).
    cli_opt "machine_max_acceleration_extruding" machine_max_acceleration_extruding: Option<f32> = None => extract_float_or_first;
    /// Maximum acceleration for travel moves, in mm/s² (optional).
    cli_opt "machine_max_acceleration_travel" machine_max_acceleration_travel: Option<f32> = None => extract_float_or_first;
    /// Maximum X-axis speed in mm/s (optional).
    cli_opt "machine_max_speed_x" machine_max_speed_x: Option<f32> = None => extract_float_or_first;
    /// Maximum Y-axis speed in mm/s (optional).
    cli_opt "machine_max_speed_y" machine_max_speed_y: Option<f32> = None => extract_float_or_first;
    /// Maximum Z-axis speed in mm/s (optional).
    cli_opt "machine_max_speed_z" machine_max_speed_z: Option<f32> = None => extract_float_or_first;
    /// Maximum extruder (E-axis) speed in mm/s (optional).
    cli_opt "machine_max_speed_e" machine_max_speed_e: Option<f32> = None => extract_float_or_first;
    /// Maximum X-axis jerk in mm/s (optional).
    cli_opt "machine_max_jerk_x" machine_max_jerk_x: Option<f32> = None => extract_float_or_first;
    /// Maximum Y-axis jerk in mm/s (optional).
    cli_opt "machine_max_jerk_y" machine_max_jerk_y: Option<f32> = None => extract_float_or_first;
    /// Maximum Z-axis jerk in mm/s (optional).
    cli_opt "machine_max_jerk_z" machine_max_jerk_z: Option<f32> = None => extract_float_or_first;
    /// Maximum extruder (E-axis) jerk in mm/s (optional).
    cli_opt "machine_max_jerk_e" machine_max_jerk_e: Option<f32> = None => extract_float_or_first;

    /// Filament density in g/cm³ (optional).
    /// Filament density in g/cm³, one entry per filament (Orca `coFloats`).
    /// Indexed by extruder/tool id — see [`ResolvedConfig::filament_density_for`].
    /// Empty means "not configured", which omits every weight output.
    cli "filament_density" filament_density: Vec<f64> = Vec::new() => extract_float_list;
}

// Touch the imports the macro expansion implicitly relies on, so a future
// reviewer doesn't think they're unused.
#[allow(dead_code)]
const _: fn() = || {
    let _: BTreeMap<String, ConfigValue> = BTreeMap::new();
    let _: HashMap<String, ConfigValue> = HashMap::new(); // to_config_map still returns HashMap
};

impl PartialEq for ResolvedConfig {
    fn eq(&self, other: &Self) -> bool {
        // f32/f64 fields compared via to_bits() so that Eq and Hash are consistent.
        self.layer_height.to_bits() == other.layer_height.to_bits()
            && self.line_width.to_bits() == other.line_width.to_bits()
            && self.first_layer_height.to_bits() == other.first_layer_height.to_bits()
            && self.first_layer_line_width.to_bits() == other.first_layer_line_width.to_bits()
            && self.filament_diameter.to_bits() == other.filament_diameter.to_bits()
            && self.wall_count == other.wall_count
            && self.outer_wall_speed.to_bits() == other.outer_wall_speed.to_bits()
            && self.inner_wall_speed.to_bits() == other.inner_wall_speed.to_bits()
            && self.wall_generator == other.wall_generator
            && self.arachne_min_feature_size.map(|f| f.to_bits())
                == other.arachne_min_feature_size.map(|f| f.to_bits())
            && self.infill_type == other.infill_type
            && self.infill_density.to_bits() == other.infill_density.to_bits()
            && self.infill_overlap.to_bits() == other.infill_overlap.to_bits()
            && self.infill_angle.to_bits() == other.infill_angle.to_bits()
            && self.infill_speed.to_bits() == other.infill_speed.to_bits()
            && self.solid_infill_speed.to_bits() == other.solid_infill_speed.to_bits()
            && self.top_shell_layers == other.top_shell_layers
            && self.bottom_shell_layers == other.bottom_shell_layers
            && self.top_fill_holder == other.top_fill_holder
            && self.bottom_fill_holder == other.bottom_fill_holder
            && self.bridge_fill_holder == other.bridge_fill_holder
            && self.sparse_fill_holder == other.sparse_fill_holder
            && self.gcode_resolution.to_bits() == other.gcode_resolution.to_bits()
            && self.infill_resolution.to_bits() == other.infill_resolution.to_bits()
            && self.support_resolution.to_bits() == other.support_resolution.to_bits()
            && self.min_segment_length.to_bits() == other.min_segment_length.to_bits()
            && self.gcode_xy_decimals == other.gcode_xy_decimals
            && self.perimeter_arc_tolerance.to_bits() == other.perimeter_arc_tolerance.to_bits()
            && self.slice_closing_radius.to_bits() == other.slice_closing_radius.to_bits()
            && self.flat_bridge_closing_join == other.flat_bridge_closing_join
            && self.support_enabled == other.support_enabled
            && self.support_type == other.support_type
            && self.support_overhang_angle.to_bits() == other.support_overhang_angle.to_bits()
            && self.support_layer_height_mm.to_bits() == other.support_layer_height_mm.to_bits()
            && self.nonplanar_max_angle_deg.map(|f| f.to_bits())
                == other.nonplanar_max_angle_deg.map(|f| f.to_bits())
            && self.nonplanar_shell_count == other.nonplanar_shell_count
            && self.nonplanar_amplitude.map(|f| f.to_bits())
                == other.nonplanar_amplitude.map(|f| f.to_bits())
            && self.smoothificator_target_height.map(|f| f.to_bits())
                == other.smoothificator_target_height.map(|f| f.to_bits())
            && self.smoothificator_adaptive == other.smoothificator_adaptive
            && self.bed_shape.len() == other.bed_shape.len()
            && self
                .bed_shape
                .iter()
                .zip(other.bed_shape.iter())
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && self.retract_length.to_bits() == other.retract_length.to_bits()
            && self.wipe_tower_enabled == other.wipe_tower_enabled
            && self.mmu_segmented_region_max_width.to_bits()
                == other.mmu_segmented_region_max_width.to_bits()
            && self.mmu_segmented_region_interlocking_depth.to_bits()
                == other.mmu_segmented_region_interlocking_depth.to_bits()
            && self.mmu_segmented_region_interlocking_beam
                == other.mmu_segmented_region_interlocking_beam
            && self.machine_max_acceleration_extruding.map(f32::to_bits)
                == other.machine_max_acceleration_extruding.map(f32::to_bits)
            && self.machine_max_acceleration_travel.map(f32::to_bits)
                == other.machine_max_acceleration_travel.map(f32::to_bits)
            && self.machine_max_speed_x.map(f32::to_bits)
                == other.machine_max_speed_x.map(f32::to_bits)
            && self.machine_max_speed_y.map(f32::to_bits)
                == other.machine_max_speed_y.map(f32::to_bits)
            && self.machine_max_speed_z.map(f32::to_bits)
                == other.machine_max_speed_z.map(f32::to_bits)
            && self.machine_max_speed_e.map(f32::to_bits)
                == other.machine_max_speed_e.map(f32::to_bits)
            && self.machine_max_jerk_x.map(f32::to_bits)
                == other.machine_max_jerk_x.map(f32::to_bits)
            && self.machine_max_jerk_y.map(f32::to_bits)
                == other.machine_max_jerk_y.map(f32::to_bits)
            && self.machine_max_jerk_z.map(f32::to_bits)
                == other.machine_max_jerk_z.map(f32::to_bits)
            && self.machine_max_jerk_e.map(f32::to_bits)
                == other.machine_max_jerk_e.map(f32::to_bits)
            && self
                .filament_density
                .iter()
                .map(|d| d.to_bits())
                .eq(other.filament_density.iter().map(|d| d.to_bits()))
            && self.extensions == other.extensions
    }
}

impl Eq for ResolvedConfig {}

/// # Hash consistency note
///
/// Hash is consistent within one process; not portable across architectures
/// with differing NaN bit patterns. f32/f64 fields are hashed via `to_bits()`
/// so that `a == b → hash(a) == hash(b)` holds.
impl std::hash::Hash for ResolvedConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.layer_height.to_bits().hash(state);
        self.line_width.to_bits().hash(state);
        self.first_layer_height.to_bits().hash(state);
        self.first_layer_line_width.to_bits().hash(state);
        self.filament_diameter.to_bits().hash(state);
        self.wall_count.hash(state);
        self.outer_wall_speed.to_bits().hash(state);
        self.inner_wall_speed.to_bits().hash(state);
        self.wall_generator.hash(state);
        self.arachne_min_feature_size
            .map(|f| f.to_bits())
            .hash(state);
        self.infill_type.hash(state);
        self.infill_density.to_bits().hash(state);
        self.infill_overlap.to_bits().hash(state);
        self.infill_angle.to_bits().hash(state);
        self.infill_speed.to_bits().hash(state);
        self.solid_infill_speed.to_bits().hash(state);
        self.top_shell_layers.hash(state);
        self.bottom_shell_layers.hash(state);
        self.top_fill_holder.hash(state);
        self.bottom_fill_holder.hash(state);
        self.bridge_fill_holder.hash(state);
        self.sparse_fill_holder.hash(state);
        self.gcode_resolution.to_bits().hash(state);
        self.infill_resolution.to_bits().hash(state);
        self.support_resolution.to_bits().hash(state);
        self.min_segment_length.to_bits().hash(state);
        self.gcode_xy_decimals.hash(state);
        self.perimeter_arc_tolerance.to_bits().hash(state);
        self.slice_closing_radius.to_bits().hash(state);
        self.flat_bridge_closing_join.hash(state);
        self.support_enabled.hash(state);
        self.support_type.hash(state);
        self.support_overhang_angle.to_bits().hash(state);
        self.support_layer_height_mm.to_bits().hash(state);
        self.nonplanar_max_angle_deg
            .map(|f| f.to_bits())
            .hash(state);
        self.nonplanar_shell_count.hash(state);
        self.nonplanar_amplitude.map(|f| f.to_bits()).hash(state);
        self.smoothificator_target_height
            .map(|f| f.to_bits())
            .hash(state);
        self.smoothificator_adaptive.hash(state);
        self.bed_shape.len().hash(state);
        for f in &self.bed_shape {
            f.to_bits().hash(state);
        }
        self.retract_length.to_bits().hash(state);
        self.wipe_tower_enabled.hash(state);
        self.mmu_segmented_region_max_width.to_bits().hash(state);
        self.mmu_segmented_region_interlocking_depth
            .to_bits()
            .hash(state);
        self.mmu_segmented_region_interlocking_beam.hash(state);
        self.machine_max_acceleration_extruding
            .map(f32::to_bits)
            .hash(state);
        self.machine_max_acceleration_travel
            .map(f32::to_bits)
            .hash(state);
        self.machine_max_speed_x.map(f32::to_bits).hash(state);
        self.machine_max_speed_y.map(f32::to_bits).hash(state);
        self.machine_max_speed_z.map(f32::to_bits).hash(state);
        self.machine_max_speed_e.map(f32::to_bits).hash(state);
        self.machine_max_jerk_x.map(f32::to_bits).hash(state);
        self.machine_max_jerk_y.map(f32::to_bits).hash(state);
        self.machine_max_jerk_z.map(f32::to_bits).hash(state);
        self.machine_max_jerk_e.map(f32::to_bits).hash(state);
        for density in &self.filament_density {
            density.to_bits().hash(state);
        }
        self.extensions.hash(state);
    }
}

#[cfg(test)]
mod machine_limit_config_tests {
    use super::*;

    const KEYS: [&str; 10] = [
        "machine_max_acceleration_extruding",
        "machine_max_acceleration_travel",
        "machine_max_speed_x",
        "machine_max_speed_y",
        "machine_max_speed_z",
        "machine_max_speed_e",
        "machine_max_jerk_x",
        "machine_max_jerk_y",
        "machine_max_jerk_z",
        "machine_max_jerk_e",
    ];

    fn field(cfg: &ResolvedConfig, key: &str) -> Option<f32> {
        match key {
            "machine_max_acceleration_extruding" => cfg.machine_max_acceleration_extruding,
            "machine_max_acceleration_travel" => cfg.machine_max_acceleration_travel,
            "machine_max_speed_x" => cfg.machine_max_speed_x,
            "machine_max_speed_y" => cfg.machine_max_speed_y,
            "machine_max_speed_z" => cfg.machine_max_speed_z,
            "machine_max_speed_e" => cfg.machine_max_speed_e,
            "machine_max_jerk_x" => cfg.machine_max_jerk_x,
            "machine_max_jerk_y" => cfg.machine_max_jerk_y,
            "machine_max_jerk_z" => cfg.machine_max_jerk_z,
            "machine_max_jerk_e" => cfg.machine_max_jerk_e,
            other => panic!("unknown key {other}"),
        }
    }

    #[test]
    fn absent_machine_limit_keys_are_none_and_omitted_from_config_map() {
        let cfg = ResolvedConfig::default();
        let map = cfg.to_config_map();
        for key in KEYS {
            assert_eq!(field(&cfg, key), None, "{key} default must be None");
            assert!(!map.contains_key(key), "{key} must be omitted when None");
        }
    }

    #[test]
    fn supplied_machine_limit_keys_round_trip() {
        let mut cfg = ResolvedConfig::default();
        for (i, key) in KEYS.iter().enumerate() {
            let v = 10.0 + i as f64;
            let applied = cfg
                .apply_cli_key(key, &ConfigValue::Float(v))
                .expect("type check");
            assert!(applied, "{key} must be a recognized CLI-bound field");
            assert_eq!(field(&cfg, key), Some(v as f32), "{key} value must apply");
        }
        let map = cfg.to_config_map();
        for (i, key) in KEYS.iter().enumerate() {
            let expected = f64::from(10.0_f32 + i as f32);
            assert_eq!(
                map.get(*key),
                Some(&ConfigValue::Float(expected)),
                "{key} must round-trip through to_config_map"
            );
        }
    }

    /// Orca writes `machine_max_*` as `coFloats`, and a real project's
    /// `project_settings.config` stores them as JSON arrays of strings — e.g.
    /// `resources/cube_4color.3mf` carries `"machine_max_jerk_x": ["9","9"]`.
    /// Before this was handled, every such slice aborted with `TypeMismatch`
    /// (`expected Float value, got List`), which crashed all four painted /
    /// modifier e2e fixtures. The two entries are machine *time modes*
    /// (normal, stealth), so index 0 is the value canonical consumers use.
    #[test]
    fn machine_limit_accepts_orca_string_list_and_takes_normal_mode() {
        let mut cfg = ResolvedConfig::default();
        let applied = cfg
            .apply_cli_key(
                "machine_max_jerk_x",
                &ConfigValue::List(vec![
                    ConfigValue::String("9".to_string()),
                    ConfigValue::String("5".to_string()),
                ]),
            )
            .expect("Orca's list-of-strings form must be accepted");
        assert!(applied, "machine_max_jerk_x must be a recognized field");
        assert_eq!(
            field(&cfg, "machine_max_jerk_x"),
            Some(9.0),
            "index 0 is normal mode; the stealth entry must be ignored"
        );
    }

    #[test]
    fn machine_limit_accepts_numeric_list_and_bare_scalar() {
        let mut cfg = ResolvedConfig::default();
        cfg.apply_cli_key(
            "machine_max_speed_e",
            &ConfigValue::List(vec![ConfigValue::Int(120), ConfigValue::Int(60)]),
        )
        .expect("numeric list must be accepted");
        assert_eq!(field(&cfg, "machine_max_speed_e"), Some(120.0));

        cfg.apply_cli_key("machine_max_speed_x", &ConfigValue::Float(500.0))
            .expect("bare scalar must still be accepted");
        assert_eq!(field(&cfg, "machine_max_speed_x"), Some(500.0));
    }

    /// Accepting Orca's list shape must not turn into accepting rubbish:
    /// unlike canonical (which substitutes 0.0 then falls back to a default),
    /// this port keeps malformed machine limits a hard error.
    #[test]
    fn machine_limit_rejects_unparseable_and_empty_values() {
        let mut cfg = ResolvedConfig::default();
        assert!(
            cfg.apply_cli_key(
                "machine_max_jerk_y",
                &ConfigValue::List(vec![ConfigValue::String("fast".to_string())]),
            )
            .is_err(),
            "a non-numeric string must not be silently accepted"
        );
        assert!(
            cfg.apply_cli_key("machine_max_jerk_z", &ConfigValue::List(vec![]))
                .is_err(),
            "an empty list has no normal-mode entry to read"
        );
        assert!(
            cfg.apply_cli_key("machine_max_jerk_e", &ConfigValue::Bool(true))
                .is_err(),
            "a bool is not a machine limit"
        );
    }
}
