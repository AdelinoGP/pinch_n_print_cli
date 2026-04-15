//! Component-model guest wrapper for `classic-perimeters`.
//!
//! This crate exists only to produce a real `slicer:world-layer@1.0.0`
//! component binary. The whole purpose is to compile the real
//! `classic-perimeters` Rust implementation for the `wasm32-unknown-unknown`
//! target as a `cdylib` so the `#[slicer_module]`-emitted
//! `__slicer_layer_world_export` module (guarded by
//! `cfg(target_arch = "wasm32")`) is pulled into the final `.wasm` and
//! registers the documented layer-world exports.
//!
//! No logic lives here — `pub use` is enough to keep the macro-emitted
//! exports from being stripped: the `#[used]`-tagged component-type
//! section and the `#[export_name]` extern "C" symbols wit-bindgen
//! emits inside the dependency crate are preserved by the wasm linker
//! when they are referenced at crate-link boundaries.

#[allow(unused_imports)]
pub use classic_perimeters::ClassicPerimeters;
