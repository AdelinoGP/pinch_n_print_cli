//! Postpass stage types for SDK.
//!
//! These types correspond to the WIT definitions in docs/03_wit_and_manifest.md (world-postpass.wit).
//! They are used by PostpassModule implementations for gcode and text postprocessing stages.

use serde::{Deserialize, Serialize};

/// Classification of GCode commands for postpass processing.
///
/// Per docs/03_wit_and_manifest.md (world-postpass.wit):
/// ```wit
/// enum gcode-command-kind { move_, retract, fan-speed, temperature, tool-change, comment, raw }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GcodeCommandKind {
    /// Move command (linear move with optional extrusion).
    Move,
    /// Retract command.
    Retract,
    /// Fan speed change command.
    FanSpeed,
    /// Temperature change command.
    Temperature,
    /// Tool change command.
    ToolChange,
    /// Comment in GCode output.
    Comment,
    /// Raw GCode string.
    Raw,
}

/// View of a single GCode command for postpass inspection.
///
/// Per docs/03_wit_and_manifest.md (world-postpass.wit):
/// ```wit
/// record gcode-command-view { index: u32, kind: gcode-command-kind }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcodeCommandView {
    /// Index of the command in the GCode command list.
    pub index: u32,
    /// Kind/classification of the command.
    pub kind: GcodeCommandKind,
}

impl GcodeCommandView {
    /// Create a new GcodeCommandView.
    pub fn new(index: u32, kind: GcodeCommandKind) -> Self {
        Self { index, kind }
    }
}
