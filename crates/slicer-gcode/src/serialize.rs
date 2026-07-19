//! G-code serialization: `GCodeIR` → G-code text.
//!
//! This module hosts the [`GCodeSerializer`] trait and the canonical
//! [`DefaultGCodeSerializer`] / [`ThumbnailAwareSerializer`] implementations
//! extracted from `crates/slicer-runtime/src/gcode_emit.rs` (packet 86).

use std::collections::HashMap;
use std::fmt::Write;

use slicer_ir::{ConfigValue, ExtrusionRole, GCodeCommand, GCodeIR, ResolvedConfig};

use crate::error::GCodeEmitError;
use crate::flavor::GcodeFlavor;
use crate::thumbnail::{serialize_thumbnail_block, RenderedThumbnail};

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
        | ExtrusionRole::Skirt
        | ExtrusionRole::Brim => cfg.gcode_resolution,
        // Infill family: relaxed tolerance.
        ExtrusionRole::SparseInfill
        | ExtrusionRole::TopSolidInfill
        | ExtrusionRole::BottomSolidInfill
        | ExtrusionRole::InternalSolidInfill
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
    flavor: GcodeFlavor,
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
            flavor: GcodeFlavor::Marlin,
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

    /// Sets the G-code dialect used for flavor-specific commands.
    pub fn with_flavor(mut self, flavor: GcodeFlavor) -> Self {
        self.flavor = flavor;
        self
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
    // Producer line. OrcaSlicer's G-code viewer (ConfigBase::load_from_gcode_file,
    // GCodeProcessor producer detection) only parses the embedded config block —
    // and therefore the filament count, per-tool colours, and tool changes — when
    // a comment line near the top begins with "; OrcaSlicer" or contains
    // "generated by OrcaSlicer". Without it the file is an "Unknown producer":
    // filaments_count stays 0 and every T<n> is dropped (preview shows 1 filament,
    // 0 tool changes). This line starts with "; OrcaSlicer" so the viewer reads our
    // config, while honestly attributing the real producer.
    writeln!(
        out,
        "; OrcaSlicer-compatible output generated by Pinch'n'Print"
    )
    .unwrap();
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
    // filament_colour / extruder_colour: OrcaSlicer's G-code preview colours each
    // extrusion by these directives in filament view. Without them a multi-tool
    // (MMU) print renders monochrome even though T<n> tool changes are present.
    // NOTE: OrcaSlicer's viewer actually infers filament count + colours from the
    // CONFIG_BLOCK at the END of the file (see serialize_config_block), not this
    // header — we emit them here too for header-reading tools, but the config
    // block is the one that drives the OrcaSlicer preview.
    let colour_list = filament_colour_csv(filament_used_mm.len());
    writeln!(out, "; filament_colour = {}", colour_list).unwrap();
    writeln!(out, "; extruder_colour = {}", colour_list).unwrap();
    writeln!(out, "; HEADER_BLOCK_END").unwrap();
    out
}

/// Build the semicolon-separated per-filament colour list used by both the
/// HEADER_BLOCK and the CONFIG_BLOCK. One distinct palette colour per filament
/// slot (cycling for >8). OrcaSlicer's preview infers the filament COUNT from the
/// number of entries in this list, so it must be sized to the tools in use.
fn filament_colour_csv(slot_count: usize) -> String {
    let n = slot_count.max(1);
    (0..n)
        .map(|i| DEFAULT_FILAMENT_PALETTE[i % DEFAULT_FILAMENT_PALETTE.len()])
        .collect::<Vec<_>>()
        .join(";")
}

/// Resolve the per-filament colour CSV (`#RRGGBB;#RRGGBB;…`): prefer the model's
/// authored palette carried in `raw_config["filament_colour"]` (a semicolon-
/// separated string seeded from the 3MF project settings), falling back to the
/// hardcoded default palette sized to the tools in use.
fn resolve_filament_colour_csv(
    raw_config: &HashMap<String, ConfigValue>,
    slot_count: usize,
) -> String {
    match raw_config.get("filament_colour") {
        Some(ConfigValue::String(s)) if !s.trim().is_empty() => s.clone(),
        _ => filament_colour_csv(slot_count),
    }
}

