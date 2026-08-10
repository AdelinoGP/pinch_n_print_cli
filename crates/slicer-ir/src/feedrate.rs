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
    pub fn from_raw_config(config: &std::collections::HashMap<String, crate::ConfigValue>) -> Self {
        let mut fc = Self::default();
        fc.outer_wall_speed = read_speed(config, "outer_wall_speed").unwrap_or(fc.outer_wall_speed);
        fc.inner_wall_speed = read_speed(config, "inner_wall_speed").unwrap_or(fc.inner_wall_speed);
        fc.thin_wall_speed = read_speed(config, "thin_wall_speed").unwrap_or(fc.thin_wall_speed);
        fc.top_surface_speed =
            read_speed(config, "top_surface_speed").unwrap_or(fc.top_surface_speed);
        fc.bottom_surface_speed =
            read_speed(config, "bottom_surface_speed").unwrap_or(fc.bottom_surface_speed);
        fc.sparse_infill_speed =
            read_speed(config, "sparse_infill_speed").unwrap_or(fc.sparse_infill_speed);
        fc.bridge_speed = read_speed(config, "bridge_speed").unwrap_or(fc.bridge_speed);
        fc.internal_bridge_speed =
            read_speed(config, "internal_bridge_speed").unwrap_or(fc.internal_bridge_speed);
        fc.support_speed = read_speed(config, "support_speed").unwrap_or(fc.support_speed);
        fc.support_interface_speed =
            read_speed(config, "support_interface_speed").unwrap_or(fc.support_interface_speed);
        fc.gap_infill_speed = read_speed(config, "gap_infill_speed").unwrap_or(fc.gap_infill_speed);
        fc.ironing_speed = read_speed(config, "ironing_speed").unwrap_or(fc.ironing_speed);
        fc.skirt_speed = read_speed(config, "skirt_speed").unwrap_or(fc.skirt_speed);
        fc.wipe_tower_speed = read_speed(config, "wipe_tower_speed").unwrap_or(fc.wipe_tower_speed);
        fc.prime_tower_speed =
            read_speed(config, "prime_tower_speed").unwrap_or(fc.prime_tower_speed);
        fc.travel_speed = read_speed(config, "travel_speed").unwrap_or(fc.travel_speed);
        fc.travel_speed_z = read_speed(config, "travel_speed_z").unwrap_or(fc.travel_speed_z);
        fc.initial_layer_speed =
            read_speed(config, "initial_layer_speed").unwrap_or(fc.initial_layer_speed);
        fc.initial_layer_infill_speed = read_speed(config, "initial_layer_infill_speed")
            .unwrap_or(fc.initial_layer_infill_speed);
        fc.initial_layer_travel_speed = read_speed(config, "initial_layer_travel_speed")
            .unwrap_or(fc.initial_layer_travel_speed);
        fc.wipe_speed = read_speed(config, "wipe_speed").unwrap_or(fc.wipe_speed);
        fc.overhang_1_4_speed =
            read_speed(config, "overhang_1_4_speed").unwrap_or(fc.overhang_1_4_speed);
        fc.overhang_2_4_speed =
            read_speed(config, "overhang_2_4_speed").unwrap_or(fc.overhang_2_4_speed);
        fc.overhang_3_4_speed =
            read_speed(config, "overhang_3_4_speed").unwrap_or(fc.overhang_3_4_speed);
        fc.overhang_4_4_speed =
            read_speed(config, "overhang_4_4_speed").unwrap_or(fc.overhang_4_4_speed);
        fc.filament_ironing_speed =
            read_speed(config, "filament_ironing_speed").unwrap_or(fc.filament_ironing_speed);
        fc
    }
}
