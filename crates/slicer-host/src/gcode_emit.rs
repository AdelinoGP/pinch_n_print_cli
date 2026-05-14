//! GCode emission and serialization (TASK-034).
//!
//! This module provides the host-built-in implementations of `GCodeEmitter` and
//! `GCodeSerializer` traits defined in `postpass.rs`. The emitter converts
//! `LayerCollectionIR` to `GCodeIR`; the serializer converts `GCodeIR` to G-code text.
//!
//! Emit behavior (from docs/02_ir_schemas.md, docs/04_host_scheduler.md):
//! - Walk `LayerCollectionIR` in Z-sorted order (already sorted by LayerFinalization)
//! - Convert `PrintEntity.path` (ExtrusionPath3D) → `GCodeCommand::Move`
//! - Insert `GCodeCommand::ToolChange` where `ToolChange` appears
//! - Insert Z-hop travel moves where `ZHop` appears
//! - Build `PrintMetadata` (estimated time, filament used, layer count, slicer version)
//!
//! Serialize behavior:
//! - Convert `GCodeIR.commands` → text G-code string
//! - Extrusion moves emit `G1 X... Y... Z... E... F...`; travel moves
//!   (`ExtrusionRole::Custom("Travel")`) emit `G0 X... Y... Z... F...` with no `E`.
//! - Handle all GCodeCommand variants (Move, Retract, Unretract, FanSpeed, Temperature,
//!   ToolChange, Comment, Raw)
//!
//! OrcaSlicer references:
//! - OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp — G-code emission patterns
//! - OrcaSlicerDocumented/tests/fff_print/test_gcodewriter.cpp — test patterns

use std::collections::HashMap;
use std::fmt::Write;

use slicer_ir::{
    ConfigValue, ExtrusionRole, GCodeCommand, GCodeIR, LayerAnnotationKind, LayerCollectionIR,
    PrintMetadata, ResolvedConfig, SemVer,
};

use crate::{Blackboard, GCodeEmitter, GCodeSerializer, PostpassError};

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

/// Default GCode emitter (host-built-in).
///
/// Converts `LayerCollectionIR` to `GCodeIR` by walking layers in Z-sorted order,
/// converting print entities to move commands, and inserting tool changes and Z-hops.
pub struct DefaultGCodeEmitter {
    /// Slicer version string to include in metadata.
    slicer_version: String,
    /// Feedrate configuration.
    feedrate_config: FeedrateConfig,
}

impl DefaultGCodeEmitter {
    /// Creates a new `DefaultGCodeEmitter` with the given slicer version.
    pub fn new(slicer_version: String) -> Self {
        Self {
            slicer_version,
            feedrate_config: FeedrateConfig::default(),
        }
    }

    /// Creates a new `DefaultGCodeEmitter` with explicit configuration.
    pub fn new_with_config(slicer_version: String, feedrate_config: FeedrateConfig) -> Self {
        Self {
            slicer_version,
            feedrate_config,
        }
    }

    /// Resolves the feedrate (in mm/min) for a given extrusion role and speed factor multiplier.
    pub fn resolve_feedrate(&self, role: &ExtrusionRole, speed_factor: f32) -> Option<f32> {
        let base_speed = match role {
            ExtrusionRole::OuterWall => self.feedrate_config.outer_wall_speed,
            ExtrusionRole::InnerWall => self.feedrate_config.inner_wall_speed,
            ExtrusionRole::ThinWall => self.feedrate_config.thin_wall_speed,
            ExtrusionRole::TopSolidInfill => self.feedrate_config.top_surface_speed,
            ExtrusionRole::BottomSolidInfill => self.feedrate_config.bottom_surface_speed,
            ExtrusionRole::SparseInfill => self.feedrate_config.sparse_infill_speed,
            ExtrusionRole::BridgeInfill => self.feedrate_config.bridge_speed,
            ExtrusionRole::SupportMaterial => self.feedrate_config.support_speed,
            ExtrusionRole::SupportInterface => self.feedrate_config.support_interface_speed,
            ExtrusionRole::Skirt => self.feedrate_config.skirt_speed,
            ExtrusionRole::WipeTower => self.feedrate_config.wipe_tower_speed,
            ExtrusionRole::PrimeTower => self.feedrate_config.prime_tower_speed,
            ExtrusionRole::Ironing => {
                if self.feedrate_config.filament_ironing_speed > 0.0 {
                    self.feedrate_config.filament_ironing_speed
                } else {
                    self.feedrate_config.ironing_speed
                }
            }
            ExtrusionRole::Custom(s) => match s.as_str() {
                "Travel" => self.feedrate_config.travel_speed,
                "Wipe" => self.feedrate_config.wipe_speed,
                "GapInfill" => self.feedrate_config.gap_infill_speed,
                "InternalBridge" => self.feedrate_config.internal_bridge_speed,
                _ => self.feedrate_config.outer_wall_speed,
            },
        };

        let clamped_factor = speed_factor.clamp(0.05, 5.0);
        let f_value = base_speed * 60.0 * clamped_factor;
        let rounded = (f_value * 1000.0).round() / 1000.0;
        Some(rounded)
    }

    /// Returns the slicer version string.
    pub fn slicer_version(&self) -> &str {
        &self.slicer_version
    }
}

