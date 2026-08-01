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
//! Core module: emits machine_start_gcode / machine_end_gcode by prepending and appending
//! Raw commands to the GCodePostProcess stream. Performs single-pass [key] substitution
//! against the effective ConfigView. Substitution lives in the WASM guest; the host
//! serializer just renders the command list.

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
        // Step 1: Read config values with defaults.
        let start_template = match config.get("machine_start_gcode") {
            Some(ConfigValue::String(s)) => s.clone(),
            _ => String::new(),
        };
        let end_template = match config.get("machine_end_gcode") {
            Some(ConfigValue::String(s)) => s.clone(),
            _ => String::new(),
        };
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

        // Step 2: Build substitution lookup.
        let mut lookup: HashMap<String, String> = HashMap::new();
        lookup.insert(
            "bed_temperature_initial_layer_single".to_string(),
            bed_temp.to_string(),
        );
        lookup.insert(
            "nozzle_temperature_initial_layer".to_string(),
            nozzle_temp.to_string(),
        );
        // Also include all other string/int/float keys from config for completeness.
        for key in config.keys() {
            if lookup.contains_key(&key) {
                continue;
            }
            let val_str = match config.get(&key).and_then(format_placeholder_value) {
                Some(s) => s,
                None => continue,
            };
            lookup.insert(key, val_str);
        }
        // Legacy aliases resolve only where the sweep above left a gap.
        for (alias, target) in PLACEHOLDER_ALIASES {
            if lookup.contains_key(*alias) {
                continue;
            }
            if let Some(val) = lookup.get(*target).cloned() {
                lookup.insert((*alias).to_string(), val);
            }
        }

        // Step 3: Perform substitution on both templates.
        //
        // An unresolved `[key]` is **not** a slice error. A module's
        // `ConfigView` is scoped to its own manifest, so "unknown to
        // machine-gcode-emit" is not "unknown to the slicer": a template may
        // legitimately name a key owned by a module that is not loaded in this
        // pipeline. Aborting would break composition. Instead the key passes
        // through verbatim (brackets included) and we warn once, aggregated
        // over both injection points.
        let (resolved_start, unresolved_start) = substitute_placeholders(&start_template, &lookup);
        let (resolved_end, unresolved_end) = substitute_placeholders(&end_template, &lookup);

        if !unresolved_start.is_empty() || !unresolved_end.is_empty() {
            let keys: BTreeSet<String> = unresolved_start
                .iter()
                .chain(unresolved_end.iter())
                .cloned()
                .collect();
            let mut sites: Vec<&str> = Vec::new();
            if !unresolved_start.is_empty() {
                sites.push("machine_start_gcode");
            }
            if !unresolved_end.is_empty() {
                sites.push("machine_end_gcode");
            }
            let key_list = keys
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

        // Step 4: Emit resolved_start (if non-empty).
        if !resolved_start.trim().is_empty() {
            output
                .push_raw(resolved_start)
                .map_err(|e| ModuleError::fatal(1, format!("push_raw start: {e}")))?;
        }

        // Step 5: Re-emit every input command.
        for cmd in commands {
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
                    output.push_raw(text).map_err(|e| {
                        ModuleError::fatal(10, format!("push_raw extrusion_mode: {e}"))
                    })?;
                }
            }
        }

        // Step 6: Emit resolved_end (if non-empty).
        if !resolved_end.trim().is_empty() {
            output
                .push_raw(resolved_end)
                .map_err(|e| ModuleError::fatal(11, format!("push_raw end: {e}")))?;
        }

        Ok(())
    }
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
