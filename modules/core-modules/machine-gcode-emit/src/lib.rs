// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/[Various]
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Core module: emits custom G-code through a closed injection-point registry.
//! `machine_start_gcode` prepends ahead of every command; the layer-scoped
//! points (`before_layer_change_gcode`, `time_lapse_gcode`,
//! `layer_change_gcode`) splice after each layer's `;LAYER_CHANGE` / `;Z:` /
//! `;HEIGHT:` marker triple, in that order; filament toolchange points bracket
//! each `ToolChange`; `machine_end_gcode` appends after the last command. Every
//! template goes through single-pass [key]
//! substitution against the effective ConfigView plus per-site layer
//! variables. Substitution lives in the WASM guest; the host serializer just
//! renders the command list.
//! The eleven registered points are machine_start_gcode,
//! before_layer_change_gcode, time_lapse_gcode, layer_change_gcode,
//! filament_end_gcode, change_filament_gcode, filament_start_gcode,
//! change_extrusion_role_gcode, filament_change_extrusion_role_gcode,
//! process_change_extrusion_role_gcode, and machine_end_gcode. The five
//! unreachable points are file_start_gcode, wrapping_detection_gcode,
//! machine_pause_gcode, template_custom_gcode, and printing_by_object_gcode;
//! they remain intentionally unimplemented.
//!
//! Per `docs/adr/0051-gcode-marker-contract-ownership.md` (amendment recorded
//! as `D-285-ADR-0051-AMENDED` in `docs/DEVIATION_LOG.md`), a malformed
//! `;LAYER_CHANGE` — one not followed within two commands by a `;Z:` marker —
//! is surfaced via exactly one `ERR_MALFORMED_LAYER_MARKER` warning, the walk
//! reuses the prior layer Z (or, for layer 1, layer 1's own initial Z
//! context), and `run_gcode_postprocess` returns `Ok`. The warning is the
//! obligation; the prior Z is the documented fallback. See the test
//! `malformed_layer_marker_warns_and_uses_prior_z_per_adr_0051` for the
//! pinning case.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::{BTreeSet, HashMap};

use slicer_ir::ConfigView;
use slicer_ir::{ConfigValue, GCodeCommand};
use slicer_sdk::error::ModuleError;
use slicer_sdk::postpass_builders::{GcodeMoveCmd, GcodeOutputBuilder};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PostpassModule;

/// Machine-gcode-emit GCodePostProcess module.
pub struct MachineGcodeEmit;

/// Legacy placeholder names that canonical accepts as aliases of a current
/// config key.
///
/// Ported from canonical `GCode::update_placeholder_parser_with_variant_params`,
/// which sets `first_layer_temperature` unconditionally under the comment
/// "first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer".
/// Applied *after* the `config.keys()` sweep, so a real config key of the same
/// name would win if one ever appeared. These are not manifest keys.
const PLACEHOLDER_ALIASES: &[(&str, &str)] = &[(
    "first_layer_temperature",
    "nozzle_temperature_initial_layer",
)];

/// Injection sites recognized by the registry.
///
/// The three layer-scoped sites resolve `layer_num`, `layer_z` and
/// `max_layer_z` against the layer boundary they follow; `PrintEnd` resolves
/// them against the final layer context; `PrintStart` has no layer context,
/// so a layer variable there passes through verbatim (with one warning).
enum InjectionSite {
    /// Ahead of every command, including the M73 progress pair and the
    /// ExtrusionMode declaration.
    PrintStart,
    /// Immediately after the layer marker triple, first of the three.
    BeforeLayerChange,
    /// Between before-layer-change and layer-change.
    TimeLapse,
    /// Immediately after the layer marker triple, last of the three.
    LayerChange,
    /// Immediately before a toolchange, first of the two pre-toolchange points.
    FilamentEnd,
    /// Immediately before a toolchange, after `FilamentEnd`.
    FilamentChange,
    /// Immediately after a toolchange.
    FilamentStart,
    /// Immediately before each `;TYPE:` extrusion-role marker.
    ExtrusionRoleChange,
    /// After the last command (ahead of the host's CONFIG_BLOCK, which is not
    /// part of this stream).
    PrintEnd,
}

