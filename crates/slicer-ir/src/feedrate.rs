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
