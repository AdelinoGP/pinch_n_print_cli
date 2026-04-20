//! Postpass stage types for SDK.
//!
//! These types correspond to the WIT definitions in docs/03_wit_and_manifest.md (world-postpass.wit).
//! They are used by PostpassModule implementations for gcode and text postprocessing stages.

use serde::{Deserialize, Serialize};

/// Payload-bearing GCode command input for postpass processing.
///
/// This mirrors `world-postpass.wit`'s `variant gcode-command` and is
/// re-exported from the shared IR so SDK modules can inspect the full
/// command payload rather than a thin kind-only view.
pub use slicer_ir::GCodeCommand as GcodeCommand;

/// Output emitted by the SDK-level postpass builder.
///
/// Most emissions map directly to `GCodeCommand`; `ZHop` remains a
/// postpass-specific builder action because it does not have a direct
/// `slicer_ir::GCodeCommand` representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GcodeOutputCommand {
    /// Standard GCode command emission.
    Command(GcodeCommand),
    /// Emit a Z hop after the referenced entity index.
    ZHop {
        /// Entity index after which the hop should occur.
        after_entity_index: u32,
        /// Hop height in millimeters.
        hop_height: f32,
    },
}
