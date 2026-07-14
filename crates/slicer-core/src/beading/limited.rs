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
use slicer_ir::UNITS_PER_MM;

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
            // Under-cap branch: mirror OrcaSlicer
            // `LimitedBeadingStrategy::compute` lines 69-84. Delegate to the
            // parent, then if the resulting count is even AND exactly equal
            // to `max_bead_count` (i.e. the requested count sat at the cap
            // boundary, not below it), insert a single 0-width sentinel
            // bead at the region centre marking where infill/skin should
            // align. The sentinel's `toolpath_location` is the inner edge
            // of the innermost real bead (centerline + half-width), matching
            // `LimitedBeadingStrategy.cpp:77-80`.
            let mut beading = self.parent.compute(thickness, bead_count);
            let actual_count = beading.toolpath_locations.len();
            if actual_count.is_multiple_of(2) && actual_count == self.max_bead_count {
                let inner_idx = self.max_bead_count / 2 - 1;
                let innermost_location = beading.toolpath_locations[inner_idx];
                let innermost_width = beading.bead_widths[inner_idx];
                let sentinel_location = innermost_location + innermost_width / 2.0;
                beading
                    .toolpath_locations
                    .insert(self.max_bead_count / 2, sentinel_location);
                beading.bead_widths.insert(self.max_bead_count / 2, 0.0);
            }
            assert_beading_invariant(&beading);
            return beading;
        }

        // Over cap: mirror OrcaSlicer's `LimitedBeadingStrategy::compute`
        // (`LimitedBeadingStrategy.cpp:64-90`). The parent is recomputed at
        // the *optimal* thickness for `max_bead_count` beads
        // (`max_bead_count * optimal_width`), NOT the full region thickness,
        // so the freshly placed beads are spaced a single `optimal_width`
        // apart (one Flow spacing, not the raw region width). The surplus
        // region thickness becomes `left_over` — i.e. infill, exactly as
        // OrcaSlicer's `ret.left_over += thickness - ret.total_thickness`
        // does. Then the result is mirrored symmetrically about the region
        // centre, after which zero-width sentinels mark the capped boundary.
        let optimal_thickness = self.parent.optimal_thickness(self.max_bead_count);
        let mut beading = self.parent.compute(optimal_thickness, self.max_bead_count);
        beading.left_over += thickness - beading.total_thickness;
        beading.total_thickness = thickness;

        // Enforce symmetry about the region centre (OrcaSlicer
        // `LimitedBeadingStrategy.cpp:70-76`).
        let n = beading.toolpath_locations.len();
        if n % 2 == 1 {
            beading.toolpath_locations[n / 2] = thickness / 2.0;
            beading.bead_widths[n / 2] = thickness - optimal_thickness;
        }
        for i in 0..n.div_ceil(2) {
            beading.toolpath_locations[n - 1 - i] = thickness - beading.toolpath_locations[i];
        }

        let n = beading.bead_widths.len();
        let sentinel_count = bead_count - self.max_bead_count;

        let left_index = n / 2;
        // Mirror position in the ORIGINAL (pre-insertion) index space.
        let right_index = n - left_index;

        // Sentinel `toolpath_location` is the **inner edge** of the
        // innermost real bead (centerline + half-width), per OrcaSlicer
        // `LimitedBeadingStrategy.cpp:118-122,126-130`. The PnP f64 type
        // is exact here (the C++ coord_t integer-division `+ width/2`
        // hazard noted at line 116-117 of the C++ source does not apply).
        let left_location = if left_index > 0 {
            beading.toolpath_locations[left_index - 1] + beading.bead_widths[left_index - 1] / 2.0
        } else {
            0.0
        };
        let right_location = if right_index < n {
            beading.toolpath_locations[right_index] - beading.bead_widths[right_index] / 2.0
        } else {
            // Degenerate fallback: mirror about the end of the vector.
            // (Matches the pre-existing `unwrap_or(&0.0)` fallback at the
            // *centreline*; corrected here to the inner-edge formulation,
            // but the fallback path itself is unreachable for the
            // distributed parent that the factory always wires in.)
            let last_idx = beading.toolpath_locations.len().saturating_sub(1);
            if last_idx > 0 {
                beading.toolpath_locations[last_idx] - beading.bead_widths[last_idx] / 2.0
            } else {
                0.0
            }
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
        // Mirror OrcaSlicer's `LimitedBeadingStrategy::getOptimalBeadCount`
        // (`LimitedBeadingStrategy.cpp:115-127`): the cap is `max_bead_count
        // + 1`, never `max_bead_count`. Returning `max_bead_count` here would
        // force `compute` down the delegated (non-sentinel) branch and make
        // the capped beads span the *full* region thickness at `optimal_width`
        // spacing — i.e. hugely over-spaced walls. Capping at `max_bead_count
        // + 1` lets `compute` take the over-cap branch, which recomputes at
        // the optimal thickness and leaves the surplus as `left_over`
        // (infill), matching OrcaSlicer's fixed-inset wall model.
        let parent_count = self.parent.optimal_bead_count(thickness);
        if parent_count <= self.max_bead_count {
            parent_count
        } else if parent_count == self.max_bead_count + 1 {
            // 0.01 mm in slicer units (OrcaSlicer
            // `LimitedBeadingStrategy.cpp:170` `scaled<coord_t>(0.01)` =
            // 10µm = 100 PNP units). Matches the convention in
            // `crates/slicer-core/src/beading/distributed.rs:199` and
            // `crates/slicer-core/src/arachne/generate_toolpaths.rs:230`.
            if thickness
                < self.parent.optimal_thickness(self.max_bead_count + 1) - 0.01 * UNITS_PER_MM
            {
                self.max_bead_count
            } else {
                self.max_bead_count + 1
            }
        } else {
            self.max_bead_count + 1
        }
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

    fn get_split_middle_threshold(&self) -> f64 {
        self.parent.get_split_middle_threshold()
    }

    fn get_add_middle_threshold(&self) -> f64 {
        self.parent.get_add_middle_threshold()
    }

    fn type_chain(&self) -> String {
        format!("{}({})", self.type_label(), self.parent.type_chain())
    }

    fn wall_transition_angle(&self) -> f64 {
        self.parent.wall_transition_angle()
    }

    fn get_transitioning_length(&self, lower_bead_count: usize) -> f64 {
        self.parent.get_transitioning_length(lower_bead_count)
    }

    fn get_transition_anchor_pos(&self, lower_bead_count: usize) -> f64 {
        self.parent.get_transition_anchor_pos(lower_bead_count)
    }

    fn get_nonlinear_thicknesses(&self, lower_bead_count: usize) -> Vec<f64> {
        self.parent.get_nonlinear_thicknesses(lower_bead_count)
    }

    fn get_transition_filter_dist(&self, lower_bead_count: usize) -> f64 {
        self.parent.get_transition_filter_dist(lower_bead_count)
    }
}
