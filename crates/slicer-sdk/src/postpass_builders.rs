//! Postpass output builder types for SDK.
//!
//! These builders correspond to the WIT resources in docs/03_wit_and_manifest.md (world-postpass.wit).
//! They are used by PostpassModule implementations to emit gcode postprocessing output.

use crate::postpass_types::GcodeOutputCommand;
use slicer_ir::{ExtrusionRole, GCodeCommand, RetractMode};

/// Move command parameters for the GCode output builder.
///
/// Per docs/03_wit_and_manifest.md (ir-types.wit):
/// ```wit
/// record gcode-move-cmd {
///     x: option<f32>, y: option<f32>, z: option<f32>,
///     e: option<f32>, f: option<f32>,
///     role: extrusion-role,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GcodeMoveCmd {
    /// X coordinate (optional).
    pub x: Option<f32>,
    /// Y coordinate (optional).
    pub y: Option<f32>,
    /// Z coordinate (optional).
    pub z: Option<f32>,
    /// Extrusion amount (optional).
    pub e: Option<f32>,
    /// Feed rate (optional).
    pub f: Option<f32>,
    /// Extrusion role for this move.
    pub role: ExtrusionRole,
}

impl GcodeMoveCmd {
    /// Create a new GcodeMoveCmd.
    pub fn new(
        x: Option<f32>,
        y: Option<f32>,
        z: Option<f32>,
        e: Option<f32>,
        f: Option<f32>,
        role: ExtrusionRole,
    ) -> Self {
        Self {
            x,
            y,
            z,
            e,
            f,
            role,
        }
    }
}

/// Output builder for GCode postprocessing stage.
///
/// Per docs/03_wit_and_manifest.md (world-postpass.wit):
/// ```wit
/// resource gcode-output-builder {
///     push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
///     push-retract:     func(length: f32, speed: f32) -> result<_, string>;
///     push-fan-speed:   func(value: u8) -> result<_, string>;
///     push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
///     push-tool-change: func(after-entity-index: u32, from: u32, to: u32) -> result<_, string>;
///     push-comment:     func(text: string) -> result<_, string>;
///     push-raw:         func(text: string) -> result<_, string>;
/// }
/// ```
pub struct GcodeOutputBuilder {
    commands: Vec<GcodeOutputCommand>,
}

impl GcodeOutputBuilder {
    /// Create a new GcodeOutputBuilder.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Push a move command.
    pub fn push_move(&mut self, cmd: GcodeMoveCmd) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Move {
                x: cmd.x,
                y: cmd.y,
                z: cmd.z,
                e: cmd.e,
                f: cmd.f,
                role: cmd.role,
            }));
        Ok(())
    }

    /// Push a retract command.
    ///
    /// `mode` selects whether the downstream emitter should serialise this as
    /// G-code-driven retraction (`RetractMode::Gcode`) or firmware-driven
    /// retraction (`RetractMode::Firmware`). Callers that pre-date packet-34
    /// should pass `RetractMode::Gcode` to preserve historical behaviour.
    pub fn push_retract(
        &mut self,
        length: f32,
        speed: f32,
        mode: RetractMode,
    ) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Retract {
                length,
                speed,
                mode,
            }));
        Ok(())
    }

    /// Push an unretract command.
    ///
    /// `mode` selects whether the downstream emitter should serialise this as
    /// G-code-driven retraction (`RetractMode::Gcode`) or firmware-driven
    /// retraction (`RetractMode::Firmware`). Callers that pre-date packet-34
    /// should pass `RetractMode::Gcode` to preserve historical behaviour.
    pub fn push_unretract(
        &mut self,
        length: f32,
        speed: f32,
        mode: RetractMode,
    ) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Unretract {
                length,
                speed,
                mode,
            }));
        Ok(())
    }

    /// Push a fan speed command.
    pub fn push_fan_speed(&mut self, value: u8) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::FanSpeed {
                value,
            }));
        Ok(())
    }

    /// Push a temperature command.
    pub fn push_temperature(&mut self, tool: u32, celsius: f32, wait: bool) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Temperature {
                tool,
                celsius,
                wait,
            }));
        Ok(())
    }

    /// Push a tool change command.
    pub fn push_tool_change(
        &mut self,
        after_entity_index: u32,
        from: u32,
        to: u32,
    ) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::ToolChange {
                after_entity_index,
                from,
                to,
            }));
        Ok(())
    }

    /// Push a comment command.
    pub fn push_comment(&mut self, text: String) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Comment { text }));
        Ok(())
    }

    /// Push a raw GCode command.
    pub fn push_raw(&mut self, text: String) -> Result<(), String> {
        self.commands
            .push(GcodeOutputCommand::Command(GCodeCommand::Raw { text }));
        Ok(())
    }

    /// Push a Z hop command.
    pub fn push_z_hop(&mut self, after_entity_index: u32, hop_height: f32) -> Result<(), String> {
        self.commands.push(GcodeOutputCommand::ZHop {
            after_entity_index,
            hop_height,
        });
        Ok(())
    }

    /// Get all emitted commands (for testing).
    #[doc(hidden)]
    pub fn commands(&self) -> &[GcodeOutputCommand] {
        &self.commands
    }
}

impl Default for GcodeOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for GcodeOutputBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcodeOutputBuilder")
            .field("commands", &self.commands.len())
            .finish()
    }
}
