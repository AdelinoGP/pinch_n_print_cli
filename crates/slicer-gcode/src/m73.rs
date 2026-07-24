// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source paths: OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp; OrcaSlicerDocumented/src/libslic3r/GCode.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! M73 progress injection and filament/time comment block for the G-code
//! serializer (packet 175, Step 2).

use crate::estimator::PrintEstimate;
use slicer_ir::{GCodeCommand, GCodeIR};

/// Inserts M73 progress pairs at layer boundaries and at the stream ends.
pub fn inject_m73(gcode_ir: &mut GCodeIR, elapsed_s: &[f64]) {
    let total = elapsed_s.last().copied().unwrap_or(0.0);
    if total <= 0.0 {
        return;
    }

    let total_min = (total / 60.0).round() as u32;
    let commands = std::mem::take(&mut gcode_ir.commands);
    let layer_changes = commands
        .iter()
        .filter(|command| {
            matches!(
                command,
                GCodeCommand::Raw { text } if text == ";LAYER_CHANGE"
            )
        })
        .count();
    let mut injected = Vec::with_capacity(commands.len() + 2 * (1 + layer_changes + 1));

    injected.push(GCodeCommand::Raw {
        text: format!("M73 P0 R{total_min}"),
    });
    injected.push(GCodeCommand::Raw {
        text: format!("M73 Q0 S{total_min}"),
    });

    let mut last_emitted = Some((0_u8, total_min));
    for (index, command) in commands.into_iter().enumerate() {
        let is_layer_change = matches!(
            &command,
            GCodeCommand::Raw { text } if text == ";LAYER_CHANGE"
        );
        injected.push(command);

        if is_layer_change {
            let elapsed = elapsed_s[index];
            let pct = ((elapsed / total) * 100.0).round().clamp(0.0, 100.0) as u8;
            let remaining_min = ((total - elapsed) / 60.0).round() as u32;
            if last_emitted != Some((pct, remaining_min)) {
                injected.push(GCodeCommand::Raw {
                    text: format!("M73 P{pct} R{remaining_min}"),
                });
                injected.push(GCodeCommand::Raw {
                    text: format!("M73 Q{pct} S{remaining_min}"),
                });
                last_emitted = Some((pct, remaining_min));
            }
        }
    }

    injected.push(GCodeCommand::Raw {
        text: "M73 P100 R0".to_string(),
    });
    injected.push(GCodeCommand::Raw {
        text: "M73 Q100 S0".to_string(),
    });
    gcode_ir.commands = injected;
}

/// Builds the filament and estimated-time comment block for a print estimate.
///
/// `filament_densities` is the per-filament `filament_density` list (Orca
/// `coFloats`), indexed by tool id like canonical `Extruder::filament_density`.
/// Empty means unconfigured, which omits the `filament used [g]` line entirely.
pub fn filament_stats_comment_block(
    estimate: &PrintEstimate,
    filament_densities: &[f64],
) -> Vec<GCodeCommand> {
    let lengths = estimate
        .filament_length_mm
        .values()
        .map(|value| format!("{value:.2}"))
        .collect::<Vec<_>>()
        .join(", ");
    let volumes = estimate
        .extruded_volume_mm3
        .values()
        .map(|value| format!("{:.2}", value / 1000.0))
        .collect::<Vec<_>>()
        .join(", ");

    let mut lines = vec![
        GCodeCommand::Raw {
            text: format!("; filament used [mm] = {lengths}"),
        },
        GCodeCommand::Raw {
            text: format!("; filament used [cm3] = {volumes}"),
        },
    ];

    if !filament_densities.is_empty() {
        // Each tool is priced with its own filament's density; a short list
        // falls back to the first entry rather than dropping the tool.
        let grams = estimate
            .extruded_volume_mm3
            .iter()
            .map(|(tool, value)| {
                let density = filament_densities
                    .get(*tool as usize)
                    .or_else(|| filament_densities.first())
                    .copied()
                    .unwrap_or_default();
                format!("{:.2}", value / 1000.0 * density)
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(GCodeCommand::Raw {
            text: format!("; filament used [g] = {grams}"),
        });
    }

    lines.push(GCodeCommand::Raw {
        text: format!(
            "; estimated printing time (normal mode) = {}",
            format_time_dhms(estimate.total_time_s)
        ),
    });
    lines
}

fn format_time_dhms(s: f64) -> String {
    if s < 1.0 {
        return format!("{s:.2}s");
    }

    let days = (s / 86_400.0).floor() as u32;
    let hours = ((s % 86_400.0) / 3_600.0).floor() as u32;
    let minutes = ((s % 3_600.0) / 60.0).floor() as u32;
    let seconds = (s % 60.0).floor() as u32;
    let mut parts = Vec::new();
    if days != 0 {
        parts.push(format!("{days}d"));
    }
    if days != 0 || hours != 0 {
        parts.push(format!("{hours}h"));
    }
    if days != 0 || hours != 0 || minutes != 0 {
        parts.push(format!("{minutes}m"));
    }
    parts.push(format!("{seconds}s"));
    parts.join(" ")
}
