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
    /// Speed for internal bridging.
    pub internal_bridge_speed: f32,
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
            internal_bridge_speed: 37.5,
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

/// Reads a single mm/s speed from a raw config source.
///
/// Accepts a plain `Float`/`Int`, a `List` whose first element is numeric
/// (Orca stores some per-filament speeds as `coFloats` arrays), and a
/// non-percent `FloatOrPercent`. Anything else (including a `Percent`, which
/// cannot be resolved without a base here) returns `None` so the caller keeps
/// its default.
fn read_speed(
    config: &std::collections::HashMap<String, crate::ConfigValue>,
    key: &str,
) -> Option<f32> {
    fn as_number(value: &crate::ConfigValue) -> Option<f32> {
        match value {
            crate::ConfigValue::Float(v) => Some(*v as f32),
            crate::ConfigValue::Int(v) => Some(*v as f32),
            crate::ConfigValue::FloatOrPercent {
                value,
                is_percent: false,
            } => Some(*value as f32),
            _ => None,
        }
    }
    match config.get(key)? {
        crate::ConfigValue::List(items) => items.iter().find_map(as_number),
        other => as_number(other),
    }
}

/// Every host speed key, paired with the [`FeedrateConfig`] field it fills.
///
/// Single source for both directions: [`FeedrateConfig::from_raw_config`]
/// reads through it, and `module config-schema` reports it as part of the
/// `host` key universe so the GUI can bind these keys (ticket 02). All are
/// `float`, mm/s, and print-scoped.
pub const SPEED_KEYS: &[(&str, fn(&mut FeedrateConfig) -> &mut f32)] = &[
    ("outer_wall_speed", |fc| &mut fc.outer_wall_speed),
    ("inner_wall_speed", |fc| &mut fc.inner_wall_speed),
    ("thin_wall_speed", |fc| &mut fc.thin_wall_speed),
    ("top_surface_speed", |fc| &mut fc.top_surface_speed),
    ("bottom_surface_speed", |fc| &mut fc.bottom_surface_speed),
    ("sparse_infill_speed", |fc| &mut fc.sparse_infill_speed),
    ("bridge_speed", |fc| &mut fc.bridge_speed),
    ("internal_bridge_speed", |fc| &mut fc.internal_bridge_speed),
    ("support_speed", |fc| &mut fc.support_speed),
    ("support_interface_speed", |fc| &mut fc.support_interface_speed),
    ("gap_infill_speed", |fc| &mut fc.gap_infill_speed),
    ("ironing_speed", |fc| &mut fc.ironing_speed),
    ("skirt_speed", |fc| &mut fc.skirt_speed),
    ("wipe_tower_speed", |fc| &mut fc.wipe_tower_speed),
    ("prime_tower_speed", |fc| &mut fc.prime_tower_speed),
    ("travel_speed", |fc| &mut fc.travel_speed),
    ("travel_speed_z", |fc| &mut fc.travel_speed_z),
    ("initial_layer_speed", |fc| &mut fc.initial_layer_speed),
    ("initial_layer_infill_speed", |fc| &mut fc.initial_layer_infill_speed),
    ("initial_layer_travel_speed", |fc| &mut fc.initial_layer_travel_speed),
    ("wipe_speed", |fc| &mut fc.wipe_speed),
    ("overhang_1_4_speed", |fc| &mut fc.overhang_1_4_speed),
    ("overhang_2_4_speed", |fc| &mut fc.overhang_2_4_speed),
    ("overhang_3_4_speed", |fc| &mut fc.overhang_3_4_speed),
    ("overhang_4_4_speed", |fc| &mut fc.overhang_4_4_speed),
    ("filament_ironing_speed", |fc| &mut fc.filament_ironing_speed),
];

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
                *field(&mut fc) = value;
            }
        }
        fc
    }
}
