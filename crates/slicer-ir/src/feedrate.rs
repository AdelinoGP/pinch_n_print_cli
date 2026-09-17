use crate::resolved_config::{HostKeyMeta, ResolvedFloatOrPercent};

/// `HostKeyMeta::NONE` as a *value*: the FRU base for annotated entries in
/// [`SPEED_META`].
const HOST_META_NONE: HostKeyMeta = HostKeyMeta::NONE;

/// Feedrate configuration holding mm/s speed values.
#[derive(Debug, Clone)]
pub struct FeedrateConfig {
    /// Speed for outer walls.
    pub outer_wall_speed: f32,
    /// Speed for inner walls.
    pub inner_wall_speed: f32,
    /// Speed for thin walls.
    pub thin_wall_speed: f32,
    /// Speed for top solid infill.
    pub top_surface_speed: f32,
    /// Speed for bottom solid infill.
    pub bottom_surface_speed: f32,
    /// Speed for sparse infill.
    pub sparse_infill_speed: f32,
    /// Speed for bridging.
    pub bridge_speed: f32,
    /// Speed for internal bridging, absolute or as a percentage of
    /// `bridge_speed`.
    pub internal_bridge_speed: ResolvedFloatOrPercent,
    /// Speed for support material.
    pub support_speed: f32,
    /// Speed for support interface.
    pub support_interface_speed: f32,
    /// Speed for gap infill.
    pub gap_infill_speed: f32,
    /// Speed for ironing.
    pub ironing_speed: f32,
    /// Speed for skirt/brim.
    pub skirt_speed: f32,
    /// Speed for wipe tower.
    pub wipe_tower_speed: f32,
    /// Speed for prime tower.
    pub prime_tower_speed: f32,
    /// Speed for non-printing travel moves.
    pub travel_speed: f32,
    /// Speed for Z-hop moves (if different from XY).
    pub travel_speed_z: f32,
    /// Base speed for initial layer.
    pub initial_layer_speed: f32,
    /// Infill speed for initial layer.
    pub initial_layer_infill_speed: f32,
    /// Travel speed for initial layer.
    pub initial_layer_travel_speed: f32,
    /// Speed for wipe moves.
    pub wipe_speed: f32,
    /// Speed for overhang 1/4.
    pub overhang_1_4_speed: f32,
    /// Speed for overhang 2/4.
    pub overhang_2_4_speed: f32,
    /// Speed for overhang 3/4.
    pub overhang_3_4_speed: f32,
    /// Speed for overhang 4/4.
    pub overhang_4_4_speed: f32,
    /// Speed for filament ironing override.
    pub filament_ironing_speed: f32,
}

impl Default for FeedrateConfig {
    fn default() -> Self {
        Self {
            outer_wall_speed: 60.0,
            inner_wall_speed: 60.0,
            thin_wall_speed: 30.0,
            top_surface_speed: 100.0,
            bottom_surface_speed: 100.0,
            sparse_infill_speed: 100.0,
            bridge_speed: 25.0,
            internal_bridge_speed: ResolvedFloatOrPercent {
                value: 37.5,
                is_percent: false,
            },
            support_speed: 80.0,
            support_interface_speed: 80.0,
            gap_infill_speed: 30.0,
            ironing_speed: 20.0,
            skirt_speed: 50.0,
            wipe_tower_speed: 90.0,
            prime_tower_speed: 90.0,
            travel_speed: 120.0,
            travel_speed_z: 0.0,
            initial_layer_speed: 30.0,
            initial_layer_infill_speed: 60.0,
            initial_layer_travel_speed: 120.0,
            wipe_speed: 96.0,
            overhang_1_4_speed: 0.0,
            overhang_2_4_speed: 0.0,
            overhang_3_4_speed: 0.0,
            overhang_4_4_speed: 0.0,
            filament_ironing_speed: 0.0,
        }
    }
}