impl InjectionSite {
    /// Stable contract name used in diagnostics.
    fn name(&self) -> &'static str {
        match self {
            Self::PrintStart => "PrintStart",
            Self::BeforeLayerChange => "BeforeLayerChange",
            Self::TimeLapse => "TimeLapse",
            Self::LayerChange => "LayerChange",
            Self::FilamentEnd => "FilamentEnd",
            Self::FilamentChange => "FilamentChange",
            Self::FilamentStart => "FilamentStart",
            Self::ExtrusionRoleChange => "ExtrusionRoleChange",
            Self::PrintEnd => "PrintEnd",
        }
    }
}

/// One registry entry: the config key that supplies the template and the site
/// it is spliced at.
struct InjectionPoint {
    config_key: &'static str,
    site: InjectionSite,
}

/// Closed registry of custom-G-code injection points, in declaration order.
///
/// Declaration order is the emission precedence: `machine_start_gcode`
/// prepends, the three layer-scoped points splice after each layer's marker
/// triple in the order listed here, the filament points bracket each
/// `ToolChange`, and `machine_end_gcode` appends.
const INJECTION_POINTS: &[InjectionPoint] = &[
    InjectionPoint {
        config_key: "machine_start_gcode",
        site: InjectionSite::PrintStart,
    },
    InjectionPoint {
        config_key: "before_layer_change_gcode",
        site: InjectionSite::BeforeLayerChange,
    },
    InjectionPoint {
        config_key: "time_lapse_gcode",
        site: InjectionSite::TimeLapse,
    },
    InjectionPoint {
        config_key: "layer_change_gcode",
        site: InjectionSite::LayerChange,
    },
    InjectionPoint {
        config_key: "filament_end_gcode",
        site: InjectionSite::FilamentEnd,
    },
    InjectionPoint {
        config_key: "change_filament_gcode",
        site: InjectionSite::FilamentChange,
    },
    InjectionPoint {
        config_key: "filament_start_gcode",
        site: InjectionSite::FilamentStart,
    },
    InjectionPoint {
        config_key: "change_extrusion_role_gcode",
        site: InjectionSite::ExtrusionRoleChange,
    },
    InjectionPoint {
        config_key: "filament_change_extrusion_role_gcode",
        site: InjectionSite::ExtrusionRoleChange,
    },
    InjectionPoint {
        config_key: "process_change_extrusion_role_gcode",
        site: InjectionSite::ExtrusionRoleChange,
    },
    InjectionPoint {
        config_key: "machine_end_gcode",
        site: InjectionSite::PrintEnd,
    },
];

/// Per-layer substitution context carried across the command walk.
///
/// `layer_num` is 1-based: the Nth `;LAYER_CHANGE` marker opens layer N.
/// `layer_z` / `max_layer_z` hold the verbatim text after `;Z:` — never a
/// re-rendered float — and `max_layer_z` is the running maximum over layers
/// seen so far, inclusive of the current one.
struct LayerContext {
    layer_num: u32,
    layer_z: String,
    max_layer_z: String,
}

/// Values available while processing one `ToolChange` command.
struct ToolChangeContext {
    previous_extruder: String,
    next_extruder: String,
    toolchange_count: u32,
}

/// Values available while processing one `;TYPE:` marker.
struct ExtrusionRoleContext {
    current_role: String,
    last_role: String,
}

/// Diagnostic identifier for a `;LAYER_CHANGE` marker not followed by a `;Z:`
/// marker within two commands. Distinct from the push-failure codes 1-11 and
/// 13 used below; surfaced in the warning text, not as a fatal error.
const ERR_MALFORMED_LAYER_MARKER: u32 = 12;

