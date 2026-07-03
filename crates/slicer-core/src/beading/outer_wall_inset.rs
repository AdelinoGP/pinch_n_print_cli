// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `OuterWallInsetBeadingStrategy`: outer-wall toolpath inset decorator.
//!
//! Wraps another `BeadingStrategy` and shifts *only* the outer wall's
//! toolpath location inward by a configured `outer_wall_offset`. Bead widths
//! and all inner toolpath locations are left untouched — this decorator
//! affects toolpath geometry only, never bead-count math (which is why
//! `optimal_bead_count`, `get_transition_thickness`, and `optimal_thickness`
//! all delegate to `parent` unmodified).
//!
//! `toolpath_locations` is ordered outermost to innermost (see
//! `Beading::toolpath_locations` doc in `super`), so "inward" means:
//! increase `toolpath_locations[0]` (move the outermost bead's centerline
//! toward the region's center).
//!
//! This faithfully ports OrcaSlicer's `OuterWallInsetBeadingStrategy::
//! compute` (`OuterWallInsetBeadingStrategy.cpp:69-92`): the offset is
//! **single-sided** — only `toolpath_locations[0]` is ever modified, clamped
//! to `thickness / 2.0` (the region's centerline). The opposite end
//! (`toolpath_locations[last]`) is never touched. The early-return for "no
//! inset needed" recounts *non-zero-width* beads (filtering out any
//! zero-width sentinel beads that might be present in the parent's output)
//! rather than trusting `bead_widths.len()` directly, matching upstream's
//! own defensive counting; in this codebase's canonical decorator stack
//! order this filtering is a no-op (there are no zero-width beads by the
//! time this decorator runs), but it is ported exactly for correctness if
//! composition order ever changes.
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use super::{Beading, BeadingStrategy};

/// Decorator over another `BeadingStrategy` that insets only the outer
/// wall's toolpath location by `outer_wall_offset`, leaving bead widths and
/// inner toolpath locations untouched.
pub struct OuterWallInsetBeadingStrategy {
    /// The wrapped strategy, whose `compute` result is post-processed.
    parent: Box<dyn BeadingStrategy>,
    /// Inward offset (slicer units) applied to the outer wall's toolpath
    /// location (`toolpath_locations[0]` only). A value of `0.0` makes this
    /// decorator effectively a no-op.
    outer_wall_offset: f64,
}

impl OuterWallInsetBeadingStrategy {
    /// Creates a new `OuterWallInsetBeadingStrategy` wrapping `parent`,
    /// insetting the outer wall's toolpath location by `outer_wall_offset`.
    pub fn new(parent: Box<dyn BeadingStrategy>, outer_wall_offset: f64) -> Self {
        Self {
            parent,
            outer_wall_offset,
        }
    }
}

impl BeadingStrategy for OuterWallInsetBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        let mut beading = self.parent.compute(thickness, bead_count);

        // Recount non-zero-width beads (filters out any zero-width
        // "signaling" beads — sentinels — that might already be present; in
        // this codebase's canonical stack order Limited is always
        // outermost/applied last, so this parent chain never actually
        // produces sentinels here, but the filtering logic is ported
        // faithfully regardless, matching upstream's own defensive
        // counting).
        let effective_bead_count = beading.bead_widths.iter().filter(|&&w| w > 0.0).count();

        // No inset needed for a single wall.
        if effective_bead_count < 2 {
            return beading;
        }

        // Move the outer wall's toolpath location inward, clamped to the
        // model's centerline. ONLY index 0 — the opposite end is never
        // touched.
        let len = beading.toolpath_locations.len();
        if len > 0 {
            beading.toolpath_locations[0] =
                (beading.toolpath_locations[0] + self.outer_wall_offset).min(thickness / 2.0);
        }

        beading
    }

    fn optimal_bead_count(&self, thickness: f64) -> usize {
        self.parent.optimal_bead_count(thickness)
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        self.parent.get_transition_thickness(lower_bead_count)
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        self.parent.optimal_thickness(bead_count)
    }

    fn type_label(&self) -> &'static str {
        "OuterWallInset"
    }

    fn type_chain(&self) -> String {
        format!("{}({})", self.type_label(), self.parent.type_chain())
    }
}
