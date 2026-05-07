//! Component-model guest wrapper for `top-surface-ironing`.
//!
//! Exists solely to compile the real `top-surface-ironing` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use top_surface_ironing::TopSurfaceIroning;