#[slicer_module]
impl PostpassModule for MachineGcodeEmit {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_gcode_postprocess(
        &self,
        commands: &[GCodeCommand],
        output: &mut GcodeOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Step 1: Read the temperature scalars with defaults.
        let bed_temp: i64 = match config.get("bed_temperature_initial_layer_single") {
            Some(ConfigValue::Int(v)) => *v,
            Some(ConfigValue::String(s)) => s.parse::<i64>().unwrap_or(60),
            _ => 60,
        };
        let nozzle_temp: i64 = match config.get("nozzle_temperature_initial_layer") {
            Some(ConfigValue::Int(v)) => *v,
            Some(ConfigValue::String(s)) => s.parse::<i64>().unwrap_or(215),
            _ => 215,
        };

        // Step 2: Build the base substitution lookup.
        let mut base_lookup: HashMap<String, String> = HashMap::new();
        base_lookup.insert(
            "bed_temperature_initial_layer_single".to_string(),
            bed_temp.to_string(),
        );
        base_lookup.insert(
            "nozzle_temperature_initial_layer".to_string(),
            nozzle_temp.to_string(),
        );
        // Also include all other string/int/float keys from config for completeness.
        for key in config.keys() {
            if base_lookup.contains_key(&key) {
                continue;
            }
            let val_str = match config.get(&key).and_then(format_placeholder_value) {
                Some(s) => s,
                None => continue,
            };
            base_lookup.insert(key, val_str);
        }
        // Legacy aliases resolve only where the sweep above left a gap.
        for (alias, target) in PLACEHOLDER_ALIASES {
            if base_lookup.contains_key(*alias) {
                continue;
            }
            if let Some(val) = base_lookup.get(*target).cloned() {
                base_lookup.insert((*alias).to_string(), val);
            }
        }

        // Step 3: Fetch every template through the registry, in declaration
        // order. Empty or absent templates are skipped at their site.
        let templates: Vec<Option<String>> = INJECTION_POINTS
            .iter()
            .map(|point| {
                let configured = config.get(point.config_key);
                match configured {
                    Some(ConfigValue::String(s)) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                }
            })
            .collect();
        // Unresolved [key]s gathered per site, so one aggregated warning at
        // the end can name every contributing site. An unresolved key is
        // **not** a slice error: a module's `ConfigView` is scoped to its own
        // manifest, so "unknown to machine-gcode-emit" is not "unknown to the
        // slicer" — a template may legitimately name a key owned by a module
        // that is not loaded in this pipeline. Aborting would break
        // composition; the key passes through verbatim instead.
        let mut unresolved_per_site: Vec<BTreeSet<String>> =
            INJECTION_POINTS.iter().map(|_| BTreeSet::new()).collect();
        let layer_change_label = INJECTION_POINTS
            .iter()
            .find(|point| matches!(&point.site, &InjectionSite::LayerChange))
            .map(injection_point_label)
            .expect("the injection registry must contain LayerChange");

        // Step 4: PrintStart prepends ahead of every command. It has no layer
        // context, so layer variables stay verbatim here.
        for (idx, point) in INJECTION_POINTS.iter().enumerate() {
            if matches!(&point.site, &InjectionSite::PrintStart) {
                if let Some(template) = &templates[idx] {
                    let lookup = site_lookup(&base_lookup, &point.site, None, None, None);
                    let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                    unresolved_per_site[idx].extend(unresolved);
                    if !resolved.trim().is_empty() {
                        output
                            .push_raw(resolved)
                            .map_err(|e| ModuleError::fatal(1, format!("push_raw start: {e}")))?;
                    }
                }
            }
        }

        // Step 5: Single forward walk over the input stream. Every command is
        // re-emitted unchanged; toolchange points splice around each
        // `ToolChange`, and at each `;LAYER_CHANGE` the layer context is
        // updated from the marker triple before the three layer-scoped points
        // splice immediately after the `;HEIGHT:` marker, in declaration
        // order.
        let mut ctx = LayerContext {
            layer_num: 0,
            layer_z: String::new(),
            max_layer_z: String::new(),
        };
        let mut toolchange_count = 0u32;
        let mut last_extrusion_role = String::new();
        let mut i = 0usize;
        while i < commands.len() {
            let toolchange = match &commands[i] {
                GCodeCommand::ToolChange { from, to, .. } => {
                    toolchange_count += 1;
                    Some(ToolChangeContext {
                        previous_extruder: from.to_string(),
                        next_extruder: to.to_string(),
                        toolchange_count,
                    })
                }
                _ => None,
            };

            if let Some(toolchange) = toolchange.as_ref() {
                for (idx, point) in INJECTION_POINTS.iter().enumerate() {
                    if matches!(
                        &point.site,
                        &InjectionSite::FilamentEnd | &InjectionSite::FilamentChange
                    ) {
                        if let Some(template) = &templates[idx] {
                            let lookup = site_lookup(
                                &base_lookup,
                                &point.site,
                                Some(&ctx),
                                Some(toolchange),
                                None,
                            );
                            let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                            unresolved_per_site[idx].extend(unresolved);
                            if !resolved.trim().is_empty() {
                                output.push_raw(resolved).map_err(|e| {
                                    ModuleError::fatal(
                                        13,
                                        format!("push_raw {}: {e}", point.config_key),
                                    )
                                })?;
                            }
                        }
                    }
                }
            }

            let extrusion_role = match &commands[i] {
                GCodeCommand::Raw { text } => {
                    text.strip_prefix(";TYPE:")
                        .map(|current_role| ExtrusionRoleContext {
                            current_role: current_role.to_string(),
                            last_role: last_extrusion_role.clone(),
                        })
                }
                _ => None,
            };

            if let Some(extrusion_role) = extrusion_role.as_ref() {
                for (idx, point) in INJECTION_POINTS.iter().enumerate() {
                    if matches!(&point.site, &InjectionSite::ExtrusionRoleChange) {
                        if let Some(template) = &templates[idx] {
                            let lookup = site_lookup(
                                &base_lookup,
                                &point.site,
                                Some(&ctx),
                                None,
                                Some(extrusion_role),
                            );
                            let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                            unresolved_per_site[idx].extend(unresolved);
                            if !resolved.trim().is_empty() {
                                output.push_raw(resolved).map_err(|e| {
                                    ModuleError::fatal(
                                        13,
                                        format!("push_raw {}: {e}", point.config_key),
                                    )
                                })?;
                            }
                        }
                    }
                }
            }

            reemit_command(&commands[i], output)?;

            if let Some(extrusion_role) = extrusion_role.as_ref() {
                last_extrusion_role = extrusion_role.current_role.clone();
            }

            if let Some(toolchange) = toolchange.as_ref() {
                for (idx, point) in INJECTION_POINTS.iter().enumerate() {
                    if matches!(&point.site, &InjectionSite::FilamentStart) {
                        if let Some(template) = &templates[idx] {
                            let lookup = site_lookup(
                                &base_lookup,
                                &point.site,
                                Some(&ctx),
                                Some(toolchange),
                                None,
                            );
                            let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                            unresolved_per_site[idx].extend(unresolved);
                            if !resolved.trim().is_empty() {
                                output.push_raw(resolved).map_err(|e| {
                                    ModuleError::fatal(
                                        13,
                                        format!("push_raw {}: {e}", point.config_key),
                                    )
                                })?;
                            }
                        }
                    }
                }
            }

            if matches!(&commands[i], GCodeCommand::Raw { text } if text == ";LAYER_CHANGE") {
                ctx.layer_num += 1;
                // The emitter always writes `;Z:` and `;HEIGHT:` as the two
                // commands right after `;LAYER_CHANGE`; tolerate one
                // intervening command before declaring the marker malformed.
                let window_end = (i + 2).min(commands.len() - 1);
                let mut found_z = false;
                let mut splice_at = i;
                for (j, ahead) in commands.iter().enumerate().take(window_end + 1).skip(i + 1) {
                    if let GCodeCommand::Raw { text } = ahead {
                        if !found_z && text.starts_with(";Z:") {
                            let z_text = text[";Z:".len()..].to_string();
                            // Running maximum, compared numerically but stored
                            // as the verbatim source text.
                            let is_max =
                                match (z_text.parse::<f64>(), ctx.max_layer_z.parse::<f64>()) {
                                    (Ok(new_z), Ok(max_z)) => new_z > max_z,
                                    (Ok(_), Err(_)) => true,
                                    (Err(_), _) => false,
                                };
                            if is_max {
                                ctx.max_layer_z = z_text.clone();
                            }
                            ctx.layer_z = z_text;
                            found_z = true;
                        }
                        if text.starts_with(";HEIGHT:") {
                            splice_at = j;
                            break;
                        }
                    }
                }
                if !found_z {
                    // Warn-and-pass: keep the prior layer Z context (layer 1
                    // keeps its initial context) and carry on.
                    slicer_sdk::host::log_warn(&format!(
                        "machine-gcode-emit: ERR_MALFORMED_LAYER_MARKER (code \
                         {ERR_MALFORMED_LAYER_MARKER}): ;LAYER_CHANGE at command index {i} is \
                          not followed by a ;Z: marker within two commands at injection point \
                          {layer_change_label}; reusing prior layer Z context"
                    ));
                }
                // Re-emit the marker triple (or whatever occupies the window)
                // unchanged, then splice.
                if splice_at > i {
                    for cmd in &commands[i + 1..=splice_at] {
                        reemit_command(cmd, output)?;
                    }
                    i = splice_at;
                }
                for (idx, point) in INJECTION_POINTS.iter().enumerate() {
                    if matches!(
                        &point.site,
                        &InjectionSite::BeforeLayerChange
                            | &InjectionSite::TimeLapse
                            | &InjectionSite::LayerChange
                    ) {
                        if let Some(template) = &templates[idx] {
                            let lookup =
                                site_lookup(&base_lookup, &point.site, Some(&ctx), None, None);
                            let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                            unresolved_per_site[idx].extend(unresolved);
                            if !resolved.trim().is_empty() {
                                output.push_raw(resolved).map_err(|e| {
                                    ModuleError::fatal(
                                        13,
                                        format!("push_raw {}: {e}", point.config_key),
                                    )
                                })?;
                            }
                        }
                    }
                }
            }
            i += 1;
        }

        // Step 6: PrintEnd appends after the last command, resolving layer
        // variables against the final layer context.
        for (idx, point) in INJECTION_POINTS.iter().enumerate() {
            if matches!(&point.site, &InjectionSite::PrintEnd) {
                if let Some(template) = &templates[idx] {
                    let lookup = site_lookup(&base_lookup, &point.site, Some(&ctx), None, None);
                    let (resolved, unresolved) = substitute_placeholders(template, &lookup);
                    unresolved_per_site[idx].extend(unresolved);
                    if !resolved.trim().is_empty() {
                        output
                            .push_raw(resolved)
                            .map_err(|e| ModuleError::fatal(11, format!("push_raw end: {e}")))?;
                    }
                }
            }
        }

        // Step 7: One aggregated warn-and-pass across every site, keys sorted
        // and deduplicated, contributing sites named in declaration order.
        let mut all_keys: BTreeSet<String> = BTreeSet::new();
        let mut sites: Vec<String> = Vec::new();
        for (idx, keys) in unresolved_per_site.iter().enumerate() {
            if !keys.is_empty() {
                all_keys.extend(keys.iter().cloned());
                sites.push(injection_point_label(&INJECTION_POINTS[idx]));
            }
        }
        if !all_keys.is_empty() {
            let key_list = all_keys
                .iter()
                .map(|k| format!("[{k}]"))
                .collect::<Vec<_>>()
                .join(", ");
            slicer_sdk::host::log_warn(&format!(
                "machine-gcode-emit: unresolved custom G-code placeholder(s): \
                 {key_list} (in {sites}); emitted verbatim",
                sites = sites.join(", ")
            ));
        }

        Ok(())
    }
}

