//! Component-model guest wrapper for `seam-placer`.
//!
//! Exists solely to compile the real `seam-placer` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use seam_placer::SeamPlacer;
