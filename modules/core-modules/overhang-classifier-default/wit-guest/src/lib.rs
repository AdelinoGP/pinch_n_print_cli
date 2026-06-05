//! WASM guest shim for overhang-classifier-default module.
//! Re-exports the #[slicer_module]-decorated type so the macro-generated
//! component exports are included in the wasm32 build.

use overhang_classifier_default::OverhangClassifierDefault;