/// Default distinct per-filament colour palette (hex), used to populate the
/// `filament_colour` / `extruder_colour` directives so OrcaSlicer's filament-view
/// preview renders each tool in a different colour. Cycles for prints with more
/// filament slots than palette entries.
const DEFAULT_FILAMENT_PALETTE: [&str; 8] = [
    "#EC0006", // red
    "#02BF06", // green
    "#1800F2", // blue
    "#FF9B00", // orange
    "#00C0C0", // cyan
    "#C000C0", // magenta
    "#C0C000", // yellow
    "#808080", // grey
];

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
fn serialize_config_block(
    raw_config: &HashMap<String, ConfigValue>,
    filament_colour_csv: &str,
    flavor: GcodeFlavor,
) -> String {
    use std::collections::BTreeSet;

    // Filament count = number of colours in the list (one per tool in use).
    let filament_count = filament_colour_csv
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .count()
        .max(1);

    let mut out = String::new();
    writeln!(out, "; CONFIG_BLOCK_START").unwrap();

    // Track emitted keys so padding never duplicates a user/host key and so we can
    // count toward OrcaSlicer's minimum-keys gate.
    let mut emitted: BTreeSet<String> = BTreeSet::new();

    // OrcaSlicer's viewer (ConfigBase::load_from_gcode_file + GCodeProcessor::
    // apply_config) reads ONLY this block. It infers the filament COUNT from the
    // `filament_diameter` array length (ConfigOptionFloats, comma-separated) and
    // applies per-tool colours from `filament_colour` (ConfigOptionStrings,
    // semicolon-separated) only when its length matches. Emit all three sized to
    // the tools in use unless the user already supplied them (dumped below).
    if !raw_config.contains_key("filament_diameter") {
        let diam = vec!["1.75"; filament_count].join(",");
        emit_config_kv(&mut out, &mut emitted, "filament_diameter", &diam);
    }
    if !raw_config.contains_key("filament_colour") {
        emit_config_kv(
            &mut out,
            &mut emitted,
            "filament_colour",
            filament_colour_csv,
        );
    }
    if !raw_config.contains_key("extruder_colour") {
        emit_config_kv(
            &mut out,
            &mut emitted,
            "extruder_colour",
            filament_colour_csv,
        );
    }
    // Synthesize printer_model when absent so OrcaSlicer's `s_IsBBLPrinter`
    // heuristic does not default to Bambu behavior on drag-in.
    // Fork-supplied values always win via the `emit_config_kv` dedup path.
    if !raw_config.contains_key("printer_model") {
        emit_config_kv(
            &mut out,
            &mut emitted,
            "printer_model",
            "Generic PNP Printer",
        );
    }
    let flavor_value = raw_config
        .get("gcode_flavor")
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| flavor.config_str());
    emit_config_kv(&mut out, &mut emitted, "gcode_flavor", flavor_value);

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
                ConfigValue::Percent(p) => format!("{p}%"),
                ConfigValue::FloatOrPercent { value, is_percent } => {
                    if *is_percent {
                        format!("{value}%")
                    } else {
                        format!("{value}")
                    }
                }
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
            emit_config_kv(&mut out, &mut emitted, key, &value_str);
        }
    }

    // Pad from the neutral table until the block reaches the OrcaSlicer minimum
    // key gate (≥80 lines). Synthetic keys (filament_diameter, filament_colour,
    // extruder_colour, printer_model) and raw_config passthrough go first, then
    // the table.
    for (key, value) in ORCA_CONFIG_PADDING {
        if emitted.len() >= 96 {
            break;
        }
        emit_config_kv(&mut out, &mut emitted, key, value);
    }

    writeln!(out, "; CONFIG_BLOCK_END").unwrap();
    out
}

/// Emit one `; key = value` config line, skipping keys already written so padding
/// never duplicates a host/user-supplied key.
fn emit_config_kv(
    out: &mut String,
    emitted: &mut std::collections::BTreeSet<String>,
    key: &str,
    value: &str,
) {
    if emitted.insert(key.to_string()) {
        let _ = writeln!(out, "; {key} = {value}");
    }
}

