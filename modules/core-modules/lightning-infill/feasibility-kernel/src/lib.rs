// -----------------------------------------------------------------------------
// Original Rust: this file and the model DTOs are not translations of OrcaSlicer source.
//
// It is the isolated, module-owned feasibility kernel for the Lightning
// portability gate (packet 07). Type and evidence shapes are defined by
// `docs/spec_packets/layer-module-preparation_07_lightning-geometry-portability-gate/design.md`.
// Translated algorithm files added in later steps carry the standard OrcaSlicer
// porting header from `docs/ORCASLICER_ATTRIBUTION.md`.
// -----------------------------------------------------------------------------

//! Isolated owned Lightning geometry feasibility kernel.
//!
//! This crate is intentionally independent of `slicer-core`, `slicer-runtime`,
//! `slicer-wasm-host`, and every WIT/guest boundary. It is a plain `rlib` with
//! no filesystem or configuration access; all inputs are explicit typed records
//! transported by the caller.
//!
//! Layout:
//! - [`model`] — strict serde input/output/evidence DTOs and [`KernelError`].
//! - `domain` — Gate A source-only planning-domain reconstruction (later step).
//! - `polygon` — portable polygon primitives (later step).
//! - `distance_field` — portable distance-field primitives (later step).
//! - `tree` — directed Lightning tree construction (later step).
//! - `planner` — `plan_lightning` / `clip_lightning` (later step).

pub mod distance_field;
pub mod domain;
pub mod model;
pub mod planner;
pub mod polygon;
pub mod tree;

pub use model::*;
