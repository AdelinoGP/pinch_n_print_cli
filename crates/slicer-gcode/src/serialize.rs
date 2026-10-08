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
        | ExtrusionRole::InternalBridgeInfill
        | ExtrusionRole::Ironing
        | ExtrusionRole::WipeTower
        | ExtrusionRole::PrimeTower => cfg.infill_resolution,
        // Support family: support tolerance.
        ExtrusionRole::SupportMaterial
        | ExtrusionRole::SupportInterface
        | ExtrusionRole::SupportBaseInterface => cfg.support_resolution,
        // Travel and other custom moves: no simplification.
        ExtrusionRole::Custom(_) => 0.0,
        // Gap-fill uses perimeter tolerance (it's wall-adjacent).
        ExtrusionRole::GapFill => cfg.gcode_resolution,
        // Raft infill: bed-adhesion layer, print at perimeter resolution so
        // D-P simplification doesn't round the contact surface.
        ExtrusionRole::RaftInfill => cfg.gcode_resolution,
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
    /// Extrusion width for outer wall in mm (default `0.4`, from the `resolve_line_width_mm` fallback in `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`).
    outer_wall_line_width: f32,
    /// Extrusion width for inner walls in mm (default `0.4`, from the `resolve_line_width_mm` fallback in `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`).
    inner_wall_line_width: f32,
    /// Extrusion width for sparse infill in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.45).
    sparse_infill_line_width: f32,
    /// Extrusion width for top surface in mm (OrcaSlicer 0.4 mm nozzle parity default: 0.42).
    top_surface_line_width: f32,
    /// Extrusion width for support material in mm (resolved from config; defaults to 0.4).
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
            // Wall-width defaults follow the 0.4 mm fallback in `resolve_line_width_mm`
            // (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`).
            outer_wall_line_width: 0.4,
            inner_wall_line_width: 0.4,
            sparse_infill_line_width: 0.45,
            top_surface_line_width: 0.42,
            support_line_width: 0.4,
            gcode_xy_decimals: 3,
        }
    }

    /// Sets the G-code dialect used for flavor-specific commands.
    pub fn with_flavor(mut self, flavor: GcodeFlavor) -> Self {
        self.flavor = flavor;
        self
    }

    /// Sets the resolved support extrusion width in millimeters.
    pub fn with_support_line_width(mut self, width_mm: f32) -> Self {
        self.support_line_width = width_mm;
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
/// authored palette carried in `raw_config["filament_colour"]` (seeded from
/// the 3MF project settings), falling back to the hardcoded default palette
/// sized to the tools in use.
///
/// Accepts two shapes for the authored value:
/// - `ConfigValue::String` — semicolon-joined `#RRGGBB` hex codes (the
///   pre-packet-XXX convention used by `main.rs` before the generic 3MF
///   extractor produced lists; still supported for `--config` users).
/// - `ConfigValue::List` — a list of `ConfigValue::String` hex codes, the
///   shape the generic 3MF project-settings extractor now produces
///   (OrcaSlicer stores `filament_colour` as a JSON array in
///   `project_settings.config`).
fn resolve_filament_colour_csv(
    raw_config: &HashMap<String, ConfigValue>,
    slot_count: usize,
) -> String {
    match raw_config.get("filament_colour") {
        Some(ConfigValue::String(s)) if !s.trim().is_empty() => s.clone(),
        Some(ConfigValue::List(items)) => {
            let joined = items
                .iter()
                .filter_map(|v| match v {
                    ConfigValue::String(s) if !s.is_empty() => Some(s.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(";");
            if joined.is_empty() {
                filament_colour_csv(slot_count)
            } else {
                joined
            }
        }
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
/// `Option`-typed fields that are `None` are omitted. The host's wall-path
/// precision is added here rather than to `ResolvedConfig::to_config_map`,
/// which also feeds module ConfigViews and must not change for disclosure alone.
pub fn resolved_config_to_map(cfg: &ResolvedConfig) -> HashMap<String, ConfigValue> {
    let mut map = cfg.to_config_map();
    // Render from the f32 field so the appendix does not display the expanded
    // f64 representation of an otherwise short decimal such as 0.025.
    map.insert(
        "gcode_resolution".into(),
        ConfigValue::String(cfg.gcode_resolution.to_string()),
    );
    map
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

    // OrcaSlicer's `escape_string_cstyle` (`Config.cpp`, reached via
    // `ConfigOptionString::serialize`) escapes these four characters so string
    // values survive the `; key = value` footer round-trip through
    // `ConfigBase::load_from_gcode_file`.
    let escape_cstyle = |s: &str| -> String {
        let mut escaped = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                other => escaped.push(other),
            }
        }
        escaped
    };

    // OrcaSlicer's viewer (ConfigBase::load_from_gcode_file + GCodeProcessor::
    // apply_config) reads ONLY this block. It infers the filament COUNT from the
    // `filament_diameter` array length (ConfigOptionFloats, comma-separated) and
    // applies per-tool colours from `filament_colour` (ConfigOptionStrings,
    // semicolon-separated) only when its length matches. `filament_diameter` is
    // therefore always rendered once per tool in use, from the map's effective
    // value (the schema default `1.75` only when absent) instead of a
    // hard-coded constant; a per-filament `List` from the map is already
    // one-entry-per-tool and reaches the block via the passthrough below
    // (`emitted` dedups it).
    if !matches!(
        raw_config.get("filament_diameter"),
        Some(ConfigValue::List(_))
    ) {
        let effective = match raw_config.get("filament_diameter") {
            Some(ConfigValue::Float(f)) => format_config_float(*f),
            Some(ConfigValue::Int(i)) => format!("{i}"),
            Some(ConfigValue::String(s)) => escape_cstyle(s),
            _ => "1.75".to_string(),
        };
        let diam = vec![effective.as_str(); filament_count].join(",");
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

    // Derive OrcaSlicer-named appendix keys whose PnP resolved-config name
    // differs (see ORCA_CONFIG_ALIAS_KEYS). Emitted before the raw passthrough
    // so a raw key in OrcaSlicer's spelling cannot disclose a value the slice
    // did not apply — PnP honours none of the alias names as inputs. The
    // padding loop below supplies the fallback when the source key is absent
    // (e.g. a serializer constructed without a resolved config).
    for (appendix_key, pnp_key) in ORCA_CONFIG_ALIAS_KEYS {
        if let Some(rendered) = raw_config
            .get(*pnp_key)
            .and_then(|value| render_appendix_alias_value(value, appendix_key))
        {
            emit_config_kv(&mut out, &mut emitted, appendix_key, &rendered);
        }
    }

    // Resolved configurations supply these values through raw_config; retain
    // canonical defaults for serializers constructed without resolved config.
    for (key, value) in SUPPORT_CONFIG_DEFAULTS {
        if !raw_config.contains_key(*key) {
            emit_config_kv(&mut out, &mut emitted, key, value);
        }
    }

    // Sort keys for deterministic output.
    let mut keys: Vec<&String> = raw_config.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = raw_config.get(key) {
            let value_str = match value {
                // OrcaSlicer's `ConfigOptionBool::serialize` writes `1`/`0` and
                // `deserialize` accepts *only* those two literals; anything else
                // falls into the substitution path (`enum_looks_like_true_value`
                // is true only for `enabled`/`on`), so `true`/`false` would load
                // every boolean as `false`. Measured against the pinned
                // `PrintConfig.cpp` 2026-10-07 and OrcaSlicer 2.4.1 output.
                //
                // The two keys PnP models as booleans but OrcaSlicer declares as
                // `coEnum`s (`dont_filter_internal_bridges`,
                // `enable_extra_bridge_layer`) are routed through the translator
                // so their `0`/`1` becomes the enum's wire spelling — Orca's
                // strict `enum_keys_map` lookup would otherwise substitute the
                // option's default and hide a `true`.
                ConfigValue::Bool(b) => {
                    translate_appendix_enum(key, if *b { "1" } else { "0" }).to_string()
                }
                ConfigValue::Int(i) => i.to_string(),
                ConfigValue::Float(f) => format_config_float(*f),
                ConfigValue::String(s) => escape_cstyle(translate_appendix_enum(key, s)),
                ConfigValue::Percent(p) => format!("{p}%"),
                ConfigValue::FloatOrPercent { value, is_percent } => {
                    if *is_percent {
                        format!("{value}%")
                    } else {
                        format_config_float(*value)
                    }
                }
                ConfigValue::List(items) => {
                    // OrcaSlicer serialises vector options with a separator
                    // that depends on the element type: `coStrings` (e.g.
                    // `filament_colour`) join with `;` so the viewer's
                    // `ConfigBase::load_from_gcode_file` can parse them, while
                    // `coFloats` (e.g. `filament_density`, `filament_diameter`)
                    // join with `,`. The element type is the only signal we
                    // have here, so numeric lists take the `coFloats` form and
                    // everything else keeps the historic `coStrings` form.
                    let all_numeric = !items.is_empty()
                        && items
                            .iter()
                            .all(|v| matches!(v, ConfigValue::Float(_) | ConfigValue::Int(_)));
                    let parts: Vec<String> = items
                        .iter()
                        .map(|v| match v {
                            ConfigValue::String(s) => escape_cstyle(s),
                            ConfigValue::Float(f) => format_config_float(*f),
                            ConfigValue::Int(i) => format!("{i}"),
                            ConfigValue::Bool(b) => i64::from(*b).to_string(),
                            other => format!("{other:?}"),
                        })
                        .collect();
                    parts.join(if all_numeric { "," } else { ";" })
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

/// Render a config float at its own precision. Some typed `ResolvedConfig`
/// scalars are `f32` and widen to `f64` at `to_config_map`, so printing the
/// `f64` directly leaks binary noise (`0.44999998807907104` for `0.45`).
/// Values that round-trip through `f32` print with `f32`'s shortest form —
/// the `format!("{}", v)` convention the header width block already uses —
/// and genuine `f64` values keep `f64` shortest form.
fn format_config_float(value: f64) -> String {
    let narrow = f64::from(value as f32);
    if narrow == value {
        format!("{}", value as f32)
    } else {
        format!("{value}")
    }
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

/// Translate a PnP-internal enum spelling into OrcaSlicer's wire spelling.
///
/// The `CONFIG_BLOCK` is read by OrcaSlicer's `ConfigBase::load_from_gcode_file`
/// → `set_deserialize` → `ConfigOptionEnum::deserialize`, which is a strict
/// `enum_keys_map` lookup (`from_string`). A value outside that map is not a
/// near-miss: it falls to the substitution branch, which *replaces the option
/// with its own default* (`opt->set(optdef->default_value.get())`), so the
/// viewer silently displays a setting the slice did not use.
///
/// Three keys in the resolved view carry a PnP-internal spelling that differs
/// from OrcaSlicer's wire value. Each is a pure rename — the value is otherwise
/// identical — so this is a disclosure fix, not a semantics change. Only the
/// appendix is translated; module `ConfigView`s keep reading the PnP spelling.
///
/// Measured 2026-10-07 against the pinned `PrintConfig.cpp`
/// (`s_keys_map_WallSequence`, `s_keys_map_PerimeterGeneratorType`,
/// `s_keys_map_WallDirection`, `s_keys_map_InternalBridgeFilter`) and OrcaSlicer
/// 2.4.1 output (`; wall_sequence = inner wall/outer wall`,
/// `; wall_generator = arachne`, `; wall_direction = ccw`).
fn translate_appendix_enum<'a>(key: &str, value: &'a str) -> &'a str {
    match (key, value) {
        // PnP's `wall_sequence` values are the Rust variant names; canonical
        // `s_keys_map_WallSequence` spells them as wall-order phrases.
        ("wall_sequence", "InnerOuter") => "inner wall/outer wall",
        ("wall_sequence", "OuterInner") => "outer wall/inner wall",
        ("wall_sequence", "InnerOuterInner") => "inner-outer-inner wall",
        // `WallGenerator::Classic` is a Rust variant name; the Orca enum value
        // is lowercase.
        ("wall_generator", "Classic") => "classic",
        ("wall_generator", "Arachne") => "arachne",
        // PnP reads `counter_clockwise` / `clockwise`; Orca's
        // `s_keys_map_WallDirection` is `ccw` / `cw`.
        ("wall_direction", "counter_clockwise") => "ccw",
        ("wall_direction", "clockwise") => "cw",
        // The appendix alias renders `seam_mode` under Orca's `seam_position`
        // name. PnP spells the rear-position variant `rear`; Orca's
        // `s_keys_map_SeamPosition` is `back`.
        ("seam_position", "rear") => "back",
        // `dont_filter_internal_bridges` is a *bool* in PnP (a deliberate
        // compatibility surface, `docs/15_config_keys_reference.md`) but a
        // three-value `coEnum` in Orca. `false` is PnP's "full filtering",
        // which is canonical `ibfDisabled`.
        ("dont_filter_internal_bridges", "0") => "disabled",
        ("dont_filter_internal_bridges", "1") => "nofilter",
        ("dont_filter_internal_bridges", "false") => "disabled",
        ("dont_filter_internal_bridges", "true") => "nofilter",
        // Same shape: `enable_extra_bridge_layer` is a bool in PnP (the extra
        // bridge layer is authored by the ShellClassification prepass) and a
        // four-value `coEnum` in Orca. PnP's `true` authors the layer for every
        // qualified bridge, which is canonical `eblApplyToAll`.
        ("enable_extra_bridge_layer", "0") => "disabled",
        ("enable_extra_bridge_layer", "1") => "apply_to_all",
        ("enable_extra_bridge_layer", "false") => "disabled",
        ("enable_extra_bridge_layer", "true") => "apply_to_all",
        _ => value,
    }
}

/// Padding table contains only neutral cosmetic CONFIG_BLOCK keys. PNP never
/// emits speed/accel/jerk/machine-limit values here; those are the fork's
/// responsibility per the CONFIG_BLOCK viewer-key contract in
/// `docs/02_ir_schemas.md`.
const ORCA_CONFIG_PADDING: &[(&str, &str)] = &[
    // Not `single_extruder_multi_material`, `ooze_prevention`, `printable_height`
    // or `z_offset`: each of those is read by the viewer's `GCodeProcessor`
    // (`printable_height` into the height check, `z_offset` into every emitted
    // position, `ooze_prevention` into `backtrace_enabled`,
    // `single_extruder_multi_material` into the extruder-offset branch), and the
    // minimum-key contract forbids padding that "the viewer's `GCodeProcessor`"
    // feeds into motion/time computation. `spiral_mode` is retained because it is
    // not a fabrication — `ORCA_CONFIG_ALIAS_KEYS` renders it from PnP's real
    // `spiral_vase` setting.
    ("seam_position", "aligned"),
    ("spiral_mode", "0"),
    ("detect_thin_wall", "0"),
    ("detect_overhang_wall", "1"),
    ("enable_support", "0"),
    ("raft_layers", "0"),
    ("brim_type", "auto_brim"),
    ("brim_width", "0"),
    ("skirt_loops", "1"),
    ("skirt_distance", "2"),
    ("sparse_infill_density", "20%"),
    ("sparse_infill_pattern", "crosshatch"),
    ("top_surface_pattern", "monotonicline"),
    ("bottom_surface_pattern", "monotonic"),
    ("ironing_type", "no ironing"),
    ("fuzzy_skin", "disabled_fuzzy"),
    ("bottom_shell_layers", "3"),
    ("reduce_crossing_wall", "0"),
    ("max_travel_detour_distance", "0"),
    ("resolution", "0.01"),
    ("xy_hole_compensation", "0"),
    ("xy_contour_compensation", "0"),
    ("elefant_foot_compensation", "0"),
    ("seam_gap", "10%"),
    ("wipe_on_loops", "0"),
    ("reduce_infill_retraction", "0"),
    ("print_sequence", "by layer"),
    ("slow_down_for_layer_cooling", "1"),
    ("slow_down_layer_time", "5"),
    ("fan_cooling_layer_time", "60"),
    ("reduce_fan_stop_start_freq", "0"),
    ("prime_tower_brim_width", "3"),
    ("wipe_tower_rotation_angle", "0"),
    ("wall_loops", "2"),
    ("top_shell_layers", "4"),
    ("infill_direction", "45"),
    ("wall_generator", "arachne"),
    ("ironing_pattern", "rectilinear"),
    ("support_type", "normal(auto)"),
    ("support_style", "default"),
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
    ("gap_fill_target", "nowhere"),
    ("enable_arc_fitting", "0"),
    ("extra_perimeters_on_overhangs", "0"),
    ("wall_direction", "ccw"),
    // Not `outer_wall_direction` / `infill_first` / `solid_infill_filament` /
    // `top_fill_pattern`: those are not OrcaSlicer keys at all (absent from the
    // pinned `PrintConfig.cpp` and from OrcaSlicer 2.4.1 output), so they cannot
    // count toward the viewer's minimum-key gate. These four replace them with
    // real cosmetic Orca keys at their upstream defaults.
    ("solid_infill_direction", "45"),
    ("is_infill_first", "0"),
    ("top_solid_infill_flow_ratio", "1.0"),
    ("top_surface_density", "100%"),
    // Replacing the four forbidden viewer-consumed rows above: same class of
    // neutral toggle/count/geometry-cosmetic keys, none of them read by
    // `GCodeProcessor`.
    ("precise_z_height", "0"),
    ("hole_to_polyhole", "0"),
    ("staggered_inner_seams", "0"),
    ("wall_distribution_count", "1"),
];

const SUPPORT_CONFIG_DEFAULTS: &[(&str, &str)] = &[
    ("support_expansion", "0"),
    ("support_top_z_distance", "0.2"),
    ("support_bottom_z_distance", "0.2"),
];

/// Appendix keys PnP carries under a different config name, as
/// `(appendix_key, pnp_key)`.
///
/// [`ResolvedConfig::to_config_map`] and the module manifests export the PnP
/// spelling — `wall_count` for the wall loop count, `infill_angle` for the
/// infill direction, `seam_mode` for the seam position — while OrcaSlicer's
/// appendix reader (`ConfigBase::load_from_gcode_file` →
/// `GCodeProcessor::apply_config`) names the same setting `wall_loops` /
/// `infill_direction` / `seam_position`. Without this derivation the appendix
/// key misses the source value in the effective config, falls through to
/// [`ORCA_CONFIG_PADDING`]'s static default, and mis-discloses every
/// non-default setting (measured 2026-09-22: `wall_count` 1 and 3 both printed
/// `wall_loops = 2` while the wall output changed accordingly).
///
/// Every row is a 1:1 setting. Keys that look related but are not the same
/// setting are deliberately absent: `sparse_infill_density` is a distinct
/// perimeter-module percentage (module default `20`, percent unit) while the
/// resolved `infill_density` is a fraction (default `0.2`);
/// `raft_layers` is split across `support_raft_layers`, `base_raft_layers`, and
/// `interface_raft_layers`; `ironing_type` is a narrowed bool upstream
/// (`ironing_enabled`), not a rename. Padding-only values remain Orca defaults,
/// not evidence of PnP's effective settings.
const ORCA_CONFIG_ALIAS_KEYS: &[(&str, &str)] = &[
    // classic-/arachne-perimeters read `wall_count`; Orca names the same
    // plain-integer count `wall_loops`.
    ("wall_loops", "wall_count"),
    // gyroid-/rectilinear-infill read `infill_angle`; Orca names the same
    // degrees value `infill_direction`.
    ("infill_direction", "infill_angle"),
    // Host precision key. Orca has one global `resolution`; the adjudicated
    // PnP counterpart is `gcode_resolution` (03-asset-scoped-gap.md), whose
    // resolved default is `0.0125` (`ResolvedConfig::gcode_resolution`), not
    // Orca's snapshot default `0.01`.
    ("resolution", "gcode_resolution"),
    // Legacy Slic3r spelling of the support toggle; the resolved bool is
    // `enable_support`.
    ("support_material", "enable_support"),
    // seam-placer / seam-planner-default read `seam_mode`; Orca names the
    // enum `seam_position`.
    ("seam_position", "seam_mode"),
    // slicer-scheduler reads `spiral_vase` (`SPIRAL_VASE_CONFIG_KEY` in
    // `crates/slicer-scheduler/src/execution_plan.rs`); Orca names the bool
    // `spiral_mode`.
    ("spiral_mode", "spiral_vase"),
    // fuzzy-skin reads the non-namespaced `thickness` / `point_distance`;
    // Orca names them `fuzzy_skin_thickness` / `fuzzy_skin_point_distance`
    // (03-asset-scoped-gap.md; rename workstream ticket 103).
    ("fuzzy_skin_thickness", "thickness"),
    ("fuzzy_skin_point_distance", "point_distance"),
];

/// Render an alias source value into the appendix's scalar spelling, or `None`
/// when the variant has no scalar rendering (the [`ORCA_CONFIG_PADDING`]
/// default then stands). Bools render as `1`/`0` — the only two literals
/// OrcaSlicer's `ConfigOptionBool::deserialize` accepts. String values pass
/// through [`translate_appendix_enum`] keyed by the *appendix* name, so an
/// alias whose PnP-internal spelling differs from OrcaSlicer's wire value
/// (`seam_mode`'s `rear` → `seam_position`'s `back`) is canonicalized too.
fn render_appendix_alias_value(value: &ConfigValue, appendix_key: &str) -> Option<String> {
    match value {
        ConfigValue::Bool(b) => Some(u8::from(*b).to_string()),
        ConfigValue::Int(i) => Some(i.to_string()),
        ConfigValue::Float(f) => Some(format!("{f}")),
        ConfigValue::String(s) => Some(translate_appendix_enum(appendix_key, s).to_string()),
        ConfigValue::List(_) | ConfigValue::Percent(_) | ConfigValue::FloatOrPercent { .. } => None,
    }
}

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
    fn support_line_width_header_sources_resolved_value() {
        let output = DefaultGCodeSerializer::new()
            .with_support_line_width(0.42)
            .serialize_gcode(&GCodeIR::default())
            .expect("default GCodeIR must serialize");

        assert!(output.contains("; support_line_width = 0.42"));
    }

    #[test]
    fn config_block_renders_f32_scalars_at_f32_precision() {
        // Some typed `ResolvedConfig` scalars are `f32` and widen to `f64` in
        // `to_config_map`; the block must render them at `f32` precision
        // (`0.45`, the `format!("{}", f32)` shortest form the header width
        // block already uses), not the widened `0.44999998807907104` that
        // broke the AC-4 `; infill_overlap = 0.45` presence check while the
        // 0.30 run's `0.30000001192092896` still prefix-matched `0.3`.
        let cfg: HashMap<String, ConfigValue> = HashMap::from([(
            "infill_overlap".to_string(),
            ConfigValue::Float(f64::from(0.45_f32)),
        )]);
        let block = serialize_config_block(&cfg, &filament_colour_csv(4), GcodeFlavor::Marlin);
        assert!(
            block.contains("; infill_overlap = 0.45"),
            "f32-derived floats must render at f32 precision; got:\n{block}"
        );
    }

    #[test]
    fn config_block_float_precision_is_consistent_across_scalars_and_numeric_lists() {
        let widened = f64::from(0.45_f32);
        let precise = 0.451_234_567_890_123_f64;
        assert_ne!(precise, f64::from(precise as f32));
        let cfg: HashMap<String, ConfigValue> = HashMap::from([
            ("infill_overlap".to_string(), ConfigValue::Float(widened)),
            ("line_width".to_string(), ConfigValue::Float(precise)),
            ("filament_diameter".to_string(), ConfigValue::Float(widened)),
            (
                "filament_density".to_string(),
                ConfigValue::List(vec![
                    ConfigValue::Float(widened),
                    ConfigValue::Float(precise),
                ]),
            ),
        ]);
        let block = serialize_config_block(&cfg, &filament_colour_csv(2), GcodeFlavor::Marlin);
        assert!(block.lines().any(|line| line == "; infill_overlap = 0.45"));
        assert!(block
            .lines()
            .any(|line| line == "; line_width = 0.451234567890123"));
        assert!(block
            .lines()
            .any(|line| line == "; filament_diameter = 0.45,0.45"));
        assert!(block
            .lines()
            .any(|line| line == "; filament_density = 0.45,0.451234567890123"));
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

    #[test]
    fn resolve_filament_colour_csv_accepts_list_of_strings() {
        // The generic 3MF `project_settings.config` extractor surfaces
        // `filament_colour` as `ConfigValue::List` (OrcaSlicer stores it as
        // a JSON array). The resolver must join it back into the
        // semicolon-separated CSV the G-code header / CONFIG_BLOCK need.
        let mut cfg: HashMap<String, ConfigValue> = HashMap::new();
        cfg.insert(
            "filament_colour".to_string(),
            ConfigValue::List(vec![
                ConfigValue::String("#FF9B00".to_string()),
                ConfigValue::String("#02BF06".to_string()),
                ConfigValue::String("#1800F2".to_string()),
                ConfigValue::String("#EC0006".to_string()),
            ]),
        );
        assert_eq!(
            resolve_filament_colour_csv(&cfg, 4),
            "#FF9B00;#02BF06;#1800F2;#EC0006",
            "List shape must be joined with `;` to match the historic String shape"
        );
    }

    #[test]
    fn config_block_joins_list_values_with_semicolons() {
        // Verify the CONFIG_BLOCK dumper renders a `List` value with `;`
        // separators — OrcaSlicer's viewer reads the block with
        // `ConfigOptionStrings` parsing (semicolon-separated) for keys like
        // `filament_colour`. If we joined with `,` the viewer would fail
        // to parse the palette.
        let mut cfg: HashMap<String, ConfigValue> = HashMap::new();
        cfg.insert(
            "filament_colour".to_string(),
            ConfigValue::List(vec![
                ConfigValue::String("#FF9B00".to_string()),
                ConfigValue::String("#02BF06".to_string()),
            ]),
        );
        let block = serialize_config_block(&cfg, &filament_colour_csv(4), GcodeFlavor::Marlin);
        assert!(
            block.contains("; filament_colour = #FF9B00;#02BF06"),
            "List must be emitted as `;`-joined string in CONFIG_BLOCK; got:\n{block}"
        );
        assert!(
            !block.contains("; filament_colour = #FF9B00,#02BF06"),
            "List must NOT be `,`-joined (regression — that was the pre-fix behaviour)"
        );
    }

    #[test]
    fn config_block_joins_numeric_list_values_with_commas() {
        // `coFloats` options (`filament_density`, `filament_diameter`, the
        // `machine_max_*` family) serialise comma-separated in canonical
        // `ConfigOptionFloatsTempl::serialize`, unlike the `;` used by
        // `coStrings` via `escape_strings_cstyle`. A 4-filament project's
        // canonical line is exactly `; filament_density = 1.24,1.24,1.24,1.24`.
        let mut cfg: HashMap<String, ConfigValue> = HashMap::new();
        cfg.insert(
            "filament_density".to_string(),
            ConfigValue::List(vec![
                ConfigValue::Float(1.24),
                ConfigValue::Float(1.24),
                ConfigValue::Float(1.27),
            ]),
        );
        let block = serialize_config_block(&cfg, &filament_colour_csv(4), GcodeFlavor::Marlin);
        assert!(
            block.contains("; filament_density = 1.24,1.24,1.27"),
            "numeric List must be `,`-joined and rendered as bare numbers; got:\n{block}"
        );
        assert!(
            !block.contains("Float("),
            "list elements must not be Debug-formatted; got:\n{block}"
        );
    }

    #[test]
    fn config_block_discloses_resolved_wall_count_under_orca_name() {
        // The old padding always claimed two walls even when the perimeter
        // generators received wall_count = 1 or 3.
        for wall_count in [1, 3] {
            let cfg = ResolvedConfig {
                wall_count,
                ..ResolvedConfig::default()
            };
            let mut map = resolved_config_to_map(&cfg);
            // An Orca-named raw field is not the setting PnP applied.
            map.insert("wall_loops".into(), ConfigValue::Int(99));
            let block = serialize_config_block(&map, &filament_colour_csv(1), GcodeFlavor::Marlin);
            let actual: Vec<_> = block
                .lines()
                .filter(|line| line.starts_with("; wall_loops = "))
                .collect();
            assert_eq!(actual, [format!("; wall_loops = {wall_count}")], "{block}");
        }
    }

    #[test]
    fn config_block_discloses_distinct_resolved_precision_and_infill_direction() {
        let cfg = ResolvedConfig {
            gcode_resolution: 0.025,
            infill_resolution: 0.08,
            infill_angle: 30.0,
            ..ResolvedConfig::default()
        };
        let mut map = resolved_config_to_map(&cfg);
        map.insert("resolution".into(), ConfigValue::Float(0.5));
        let block = serialize_config_block(&map, &filament_colour_csv(1), GcodeFlavor::Marlin);
        let resolution: Vec<_> = block
            .lines()
            .filter(|line| line.starts_with("; resolution = "))
            .collect();
        let direction: Vec<_> = block
            .lines()
            .filter(|line| line.starts_with("; infill_direction = "))
            .collect();
        assert_eq!(resolution, ["; resolution = 0.025"], "{block}");
        assert_eq!(direction, ["; infill_direction = 30"], "{block}");
    }

    #[test]
    fn config_block_keeps_sparse_infill_density_distinct_from_fractional_infill_density() {
        // sparse_infill_density belongs to perimeter modules and is measured
        // in percent; the resolved infill_density is a separate fraction.
        let block = serialize_config_block(
            &resolved_config_to_map(&ResolvedConfig::default()),
            &filament_colour_csv(1),
            GcodeFlavor::Marlin,
        );
        let density: Vec<_> = block
            .lines()
            .filter(|line| line.starts_with("; sparse_infill_density = "))
            .collect();
        assert_eq!(density, ["; sparse_infill_density = 20%"], "{block}");
    }

    /// OrcaSlicer's `ConfigOptionBool::serialize`/`deserialize` speak `1`/`0`
    /// only. Emitting `true`/`false` sends every boolean down the substitution
    /// branch (`enum_looks_like_true_value` is true only for `enabled`/`on`), so
    /// the viewer would load `false` for *every* boolean the fork did not
    /// itself supply — including `enable_support = 1`.
    #[test]
    fn config_block_renders_booleans_as_orca_wire_literals() {
        // Explicit inputs: the rendered literal must be *exactly* `1`/`0`, not
        // merely "not true/false" — this is the block path OrcaSlicer reads.
        let cfg: HashMap<String, ConfigValue> = HashMap::from([
            ("enable_support".to_string(), ConfigValue::Bool(true)),
            ("detect_overhang_wall".to_string(), ConfigValue::Bool(false)),
            (
                "support_remove_small_overhang".to_string(),
                ConfigValue::Bool(true),
            ),
        ]);
        let explicit = serialize_config_block(&cfg, &filament_colour_csv(1), GcodeFlavor::Marlin);
        for (key, literal) in [
            ("enable_support", "1"),
            ("detect_overhang_wall", "0"),
            ("support_remove_small_overhang", "1"),
        ] {
            let expected = format!("; {key} = {literal}");
            assert!(
                explicit.lines().any(|line| line == expected.as_str()),
                "`{key}` must render as the Orca wire literal `{literal}`; got:\n{explicit}"
            );
        }

        let block = serialize_config_block(
            &resolved_config_to_map(&ResolvedConfig::default()),
            &filament_colour_csv(1),
            GcodeFlavor::Marlin,
        );
        let offenders: Vec<&str> = block
            .lines()
            .filter(|line| line.starts_with("; ") && line.contains(" = "))
            .filter(|line| {
                let value = line.split(" = ").nth(1).unwrap_or_default();
                value == "true" || value == "false"
            })
            .collect();
        assert!(
            offenders.is_empty(),
            "CONFIG_BLOCK must use OrcaSlicer's 1/0 wire literals for booleans; \
             OrcaSlicer's parser rejects `true`/`false` and substitutes `false`. \
             Offending lines: {offenders:#?}\n{block}"
        );
    }

    /// Helper coverage ({`appendix_enum_translation_is_a_pure_rename`}) is not
    /// path coverage: a PnP-internal spelling reaching
    /// `serialize_config_block` must be canonicalized in the emitted line —
    /// including the `seam_mode` alias (rendered under Orca's `seam_position`)
    /// and the two keys PnP models as bools but Orca declares as `coEnum`s.
    /// Wire tokens follow the pinned `s_keys_map_WallSequence`,
    /// `s_keys_map_PerimeterGeneratorType`, `s_keys_map_WallDirection`,
    /// `s_keys_map_SeamPosition`, and `s_keys_map_InternalBridgeFilter` maps.
    #[test]
    fn config_block_canonicalizes_enum_spellings_on_the_wire() {
        let cfg: HashMap<String, ConfigValue> = HashMap::from([
            (
                "wall_sequence".to_string(),
                ConfigValue::String("InnerOuter".to_string()),
            ),
            (
                "wall_generator".to_string(),
                ConfigValue::String("Classic".to_string()),
            ),
            (
                "wall_direction".to_string(),
                ConfigValue::String("counter_clockwise".to_string()),
            ),
            (
                "dont_filter_internal_bridges".to_string(),
                ConfigValue::Bool(true),
            ),
            (
                "seam_mode".to_string(),
                ConfigValue::String("rear".to_string()),
            ),
        ]);
        let block = serialize_config_block(&cfg, &filament_colour_csv(1), GcodeFlavor::Marlin);
        for expected in [
            "; wall_sequence = inner wall/outer wall",
            "; wall_generator = classic",
            "; wall_direction = ccw",
            "; dont_filter_internal_bridges = nofilter",
            "; seam_position = back",
        ] {
            assert!(
                block.lines().any(|line| line == expected),
                "block must carry `{expected}`; got:\n{block}"
            );
        }
        assert!(
            !block.contains("; wall_generator = Classic"),
            "the PnP-internal spelling must not reach the wire; got:\n{block}"
        );
    }

    /// The `CONFIG_BLOCK` is an OrcaSlicer *viewer* surface: `set_deserialize`
    /// ignores unknown keys silently and does not count them toward the `>= 80`
    /// pair gate. A padding row naming a key OrcaSlicer cannot load is therefore
    /// worse than useless — it pads the line count without satisfying the gate
    /// the padding exists to satisfy.
    ///
    /// The reference snapshot (`docs/ORCA_CONFIG_REFERENCE.md`) is the in-repo
    /// record of OrcaSlicer's declared keys; `handle_legacy` renames are accepted
    /// too because the loader applies them before the map lookup.
    #[test]
    fn config_block_padding_rows_name_orca_loadable_keys() {
        let reference = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/ORCA_CONFIG_REFERENCE.md"),
        )
        .expect("ORCA_CONFIG_REFERENCE.md must be present");

        // Keys the loader can reach: a reference row, or a `handle_legacy`
        // rename source (those are re-keyed before lookup, e.g.
        // `wipe_tower_width` -> `prime_tower_width`).
        let legacy_renames: [&str; 1] = ["wipe_tower_width"];

        let block = serialize_config_block(
            // The table alone: an empty map reproduces the bare fallback.
            &std::collections::HashMap::new(),
            &filament_colour_csv(1),
            GcodeFlavor::Marlin,
        );
        let padding_lines: Vec<&str> = block
            .lines()
            .filter(|line| line.starts_with("; ") && line.contains(" = "))
            .filter(|line| {
                let key = line[2..].split(" = ").next().unwrap_or_default();
                // Only rows that came from the padding table are judged here;
                // the synthetic keys are asserted by their own test.
                ORCA_CONFIG_PADDING.iter().any(|(k, _)| *k == key)
            })
            .collect();

        let unloadable: Vec<&&str> = padding_lines
            .iter()
            .filter(|line| {
                let key = &line[2..line.find(" = ").unwrap_or(line.len())];
                !legacy_renames.contains(&key) && !reference.contains(&format!("\"{key}\""))
            })
            .collect();

        assert!(
            unloadable.is_empty(),
            "every ORCA_CONFIG_PADDING row must name a key OrcaSlicer's loader \
             recognizes (silently-ignored keys do not count toward its minimum-key \
             gate); unloadable rows: {unloadable:#?}"
        );
    }

    /// The minimum-key contract (`docs/02_ir_schemas.md`) forbids padding rows
    /// that the viewer's `GCodeProcessor` feeds into motion or time computation,
    /// and forbids the `*speed*` / `*acceleration*` / `*jerk*` / `machine_max_*`
    /// family outright. This pins the PnP-side half of that rule (a reader can
    /// check the Orca side in `GCodeProcessor.cpp`).
    #[test]
    fn padding_never_fabricates_viewer_consumed_keys() {
        let banned_family = |key: &str| {
            ["speed", "acceleration", "jerk", "machine_max"]
                .iter()
                .any(|needle| key.to_ascii_lowercase().contains(needle))
        };
        // Keys the viewer's `GCodeProcessor` reads. `spiral_mode` is exempt
        // because it is not a fabrication: `ORCA_CONFIG_ALIAS_KEYS` renders it
        // from PnP's real `spiral_vase` setting.
        let viewer_consumed: [&str; 4] = [
            "single_extruder_multi_material",
            "ooze_prevention",
            "printable_height",
            "z_offset",
        ];

        let offenders: Vec<&str> = ORCA_CONFIG_PADDING
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| banned_family(key) || viewer_consumed.contains(key))
            .collect();
        assert!(
            offenders.is_empty(),
            "padding rows must not name motion/time keys or the banned \
             speed/accel/jerk/machine_max family; offenders: {offenders:?}"
        );
    }

    /// The wire-spelling translator is a pure rename: it must map the three
    /// PnP-internal spellings onto OrcaSlicer's `enum_keys_map` literals and
    /// leave everything else untouched, so a module `ConfigView` never receives
    /// a canonicalized value.
    #[test]
    fn appendix_enum_translation_is_a_pure_rename() {
        assert_eq!(
            translate_appendix_enum("wall_sequence", "InnerOuter"),
            "inner wall/outer wall"
        );
        assert_eq!(
            translate_appendix_enum("wall_sequence", "OuterInner"),
            "outer wall/inner wall"
        );
        assert_eq!(
            translate_appendix_enum("wall_sequence", "InnerOuterInner"),
            "inner-outer-inner wall"
        );
        assert_eq!(
            translate_appendix_enum("wall_generator", "Classic"),
            "classic"
        );
        assert_eq!(
            translate_appendix_enum("wall_generator", "Arachne"),
            "arachne"
        );
        assert_eq!(
            translate_appendix_enum("wall_direction", "counter_clockwise"),
            "ccw"
        );
        assert_eq!(translate_appendix_enum("wall_direction", "clockwise"), "cw");
        // The appendix alias renders `seam_mode` under `seam_position`; PnP's
        // `rear` is Orca's `back`.
        assert_eq!(translate_appendix_enum("seam_position", "rear"), "back");
        assert_eq!(
            translate_appendix_enum("seam_position", "aligned"),
            "aligned"
        );
        assert_eq!(translate_appendix_enum("seam_mode", "rear"), "rear");
        // `dont_filter_internal_bridges` is a bool in PnP and a three-value
        // enum in Orca; the mapping is the documented one. Both the rendered
        // wire literal (`0`/`1`) and the manifest text are accepted.
        assert_eq!(
            translate_appendix_enum("dont_filter_internal_bridges", "0"),
            "disabled"
        );
        assert_eq!(
            translate_appendix_enum("dont_filter_internal_bridges", "1"),
            "nofilter"
        );
        assert_eq!(
            translate_appendix_enum("enable_extra_bridge_layer", "0"),
            "disabled"
        );
        assert_eq!(
            translate_appendix_enum("enable_extra_bridge_layer", "1"),
            "apply_to_all"
        );
        // Untouched cases, including a value that happens to be a variant name
        // for a *different* key.
        assert_eq!(
            translate_appendix_enum("wall_generator", "arachne"),
            "arachne"
        );
        assert_eq!(
            translate_appendix_enum("unrelated_key", "Classic"),
            "Classic"
        );
        assert_eq!(translate_appendix_enum("wall_direction", "ccw"), "ccw");
    }
}