/// Returns true if two extrusion roles are functionally equal for ;TYPE: labeling.
fn role_equals(a: &ExtrusionRole, b: &ExtrusionRole) -> bool {
    match (a, b) {
        (ExtrusionRole::OuterWall, ExtrusionRole::OuterWall) => true,
        (ExtrusionRole::InnerWall, ExtrusionRole::InnerWall) => true,
        (ExtrusionRole::ThinWall, ExtrusionRole::ThinWall) => true,
        (ExtrusionRole::TopSolidInfill, ExtrusionRole::TopSolidInfill) => true,
        (ExtrusionRole::BottomSolidInfill, ExtrusionRole::BottomSolidInfill) => true,
        (ExtrusionRole::SparseInfill, ExtrusionRole::SparseInfill) => true,
        (ExtrusionRole::BridgeInfill, ExtrusionRole::BridgeInfill) => true,
        (ExtrusionRole::SupportMaterial, ExtrusionRole::SupportMaterial) => true,
        (ExtrusionRole::SupportInterface, ExtrusionRole::SupportInterface) => true,
        (ExtrusionRole::Skirt, ExtrusionRole::Skirt) => true,
        (ExtrusionRole::WipeTower, ExtrusionRole::WipeTower) => true,
        (ExtrusionRole::PrimeTower, ExtrusionRole::PrimeTower) => true,
        (ExtrusionRole::Ironing, ExtrusionRole::Ironing) => true,
        (ExtrusionRole::Custom(a_str), ExtrusionRole::Custom(b_str)) => a_str == b_str,
        _ => false,
    }
}

/// Returns the canonical OrcaSlicer ";TYPE:{label}" comment text for an extrusion role.
fn orca_type_label(role: &ExtrusionRole) -> &'static str {
    match role {
        ExtrusionRole::OuterWall => ";TYPE:Outer wall",
        ExtrusionRole::InnerWall => ";TYPE:Inner wall",
        ExtrusionRole::ThinWall => ";TYPE:Inner wall",
        ExtrusionRole::TopSolidInfill => ";TYPE:Top surface",
        ExtrusionRole::BottomSolidInfill => ";TYPE:Bottom surface",
        ExtrusionRole::SparseInfill => ";TYPE:Sparse infill",
        ExtrusionRole::BridgeInfill => ";TYPE:Bridge infill",
        ExtrusionRole::SupportMaterial => ";TYPE:Support",
        ExtrusionRole::SupportInterface => ";TYPE:Support interface",
        ExtrusionRole::Skirt => ";TYPE:Skirt/Brim",
        ExtrusionRole::WipeTower => ";TYPE:Wipe tower",
        ExtrusionRole::PrimeTower => ";TYPE:Prime tower",
        ExtrusionRole::Ironing => ";TYPE:Ironing",
        ExtrusionRole::Custom(_) => ";TYPE:Custom",
    }
}

