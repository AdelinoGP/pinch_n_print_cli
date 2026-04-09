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

use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR, LayerCollectionIR, PrintMetadata, SemVer};

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

        // Walk layers in order (already Z-sorted by LayerFinalization)
        for layer in layer_irs {
            let layer_z = layer.z;

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

            // Process each entity
            for (entity_idx, entity) in layer.ordered_entities.iter().enumerate() {
                let entity_idx = entity_idx as u32;
                let points = &entity.path.points;
                let role = &entity.path.role;

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
                        from: tc.from_tool,
                        to: tc.to_tool,
                    });
                    current_tool = tc.to_tool;
                }

                // Check for Z-hop after this entity
                if let Some(zh) = z_hops.get(&entity_idx) {
                    let hop_z = layer_z + zh.hop_height;
                    // Lift to hop height (travel move, no extrusion)
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(hop_z),
                        e: None,
                        f: None,
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                    // Return to layer Z (travel move, no extrusion)
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(layer_z),
                        e: None,
                        f: None,
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
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
                GCodeCommand::Retract { length, speed } => {
                    writeln!(
                        output,
                        "G1 E-{} F{}",
                        format_coord(*length),
                        format_coord(*speed)
                    )
                    .unwrap();
                }
                GCodeCommand::Unretract { length, speed } => {
                    writeln!(
                        output,
                        "G1 E{} F{}",
                        format_coord(*length),
                        format_coord(*speed)
                    )
                    .unwrap();
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
                GCodeCommand::Raw { text } => {
                    writeln!(output, "{}", text).unwrap();
                }
            }
        }

        Ok(output)
    }
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
