//! Component-model guest wrapper for `wipe-tower`.
//!
//! Exists solely to compile the real `wipe-tower` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use wipe_tower::WipeTower;