impl GCodeEmitter for DefaultGCodeEmitter {
    fn emit_gcode(
        &self,
        layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        let layer_count = layer_irs.len() as u32;

        let mut commands = Vec::new();
        // Track filament used per tool (tool index -> filament mm)
        let mut filament_per_tool: HashMap<u32, f32> = HashMap::new();
        // Current tool (default 0)
        let mut current_tool: u32 = 0;
        // Cumulative E position
        let mut e_position: f32 = 0.0;

        // Previous layer Z for computing ;HEIGHT: delta
        let mut prev_layer_z: Option<f32> = None;
        // Track the last non-zero height delta (for first-layer fallback)
        let mut last_height_delta: f32 = 0.2;
        // Previous role for ;TYPE: emission
        let mut prev_role: Option<ExtrusionRole> = None;

        // Walk layers in order (already Z-sorted by LayerFinalization)
        for layer in layer_irs {
            let layer_z = layer.z;

            // Emit Orca layer-change headers BEFORE the first Move of this layer
            // Insert ;LAYER_CHANGE, ;Z:{z}, ;HEIGHT:{h} before the first command
            // Note: push bare text; serializer adds "; " prefix for regular comments.
            // Orca header lines are output via Raw so they go through verbatim.
            let height_delta = if let Some(prev_z) = prev_layer_z {
                layer_z - prev_z
            } else {
                last_height_delta
            };
            if prev_layer_z.is_some() {
                last_height_delta = height_delta;
            }
            prev_layer_z = Some(layer_z);

            commands.push(GCodeCommand::Raw {
                text: ";LAYER_CHANGE".to_string(),
            });
            commands.push(GCodeCommand::Raw {
                text: format!(";Z:{}", format_coord(layer_z)),
            });
            commands.push(GCodeCommand::Raw {
                text: format!(";HEIGHT:{}", format_coord(height_delta)),
            });

            // Cross-layer tool reset: path-optimization-default only records
            // intra-layer tool transitions, so layer N+1's first cluster
            // inherits whatever tool layer N ended on. Without this reset,
            // unpainted (T0) body extrusions are silently emitted under the
            // last painted tool of the previous layer. By host convention,
            // each ordered entity's `region_key.region_id` is its required
            // tool index (see layer_executor::assemble_ordered_entities and
            // path-optimization-default::tool_index_of). Emit a tool change
            // before the first entity whenever it differs from `current_tool`.
            if let Some(first_entity) = layer.ordered_entities.first() {
                let required_tool = first_entity.region_key.region_id as u32;
                if required_tool != current_tool {
                    commands.push(GCodeCommand::ToolChange {
                        after_entity_index: u32::MAX,
                        from: current_tool,
                        to: required_tool,
                    });
                    current_tool = required_tool;
                }
            }

            // Build lookup maps for tool_changes and z_hops by after_entity_index
            let tool_changes: HashMap<u32, &_> = layer
                .tool_changes
                .iter()
                .map(|tc| (tc.after_entity_index, tc))
                .collect();
            let z_hops: HashMap<u32, &_> = layer
                .z_hops
                .iter()
                .map(|zh| (zh.after_entity_index, zh))
                .collect();
            // retracts: per entity index, collect all in order (Retract entries first, Unretract entries last)
            let mut retracts_by_entity: std::collections::HashMap<
                u32,
                Vec<&slicer_ir::TravelRetract>,
            > = std::collections::HashMap::new();
            for r in &layer.retracts {
                retracts_by_entity
                    .entry(r.after_entity_index)
                    .or_default()
                    .push(r);
            }
            // travel_moves: per entity_id, collect all in order
            let mut travel_moves_by_entity: std::collections::HashMap<
                u64,
                Vec<&slicer_ir::TravelMove>,
            > = std::collections::HashMap::new();
            for tm in &layer.travel_moves {
                travel_moves_by_entity
                    .entry(tm.entity_id)
                    .or_default()
                    .push(tm);
            }

            // Process each entity
            for (entity_idx, entity) in layer.ordered_entities.iter().enumerate() {
                let entity_idx = entity_idx as u32;
                let points = &entity.path.points;
                let role = &entity.path.role;

                // Emit ;TYPE: comment when role changes from previous entity
                let role_changed = prev_role
                    .as_ref()
                    .is_none_or(|prev| !role_equals(prev, role));
                if role_changed {
                    commands.push(GCodeCommand::Raw {
                        text: orca_type_label(role).to_string(),
                    });
                }
                prev_role = Some(role.clone());

                // Emit Move commands for each point in the path
                let mut prev_point: Option<&slicer_ir::Point3WithWidth> = None;
                for point in points {
                    // Calculate extrusion (E) based on travel distance and width
                    let e_delta = if let Some(prev) = prev_point {
                        // Calculate 3D distance
                        let dx = point.x - prev.x;
                        let dy = point.y - prev.y;
                        let dz = point.z - prev.z;
                        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                        // E = distance * width * flow_factor (simplified)
                        distance * point.width * point.flow_factor
                    } else {
                        0.0 // First point, no extrusion
                    };

                    e_position += e_delta;
                    *filament_per_tool.entry(current_tool).or_insert(0.0) += e_delta;

                    commands.push(GCodeCommand::Move {
                        x: Some(point.x),
                        y: Some(point.y),
                        z: Some(point.z),
                        e: if e_delta > 0.0 {
                            Some(e_position)
                        } else {
                            None
                        },
                        f: self.resolve_feedrate(role, entity.path.speed_factor),
                        role: role.clone(),
                    });

                    prev_point = Some(point);
                }

                // Check for tool change after this entity
                if let Some(tc) = tool_changes.get(&entity_idx) {
                    commands.push(GCodeCommand::ToolChange {
                        after_entity_index: tc.after_entity_index,
                        from: tc.from_tool,
                        to: tc.to_tool,
                    });
                    current_tool = tc.to_tool;
                }

                // Emit Comment/Raw annotations attached to this entity index,
                // in the deterministic order they appear in `annotations`.
                for ann in layer
                    .annotations
                    .iter()
                    .filter(|a| a.after_entity_index == entity_idx)
                {
                    match &ann.kind {
                        LayerAnnotationKind::Comment(text) => {
                            commands.push(GCodeCommand::Comment { text: text.clone() });
                        }
                        LayerAnnotationKind::Raw(text) => {
                            commands.push(GCodeCommand::Raw { text: text.clone() });
                        }
                    }
                }

                // Emit canonical retract/z-hop/travel/unretract sequence after this entity.
                let entity_retracts = retracts_by_entity.get(&entity_idx);
                let entity_travels = travel_moves_by_entity.get(&entity.entity_id);
                let entity_z_hop = z_hops.get(&entity_idx);

                if let Some(retracts) = entity_retracts {
                    for r in retracts.iter().filter(|r| !r.is_unretract) {
                        commands.push(GCodeCommand::Retract {
                            length: r.length,
                            speed: r.speed,
                            mode: r.mode,
                        });
                    }
                }
                if let Some(zh) = entity_z_hop {
                    let hop_z = layer_z + zh.hop_height;
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(hop_z),
                        e: None,
                        f: self.resolve_feedrate(&ExtrusionRole::Custom("Travel".to_string()), 1.0),
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                }
                if let Some(travels) = entity_travels {
                    for tm in travels.iter() {
                        debug_assert!(
                            tm.entity_id == entity.entity_id,
                            "dangling travel anchor: entity_id={}",
                            tm.entity_id
                        );
                        commands.push(GCodeCommand::Move {
                            x: tm.x,
                            y: tm.y,
                            z: None,
                            e: None,
                            f: tm.f.or_else(|| {
                                self.resolve_feedrate(
                                    &ExtrusionRole::Custom("Travel".to_string()),
                                    1.0,
                                )
                            }),
                            role: ExtrusionRole::Custom("Travel".to_string()),
                        });
                    }
                }
                if let Some(zh) = entity_z_hop {
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(layer_z),
                        e: None,
                        f: self.resolve_feedrate(&ExtrusionRole::Custom("Travel".to_string()), 1.0),
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                    let _ = zh;
                }
                if let Some(retracts) = entity_retracts {
                    for r in retracts.iter().filter(|r| r.is_unretract) {
                        commands.push(GCodeCommand::Unretract {
                            length: r.length,
                            speed: r.speed,
                            mode: r.mode,
                        });
                    }
                }
            }

            // Trailing annotations whose anchor lies past the last entity
            // (e.g. layer with no ordered_entities) are still emitted in
            // declaration order so guest-emitted comments/raw lines are not
            // silently dropped.
            let entity_count = layer.ordered_entities.len() as u32;
            for ann in layer
                .annotations
                .iter()
                .filter(|a| a.after_entity_index >= entity_count)
            {
                match &ann.kind {
                    LayerAnnotationKind::Comment(text) => {
                        commands.push(GCodeCommand::Comment { text: text.clone() });
                    }
                    LayerAnnotationKind::Raw(text) => {
                        commands.push(GCodeCommand::Raw { text: text.clone() });
                    }
                }
            }
        }

        // Build filament_used_mm vector (indexed by tool)
        let max_tool = filament_per_tool.keys().max().copied().unwrap_or(0);
        let mut filament_used_mm = vec![0.0; (max_tool + 1) as usize];
        for (tool, amount) in filament_per_tool {
            filament_used_mm[tool as usize] = amount;
        }
        // Ensure at least one entry
        if filament_used_mm.is_empty() {
            filament_used_mm.push(0.0);
        }

        Ok(GCodeIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            commands,
            metadata: PrintMetadata {
                estimated_print_time_s: 0, // Not calculated in this implementation
                filament_used_mm,
                layer_count,
                slicer_version: self.slicer_version.clone(),
            },
        })
    }

    fn travel_feedrate_mm_per_min(&self) -> Option<f32> {
        Some(self.feedrate_config.travel_speed * 60.0)
    }
}

