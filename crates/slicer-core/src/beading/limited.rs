// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `LimitedBeadingStrategy`: caps total bead count and marks the cap boundary
//! with zero-width "sentinel" beads.
//!
//! OrcaSlicer's `LimitedBeadingStrategy::compute` only ever handles a
//! delegated `bead_count` of exactly `max_bead_count + 1` (its own
//! `getOptimalBeadCount` clamps any larger parent count down to
//! `max_bead_count + 1` before it can reach `compute`), inserting a single
//! symmetric pair of zero-width sentinels at `max_bead_count / 2` and a
//! mirrored index. This port generalizes that mechanism to an arbitrary
//! excess `bead_count - max_bead_count` (see packet 111 Step 6 follow_up for
//! the mapping): the parent is recomputed at the capped `max_bead_count`,
//! then `sentinel_count = bead_count - max_bead_count` zero-width beads are
//! inserted as two blocks — one at index `n / 2` of the parent's beading
//! (`n == max_bead_count`), and a mirrored block at index `n - n / 2`
//! (shifted to account for the first block's insertion) — so the result
//! always carries `bead_widths.len() == max_bead_count + 2 * sentinel_count`,
//! matching OrcaSlicer's shape exactly when `sentinel_count == 1`.
//!
//! Each sentinel's `toolpath_locations` entry is a placeholder: it
//! duplicates the location of the nearest real bead across the cut (the bead
//! just before the cut for the left block, the bead just after the cut for
//! the right block). Downstream centrality propagation reads this value but
//! does not rely on it being geometrically meaningful — sentinels carry zero
//! width, so they never contribute to toolpath geometry.
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use super::distributed::assert_beading_invariant;
use super::{Beading, BeadingStrategy};

/// Decorator over another `BeadingStrategy` that caps `optimal_bead_count`
/// (and any delegated `bead_count` above the cap) at `max_bead_count`,
/// inserting zero-width sentinel beads at the cap boundary so downstream
/// bookkeeping can locate where beads were elided.
pub struct LimitedBeadingStrategy {
    /// The wrapped strategy, whose `compute` result is post-processed.
    parent: Box<dyn BeadingStrategy>,
    /// The maximum number of (non-sentinel) beads this strategy will ever
    /// request from `parent`.
    max_bead_count: usize,
}

impl LimitedBeadingStrategy {
    /// Creates a new `LimitedBeadingStrategy` wrapping `parent`, capping bead
    /// count at `max_bead_count`.
    pub fn new(parent: Box<dyn BeadingStrategy>, max_bead_count: usize) -> Self {
        Self {
            parent,
            max_bead_count,
        }
    }

    /// Computes the beading via `compute`, then removes every zero-width
    /// sentinel entry (in lockstep with its matching `toolpath_locations`
    /// entry), returning a `Beading` with no zero-width beads. Intended for
    /// production/external callers that only care about real, extrudable
    /// beads; `compute` itself retains sentinels for internal invariant
    /// testing and downstream centrality propagation.
    ///
    /// `total_thickness` and `left_over` pass through unchanged from the raw
    /// `compute` result: removed sentinels contributed exactly `0.0` width,
    /// so the `total_thickness == sum(bead_widths) + left_over` invariant
    /// still holds numerically after stripping.
    pub fn compute_and_strip(&self, thickness: f64, bead_count: usize) -> Beading {
        let raw = self.compute(thickness, bead_count);

        let mut bead_widths = Vec::with_capacity(raw.bead_widths.len());
        let mut toolpath_locations = Vec::with_capacity(raw.toolpath_locations.len());
        for (&width, &location) in raw.bead_widths.iter().zip(raw.toolpath_locations.iter()) {
            if width > 0.0 {
                bead_widths.push(width);
                toolpath_locations.push(location);
            }
        }

        let stripped = Beading {
            total_thickness: raw.total_thickness,
            bead_widths,
            toolpath_locations,
            left_over: raw.left_over,
        };

        assert_beading_invariant(&stripped);
        stripped
    }
}

impl BeadingStrategy for LimitedBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        if bead_count <= self.max_bead_count {
            // Cap not exceeded: delegate unchanged, no sentinels needed.
            let beading = self.parent.compute(thickness, bead_count);
            assert_beading_invariant(&beading);
            return beading;
        }

        // Over cap: request the CAPPED count from the parent, then insert
        // sentinels marking where the elided beads would have gone.
        let mut beading = self.parent.compute(thickness, self.max_bead_count);
        let n = beading.bead_widths.len();
        let sentinel_count = bead_count - self.max_bead_count;

        let left_index = n / 2;
        // Mirror position in the ORIGINAL (pre-insertion) index space.
        let right_index = n - left_index;

        let left_location = if left_index > 0 {
            beading.toolpath_locations[left_index - 1]
        } else {
            0.0
        };
        let right_location = if right_index < n {
            beading.toolpath_locations[right_index]
        } else {
            *beading.toolpath_locations.last().unwrap_or(&0.0)
        };

        // Insert the left sentinel block first (lower index).
        for _ in 0..sentinel_count {
            beading.bead_widths.insert(left_index, 0.0);
            beading.toolpath_locations.insert(left_index, left_location);
        }
        // The right block's mirrored index shifts by sentinel_count because
        // the left block's insertion already grew the vectors below it.
        let shifted_right_index = right_index + sentinel_count;
        for _ in 0..sentinel_count {
            beading.bead_widths.insert(shifted_right_index, 0.0);
            beading
                .toolpath_locations
                .insert(shifted_right_index, right_location);
        }

        assert_beading_invariant(&beading);
        beading
    }

    fn optimal_bead_count(&self, thickness: f64) -> usize {
        self.parent
            .optimal_bead_count(thickness)
            .min(self.max_bead_count)
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        self.parent.get_transition_thickness(lower_bead_count)
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        self.parent.optimal_thickness(bead_count)
    }

    fn type_label(&self) -> &'static str {
        "Limited"
    }

    fn type_chain(&self) -> String {
        format!("{}({})", self.type_label(), self.parent.type_chain())
    }
}
