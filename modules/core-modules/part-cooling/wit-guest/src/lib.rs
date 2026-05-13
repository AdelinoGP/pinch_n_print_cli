//! WASM guest shim for part-cooling module.
//! Re-exports the #[slicer_module]-decorated type so the macro-generated
//! component exports are included in the wasm32 build.

use part_cooling::PartCooling;