/// Formats a registry point for diagnostics without losing either identity.
fn injection_point_label(point: &InjectionPoint) -> String {
    format!("{} ({})", point.config_key, point.site.name())
}

/// Re-emit one input command into the output builder, preserving variant,
/// fields, and order. Push-failure codes 2-10 are stable and keyed by
/// variant.
fn reemit_command(cmd: &GCodeCommand, output: &mut GcodeOutputBuilder) -> Result<(), ModuleError> {
    match cmd {
        GCodeCommand::Move {
            x,
            y,
            z,
            e,
            f,
            role,
        } => {
            output
                .push_move(GcodeMoveCmd::new(*x, *y, *z, *e, *f, role.clone()))
                .map_err(|e| ModuleError::fatal(2, format!("push_move: {e}")))?;
        }
        GCodeCommand::Retract {
            length,
            speed,
            mode,
        } => {
            output
                .push_retract(*length, *speed, *mode)
                .map_err(|e| ModuleError::fatal(3, format!("push_retract: {e}")))?;
        }
        GCodeCommand::Unretract {
            length,
            speed,
            mode,
        } => {
            output
                .push_unretract(*length, *speed, *mode)
                .map_err(|e| ModuleError::fatal(4, format!("push_unretract: {e}")))?;
        }
        GCodeCommand::FanSpeed { value } => {
            output
                .push_fan_speed(*value)
                .map_err(|e| ModuleError::fatal(5, format!("push_fan_speed: {e}")))?;
        }
        GCodeCommand::Temperature {
            tool,
            celsius,
            wait,
        } => {
            output
                .push_temperature(*tool, *celsius, *wait)
                .map_err(|e| ModuleError::fatal(6, format!("push_temperature: {e}")))?;
        }
        GCodeCommand::ToolChange {
            after_entity_index,
            from,
            to,
        } => {
            output
                .push_tool_change(*after_entity_index, *from, *to)
                .map_err(|e| ModuleError::fatal(7, format!("push_tool_change: {e}")))?;
        }
        GCodeCommand::Comment { text } => {
            output
                .push_comment(text.clone())
                .map_err(|e| ModuleError::fatal(8, format!("push_comment: {e}")))?;
        }
        GCodeCommand::Raw { text } => {
            output
                .push_raw(text.clone())
                .map_err(|e| ModuleError::fatal(9, format!("push_raw: {e}")))?;
        }
        GCodeCommand::ExtrusionMode { absolute } => {
            // Step 3 bridged ExtrusionMode → Raw at the host dispatch boundary,
            // so the guest normally receives Raw("M82") or Raw("M83") at index 0.
            // If the guest-side WIT variant is present, re-emit as raw text.
            let text = if *absolute {
                "M82".to_string()
            } else {
                "M83".to_string()
            };
            output
                .push_raw(text)
                .map_err(|e| ModuleError::fatal(10, format!("push_raw extrusion_mode: {e}")))?;
        }
    }
    Ok(())
}

