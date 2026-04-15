//! Component-model guest wrapper for `path-optimization-default`.
//!
//! Exists solely to compile the real `path-optimization-default` crate
//! for the `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module (guarded by
//! `#[cfg(target_arch = "wasm32")]`) is preserved in the final
//! `.wasm`. No logic lives here — `pub use` is enough to keep the
//! macro-emitted exports alive at link time.

#[allow(unused_imports)]
pub use path_optimization_default::PathOptimizationDefault;
