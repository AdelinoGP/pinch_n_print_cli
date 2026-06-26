//! Host-side model file ingestion and geometry-only writers.
//!
//! Parses STL, OBJ, and 3MF into [`slicer_ir::MeshIR`]; writes the geometry-only
//! 3MF and OBJ formats. Extracted from `slicer-runtime` in packet 81 so the
//! runtime no longer touches bytes — its slice entry consumes a pre-loaded
//! [`slicer_ir::MeshIR`] instead of a path.

#![warn(missing_docs)]

pub mod loader;
pub mod sidecar;
pub mod writer;

pub use loader::{
    assemble_object, detect_format, load_model, read_3mf_filament_colours, ModelFormat,
    ModelLoadError,
};
pub use sidecar::{parse_3mf_sidecar, ObjectSidecarInfo, PartSubtype};
pub use writer::{write_3mf, write_obj};