/// A mutable feedrate field exposed through [`SPEED_KEYS`].
pub trait FeedrateField {
    /// Return the field's schema wire type.
    fn wire_type(&self) -> &'static str;

    /// Render the field's default for a host declaration.
    fn default_string(&self) -> String;

    /// Apply a raw speed value, preserving relative-value state where the
    /// field supports it.
    fn apply_raw_value(&mut self, value: ResolvedFloatOrPercent);
}

impl FeedrateField for f32 {
    fn wire_type(&self) -> &'static str {
        "float"
    }

    fn default_string(&self) -> String {
        self.to_string()
    }

    fn apply_raw_value(&mut self, value: ResolvedFloatOrPercent) {
        if !value.is_percent {
            *self = value.value as f32;
        }
    }
}

impl FeedrateField for ResolvedFloatOrPercent {
    fn wire_type(&self) -> &'static str {
        "float_or_percent"
    }

    fn default_string(&self) -> String {
        if self.is_percent {
            format!("{}%", self.value)
        } else {
            self.value.to_string()
        }
    }

    fn apply_raw_value(&mut self, value: ResolvedFloatOrPercent) {
        *self = value;
    }
}

/// Resolve internal-bridge speed to millimetres per second.
///
/// Relative values use the canonical `bridge_speed` base; absolute values are
/// returned unchanged.
#[must_use]
pub fn resolve_internal_bridge_speed_mm(value: ResolvedFloatOrPercent, bridge_speed: f32) -> f32 {
    if value.is_percent {
        value.value as f32 / 100.0 * bridge_speed
    } else {
        value.value as f32
    }
}

/// Reads a single mm/s speed from a raw config source.
///
/// Accepts a plain `Float`/`Int`, a `List` whose first element is numeric
/// (Orca stores some per-filament speeds as `coFloats` arrays), and a
/// `FloatOrPercent`, retaining its percent bit for the one speed field that
/// resolves against another speed. Anything else (including a `Percent`, which
/// cannot be resolved without a base here) returns `None` so the caller keeps
/// its default.
fn read_speed(
    config: &std::collections::HashMap<String, crate::ConfigValue>,
    key: &str,
) -> Option<ResolvedFloatOrPercent> {
    fn as_number(value: &crate::ConfigValue) -> Option<ResolvedFloatOrPercent> {
        match value {
            crate::ConfigValue::Float(value) => Some(ResolvedFloatOrPercent {
                value: *value,
                is_percent: false,
            }),
            crate::ConfigValue::Int(value) => Some(ResolvedFloatOrPercent {
                value: *value as f64,
                is_percent: false,
            }),
            crate::ConfigValue::FloatOrPercent { value, is_percent } => {
                Some(ResolvedFloatOrPercent {
                    value: *value,
                    is_percent: *is_percent,
                })
            }
            _ => None,
        }
    }
    match config.get(key)? {
        crate::ConfigValue::List(items) => items.iter().find_map(as_number),
        other => as_number(other),
    }
}

