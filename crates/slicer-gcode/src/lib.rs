//! `slicer-gcode` — G-code emission and serialization extracted from
//! `slicer-runtime` (packet 86).
//!
//! This crate hosts the pure-IR transformations that convert
//! `LayerCollectionIR` to `GCodeIR` and `GCodeIR` to G-code text. It carries
//! no dependency on `wasmtime`, the WIT host, the scheduler, or the runtime
//! blackboard — the seam confirmed in Step 1.

pub mod emit;
pub mod error;
pub mod estimator;
pub mod serialize;
pub mod thumbnail;

pub use emit::{reconcile_finalization_travel, DefaultGCodeEmitter, GCodeEmitter};
pub use error::GCodeEmitError;
pub use estimator::{estimate_print, EstimatorLimits, PrintEstimate};
pub use serialize::{
    format_coord, format_xyz, resolved_config_to_map, tolerance_for_role, DefaultGCodeSerializer,
    GCodeSerializer, ThumbnailAwareSerializer,
};
pub use thumbnail::serialize_thumbnail_block;
/// G-code flavor dialect layer ported from OrcaSlicer's `GCodeWriter.cpp`.
pub mod flavor;
pub use flavor::GcodeFlavor;