const LAYER_VARIABLES: &[&str] = &["layer_num", "layer_z", "max_layer_z"];
const FILAMENT_END_VARIABLES: &[&str] = &[
    "layer_num",
    "layer_z",
    "max_layer_z",
    "filament_extruder_id",
];
const FILAMENT_CHANGE_VARIABLES: &[&str] = &[
    "layer_num",
    "layer_z",
    "max_layer_z",
    "previous_extruder",
    "next_extruder",
    "toolchange_count",
];
const FILAMENT_START_VARIABLES: &[&str] = &[
    "layer_num",
    "layer_z",
    "max_layer_z",
    "filament_extruder_id",
];
const EXTRUSION_ROLE_VARIABLES: &[&str] = &[
    "layer_num",
    "layer_z",
    "extrusion_role",
    "last_extrusion_role",
];
const SITE_VARIABLES: &[&str] = &[
    "layer_num",
    "layer_z",
    "max_layer_z",
    "filament_extruder_id",
    "previous_extruder",
    "next_extruder",
    "toolchange_count",
    "extrusion_role",
    "last_extrusion_role",
];

/// Returns the dynamic variables permitted at one injection site.
fn site_variables(site: &InjectionSite) -> &'static [&'static str] {
    match site {
        InjectionSite::PrintStart => &[],
        InjectionSite::BeforeLayerChange
        | InjectionSite::TimeLapse
        | InjectionSite::LayerChange
        | InjectionSite::PrintEnd => LAYER_VARIABLES,
        InjectionSite::FilamentEnd => FILAMENT_END_VARIABLES,
        InjectionSite::FilamentChange => FILAMENT_CHANGE_VARIABLES,
        InjectionSite::FilamentStart => FILAMENT_START_VARIABLES,
        InjectionSite::ExtrusionRoleChange => EXTRUSION_ROLE_VARIABLES,
    }
}

