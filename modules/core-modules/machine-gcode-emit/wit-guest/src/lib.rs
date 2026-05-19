//! WASM guest shim for machine-gcode-emit module.
//! Re-exports the #[slicer_module]-decorated type so the macro-generated
//! component exports are included in the wasm32 build.

use machine_gcode_emit::MachineGcodeEmit;
