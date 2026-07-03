// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Arachne bead-width distribution strategies.
//!
//! A `BeadingStrategy` decides how a variable-width wall region (a "thickness")
//! is subdivided into a sequence of extrusion beads (widths and their toolpath
//! offsets from the region's outer edge). This module defines the shared trait
//! and result type that concrete strategies (Distributed, Redistribute,
//! Widening, OuterWallInset, Limited — added in later steps of this packet)
//! implement.
//!
//! All thickness, width, and location values in this module are in **slicer
//! units**, where 1 unit = 100 nm (10⁻⁴ mm) — see `docs/08_coordinate_system.md`.
//! This module defines only the trait/data surface; unit conversion is the
//! responsibility of callers and of the concrete strategies added in later
//! steps.

pub mod distributed;
pub mod factory;
pub mod limited;
pub mod outer_wall_inset;
pub mod redistribute;
pub mod widening;

/// The result of distributing a wall region's thickness into extrusion beads.
///
/// Widths and locations are both in slicer units (1 unit = 100 nm).
///
/// Invariant: `bead_widths.len() == toolpath_locations.len()`. This is not
/// enforced by `Beading` itself (it is a plain data holder); each strategy's
/// `compute` implementation is responsible for debug-asserting it before
/// returning.
///
/// `bead_widths` (and correspondingly `toolpath_locations`) are ordered
/// outermost to innermost: index 0 is the bead closest to the region's outer
/// edge, and the last index is the bead closest to the region's inner edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Beading {
    /// The total thickness of the region this beading was computed for.
    pub total_thickness: f64,
    /// Width of each bead, ordered outermost to innermost.
    pub bead_widths: Vec<f64>,
    /// Toolpath center-line offset of each bead from the region's outer edge,
    /// ordered outermost to innermost, parallel to `bead_widths`.
    pub toolpath_locations: Vec<f64>,
    /// Thickness left over that could not be distributed into a bead (e.g. a
    /// sliver narrower than the minimum extrudable width).
    pub left_over: f64,
}

/// Strategy for distributing a variable wall thickness into extrusion beads.
///
/// Implementations must be stateless with respect to a single `compute` call
/// (safe to share across threads) and object-safe, since decorator strategies
/// added in later steps of this packet wrap a `Box<dyn BeadingStrategy>`.
pub trait BeadingStrategy: Send + Sync {
    /// Computes the beading (widths and toolpath locations) for a region of
    /// the given `thickness`, distributed into exactly `bead_count` beads.
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading;

    /// Returns the number of beads this strategy considers optimal for a
    /// region of the given `thickness`.
    fn optimal_bead_count(&self, thickness: f64) -> usize;

    /// Returns the thickness at which this strategy transitions from
    /// `lower_bead_count` beads to `lower_bead_count + 1` beads.
    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64;

    /// Returns the thickness this strategy considers optimal for producing
    /// exactly `bead_count` beads.
    fn optimal_thickness(&self, bead_count: usize) -> f64;

    /// Returns a short, stable label identifying the concrete strategy type
    /// (e.g. `"Distributed"`, `"Redistribute"`). Used by later steps of this
    /// packet to verify decorator composition order.
    fn type_label(&self) -> &'static str;

    /// Returns a string describing the full decorator composition chain from
    /// this strategy down to its innermost (base) parent, e.g.
    /// `"Limited(OuterWallInset(Widening(Redistribute(Distributed))))"`.
    ///
    /// The default implementation (correct for `DistributedBeadingStrategy`,
    /// which has no parent) simply returns `type_label()`. Decorator
    /// strategies override this to wrap their parent's `type_chain()` in
    /// `type_label()(...)`.
    fn type_chain(&self) -> String {
        self.type_label().to_string()
    }
}