/// Build the per-site substitution lookup from the base config plus only the
/// dynamic variables registered for that site.
fn site_lookup(
    base: &HashMap<String, String>,
    site: &InjectionSite,
    layer: Option<&LayerContext>,
    toolchange: Option<&ToolChangeContext>,
    role_context: Option<&ExtrusionRoleContext>,
) -> HashMap<String, String> {
    let mut lookup = base.clone();
    for variable in SITE_VARIABLES {
        lookup.remove(*variable);
    }
    for variable in site_variables(site) {
        let value = if *variable == "layer_num" {
            layer.map(|ctx| {
                if matches!(site, InjectionSite::ExtrusionRoleChange) {
                    (ctx.layer_num + 1).to_string()
                } else {
                    ctx.layer_num.to_string()
                }
            })
        } else if *variable == "layer_z" {
            layer.map(|ctx| ctx.layer_z.clone())
        } else if *variable == "max_layer_z" {
            layer.map(|ctx| ctx.max_layer_z.clone())
        } else if *variable == "extrusion_role" {
            role_context.map(|role| role.current_role.clone())
        } else if *variable == "last_extrusion_role" {
            role_context.map(|role| role.last_role.clone())
        } else if *variable == "filament_extruder_id" {
            match site {
                InjectionSite::FilamentEnd => {
                    toolchange.map(|toolchange| toolchange.previous_extruder.clone())
                }
                InjectionSite::FilamentStart => {
                    toolchange.map(|toolchange| toolchange.next_extruder.clone())
                }
                InjectionSite::PrintStart
                | InjectionSite::BeforeLayerChange
                | InjectionSite::TimeLapse
                | InjectionSite::LayerChange
                | InjectionSite::FilamentChange
                | InjectionSite::ExtrusionRoleChange
                | InjectionSite::PrintEnd => None,
            }
        } else if *variable == "previous_extruder" {
            toolchange.map(|toolchange| toolchange.previous_extruder.clone())
        } else if *variable == "next_extruder" {
            toolchange.map(|toolchange| toolchange.next_extruder.clone())
        } else if *variable == "toolchange_count" {
            toolchange.map(|toolchange| toolchange.toolchange_count.to_string())
        } else {
            None
        };
        if let Some(value) = value {
            lookup.insert((*variable).to_string(), value);
        }
    }
    lookup
}