/// Default GCode serializer (host-built-in).
///
/// Converts `GCodeIR` to a G-code text string by serializing each command
/// according to standard G-code formatting rules.
///
/// `relative` controls whether `M83` (relative-E) or `M82` (absolute-E) is
/// emitted in the preamble, and how E values are rendered during serialization.
pub struct DefaultGCodeSerializer {
    /// `true` = relative-E mode (M83); `false` = absolute-E mode (M82).
    relative: bool,
    /// Filament diameter in mm (default 1.75 per schema).
    filament_diameter_mm: f32,
    /// Filament density in g/cm³ (default 1.24 per schema).
    filament_density_g_cm3: f32,
    /// Minimum max_z_height in mm used when no Z moves appear in the output
    /// (default 256.0 per config schema — the schema's configured build height).
    max_z_height_floor_mm: f32,
    /// Extrusion width for outer wall in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.42).
    outer_wall_line_width: f32,
    /// Extrusion width for inner walls in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.45).
    inner_wall_line_width: f32,
    /// Extrusion width for sparse infill in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.45).
    sparse_infill_line_width: f32,
    /// Extrusion width for top surface in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.42).
    top_surface_line_width: f32,
    /// Extrusion width for support material in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.35).
    support_line_width: f32,
}

impl DefaultGCodeSerializer {
    /// Creates a new `DefaultGCodeSerializer` in relative-E mode (default).
    pub fn new() -> Self {
        Self::with_extrusion_mode(true)
    }

    /// Creates a `DefaultGCodeSerializer` with an explicit extrusion mode.
    ///
    /// - `relative = true`  → emits `M83` in preamble; E values are deltas.
    /// - `relative = false` → emits `M82` in preamble; E values are absolute.
    pub fn with_extrusion_mode(relative: bool) -> Self {
        Self {
            relative,
            filament_diameter_mm: 1.75,
            filament_density_g_cm3: 1.24,
            max_z_height_floor_mm: 256.0,
            // OrcaSlicer 0.4 mm nozzle parity defaults (matches config_schema.rs registration).
            outer_wall_line_width: 0.42,
            inner_wall_line_width: 0.45,
            sparse_infill_line_width: 0.45,
            top_surface_line_width: 0.42,
            support_line_width: 0.35,
        }
    }

    /// Sets filament diameter and density (overrides schema defaults).
    pub fn with_filament_config(mut self, diameter_mm: f32, density_g_cm3: f32) -> Self {
        self.filament_diameter_mm = diameter_mm;
        self.filament_density_g_cm3 = density_g_cm3;
        self
    }
}

impl Default for DefaultGCodeSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Produce the HEADER_BLOCK text (OrcaSlicer wire format, packet 55 Step 3).
///
/// Format (FACT from OrcaSlicerDocumented/src/libslic3r/GCode.cpp:2644-2704):
/// ```text
/// ; HEADER_BLOCK_START
/// ; total layer number: <N>
/// ; filament_diameter: <d>
/// ; filament_density: <rho>
/// ; max_z_height: <z>
/// ; filament: <i0>,<i1>,...
/// ; HEADER_BLOCK_END
/// ```
/// Tool indices are 1-based (OrcaSlicer uses `used_filaments[idx] + 1`).
/// `max_z_mm` is the Z of the highest layer in millimeters.
fn serialize_header_block(
    layer_count: u32,
    filament_diameter_mm: f32,
    filament_density_g_cm3: f32,
    max_z_mm: f32,
    filament_used_mm: &[f32],
) -> String {
    let mut out = String::new();
    writeln!(out, "; HEADER_BLOCK_START").unwrap();
    writeln!(out, "; total layer number: {}", layer_count).unwrap();
    // Format floats without unnecessary trailing zeros.
    writeln!(out, "; filament_diameter: {}", filament_diameter_mm).unwrap();
    writeln!(out, "; filament_density: {}", filament_density_g_cm3).unwrap();
    // OrcaSlicer uses fixed precision 2 for max_z_height.
    writeln!(out, "; max_z_height: {:.2}", max_z_mm).unwrap();
    // Collect 1-based tool indices where filament was used (> 0.0), ascending.
    let used_indices: Vec<String> = filament_used_mm
        .iter()
        .enumerate()
        .filter(|(_, &mm)| mm > 0.0)
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    // If no filament was used (e.g. empty plan), emit tool 1 as a fallback
    // so the field always has a non-empty value (AC-6 requires ≥ 1 digit).
    let filament_value = if used_indices.is_empty() {
        "1".to_string()
    } else {
        used_indices.join(",")
    };
    writeln!(out, "; filament: {}", filament_value).unwrap();
    writeln!(out, "; HEADER_BLOCK_END").unwrap();
    out
}

