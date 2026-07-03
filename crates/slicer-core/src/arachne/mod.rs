// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp,
// src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Arachne input preprocessing (M2 Arachne port, packet 110 step 5: T-204 and
//! T-P96-E).
//!
//! Two independent pieces of Arachne-input hygiene live here:
//!
//! - [`preprocess::preprocess_input_outline`] — the verified 9-stage
//!   Arachne input-outline hygiene pass ported from
//!   `WallToolPaths.cpp:565-604`.
//! - [`preprocess::preprocess_per_color_inputs`] — a **validated
//!   pass-through** for per-color MMU painted cells (T-P96-E). See that
//!   function's doc-comment for why this is pass-through rather than the
//!   bisector-contraction algorithm the packet's original design text
//!   described — that description cited a stale/retired revision of
//!   `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`.
//!
//! Both are pure polygon math over [`slicer_ir::ExPolygon`], independent of
//! the [`crate::voronoi`] / [`crate::skeletal_trapezoidation`] SKT graph
//! (this module does not build on either).
//!
//! # Host availability (default features, not `host-algos`-gated)
//!
//! Unlike [`crate::voronoi`], [`crate::skeletal_trapezoidation`], and
//! [`crate::algos`], this module calls no `boostvoronoi` primitive — only
//! [`crate::polygon_ops`] (Clipper2-backed) and [`slicer_ir::point_in_polygon_winding`]
//! (the same containment primitive [`crate::polygon_tree`] uses). It is
//! therefore compiled under default features rather than gated behind
//! `host-algos`, so it stays available to any caller that only needs
//! polygon-level preprocessing without pulling in the voronoi/SKT stack.

pub mod preprocess;

pub use preprocess::{preprocess_input_outline, preprocess_per_color_inputs, PreprocessParams};

/// Identifies the paint/MMU tool (extruder or color) a per-color Arachne
/// input cell belongs to.
///
/// No standalone `ToolIndex` type exists anywhere in this workspace as of
/// packet 110 (verified: grepping `crates/slicer-ir/src/**` and
/// `crates/slicer-core/src/**` for `ToolIndex` finds only the
/// `slicer_ir::slice_ir::PaintValue::ToolIndex(u32)` enum variant used
/// throughout the paint/MMU pipeline from packets P91-94 — never a bare
/// type). This alias mirrors that variant's inner representation (`u32`)
/// exactly, so per-color cell lists built by this module line up with
/// `PaintValue::ToolIndex(n)` values without introducing a second,
/// structurally-incompatible "tool index" concept.
pub type ToolIndex = u32;