/// Renders one `ConfigValue` as the text a `[key]` placeholder substitutes to,
/// or `None` when the value has no single-value rendering.
///
/// `ConfigValue::List` resolves to its **first element**. Real 3MF input
/// supplies per-extruder settings as vectors — `nozzle_diameter` arrives as
/// `['0.4']`, a `List`, never a scalar — and canonical reads element 0 when a
/// placeholder needs a single value (`nozzle_temperature_initial_layer
/// .get_at(0)` in `GCode::_do_export`'s `; first_layer_temperature = %d`
/// preamble). Without this the module's headline placeholders are inert for
/// every real slice.
///
/// An **empty** list yields `None`, so the key stays out of the lookup and its
/// placeholder stays unresolved (passed through verbatim, and warned about).
/// Substituting an empty string instead would silently emit `M104 S`, which
/// `design.md` §Rejected alternatives rules out.
///
/// `Percent` and `FloatOrPercent` stay unrendered: they are meaningless without
/// the base they resolve against.
fn format_placeholder_value(value: &ConfigValue) -> Option<String> {
    match value {
        ConfigValue::String(s) => Some(s.clone()),
        ConfigValue::Int(i) => Some(i.to_string()),
        ConfigValue::Float(f) => Some(f.to_string()),
        ConfigValue::Bool(b) => Some(b.to_string()),
        ConfigValue::List(items) => items.first().and_then(format_placeholder_value),
        _ => None,
    }
}

/// Single-pass left-to-right placeholder substitution.
///
/// Replaces `[snake_case_key]` with the corresponding value from `lookup`.
/// Returns the rendered text plus the sorted, deduplicated list of bracketed
/// keys that had no entry in `lookup`. The rendered text keeps the verbatim
/// `[key]` for an unresolved key; the unresolved list exists so the caller can
/// warn about them once, not so it can fail.
///
/// An unclosed `[` with no `]` before end-of-line is literal text and is *not*
/// a failure. No recursion; substituted values are not re-scanned.
///
/// The scan stays byte-oriented: `[`, `]` and `\n` are ASCII, and UTF-8 is
/// self-synchronising, so any byte equal to one of them is always at a char
/// boundary. Literal runs are therefore copied as whole `&str` slices, which
/// keeps multi-byte characters intact.
fn substitute_placeholders(
    template: &str,
    lookup: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let mut out = String::with_capacity(template.len());
    let mut unresolved: BTreeSet<String> = BTreeSet::new();
    let bytes = template.as_bytes();
    let mut i = 0;
    // Start of the literal run pending copy into `out`.
    let mut run_start = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        // Flush the literal run that ends at this '['.
        out.push_str(&template[run_start..i]);

        // Found '['. Scan for matching ']' on the same line (no newline before ']').
        let mut j = i + 1;
        let mut found = None;
        while j < bytes.len() && bytes[j] != b'\n' {
            if bytes[j] == b']' {
                found = Some(j);
                break;
            }
            j += 1;
        }
        match found {
            Some(end) => {
                let key = &template[i + 1..end];
                if let Some(val) = lookup.get(key) {
                    out.push_str(val);
                } else {
                    // Unresolved key: keep it verbatim (brackets included) and
                    // report it so the caller can warn once.
                    unresolved.insert(key.to_string());
                    out.push_str(&template[i..=end]);
                }
                i = end + 1;
            }
            None => {
                // Unclosed '['. Treat remainder of this line as literal.
                let line_end = j; // position of '\n' or bytes.len()
                out.push_str(&template[i..line_end]);
                i = line_end;
            }
        }
        run_start = i;
    }
    // Flush the trailing literal run.
    out.push_str(&template[run_start..]);
    (out, unresolved.into_iter().collect())
}
