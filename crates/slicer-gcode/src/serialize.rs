//! G-code serialization: `GCodeIR` → G-code text.
//!
//! This module hosts the [`GCodeSerializer`] trait and the canonical
//! [`DefaultGCodeSerializer`] / [`ThumbnailAwareSerializer`] implementations
//! extracted from `crates/slicer-runtime/src/gcode_emit.rs` (packet 86).

use std::collections::HashMap;
use std::fmt::Write;

use slicer_ir::{ConfigValue, ExtrusionRole, GCodeCommand, GCodeIR, ResolvedConfig};

use crate::error::GCodeEmitError;
use crate::thumbnail::serialize_thumbnail_block;

/// Trait for GCode serialization (host-built-in).
pub trait GCodeSerializer {
    /// Serialize GCodeIR to text.
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError>;
}

/// Return the D-P simplification tolerance (mm) for a given extrusion role.
///
/// Exhaustive — adding an ExtrusionRole variant must force a compile error here so
/// the new variant gets an explicit precision policy rather than silently inheriting one.
pub fn tolerance_for_role(role: &ExtrusionRole, cfg: &ResolvedConfig) -> f32 {
    match role {
        // Perimeter / wall family and skirt/brim: tightest tolerance.
        ExtrusionRole::OuterWall
        | ExtrusionRole::InnerWall
        | ExtrusionRole::ThinWall
        | ExtrusionRole::Skirt => cfg.gcode_resolution,
        // Infill family: relaxed tolerance.
        ExtrusionRole::SparseInfill
        | ExtrusionRole::TopSolidInfill
        | ExtrusionRole::BottomSolidInfill
        | ExtrusionRole::BridgeInfill
        | ExtrusionRole::Ironing
        | ExtrusionRole::WipeTower
        | ExtrusionRole::PrimeTower => cfg.infill_resolution,
        // Support family: support tolerance.
        ExtrusionRole::SupportMaterial | ExtrusionRole::SupportInterface => cfg.support_resolution,
        // Travel and other custom moves: no simplification.
        ExtrusionRole::Custom(_) => 0.0,
        // Gap-fill uses perimeter tolerance (it's wall-adjacent).
        ExtrusionRole::GapFill => cfg.gcode_resolution,
        _ => 0.0,
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
    /// Decimal places for XYZ coordinate values in serialized GCode (default 3).
    gcode_xy_decimals: u32,
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
            gcode_xy_decimals: 3,
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

/// Convert a `ResolvedConfig` to a flat `HashMap<String, ConfigValue>`.
///
/// Used to populate the CONFIG_BLOCK with the effective slicer settings.
/// `Option`-typed fields that are `None` are omitted; all others are included.
pub fn resolved_config_to_map(cfg: &ResolvedConfig) -> HashMap<String, ConfigValue> {
    cfg.to_config_map()
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
    inner: Box<dyn GCodeSerializer>,
    thumbnail_bytes: Option<Vec<u8>>,
    raw_config: HashMap<String, ConfigValue>,
}

impl ThumbnailAwareSerializer {
    /// Create a new wrapper around `inner`, optionally attaching thumbnail bytes
    /// and a raw config map for CONFIG_BLOCK emission.
    pub fn new(
        inner: Box<dyn GCodeSerializer>,
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

impl GCodeSerializer for ThumbnailAwareSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
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
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
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

        // e_accumulator tracks the absolute E position seen so far (from GCodeIR,
        // which always stores absolute E values).  Only used in relative mode to
        // compute per-move deltas.
        let mut e_accumulator: f64 = 0.0;

        // ExtrusionMode is now at index 0 of gcode_ir.commands (pushed by the emitter).
        // The per-command renderer handles it — no special-case prepend needed here.
        for command in gcode_ir.commands.iter() {
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
                        write!(output, " X{}", format_xyz(*x_val, self.gcode_xy_decimals)).unwrap();
                    }
                    if let Some(y_val) = y {
                        write!(output, " Y{}", format_xyz(*y_val, self.gcode_xy_decimals)).unwrap();
                    }
                    if let Some(z_val) = z {
                        write!(output, " Z{}", format_xyz(*z_val, self.gcode_xy_decimals)).unwrap();
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
                GCodeCommand::ExtrusionMode { absolute } => {
                    if *absolute {
                        writeln!(output, "M82").unwrap();
                    } else {
                        writeln!(output, "M83").unwrap();
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Format a coordinate value, trimming unnecessary trailing zeros.
/// Uses fixed 4-decimal precision (legacy behavior for F/E/temperature emit).
pub fn format_coord(value: f32) -> String {
    // Format with enough precision, then trim trailing zeros
    let s = format!("{:.4}", value);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// Format an XYZ coordinate value with configurable decimal precision,
/// trimming unnecessary trailing zeros.
pub fn format_xyz(value: f32, decimals: u32) -> String {
    let s = format!("{:.*}", decimals as usize, value);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gcode_serializer_can_be_created() {
        let _serializer = DefaultGCodeSerializer::new();
        let _default_serializer = DefaultGCodeSerializer::default();
    }
}