/// Padding table contains only neutral cosmetic CONFIG_BLOCK keys. PNP never
/// emits speed/accel/jerk/machine-limit values here; those are the fork's
/// responsibility per the CONFIG_BLOCK viewer-key contract in
/// `docs/02_ir_schemas.md`.
const ORCA_CONFIG_PADDING: &[(&str, &str)] = &[
    ("single_extruder_multi_material", "1"),
    ("seam_position", "aligned"),
    ("spiral_mode", "0"),
    ("detect_thin_wall", "1"),
    ("detect_overhang_wall", "1"),
    ("enable_support", "0"),
    ("support_material", "0"),
    ("raft_layers", "0"),
    ("brim_type", "auto_brim"),
    ("brim_width", "0"),
    ("skirt_loops", "1"),
    ("skirt_distance", "2"),
    ("sparse_infill_density", "15%"),
    ("sparse_infill_pattern", "grid"),
    ("top_surface_pattern", "monotonic"),
    ("bottom_surface_pattern", "monotonic"),
    ("ironing_type", "no ironing"),
    ("fuzzy_skin", "none"),
    ("bottom_shell_layers", "3"),
    ("reduce_crossing_wall", "0"),
    ("max_travel_detour_distance", "0"),
    ("resolution", "0.012"),
    ("xy_hole_compensation", "0"),
    ("xy_contour_compensation", "0"),
    ("elefant_foot_compensation", "0"),
    ("seam_gap", "10%"),
    ("wipe_on_loops", "1"),
    ("reduce_infill_retraction", "1"),
    ("z_offset", "0"),
    ("printable_height", "250"),
    ("print_sequence", "by layer"),
    ("slow_down_for_layer_cooling", "1"),
    ("slow_down_layer_time", "8"),
    ("fan_cooling_layer_time", "100"),
    ("reduce_fan_stop_start_freq", "1"),
    ("prime_tower_brim_width", "3"),
    ("wipe_tower_rotation_angle", "0"),
    ("wall_loops", "2"),
    ("top_shell_layers", "4"),
    ("infill_direction", "45"),
    ("wall_generator", "arachne"),
    ("ironing_pattern", "rectilinear"),
    ("support_type", "normal(auto)"),
    ("support_style", "default"),
    ("support_expansion", "0"),
    ("support_top_z_distance", "0.2"),
    ("support_bottom_z_distance", "0.2"),
    ("tree_support_branch_angle", "40"),
    ("tree_support_branch_diameter", "5"),
    ("tree_support_branch_diameter_angle", "5"),
    ("interface_shells", "0"),
    ("seam_slope_type", "none"),
    ("brim_object_gap", "0"),
    ("skirt_height", "1"),
    ("raft_first_layer_density", "90%"),
    ("filter_out_gap_fill", "0"),
    ("fuzzy_skin_mode", "displacement"),
    ("fuzzy_skin_thickness", "0.2"),
    ("fuzzy_skin_point_distance", "0.3"),
    ("wipe_tower_no_sparse_layers", "0"),
    ("wipe_tower_x", "15"),
    ("wipe_tower_y", "220"),
    ("wipe_tower_width", "60"),
    ("ooze_prevention", "0"),
    ("gap_fill_target", "nowhere"),
    ("extra_perimeters", "0"),
    ("extra_perimeters_on_overhangs", "0"),
    ("wall_direction", "ccw"),
    ("outer_wall_direction", "ccw"),
    ("infill_first", "0"),
    ("solid_infill_filament", "0"),
    ("top_fill_pattern", "monotonic"),
];

/// A `GCodeSerializer` wrapper that injects `THUMBNAIL_BLOCK` and `CONFIG_BLOCK`
/// from the raw config source, delegating core serialization to the inner serializer.
///
/// Used by `run_pipeline_with_raw_config` to inject thumbnail data and config
/// view into the serialization step without changing the `GCodeSerializer` trait.
pub struct ThumbnailAwareSerializer {
    inner: Box<dyn GCodeSerializer>,
    thumbnails: Option<Vec<RenderedThumbnail>>,
    raw_config: HashMap<String, ConfigValue>,
    flavor: GcodeFlavor,
}

