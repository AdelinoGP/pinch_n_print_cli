// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: GCodeWriter.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

/// The G-code command dialect emitted by the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GcodeFlavor {
    #[default]
    /// Stock Marlin firmware dialect.
    Marlin,
    /// Marlin 2.x dialect with extended kinematics (M204 P, M205 J, M900 K).
    Marlin2,
    /// Klipper input shaping dialect using `SET_VELOCITY_LIMIT` macros.
    Klipper,
    /// RepRapFirmware (RRF) dialect using `G10`/`M116` for temperature.
    RepRapFirmware,
    /// Repetier dialect using `M201`/`M202`/`M207`/`M233` command forms.
    Repetier,
}

impl GcodeFlavor {
    /// Parse a `GcodeFlavor` from its config-file string form.
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "marlin" => Self::Marlin,
            "marlin2" => Self::Marlin2,
            "klipper" => Self::Klipper,
            "reprapfirmware" => Self::RepRapFirmware,
            "repetier" => Self::Repetier,
            unknown => {
                log::warn!("Unknown gcode_flavor {:?}; defaulting to marlin", unknown);
                Self::default()
            }
        }
    }

    /// Return the config-file string form of this flavor.
    pub fn config_str(&self) -> &'static str {
        match self {
            Self::Marlin => "marlin",
            Self::Marlin2 => "marlin2",
            Self::Klipper => "klipper",
            Self::RepRapFirmware => "reprapfirmware",
            Self::Repetier => "repetier",
        }
    }

    /// Mirrors `GCodeWriter.cpp::set_temperature`.
    pub fn set_temperature(&self, tool: u32, celsius: f32, wait: bool) -> String {
        match self {
            Self::RepRapFirmware => {
                let mut command = format!("G10 P{} S{}\n", tool, celsius);
                if wait {
                    command.push_str("M116\n");
                }
                command
            }
            _ => format!("M{} T{} S{}\n", if wait { 109 } else { 104 }, tool, celsius),
        }
    }

    /// Emit the bed-temperature command (`M140` or `M190` on wait).
    pub fn set_bed_temperature(&self, celsius: f32, wait: bool) -> String {
        format!("M{} S{}\n", if wait { 190 } else { 140 }, celsius)
    }

    /// Mirrors `GCodeWriter.cpp::set_acceleration_internal`.
    pub fn set_acceleration(&self, mm_s2: u32) -> String {
        match self {
            Self::Marlin => format!("M204 S{}\n", mm_s2),
            Self::Marlin2 | Self::RepRapFirmware => format!("M204 P{}\n", mm_s2),
            Self::Klipper => format!("SET_VELOCITY_LIMIT ACCEL={}\n", mm_s2),
            Self::Repetier => format!("M201 X{} Y{}\n", mm_s2, mm_s2),
        }
    }

    /// Emit the per-flavor travel-acceleration command, or `None` when the
    /// flavor does not support a separate travel acceleration.
    pub fn set_travel_acceleration(&self, mm_s2: u32) -> Option<String> {
        match self {
            Self::Marlin2 | Self::RepRapFirmware => Some(format!("M204 T{}\n", mm_s2)),
            Self::Repetier => Some(format!("M202 X{} Y{}\n", mm_s2, mm_s2)),
            Self::Marlin | Self::Klipper => None,
        }
    }

    /// Mirrors `GCodeWriter.cpp::supports_separate_travel_acceleration`.
    pub fn supports_separate_travel_acceleration(&self) -> bool {
        matches!(self, Self::Repetier | Self::Marlin2 | Self::RepRapFirmware)
    }

    /// Mirrors `GCodeWriter.cpp::set_jerk_xy`.
    pub fn set_jerk_xy(&self, jerk: f32) -> String {
        match self {
            Self::Klipper => {
                format!("SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY={:.0}\n", jerk)
            }
            Self::Repetier => format!("M207 X{:.0}\n", jerk),
            Self::Marlin | Self::Marlin2 | Self::RepRapFirmware => {
                format!("M205 X{:.0} Y{:.0}\n", jerk, jerk)
            }
        }
    }

    /// Mirrors `GCodeWriter.cpp::set_junction_deviation`.
    pub fn set_junction_deviation(&self, jd: f32) -> Option<String> {
        match self {
            Self::Marlin2 => Some(format!("M205 J{}\n", jd)),
            Self::Marlin | Self::Klipper | Self::RepRapFirmware | Self::Repetier => None,
        }
    }

    /// Mirrors `GCodeWriter.cpp::set_pressure_advance`.
    pub fn set_pressure_advance(&self, pa: f32) -> String {
        match self {
            Self::Klipper => format!("SET_PRESSURE_ADVANCE ADVANCE={:.4}\n", pa),
            Self::RepRapFirmware => format!("M572 D0 S{:.4}\n", pa),
            Self::Repetier => format!("M233 X{:.4} Y{:.4}\n", pa, pa),
            Self::Marlin | Self::Marlin2 => format!("M900 K{:.4}\n", pa),
        }
    }
}
