/// Internal bridge-over-infill geometry.
pub mod bridge_over_infill;
/// Lightning tree generator skeleton (packet 137 contract; algorithm ships in 138/139).
#[cfg(feature = "host-algos")]
pub mod lightning;
/// Mesh analysis utilities.
#[cfg(feature = "host-algos")]
pub mod mesh_analysis;
/// Single-Z-plane cross-section helper (wraps `slice_mesh_ex`).
#[cfg(feature = "host-algos")]
pub mod mesh_cross_section;
/// Per-layer overhang quartile-band annotation.
#[cfg(feature = "host-algos")]
pub mod overhang_annotation;
/// Paint segmentation algorithms.
#[cfg(feature = "host-algos")]
pub mod paint_segmentation;
/// Pre-pass slicing routines.
#[cfg(feature = "host-algos")]
pub mod prepass_slice;
/// Pure region-mapping kernel (IR-only; no scheduler/runtime deps).
#[cfg(feature = "host-algos")]
pub mod region_mapping;
/// Support geometry computation.
#[cfg(feature = "host-algos")]
pub mod support_geometry;