/// Display metadata for the host speed keys that carry any (wire 1.2.0,
/// SchemaBridgeMap ticket 10), positionally aligned with [`SPEED_KEYS`].
///
/// `None` means "no metadata": `module config-schema` reports `null` and the
/// GUI falls back to the raw key name. Identity-routed speeds are listed when
/// their host wire type needs an explicit override. `min = 0.0` encodes
/// `docs/config/host-keys.toml`'s speed ranges (`"> 0"`) as an inclusive
/// floor, because the GUI clamps at `min` rather than rejecting; the strict
/// inequality stays prose in the toml.
pub const SPEED_META: [Option<HostKeyMeta>; SPEED_KEY_COUNT] = [
    None, // outer_wall_speed (Orca identity)
    None, // inner_wall_speed (Orca identity)
    Some(HostKeyMeta {
        // thin_wall_speed
        display: Some("Thin Wall Speed"),
        description: Some("Speed used to fill thin-wall regions the perimeter pass cannot (mm/s)."),
        group: Some("Speed"),
        unit: Some("mm/s"),
        min: Some(0.0),
        ..HOST_META_NONE
    }),
    None, // top_surface_speed (Orca identity)
    Some(HostKeyMeta {
        // bottom_surface_speed
        display: Some("Bottom surface speed"),
        description: Some("Speed for bottom solid infill surfaces (mm/s)."),
        group: Some("Speed"),
        unit: Some("mm/s"),
        min: Some(0.0),
        ..HOST_META_NONE
    }),
    None, // sparse_infill_speed (Orca identity)
    None, // bridge_speed (Orca identity)
    None, // internal_bridge_speed (Orca identity)
    None, // support_speed (Orca identity)
    None, // support_interface_speed (Orca identity)
    None, // gap_infill_speed (Orca identity)
    None, // ironing_speed (Orca identity)
    None, // skirt_speed (Orca identity)
    Some(HostKeyMeta {
        // wipe_tower_speed
        display: Some("Wipe tower speed"),
        description: Some("Speed inside the wipe/prime tower (mm/s)."),
        group: Some("Wipe Tower"),
        unit: Some("mm/s"),
        min: Some(0.0),
        ..HOST_META_NONE
    }),
    Some(HostKeyMeta {
        // prime_tower_speed
        display: Some("Prime tower speed"),
        description: Some("Speed for prime-tower purge moves (mm/s)."),
        group: Some("Multimaterial"),
        unit: Some("mm/s"),
        min: Some(0.0),
        ..HOST_META_NONE
    }),
    None, // travel_speed (Orca identity)
    None, // travel_speed_z (Orca identity)
    None, // initial_layer_speed (Orca identity)
    None, // initial_layer_infill_speed (Orca identity)
    None, // initial_layer_travel_speed (Orca identity)
    None, // wipe_speed (Orca identity)
    Some(HostKeyMeta {
        // overhang_1_4_speed
        wire_type: Some("float_or_percent"),
        ..HOST_META_NONE
    }),
    Some(HostKeyMeta {
        // overhang_2_4_speed
        wire_type: Some("float_or_percent"),
        ..HOST_META_NONE
    }),
    Some(HostKeyMeta {
        // overhang_3_4_speed
        wire_type: Some("float_or_percent"),
        ..HOST_META_NONE
    }),
    Some(HostKeyMeta {
        // overhang_4_4_speed
        wire_type: Some("float_or_percent"),
        ..HOST_META_NONE
    }),
    None, // filament_ironing_speed (Orca identity)
];

/// `SPEED_META` must stay positionally aligned with [`SPEED_KEYS`]; this
/// assertion fails the build when the two arrays' lengths drift apart.
const _: () = assert!(SPEED_KEYS.len() == SPEED_META.len());

