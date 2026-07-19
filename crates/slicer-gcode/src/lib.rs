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
pub mod m73;
pub mod serialize;
pub mod thumbnail;
pub mod thumbnail_btt;
pub mod thumbnail_colpic;

pub use emit::{reconcile_finalization_travel, DefaultGCodeEmitter, GCodeEmitter};
pub use error::GCodeEmitError;
pub use estimator::{estimate_print, EstimatorLimits, PrintEstimate};
pub use m73::{filament_stats_comment_block, inject_m73};
pub use serialize::{
    format_coord, format_xyz, resolved_config_to_map, tolerance_for_role, DefaultGCodeSerializer,
    GCodeSerializer, ThumbnailAwareSerializer,
};
pub use thumbnail::{
    decode_base64, encode_base64, parse_thumbnails_key, render_thumbnail_entries,
    serialize_thumbnail_block, RenderedThumbnail, ThumbnailBody, ThumbnailError, ThumbnailFormat,
    ThumbnailSpec,
};
pub use thumbnail_btt::encode_btt_tft;
pub use thumbnail_colpic::{encode_colpic, encode_colpic_with_capped_dims};
/// G-code flavor dialect layer ported from OrcaSlicer's `GCodeWriter.cpp`.
pub mod flavor;
pub use flavor::GcodeFlavor;
