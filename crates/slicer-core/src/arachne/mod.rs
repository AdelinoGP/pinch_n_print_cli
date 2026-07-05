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
//! Packet 112 (Track B, Steps 5-7) adds three post-process transforms over
//! `Vec<ExtrusionLine>` output — also independent of the SKT graph:
//!
//! - [`stitch::stitch_extrusions`] (T-225) — joins open polylines across
//!   small gaps.
//! - [`simplify::simplify_toolpaths`] (T-226) — Visvalingam-Whyatt polyline
//!   simplification, width-preserving.
//! - [`remove_small::remove_small_lines`] (T-227) — drops degenerate odd,
//!   non-closed transition slivers.
//!
//! Packet 112 (Track B, Step 4) additionally adds
//! [`generate_toolpaths::generate_toolpaths`] (T-223) — variable-width inset
//! emission *from* the SKT graph (unlike the pass-through/post-process
//! modules above, this one does depend on
//! [`crate::skeletal_trapezoidation`], hence the narrower `host-algos` gate
//! on that one submodule; see its own module doc comment for the full
//! ADAPTATION contract).
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
//! [`generate_toolpaths`] is the one exception (see above): it is gated
//! behind `host-algos` individually, since it takes a
//! [`crate::skeletal_trapezoidation::SkeletalTrapezoidationGraph`] which only
//! exists under that feature.
//!
//! Packet 112 (Step 9A) adds [`pipeline::run_arachne_pipeline`], which chains
//! every stage above (plus the `skeletal_trapezoidation`/`beading` modules)
//! end to end. Gated behind `host-algos` like [`generate_toolpaths`], for the
//! same reason. This is the native pipeline the `generate-arachne-walls`
//! host-service bridge calls (mirroring the existing `medial-axis` bridge)
//! so a WASM guest — which cannot link `host-algos` itself — can still reach
//! the Arachne beading-strategy stack.

#[cfg(feature = "host-algos")]
pub mod generate_toolpaths;
#[cfg(feature = "host-algos")]
pub mod pipeline;
pub mod preprocess;
pub mod remove_small;
pub mod simplify;
pub mod stitch;

#[cfg(feature = "host-algos")]
pub use generate_toolpaths::generate_toolpaths;
#[cfg(feature = "host-algos")]
pub use pipeline::{run_arachne_pipeline, ArachneParams, ArachnePipelineError};
pub use preprocess::{preprocess_input_outline, preprocess_per_color_inputs, PreprocessParams};
pub use remove_small::remove_small_lines;
pub use simplify::simplify_toolpaths;
pub use stitch::stitch_extrusions;

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
