// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/RedistributeBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `RedistributeBeadingStrategy`: outer-wall-width-consistency decorator.
//!
//! Wraps another `BeadingStrategy` and forces the outermost and innermost
//! bead widths of the region to (as close as possible to) `optimal_width_outer`,
//! by recursing into the wrapped strategy on a *reduced* thickness/bead_count
//! (subtracting two outer-wall widths and two beads) and prepending/appending
//! freshly computed outer beads around whatever the parent returns for that
//! reduced inner region.
//!
//! This faithfully ports OrcaSlicer's `RedistributeBeadingStrategy::compute`
//! (`RedistributeBeadingStrategy.cpp:99-141`): it does NOT call
//! `parent.compute(thickness, bead_count)` at full size and then clamp the
//! result; it calls `parent.compute(inner_thickness, inner_bead_count)` at
//! reduced size (`thickness - 2*optimal_width_outer`, `bead_count - 2`, with
//! the reduced bead count computed with signed arithmetic since it may go
//! negative) and then inserts fresh outer beads around whatever the parent
//! returns for that reduced region (which may be empty). `left_over` is
//! always recomputed from scratch as `thickness - sum(bead_widths)` rather
//! than inherited from the parent's own `left_over`.
//!
//! Upstream's `RedistributeBeadingStrategy` also overrides
//! `getOptimalBeadCount`/`getOptimalThickness`/`getTransitionThickness` with
//! logic that recurses into `parent` on a thickness/bead_count reduced by the
//! two outer beads, then re-adds the two outer `optimal_width_outer` beads.
//! These three methods are ported below (packet 155, Step 3) and consult
//! `parent.get_split_middle_threshold()` in the `case 1` branch of
//! `get_transition_thickness` and the 2-bead branch of `optimal_bead_count`.
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use super::distributed::assert_beading_invariant;
use super::{Beading, BeadingStrategy};

/// Decorator over another `BeadingStrategy` that forces the outermost and
/// innermost bead widths toward `optimal_width_outer` by recursing into the
/// wrapped strategy on a thickness/bead_count reduced by the two outer beads,
/// then prepending/appending freshly computed outer beads around the result.
pub struct RedistributeBeadingStrategy {
    /// The wrapped strategy whose `compute` output supplies the "inner"
    /// beads (everything between the two outer beads this decorator adds).
    parent: Box<dyn BeadingStrategy>,
    /// The target width (slicer units) for the outermost and innermost bead,
    /// and the amount of thickness/bead_count subtracted before recursing
    /// into `parent`.
    optimal_width_outer: f64,
    /// Below `minimum_variable_line_ratio * optimal_width_outer` total
    /// thickness, no beads are produced at all (the whole thickness becomes
    /// `left_over`).
    minimum_variable_line_ratio: f64,
}

impl RedistributeBeadingStrategy {
    /// Creates a new `RedistributeBeadingStrategy` wrapping `parent`.
    ///
    /// `optimal_width_outer` is the target width (slicer units) for the
    /// outermost and innermost bead. `minimum_variable_line_ratio` is the
    /// fraction of `optimal_width_outer` below which a region produces no
    /// beads at all.
    pub fn new(
        parent: Box<dyn BeadingStrategy>,
        optimal_width_outer: f64,
        minimum_variable_line_ratio: f64,
    ) -> Self {
        Self {
            parent,
            optimal_width_outer,
            minimum_variable_line_ratio,
        }
    }
}

impl BeadingStrategy for RedistributeBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        // Early-out: no lines produced at all.
        if bead_count == 0
            || thickness < self.minimum_variable_line_ratio * self.optimal_width_outer
        {
            let beading = Beading {
                total_thickness: thickness,
                bead_widths: Vec::new(),
                toolpath_locations: Vec::new(),
                left_over: thickness,
            };
            assert_beading_invariant(&beading);
            return beading;
        }

        // Inner walls (if any). `bead_count - 2` must use signed arithmetic:
        // `bead_count` is `usize` but upstream's `coord_t` is signed, and
        // `bead_count == 1` makes this go negative — a valid "skip inner"
        // signal, not an error.
        let inner_bead_count_signed: i64 = bead_count as i64 - 2;
        let inner_thickness = thickness - 2.0 * self.optimal_width_outer;

        let (mut bead_widths, mut toolpath_locations): (Vec<f64>, Vec<f64>) =
            if inner_bead_count_signed > 0 && inner_thickness > 0.0 {
                let inner = self
                    .parent
                    .compute(inner_thickness, inner_bead_count_signed as usize);
                let shifted_locations: Vec<f64> = inner
                    .toolpath_locations
                    .iter()
                    .map(|&loc| loc + self.optimal_width_outer)
                    .collect();
                (inner.bead_widths, shifted_locations)
            } else {
                (Vec::new(), Vec::new())
            };

        // Insert outer wall(s) around the (possibly empty) inner result.
        let actual_outer_thickness = if bead_count > 2 {
            (thickness / 2.0).min(self.optimal_width_outer)
        } else {
            // bead_count is 1 or 2 here (0 already handled above).
            thickness / bead_count as f64
        };

        bead_widths.insert(0, actual_outer_thickness);
        toolpath_locations.insert(0, actual_outer_thickness / 2.0);
        if bead_count > 1 {
            bead_widths.push(actual_outer_thickness);
            toolpath_locations.push(thickness - actual_outer_thickness / 2.0);
        }

        let sum: f64 = bead_widths.iter().sum();
        let beading = Beading {
            total_thickness: thickness,
            bead_widths,
            toolpath_locations,
            left_over: thickness - sum,
        };
        assert_beading_invariant(&beading);
        beading
    }

    fn optimal_bead_count(&self, thickness: f64) -> usize {
        // Ported from OrcaSlicer RedistributeBeadingStrategy.cpp:73-85.
        if thickness < self.minimum_variable_line_ratio * self.optimal_width_outer {
            0
        } else if thickness <= 2.0 * self.optimal_width_outer {
            if thickness
                > (1.0 + self.parent.get_split_middle_threshold()) * self.optimal_width_outer
            {
                2
            } else {
                1
            }
        } else {
            self.parent
                .optimal_bead_count(thickness - 2.0 * self.optimal_width_outer)
                + 2
        }
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        // Ported from OrcaSlicer RedistributeBeadingStrategy.cpp:54-59.
        // The split-middle threshold is consulted in the `1` branch (NOT the
        // `0` branch): `case 0` yields `minimum_variable_line_ratio * W`.
        match lower_bead_count {
            0 => self.minimum_variable_line_ratio * self.optimal_width_outer,
            1 => (1.0 + self.parent.get_split_middle_threshold()) * self.optimal_width_outer,
            _ => {
                self.parent.get_transition_thickness(lower_bead_count - 2)
                    + 2.0 * self.optimal_width_outer
            }
        }
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        // Ported from OrcaSlicer RedistributeBeadingStrategy.cpp:50-52.
        let inner = (bead_count as i64 - 2).max(0) as usize;
        let outer = bead_count - inner;
        self.parent.optimal_thickness(inner) + self.optimal_width_outer * outer as f64
    }

    fn type_label(&self) -> &'static str {
        "Redistribute"
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
