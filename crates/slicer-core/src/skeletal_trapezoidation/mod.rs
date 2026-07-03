// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/utils/SkeletalTrapezoidationGraph.hpp,
// SkeletalTrapezoidationJoint.hpp, SkeletalTrapezoidationEdge.hpp, HalfEdge.hpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Skeletal trapezoidation graph (T-202 of the M2 Arachne port,
//! `docs/adr/0023-arachne-port-strategy.md`).
//!
//! This is the Orca-shaped half-edge graph (`SkeletalTrapezoidationGraph` in
//! OrcaSlicer) built on top of [`crate::voronoi`]'s boostvoronoi-shaped
//! segment Voronoi diagram. It anchors bead-count assignment (P111) and
//! centrality/discretization (P112).
//!
//! Host-only: gated behind the `host-algos` feature, matching
//! [`crate::voronoi`], [`crate::algos`], and [`crate::medial_axis`].

pub mod discretize;
pub mod graph;

pub use discretize::discretize_parabolic_edge;
pub use graph::{STHalfEdge, STVertex, SkeletalTrapezoidationGraph, SktError};