impl ThumbnailAwareSerializer {
    /// Create a new wrapper around `inner`, optionally attaching rendered
    /// thumbnails and a raw config map for CONFIG_BLOCK emission.
    pub fn new(
        inner: Box<dyn GCodeSerializer>,
        thumbnails: Option<Vec<RenderedThumbnail>>,
        raw_config: HashMap<String, ConfigValue>,
    ) -> Self {
        let flavor = raw_config
            .get("gcode_flavor")
            .and_then(|value| match value {
                ConfigValue::String(value) => Some(GcodeFlavor::from_config_str(value)),
                _ => None,
            })
            .unwrap_or_default();
        Self {
            inner,
            thumbnails,
            raw_config,
            flavor,
        }
    }

    /// Set the resolved G-code dialect used for synthetic CONFIG_BLOCK keys.
    pub fn with_flavor(mut self, flavor: GcodeFlavor) -> Self {
        self.flavor = flavor;
        self
    }
}

impl Default for ThumbnailAwareSerializer {
    fn default() -> Self {
        Self {
            inner: Box::new(DefaultGCodeSerializer::default()),
            thumbnails: None,
            raw_config: HashMap::new(),
            flavor: GcodeFlavor::Marlin,
        }
    }
}

impl GCodeSerializer for ThumbnailAwareSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
        let base = self.inner.serialize_gcode(gcode_ir)?;

        // 1. Insert THUMBNAIL_BLOCK immediately after HEADER_BLOCK_END (if thumbnail present).
        let base = if let Some(ref entries) = self.thumbnails {
            let sentinel = "; HEADER_BLOCK_END\n";
            let block = serialize_thumbnail_block(entries);
            if let Some(pos) = base.find(sentinel) {
                let insert_at = pos + sentinel.len();
                let mut result = String::with_capacity(base.len() + block.len());
                result.push_str(&base[..insert_at]);
                result.push_str(&block);
                result.push_str(&base[insert_at..]);
                result
            } else {
                let mut result = block;
                result.push_str(&base);
                result
            }
        } else {
            base
        };

        // 2. Rewrite the HEADER_BLOCK's filament/extruder colour lines to the
        // model's authored palette when the config carries one (the inner
        // serializer emits a hardcoded default palette and has no config access).
        let slot_count = gcode_ir.metadata.filament_used_mm.len();
        let default_csv = filament_colour_csv(slot_count);
        let colour_csv = resolve_filament_colour_csv(&self.raw_config, slot_count);
        let base = if colour_csv != default_csv {
            base.replace(
                &format!("; filament_colour = {default_csv}"),
                &format!("; filament_colour = {colour_csv}"),
            )
            .replace(
                &format!("; extruder_colour = {default_csv}"),
                &format!("; extruder_colour = {colour_csv}"),
            )
        } else {
            base
        };

        // 3. Append CONFIG_BLOCK at the end of the output, with the per-filament
        // colour list sized to the tools in use (drives the OrcaSlicer preview).
        let config_block = serialize_config_block(&self.raw_config, &colour_csv, self.flavor);
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
                    output.push_str(&self.flavor.set_temperature(*tool, *celsius, *wait));
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

    #[test]
    fn config_filament_colour_overrides_hardcoded_palette() {
        // Regression (RC1): when the model's authored palette is present in the
        // config, both the CONFIG_BLOCK and the resolved CSV must use it instead
        // of the hardcoded default (which put red first, swapping tools 1 and 4).
        let mut cfg: HashMap<String, ConfigValue> = HashMap::new();
        let authored = "#FF9B00;#02BF06;#1800F2;#EC0006";
        cfg.insert(
            "filament_colour".to_string(),
            ConfigValue::String(authored.to_string()),
        );

        assert_eq!(resolve_filament_colour_csv(&cfg, 4), authored);

        let block = serialize_config_block(&cfg, &filament_colour_csv(4), GcodeFlavor::Marlin);
        assert!(
            block.contains(&format!("; filament_colour = {authored}")),
            "config block must emit authored palette; got:\n{block}"
        );
        // The default hardcoded palette (red first) must NOT appear.
        assert!(!block.contains("; filament_colour = #EC0006;#02BF06;#1800F2;#FF9B00"));
    }

    #[test]
    fn resolve_filament_colour_csv_falls_back_to_default_palette() {
        let cfg: HashMap<String, ConfigValue> = HashMap::new();
        assert_eq!(
            resolve_filament_colour_csv(&cfg, 4),
            filament_colour_csv(4),
            "with no config palette, fall back to the default"
        );
    }
}
