//! Error type returned by the `slicer-gcode` emission and serialization paths.
//!
//! `GCodeEmitError` mirrors the failure modes that the original
//! `crates/slicer-runtime/src/gcode_emit.rs` surfaced via
//! `slicer_ir::PostpassError`. Keeping the error type local to
//! `slicer-gcode` lets the crate stand alone without re-importing the
//! runtime's broader postpass error vocabulary; the runtime adapter is
//! responsible for translating `GCodeEmitError` back into the
//! `PostpassError` variants that `Postpass` callers expect.

use thiserror::Error;

/// Failure modes produced by `slicer-gcode` emission and serialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GCodeEmitError {
    /// A `ToolChange` was emitted without a surrounding wipe/purge while
    /// `wipe_tower_enabled` is true. Maps to
    /// `PostpassError::MissingToolchangePurge` at the runtime seam.
    #[error(
        "missing toolchange purge: layer {layer_index} tool_change[{tool_change_index}] \
         has no wipe entity in the layer"
    )]
    MissingToolchangePurge {
        /// Layer index (global) where the bare ToolChange was detected.
        layer_index: u32,
        /// Index of the ToolChange within `layer.tool_changes` (0-based).
        tool_change_index: u32,
    },

    /// G-code emission failed (e.g. invalid layer data, malformed feedrate
    /// config). Maps to `PostpassError::GCodeEmit { message }` at the
    /// runtime seam.
    #[error("gcode emit failed: {0}")]
    Emit(String),

    /// G-code serialization failed (e.g. unsupported command, writer error).
    /// Maps to `PostpassError::GCodeSerialization { message }` at the
    /// runtime seam.
    #[error("gcode serialization failed: {0}")]
    Serialization(String),
}