/// Produce the extrusion-width comment block (packet 55 Step 4 / AC-7).
///
/// Format (five lines, immediately after HEADER_BLOCK_END):
/// ```text
/// ; outer_wall_line_width = <value>
/// ; inner_wall_line_width = <value>
/// ; sparse_infill_line_width = <value>
/// ; top_surface_line_width = <value>
/// ; support_line_width = <value>
/// ```
/// Values are formatted with trailing-zero-stripped decimal notation (e.g. `0.42`, not `0.4200`).
fn serialize_width_comments(
    outer_wall: f32,
    inner_wall: f32,
    sparse_infill: f32,
    top_surface: f32,
    support: f32,
) -> String {
    let mut out = String::new();
    // Strip trailing zeros while keeping at least one decimal digit.
    // `format!("{}", v)` on f32 already does this for values like 0.42 and 0.45.
    writeln!(out, "; outer_wall_line_width = {outer_wall}").unwrap();
    writeln!(out, "; inner_wall_line_width = {inner_wall}").unwrap();
    writeln!(out, "; sparse_infill_line_width = {sparse_infill}").unwrap();
    writeln!(out, "; top_surface_line_width = {top_surface}").unwrap();
    writeln!(out, "; support_line_width = {support}").unwrap();
    out
}

/// Encode raw bytes to a Base64 string (RFC 4648 standard alphabet, no line breaks).
///
/// Hand-rolled to avoid requiring `base64` as a non-dev dependency.
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((combined >> 18) & 63) as usize] as char);
        out.push(TABLE[((combined >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((combined >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(combined & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Produce the THUMBNAIL_BLOCK text (OrcaSlicer wire format, packet 55 Step 5).
///
/// Format (FACT from OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp:111-129):
/// - Sentinel: `; THUMBNAIL_BLOCK_START`
/// - Base64 lines: `; <chunk>` where each chunk is ≤ 76 characters (OrcaSlicer max_row_length)
/// - Sentinel: `; THUMBNAIL_BLOCK_END`
///
/// No metadata header line is emitted so that the region between the sentinels
/// contains only base64 data (required for the roundtrip test to decode cleanly).
pub fn serialize_thumbnail_block(png_bytes: &[u8]) -> String {
    const MAX_ROW_LENGTH: usize = 76;
    let encoded = base64_encode(png_bytes);
    let mut out = String::new();
    writeln!(out, "; THUMBNAIL_BLOCK_START").unwrap();
    let mut remaining = encoded.as_str();
    while remaining.len() > MAX_ROW_LENGTH {
        writeln!(out, "; {}", &remaining[..MAX_ROW_LENGTH]).unwrap();
        remaining = &remaining[MAX_ROW_LENGTH..];
    }
    if !remaining.is_empty() {
        writeln!(out, "; {}", remaining).unwrap();
    }
    writeln!(out, "; THUMBNAIL_BLOCK_END").unwrap();
    out
}

/// Convert a `ResolvedConfig` to a flat `HashMap<String, ConfigValue>`.
///
/// Used to populate the CONFIG_BLOCK with the effective slicer settings.
/// `Option`-typed fields that are `None` are omitted; all others are included.
pub fn resolved_config_to_map(cfg: &ResolvedConfig) -> HashMap<String, ConfigValue> {
    let mut m: HashMap<String, ConfigValue> = HashMap::new();
    m.insert(
        "layer_height".into(),
        ConfigValue::Float(f64::from(cfg.layer_height)),
    );
    m.insert(
        "line_width".into(),
        ConfigValue::Float(f64::from(cfg.line_width)),
    );
    m.insert(
        "first_layer_height".into(),
        ConfigValue::Float(f64::from(cfg.first_layer_height)),
    );
    m.insert(
        "first_layer_line_width".into(),
        ConfigValue::Float(f64::from(cfg.first_layer_line_width)),
    );
    m.insert(
        "wall_count".into(),
        ConfigValue::Int(i64::from(cfg.wall_count)),
    );
    m.insert(
        "outer_wall_speed".into(),
        ConfigValue::Float(f64::from(cfg.outer_wall_speed)),
    );
    m.insert(
        "inner_wall_speed".into(),
        ConfigValue::Float(f64::from(cfg.inner_wall_speed)),
    );
    m.insert(
        "wall_generator".into(),
        ConfigValue::String(format!("{:?}", cfg.wall_generator)),
    );
    if let Some(v) = cfg.arachne_min_feature_size {
        m.insert(
            "arachne_min_feature_size".into(),
            ConfigValue::Float(f64::from(v)),
        );
    }
    m.insert(
        "infill_type".into(),
        ConfigValue::String(format!("{:?}", cfg.infill_type)),
    );
    m.insert(
        "infill_density".into(),
        ConfigValue::Float(f64::from(cfg.infill_density)),
    );
    m.insert(
        "infill_angle".into(),
        ConfigValue::Float(f64::from(cfg.infill_angle)),
    );
    m.insert(
        "infill_speed".into(),
        ConfigValue::Float(f64::from(cfg.infill_speed)),
    );
    m.insert(
        "solid_infill_speed".into(),
        ConfigValue::Float(f64::from(cfg.solid_infill_speed)),
    );
    m.insert(
        "top_shell_layers".into(),
        ConfigValue::Int(i64::from(cfg.top_shell_layers)),
    );
    m.insert(
        "bottom_shell_layers".into(),
        ConfigValue::Int(i64::from(cfg.bottom_shell_layers)),
    );
    m.insert(
        "top_fill_holder".into(),
        ConfigValue::String(cfg.top_fill_holder.clone()),
    );
    m.insert(
        "bottom_fill_holder".into(),
        ConfigValue::String(cfg.bottom_fill_holder.clone()),
    );
    m.insert(
        "bridge_fill_holder".into(),
        ConfigValue::String(cfg.bridge_fill_holder.clone()),
    );
    m.insert(
        "sparse_fill_holder".into(),
        ConfigValue::String(cfg.sparse_fill_holder.clone()),
    );
    m.insert(
        "support_enabled".into(),
        ConfigValue::Bool(cfg.support_enabled),
    );
    m.insert(
        "support_type".into(),
        ConfigValue::String(format!("{:?}", cfg.support_type)),
    );
    m.insert(
        "support_overhang_angle".into(),
        ConfigValue::Float(f64::from(cfg.support_overhang_angle)),
    );
    if let Some(v) = cfg.nonplanar_max_angle_deg {
        m.insert(
            "nonplanar_max_angle_deg".into(),
            ConfigValue::Float(f64::from(v)),
        );
    }
    if let Some(v) = cfg.nonplanar_shell_count {
        m.insert(
            "nonplanar_shell_count".into(),
            ConfigValue::Int(i64::from(v)),
        );
    }
    if let Some(v) = cfg.nonplanar_amplitude {
        m.insert(
            "nonplanar_amplitude".into(),
            ConfigValue::Float(f64::from(v)),
        );
    }
    if let Some(v) = cfg.smoothificator_target_height {
        m.insert(
            "smoothificator_target_height".into(),
            ConfigValue::Float(f64::from(v)),
        );
    }
    if let Some(v) = cfg.smoothificator_adaptive {
        m.insert("smoothificator_adaptive".into(), ConfigValue::Bool(v));
    }
    // Merge extension keys (module-contributed, already in ConfigValue form)
    for (k, v) in &cfg.extensions {
        m.insert(k.clone(), v.clone());
    }
    m
}

/// Produce the CONFIG_BLOCK text (packet 55 Step 5 / AC-8, AC-9).
///
/// Format:
/// ```text
/// ; CONFIG_BLOCK_START
/// ; key = value
/// ...
/// ; CONFIG_BLOCK_END
/// ```
/// Keys are sorted for determinism. Callers are responsible for stripping
/// invocation-time keys (e.g. `thumbnail_path`) before passing the map.
fn serialize_config_block(raw_config: &HashMap<String, ConfigValue>) -> String {
    let mut out = String::new();
    writeln!(out, "; CONFIG_BLOCK_START").unwrap();
    // Sort keys for deterministic output.
    let mut keys: Vec<&String> = raw_config.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = raw_config.get(key) {
            let value_str = match value {
                ConfigValue::Bool(b) => b.to_string(),
                ConfigValue::Int(i) => i.to_string(),
                ConfigValue::Float(f) => {
                    // Strip trailing zeros like "22.0" → "22" not wanted by test, keep "22.0"
                    format!("{f}")
                }
                ConfigValue::String(s) => s.clone(),
                ConfigValue::List(items) => {
                    let parts: Vec<String> = items
                        .iter()
                        .map(|v| match v {
                            ConfigValue::String(s) => s.clone(),
                            _ => format!("{v:?}"),
                        })
                        .collect();
                    parts.join(",")
                }
            };
            writeln!(out, "; {key} = {value_str}").unwrap();
        }
    }
    writeln!(out, "; CONFIG_BLOCK_END").unwrap();
    out
}

/// A `GCodeSerializer` wrapper that injects `THUMBNAIL_BLOCK` and `CONFIG_BLOCK`
/// from the raw config source, delegating core serialization to the inner serializer.
///
/// Used by `run_pipeline_with_raw_config` to inject thumbnail data and config
/// view into the serialization step without changing the `GCodeSerializer` trait.
pub struct ThumbnailAwareSerializer {
    inner: Box<dyn crate::GCodeSerializer>,
    thumbnail_bytes: Option<Vec<u8>>,
    raw_config: HashMap<String, ConfigValue>,
}

impl ThumbnailAwareSerializer {
    /// Create a new wrapper around `inner`, optionally attaching thumbnail bytes
    /// and a raw config map for CONFIG_BLOCK emission.
    pub fn new(
        inner: Box<dyn crate::GCodeSerializer>,
        thumbnail_bytes: Option<Vec<u8>>,
        raw_config: HashMap<String, ConfigValue>,
    ) -> Self {
        Self {
            inner,
            thumbnail_bytes,
            raw_config,
        }
    }
}

impl crate::GCodeSerializer for ThumbnailAwareSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, crate::PostpassError> {
        let base = self.inner.serialize_gcode(gcode_ir)?;

        // 1. Insert THUMBNAIL_BLOCK immediately after HEADER_BLOCK_END (if thumbnail present).
        let base = if let Some(ref bytes) = self.thumbnail_bytes {
            let sentinel = "; HEADER_BLOCK_END\n";
            if let Some(pos) = base.find(sentinel) {
                let insert_at = pos + sentinel.len();
                let mut result = String::with_capacity(base.len() + bytes.len() * 2);
                result.push_str(&base[..insert_at]);
                result.push_str(&serialize_thumbnail_block(bytes));
                result.push_str(&base[insert_at..]);
                result
            } else {
                let mut result = serialize_thumbnail_block(bytes);
                result.push_str(&base);
                result
            }
        } else {
            base
        };

        // 2. Append CONFIG_BLOCK at the end of the output.
        let config_block = serialize_config_block(&self.raw_config);
        let mut result = base;
        result.push_str(&config_block);
        Ok(result)
    }
}

impl GCodeSerializer for DefaultGCodeSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        let mut output = String::new();

        // Compute max Z height (mm) from the GCodeIR commands.
        // Z fields in Move commands are already in mm (f32).
        let computed_z: f32 = gcode_ir
            .commands
            .iter()
            .filter_map(|cmd| {
                if let GCodeCommand::Move { z, .. } = cmd {
                    *z
                } else {
                    None
                }
            })
            .fold(0.0_f32, f32::max);
        // Use the config floor when no Z moves appear (e.g. empty-plan test runs).
        // For real prints the scanned value is always larger than the floor.
        let max_z_mm = if computed_z > 0.0 {
            computed_z
        } else {
            self.max_z_height_floor_mm
        };

        // Emit HEADER_BLOCK as the first thing in the file (AC-3 through AC-6,
        // OrcaSlicer parity: GCode.cpp:2644-2704).
        output.push_str(&serialize_header_block(
            gcode_ir.metadata.layer_count,
            self.filament_diameter_mm,
            self.filament_density_g_cm3,
            max_z_mm,
            &gcode_ir.metadata.filament_used_mm,
        ));

        // Emit extrusion-width comments immediately after HEADER_BLOCK_END (AC-7,
        // packet 55 Step 4).
        output.push_str(&serialize_width_comments(
            self.outer_wall_line_width,
            self.inner_wall_line_width,
            self.sparse_infill_line_width,
            self.top_surface_line_width,
            self.support_line_width,
        ));

        // Preamble: emit extruder mode selector exactly once.
        if self.relative {
            writeln!(output, "M83").unwrap();
        } else {
            writeln!(output, "M82").unwrap();
        }

        // e_accumulator tracks the absolute E position seen so far (from GCodeIR,
        // which always stores absolute E values).  Only used in relative mode to
        // compute per-move deltas.
        let mut e_accumulator: f64 = 0.0;

        for command in &gcode_ir.commands {
            match command {
                GCodeCommand::Move {
                    x,
                    y,
                    z,
                    e,
                    f,
                    role,
                    ..
                } => {
                    // Emit G0 for travel moves (Custom("Travel") role), G1 for extrusion moves.
                    let is_travel = matches!(role, ExtrusionRole::Custom(s) if s == "Travel");
                    let cmd = if is_travel { "G0" } else { "G1" };
                    write!(output, "{cmd}").unwrap();
                    if let Some(x_val) = x {
                        write!(output, " X{}", format_coord(*x_val)).unwrap();
                    }
                    if let Some(y_val) = y {
                        write!(output, " Y{}", format_coord(*y_val)).unwrap();
                    }
                    if let Some(z_val) = z {
                        write!(output, " Z{}", format_coord(*z_val)).unwrap();
                    }
                    if let Some(e_val) = e {
                        if self.relative {
                            let abs_e = *e_val as f64;
                            let delta = abs_e - e_accumulator;
                            write!(output, " E{:.5}", delta).unwrap();
                            e_accumulator = abs_e;
                        } else {
                            write!(output, " E{:.5}", e_val).unwrap();
                        }
                    }
                    if let Some(f_val) = f {
                        write!(output, " F{}", format_coord(*f_val)).unwrap();
                    }
                    writeln!(output).unwrap();
                }
                GCodeCommand::Retract {
                    length,
                    speed,
                    mode,
                } => match mode {
                    slicer_ir::RetractMode::Gcode => {
                        // Retract is always a delta (negative E movement) regardless of mode,
                        // because it represents a physical retraction amount.  In relative mode
                        // the retract length IS the delta.  In absolute mode we subtract from
                        // the accumulator and emit the new absolute position.
                        if self.relative {
                            writeln!(output, "G1 E-{:.5} F{}", length, format_coord(*speed))
                                .unwrap();
                            e_accumulator -= *length as f64;
                        } else {
                            writeln!(
                                output,
                                "G1 E-{} F{}",
                                format_coord(*length),
                                format_coord(*speed)
                            )
                            .unwrap();
                        }
                    }
                    slicer_ir::RetractMode::Firmware => {
                        writeln!(output, "G10").unwrap();
                    }
                },
                GCodeCommand::Unretract {
                    length,
                    speed,
                    mode,
                } => match mode {
                    slicer_ir::RetractMode::Gcode => {
                        if self.relative {
                            writeln!(output, "G1 E{:.5} F{}", length, format_coord(*speed))
                                .unwrap();
                            e_accumulator += *length as f64;
                        } else {
                            writeln!(
                                output,
                                "G1 E{} F{}",
                                format_coord(*length),
                                format_coord(*speed)
                            )
                            .unwrap();
                        }
                    }
                    slicer_ir::RetractMode::Firmware => {
                        writeln!(output, "G11").unwrap();
                    }
                },
                // Raw commands: detect G92 E resets and sync the accumulator.
                GCodeCommand::Raw { text } => {
                    // Detect "G92 E0" (or "G92 E0.0" etc.) to reset accumulator.
                    let trimmed = text.trim();
                    if trimmed.starts_with("G92") {
                        // Parse the E value from the G92 line (e.g. "G92 E0" → 0.0).
                        if let Some(e_str) = trimmed
                            .split_whitespace()
                            .find(|tok| tok.starts_with('E') || tok.starts_with('e'))
                        {
                            if let Ok(val) = e_str[1..].parse::<f64>() {
                                e_accumulator = val;
                            }
                        }
                    }
                    writeln!(output, "{}", text).unwrap();
                }
                GCodeCommand::FanSpeed { value } => {
                    writeln!(output, "M106 S{}", value).unwrap();
                }
                GCodeCommand::Temperature {
                    tool,
                    celsius,
                    wait,
                } => {
                    let cmd = if *wait { "M109" } else { "M104" };
                    writeln!(output, "{} T{} S{}", cmd, tool, format_coord(*celsius)).unwrap();
                }
                GCodeCommand::ToolChange { to, .. } => {
                    writeln!(output, "T{}", to).unwrap();
                }
                GCodeCommand::Comment { text } => {
                    writeln!(output, "; {}", text).unwrap();
                }
            }
        }

        Ok(output)
    }
}

/// Reconcile travel moves to route through finalization geometry (Skirt/Brim,
/// WipeTower) without modifying `ordered_entities`.
///
/// This pass runs on each `LayerCollectionIR` *before* `emit_gcode` so that
/// travel transitions correctly incorporate finalization geometry.
///
/// Invariants:
/// - `ordered_entities` is never modified.
/// - Only `travel_moves` is mutated (new entries appended).
/// - If no Skirt or WipeTower entities exist, the layer is unchanged (no-op).
pub fn reconcile_finalization_travel(
    layer: &mut LayerCollectionIR,
    travel_f_mm_per_min: Option<f32>,
) {
    use slicer_ir::TravelMove;

    let entities = &layer.ordered_entities;

    // Collect indices of finalization entities
    let skirt_indices: Vec<usize> = entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.role == ExtrusionRole::Skirt)
        .map(|(i, _)| i)
        .collect();
    let wipe_indices: Vec<usize> = entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.role == ExtrusionRole::WipeTower)
        .map(|(i, _)| i)
        .collect();

    if skirt_indices.is_empty() && wipe_indices.is_empty() {
        return; // no-op
    }

    // Find the first model (non-finalization) entity index
    let first_model = entities.iter().enumerate().find_map(|(i, e)| {
        if e.role != ExtrusionRole::Skirt && e.role != ExtrusionRole::WipeTower {
            Some(i)
        } else {
            None
        }
    });

    // AC1: If skirt entities exist before model entities, add a travel move
    // from the last skirt entity's endpoint to the first model entity's start.
    if let (Some(&last_skirt_idx), Some(model_idx)) = (skirt_indices.last(), first_model) {
        if last_skirt_idx < model_idx {
            let skirt_entity = &entities[last_skirt_idx];
            let model_entity = &entities[model_idx];
            if let (Some(_skirt_end), Some(model_start)) = (
                skirt_entity.path.points.last(),
                model_entity.path.points.first(),
            ) {
                layer.travel_moves.push(TravelMove {
                    entity_id: entities[last_skirt_idx].entity_id,
                    x: Some(model_start.x),
                    y: Some(model_start.y),
                    z: None,
                    f: travel_f_mm_per_min,
                });
            }
        }
    }

    // AC2: If wipe tower entities exist, add travel moves that route to the
    // wipe tower start from the preceding entity.
    for &wipe_idx in &wipe_indices {
        if wipe_idx > 0 {
            let wipe_entity = &entities[wipe_idx];
            if let Some(wipe_start) = wipe_entity.path.points.first() {
                layer.travel_moves.push(TravelMove {
                    entity_id: entities[wipe_idx - 1].entity_id,
                    x: Some(wipe_start.x),
                    y: Some(wipe_start.y),
                    z: None,
                    f: travel_f_mm_per_min,
                });
            }
        }
    }

    // Keep travel_moves sorted by anchored entity position for deterministic emission.
    let id_to_idx: std::collections::HashMap<u64, usize> = entities
        .iter()
        .enumerate()
        .map(|(i, e)| (e.entity_id, i))
        .collect();
    layer
        .travel_moves
        .sort_by_key(|tm| id_to_idx.get(&tm.entity_id).copied().unwrap_or(usize::MAX));
}

/// Format a coordinate value, trimming unnecessary trailing zeros.
fn format_coord(value: f32) -> String {
    // Format with enough precision, then trim trailing zeros
    let s = format!("{:.4}", value);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gcode_emitter_stores_slicer_version() {
        let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
        assert_eq!(emitter.slicer_version(), "1.0.0-test");
    }

    #[test]
    fn default_gcode_serializer_can_be_created() {
        let _serializer = DefaultGCodeSerializer::new();
        let _default_serializer = DefaultGCodeSerializer::default();
    }
}
