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
        m.insert(
            "layer_height".into(),
            ConfigValue::Float(f64::from(self.layer_height)),
        );
        m.insert(
            "line_width".into(),
            ConfigValue::Float(f64::from(self.line_width)),
        );
        m.insert(
            "first_layer_height".into(),
            ConfigValue::Float(f64::from(self.first_layer_height)),
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
            "support_enabled".into(),
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
        // Merge extension keys (module-contributed, already in ConfigValue form).
        for (k, v) in &self.extensions {
            m.insert(k.clone(), v.clone());
        }
        m
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
    }
}

/// Extract an `f32` from a `Float`/`Int` `ConfigValue`. Used by the
/// [`declare_resolved_config!`] macro expansion.
#[doc(hidden)]
pub fn extract_float(key: &str, value: &ConfigValue) -> Result<f32, ConfigResolutionError> {
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

/// Extract a `u32` from an `Int` `ConfigValue`.
#[doc(hidden)]
pub fn extract_int_as_u32(key: &str, value: &ConfigValue) -> Result<u32, ConfigResolutionError> {
    match value {
        ConfigValue::Int(i) => Ok(*i as u32),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Int",
            actual: variant_name(other),
        }),
    }
}

/// Extract a `bool` from a `Bool` `ConfigValue`.
#[doc(hidden)]
pub fn extract_bool(key: &str, value: &ConfigValue) -> Result<bool, ConfigResolutionError> {
    match value {
        ConfigValue::Bool(b) => Ok(*b),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "Bool",
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
    match value {
        ConfigValue::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                ConfigValue::Float(f) => Ok(*f),
                ConfigValue::Int(n) => Ok(*n as f64),
                other => Err(ConfigResolutionError::TypeMismatch {
                    key: format!("{key}[{i}]"),
                    expected: "Float",
                    actual: variant_name(other),
                }),
            })
            .collect(),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: key.to_string(),
            expected: "List",
            actual: variant_name(other),
        }),
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
    cli "layer_height"           layer_height: f32 = 0.2 => extract_float;
    /// Line width in millimeters.
    cli "line_width"             line_width: f32 = 0.4 => extract_float;
    /// First layer height in millimeters.
    cli "first_layer_height"     first_layer_height: f32 = 0.2 => extract_float;
    /// First layer line width in millimeters.
    cli "first_layer_line_width" first_layer_line_width: f32 = 0.4 => extract_float;

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
    plain                        top_fill_holder: String = String::from("rectilinear-infill");
    /// Module ID holding `claim:bottom-fill`.
    plain                        bottom_fill_holder: String = String::from("rectilinear-infill");
    /// Module ID holding `claim:bridge-fill`.
    plain                        bridge_fill_holder: String = String::from("rectilinear-infill");
    /// Module ID holding `claim:sparse-fill`.
    plain                        sparse_fill_holder: String = String::from("rectilinear-infill");

    // Precision / resolution
    /// G-code path resolution in mm (OrcaSlicer: gcode_resolution).
    cli "gcode_resolution"       gcode_resolution: f32 = 0.0125 => extract_float;
    /// Infill path resolution in mm (OrcaSlicer: infill_anchor_max).
    cli "infill_resolution"      infill_resolution: f32 = 0.04 => extract_float;
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

    // Support
    /// Whether support is enabled.
    cli "support_enabled"        support_enabled: bool = false => extract_bool;
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
        // f32 fields compared via to_bits() so that Eq and Hash are consistent.
        self.layer_height.to_bits() == other.layer_height.to_bits()
            && self.line_width.to_bits() == other.line_width.to_bits()
            && self.first_layer_height.to_bits() == other.first_layer_height.to_bits()
            && self.first_layer_line_width.to_bits() == other.first_layer_line_width.to_bits()
            && self.wall_count == other.wall_count
            && self.outer_wall_speed.to_bits() == other.outer_wall_speed.to_bits()
            && self.inner_wall_speed.to_bits() == other.inner_wall_speed.to_bits()
            && self.wall_generator == other.wall_generator
            && self.arachne_min_feature_size.map(|f| f.to_bits())
                == other.arachne_min_feature_size.map(|f| f.to_bits())
            && self.infill_type == other.infill_type
            && self.infill_density.to_bits() == other.infill_density.to_bits()
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
        self.wall_count.hash(state);
        self.outer_wall_speed.to_bits().hash(state);
        self.inner_wall_speed.to_bits().hash(state);
        self.wall_generator.hash(state);
        self.arachne_min_feature_size
            .map(|f| f.to_bits())
            .hash(state);
        self.infill_type.hash(state);
        self.infill_density.to_bits().hash(state);
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
        self.extensions.hash(state);
    }
}