/// Every host speed key, paired with the [`FeedrateConfig`] field it fills.
///
/// Single source for both directions: [`FeedrateConfig::from_raw_config`]
/// reads through it, and `module config-schema` reports it as part of the
/// `host` key universe so the GUI can bind these keys (ticket 02). All are
/// print-scoped and use mm/s-compatible values; `internal_bridge_speed` is
/// `float_or_percent` and resolves against `bridge_speed`. [`SPEED_META`]
/// carries the wire 1.2.0 display metadata (SchemaBridgeMap ticket 10) for
/// the speeds rendered as their own controls rather than bound to an Orca
/// identity row.
pub const SPEED_KEYS: &[(&str, fn(&mut FeedrateConfig) -> &mut dyn FeedrateField)] = &[
    ("outer_wall_speed", |fc| &mut fc.outer_wall_speed),
    ("inner_wall_speed", |fc| &mut fc.inner_wall_speed),
    ("thin_wall_speed", |fc| &mut fc.thin_wall_speed),
    ("top_surface_speed", |fc| &mut fc.top_surface_speed),
    ("bottom_surface_speed", |fc| &mut fc.bottom_surface_speed),
    ("sparse_infill_speed", |fc| &mut fc.sparse_infill_speed),
    ("bridge_speed", |fc| &mut fc.bridge_speed),
    ("internal_bridge_speed", |fc| &mut fc.internal_bridge_speed),
    ("support_speed", |fc| &mut fc.support_speed),
    ("support_interface_speed", |fc| {
        &mut fc.support_interface_speed
    }),
    ("gap_infill_speed", |fc| &mut fc.gap_infill_speed),
    ("ironing_speed", |fc| &mut fc.ironing_speed),
    ("skirt_speed", |fc| &mut fc.skirt_speed),
    ("wipe_tower_speed", |fc| &mut fc.wipe_tower_speed),
    ("prime_tower_speed", |fc| &mut fc.prime_tower_speed),
    ("travel_speed", |fc| &mut fc.travel_speed),
    ("travel_speed_z", |fc| &mut fc.travel_speed_z),
    ("initial_layer_speed", |fc| &mut fc.initial_layer_speed),
    ("initial_layer_infill_speed", |fc| {
        &mut fc.initial_layer_infill_speed
    }),
    ("initial_layer_travel_speed", |fc| {
        &mut fc.initial_layer_travel_speed
    }),
    ("wipe_speed", |fc| &mut fc.wipe_speed),
    ("overhang_1_4_speed", |fc| &mut fc.overhang_1_4_speed),
    ("overhang_2_4_speed", |fc| &mut fc.overhang_2_4_speed),
    ("overhang_3_4_speed", |fc| &mut fc.overhang_3_4_speed),
    ("overhang_4_4_speed", |fc| &mut fc.overhang_4_4_speed),
    ("filament_ironing_speed", |fc| {
        &mut fc.filament_ironing_speed
    }),
];

/// Number of entries in [`SPEED_KEYS`] (and therefore in [`SPEED_META`]),
/// spelled out so the meta table can be typed against it before the const
/// assertion below compares the two.
pub const SPEED_KEY_COUNT: usize = SPEED_KEYS.len();

impl FeedrateConfig {
    /// Builds the feedrate table from a raw config source keyed by the
    /// `[speeds]` host names (the Orca key names the GUI's translated config
    /// uses; all mm/s). Keys that are absent or not numeric keep the
    /// [`FeedrateConfig::default`] value, so `docs/config/host-keys.toml`'s
    /// `[speeds]` table stays the source of truth for the defaults.
    ///
    /// This is what wires host speeds into the G-code emitter: the emitter's
    /// `resolve_feedrate` previously read `FeedrateConfig::default()` on every
    /// run, so every F value in the G-code was a pnp default scaled by module
    /// speed factors.
    /// Builds the feedrate table from a raw config source keyed by the
    /// `[speeds]` host names (the Orca key names the GUI's translated config
    /// uses; all mm/s). Keys that are absent or not numeric keep the
    /// [`FeedrateConfig::default`] value, so `docs/config/host-keys.toml`'s
    /// `[speeds]` table stays the source of truth for the defaults.
    ///
    /// This is what wires host speeds into the G-code emitter: the emitter's
    /// `resolve_feedrate` previously read `FeedrateConfig::default()` on every
    /// run, so every F value in the G-code was a pnp default scaled by module
    /// speed factors.
    ///
    /// Driven by [`SPEED_KEYS`], which is also what `module config-schema`
    /// reports as the feedrate half of its `host` array — so a speed the
    /// slicer reads and a speed the GUI can bind cannot diverge.
    pub fn from_raw_config(config: &std::collections::HashMap<String, crate::ConfigValue>) -> Self {
        let mut fc = Self::default();
        for (key, field) in SPEED_KEYS {
            if let Some(value) = read_speed(config, key) {
                field(&mut fc).apply_raw_value(value);
            }
        }
        fc
    }
}
