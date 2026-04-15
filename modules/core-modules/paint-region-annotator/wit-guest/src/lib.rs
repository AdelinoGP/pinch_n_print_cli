//! Component-model guest wrapper for `paint-region-annotator`.
//!
//! Exists solely to compile the real `paint-region-annotator` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use paint_region_annotator::PaintRegionAnnotator;
