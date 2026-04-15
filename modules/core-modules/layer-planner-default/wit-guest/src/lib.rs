//! Component-model guest wrapper for `layer-planner-default`.
//!
//! Exists solely to compile the real `layer-planner-default` crate for
//! the `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module (guarded by
//! `#[cfg(target_arch = "wasm32")]`) is preserved in the final `.wasm`.
//! No logic lives here: the planner's behavior is authored once in the
//! macro-decorated `PrepassModule` impl in the main crate.
//!
//! This replaces the previous hand-written `wit_bindgen::generate!`
//! duplicate that shipped its own reduced planning logic — now the
//! wit-guest follows the same pattern as every other core module and
//! the `#[slicer_module]` macro owns the WIT glue, including the
//! `objects`-forward and `LayerPlanOutput`-drain bridge to the SDK
//! trait.

#[allow(unused_imports)]
pub use layer_planner_default::DefaultLayerPlanner;
