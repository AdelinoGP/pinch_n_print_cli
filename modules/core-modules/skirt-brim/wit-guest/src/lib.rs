//! Component-model guest wrapper for `skirt-brim`.
//!
//! Exists solely to compile the real `skirt-brim` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use skirt_brim::SkirtBrim;
