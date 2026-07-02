/// Mesh analysis utilities.
pub mod mesh_analysis;
/// Single-Z-plane cross-section helper (wraps `slice_mesh_ex`).
pub mod mesh_cross_section;
/// Per-layer overhang quartile-band annotation.
pub mod overhang_annotation;
/// Paint segmentation algorithms.
pub mod paint_segmentation;
/// Pre-pass slicing routines.
pub mod prepass_slice;
/// Pure region-mapping kernel (IR-only; no scheduler/runtime deps).
pub mod region_mapping;
/// Support geometry computation.
pub mod support_geometry;
