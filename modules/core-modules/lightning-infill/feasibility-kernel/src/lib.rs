// -----------------------------------------------------------------------------
// Original Rust: this file and the model DTOs are not translations of OrcaSlicer source.
//
// It is the isolated, module-owned feasibility kernel for the Lightning
// portability gate (packet 07). Type and evidence shapes are defined by
// `docs/spec_packets/layer-module-preparation_07_lightning-geometry-portability-gate/design.md`.
// Translated algorithm files carry the standard OrcaSlicer porting header
// from `docs/ORCASLICER_ATTRIBUTION.md`.
// -----------------------------------------------------------------------------

//! Isolated owned Lightning geometry feasibility kernel.
//!
//! This crate stays independent of `slicer-runtime`, `slicer-wasm-host`, every
//! host algorithm (no `host-algos`/`rayon`/`boostvoronoi`), and every WIT/guest
//! boundary. It takes `slicer-core` with default features only because the
//! included module kernel source links against it. It is a plain `rlib` with
//! no filesystem or configuration access; all inputs are explicit typed records
//! transported by the caller.
//!
//! Layout:
//! - [`model`] — strict serde input/output/evidence DTOs and [`KernelError`].
//! - `domain` — Gate A source-only planning-domain reconstruction.
//! - `polygon` — portable polygon primitives.
//! - `distance_field` — portable distance-field primitives.
//! - `tree` — directed Lightning tree construction.
//! - `planner` — `plan_lightning` / `clip_lightning`.

pub mod distance_field;
pub mod domain;
pub mod kernel;
pub mod model;
#[allow(clippy::result_large_err)]
pub mod planner;
pub mod polygon;
#[allow(clippy::result_large_err)]
pub mod tree;

pub use model::*;
