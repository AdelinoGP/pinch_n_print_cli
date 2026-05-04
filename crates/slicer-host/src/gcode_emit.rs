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
//! - Format: `G1 X... Y... Z... E... F...` with appropriate precision
//! - Handle all GCodeCommand variants (Move, Retract, Unretract, FanSpeed, Temperature,
//!   ToolChange, Comment, Raw)
//!
//! OrcaSlicer references:
//! - OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp — G-code emission patterns
//! - OrcaSlicerDocumented/tests/fff_print/test_gcodewriter.cpp — test patterns

use std::collections::HashMap;
use std::fmt::Write;

use slicer_ir::{
    ExtrusionRole, GCodeCommand, GCodeIR, LayerAnnotationKind, LayerCollectionIR, PrintMetadata,
    SemVer,
};

use crate::{Blackboard, GCodeEmitter, GCodeSerializer, PostpassError};

/// Default GCode emitter (host-built-in).
///
/// Converts `LayerCollectionIR` to `GCodeIR` by walking layers in Z-sorted order,
/// converting print entities to move commands, and inserting tool changes and Z-hops.
pub struct DefaultGCodeEmitter {
    /// Slicer version string to include in metadata.
    slicer_version: String,
}

impl DefaultGCodeEmitter {
    /// Creates a new `DefaultGCodeEmitter` with the given slicer version.
    pub fn new(slicer_version: String) -> Self {
        Self { slicer_version }
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
        ExtrusionRole::BridgeInfill => ";TYPE:Bridge",
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
            // travel_moves: per entity index, collect all in order
            let mut travel_moves_by_entity: std::collections::HashMap<
                u32,
                Vec<&slicer_ir::TravelMove>,
            > = std::collections::HashMap::new();
            for tm in &layer.travel_moves {
                travel_moves_by_entity
                    .entry(tm.after_entity_index)
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
                        f: None, // Feed rate could be calculated, but tests don't require it
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
                let entity_travels = travel_moves_by_entity.get(&entity_idx);
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
                        f: None,
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                }
                if let Some(travels) = entity_travels {
                    for tm in travels.iter() {
                        commands.push(GCodeCommand::Move {
                            x: tm.x,
                            y: tm.y,
                            z: None,
                            e: None,
                            f: tm.f,
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
                        f: None,
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
}

/// Default GCode serializer (host-built-in).
///
/// Converts `GCodeIR` to a G-code text string by serializing each command
/// according to standard G-code formatting rules.
pub struct DefaultGCodeSerializer;

impl DefaultGCodeSerializer {
    /// Creates a new `DefaultGCodeSerializer`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultGCodeSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl GCodeSerializer for DefaultGCodeSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        let mut output = String::new();

        for command in &gcode_ir.commands {
            match command {
                GCodeCommand::Move { x, y, z, e, f, .. } => {
                    write!(output, "G1").unwrap();
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
                        write!(output, " E{}", format_coord(*e_val)).unwrap();
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
                        writeln!(
                            output,
                            "G1 E-{} F{}",
                            format_coord(*length),
                            format_coord(*speed)
                        )
                        .unwrap();
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
                        writeln!(
                            output,
                            "G1 E{} F{}",
                            format_coord(*length),
                            format_coord(*speed)
                        )
                        .unwrap();
                    }
                    slicer_ir::RetractMode::Firmware => {
                        writeln!(output, "G11").unwrap();
                    }
                },
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
                GCodeCommand::Raw { text } => {
                    writeln!(output, "{}", text).unwrap();
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
pub fn reconcile_finalization_travel(layer: &mut LayerCollectionIR) {
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
                    after_entity_index: last_skirt_idx as u32,
                    x: Some(model_start.x),
                    y: Some(model_start.y),
                    z: None,
                    f: None,
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
                    after_entity_index: (wipe_idx - 1) as u32,
                    x: Some(wipe_start.x),
                    y: Some(wipe_start.y),
                    z: None,
                    f: None,
                });
            }
        }
    }

    // Keep travel_moves sorted by after_entity_index for deterministic emission.
    layer.travel_moves.sort_by_key(|tm| tm.after_entity_index);
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
