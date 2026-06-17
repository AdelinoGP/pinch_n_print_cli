// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/[Various]
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Core module: emits machine_start_gcode / machine_end_gcode by prepending and appending
//! Raw commands to the GCodePostProcess stream. Performs single-pass [key] substitution
//! against the effective ConfigView. Substitution lives in the WASM guest; the host
//! serializer just renders the command list.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_ir::ConfigView;
use slicer_ir::{ConfigValue, GCodeCommand};
use slicer_sdk::error::ModuleError;
use slicer_sdk::postpass_builders::{GcodeMoveCmd, GcodeOutputBuilder};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PostpassModule;

/// Machine-gcode-emit GCodePostProcess module.
pub struct MachineGcodeEmit;

#[slicer_module]
impl PostpassModule for MachineGcodeEmit {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
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
            let val_str = match config.get(&key) {
                Some(ConfigValue::String(s)) => s.clone(),
                Some(ConfigValue::Int(i)) => i.to_string(),
                Some(ConfigValue::Float(f)) => f.to_string(),
                Some(ConfigValue::Bool(b)) => b.to_string(),
                _ => continue,
            };
            lookup.insert(key, val_str);
        }

        // Step 3: Perform substitution on both templates.
        let resolved_start = substitute_placeholders(&start_template, &lookup);
        let resolved_end = substitute_placeholders(&end_template, &lookup);

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

/// Single-pass left-to-right placeholder substitution.
///
/// Replaces `[snake_case_key]` with the corresponding value from `lookup`.
/// Unknown keys pass through verbatim (including brackets). Unclosed `[` on a
/// line is treated as literal text. No recursion; substituted values are not
/// re-scanned.
fn substitute_placeholders(template: &str, lookup: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
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
                let key = std::str::from_utf8(&bytes[i + 1..end]).unwrap_or("");
                if let Some(val) = lookup.get(key) {
                    out.push_str(val);
                    i = end + 1;
                } else {
                    // Unknown key: pass through verbatim including brackets.
                    out.push_str(std::str::from_utf8(&bytes[i..=end]).unwrap_or(""));
                    i = end + 1;
                }
            }
            None => {
                // Unclosed '['. Treat remainder of this line as literal.
                let line_end = j; // position of '\n' or bytes.len()
                out.push_str(std::str::from_utf8(&bytes[i..line_end]).unwrap_or(""));
                i = line_end;
            }
        }
    }
    out
}
