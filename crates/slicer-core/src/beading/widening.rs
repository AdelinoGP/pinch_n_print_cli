// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/WideningBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `WideningBeadingStrategy`: sub-`optimal_width` thin-wall decorator.
//!
//! Faithfully ports OrcaSlicer's `WideningBeadingStrategy::compute` /
//! `getOptimalBeadCount` / `getTransitionThickness`
//! (`WideningBeadingStrategy.cpp:48-68,72-91`), including the real 3-way
//! branch structure:
//!
//! - `thickness >= optimal_width`: delegates to the wrapped strategy
//!   unmodified (`parent.compute(thickness, bead_count)`).
//! - `min_input_width <= thickness < optimal_width`: emits a single bead of
//!   width `thickness.max(min_output_width)` (i.e. the bead is `thickness`
//!   itself unless that would be narrower than `min_output_width`, in which
//!   case it is clamped up), with `left_over = 0.0`.
//! - `thickness < min_input_width`: emits an empty `bead_widths` /
//!   `toolpath_locations` with `left_over = thickness` — the entire
//!   thickness goes unprinted.
//!
//! `optimal_width` is, in upstream C++, inherited from `parent` via
//! base-class copy-construction at `WideningBeadingStrategy` construction
//! time. Rust has no equivalent mechanism (there is no way to read a private
//! field off a `Box<dyn BeadingStrategy>`), so it is an explicit constructor
//! parameter here instead. The caller (the beading factory) is responsible
//! for passing the same `optimal_width` value used to build the wrapped
//! `RedistributeBeadingStrategy`/`DistributedBeadingStrategy` chain.
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use super::distributed::assert_beading_invariant;
use super::{Beading, BeadingStrategy};

/// Decorator over another `BeadingStrategy` that intercepts regions whose
/// `thickness` is below `optimal_width` before delegating: at/above
/// `min_input_width` (but still below `optimal_width`) it reports a single
/// bead sized `thickness.max(min_output_width)`; below `min_input_width` it
/// reports no beads at all (`left_over = thickness`). At/above
/// `optimal_width`, the wrapped strategy's `compute` result is returned
/// unmodified.
pub struct WideningBeadingStrategy {
    /// The wrapped strategy, delegated to unmodified at/above
    /// `optimal_width`.
    parent: Box<dyn BeadingStrategy>,
    /// The base optimal bead width (slicer units) of the wrapped strategy
    /// chain. `thickness >= optimal_width` bypasses this decorator entirely.
    /// See the module doc comment for why this is an explicit field rather
    /// than inherited from `parent`.
    optimal_width: f64,
    /// Threshold (slicer units) below which `thickness` produces no beads at
    /// all (the entire thickness becomes `left_over`).
    min_input_width: f64,
    /// Minimum bead width (slicer units) emitted in the
    /// `min_input_width <= thickness < optimal_width` regime; the emitted
    /// bead is `thickness.max(min_output_width)`.
    min_output_width: f64,
    /// Layer-specific minimum bead width (slicer units) applied on the initial
    /// layer, overriding `min_output_width` when the caller indicates a layer
    /// is the first layer.
    initial_layer_min_bead_width: f64,
}

impl WideningBeadingStrategy {
    /// Creates a new `WideningBeadingStrategy` wrapping `parent`.
    ///
    /// `optimal_width` must match the base optimal bead width of the wrapped
    /// `parent` chain (see the module doc comment). `min_input_width` is the
    /// threshold below which no beads are produced at all. `min_output_width`
    /// is the minimum width of the single bead emitted in the middle regime
    /// (`min_input_width <= thickness < optimal_width`).
    pub fn new(
        parent: Box<dyn BeadingStrategy>,
        optimal_width: f64,
        min_input_width: f64,
        min_output_width: f64,
    ) -> Self {
        Self {
            parent,
            optimal_width,
            min_input_width,
            min_output_width,
            initial_layer_min_bead_width: min_output_width,
        }
    }

    /// Sets the layer-specific minimum bead width override used for the
    /// initial layer. Returns `self` for chained construction.
    pub fn with_initial_layer_min_bead_width(mut self, width: f64) -> Self {
        self.initial_layer_min_bead_width = width;
        self
    }
}

impl BeadingStrategy for WideningBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        if thickness < self.optimal_width {
            let beading = if thickness >= self.min_input_width {
                // Middle regime: a single bead sized to the thickness itself,
                // clamped up to min_output_width if the thickness is
                // narrower than that.
                Beading {
                    total_thickness: thickness,
                    bead_widths: vec![thickness.max(self.min_output_width)],
                    toolpath_locations: vec![thickness / 2.0],
                    left_over: 0.0,
                }
            } else {
                // Bottom regime: too thin to print at all; the whole
                // thickness is unprinted left-over.
                Beading {
                    total_thickness: thickness,
                    bead_widths: Vec::new(),
                    toolpath_locations: Vec::new(),
                    left_over: thickness,
                }
            };
            assert_beading_invariant(&beading);
            return beading;
        }

        // At/above optimal_width: delegate to the wrapped strategy
        // unmodified.
        self.parent.compute(thickness, bead_count)
    }

    fn optimal_bead_count(&self, thickness: f64) -> usize {
        if thickness < self.min_input_width {
            return 0;
        }
        let parent_count = self.parent.optimal_bead_count(thickness);
        if thickness >= self.min_input_width && parent_count < 1 {
            return 1;
        }
        parent_count
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        if lower_bead_count == 0 {
            return self.min_input_width;
        }
        self.parent.get_transition_thickness(lower_bead_count)
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        self.parent.optimal_thickness(bead_count)
    }

    fn type_label(&self) -> &'static str {
        "Widening"
    }

    fn type_chain(&self) -> String {
        format!("{}({})", self.type_label(), self.parent.type_chain())
    }

    fn wall_transition_angle(&self) -> f64 {
        self.parent.wall_transition_angle()
    }
}
