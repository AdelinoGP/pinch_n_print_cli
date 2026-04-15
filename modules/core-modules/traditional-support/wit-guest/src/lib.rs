//! Component-model guest wrapper for `traditional-support`.
//!
//! Exists solely to compile the real `traditional-support` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use traditional_support::TraditionalSupport;
