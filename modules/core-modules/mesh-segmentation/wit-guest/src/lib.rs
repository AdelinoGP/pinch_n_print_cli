//! Component-model guest wrapper for `mesh-segmentation`.
//!
//! Exists solely to compile the real `mesh-segmentation` crate for
//! the `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module (guarded by
//! `#[cfg(target_arch = "wasm32")]`) is preserved in the final
//! `.wasm`. No logic lives here: the marker-emitting behavior is
//! authored once in the macro-decorated `PrepassModule` impl in the
//! main crate.
//!
//! This replaces the previous hand-written `wit_bindgen::generate!`
//! duplicate that shipped its own `mesh_seg_mark:*` parser — now the
//! wit-guest follows the same pattern as every other core module
//! (STEP F / STEP H) and the `#[slicer_module]` macro owns the WIT
//! glue, including the `objects`-forward + `mark-triangle-paint`
//! drain bridge introduced in STEP H.

#[allow(unused_imports)]
pub use mesh_segmentation::MeshSegmentation;
