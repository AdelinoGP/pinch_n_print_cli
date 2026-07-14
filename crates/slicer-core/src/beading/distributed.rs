// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `DistributedBeadingStrategy`: the base decorator-chain strategy.
//!
//! For `bead_count > 2` beads, the surplus/deficit thickness versus
//! `bead_count * optimal_width` (`to_be_divided`) is redistributed across the
//! beads using a Gaussian-shaped weight centered on the middle bead index:
//! beads near the middle absorb most of the surplus/deficit, while beads far
//! from the middle (beyond `distribution_count` bead-widths away) are clipped
//! to zero weight and stay at exactly `optimal_width`. `bead_count` of 0, 1,
//! and 2 are handled as special cases (no beads / one full-thickness bead /
//! two equal half-thickness beads).
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use super::{Beading, BeadingStrategy};

/// Distributes a wall region's thickness into beads using OrcaSlicer's
/// Gaussian-weighted redistribution scheme. This is the base of the Arachne
/// beading-strategy decorator chain; later steps of this packet wrap it (or
/// another `BeadingStrategy`) to add redistribution, widening, outer-wall
/// inset, and bead-count limiting behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributedBeadingStrategy {
    /// The width (in slicer units) a bead is given when there is no surplus
    /// or deficit thickness to redistribute.
    optimal_width: f64,
    /// Reserved for later decorator steps of this packet (e.g. transition
    /// ramp length calculations); not read by `compute` itself, which is why
    /// OrcaSlicer's own `DistributedBeadingStrategy::compute` never
    /// references it either — it is only consumed by the base-class
    /// transitioning-length logic, which this trait does not expose.
    default_transition_length: f64,
    /// Reserved for later decorator steps of this packet; see
    /// `default_transition_length` for why it is unused here.
    transition_filter_dist: f64,
    /// The Gaussian decay radius (in bead-count units), i.e. OrcaSlicer's
    /// `distribution_radius`. Beads more than roughly this many indices away
    /// from the middle bead get zero weight and stay at `optimal_width`.
    distribution_count: usize,
    /// Wall-transition angle (radians) exposed by the beading strategy. This
    /// value is not used by `compute` itself, but it is read by callers that
    /// need to know the configured transition-angle threshold.
    wall_transition_angle: f64,
    /// Threshold (fraction of `optimal_width`) above which a middle bead may be
    /// split into two beads during bead-count transitions when the current
    /// bead count is odd. Matches OrcaSlicer's `getSplitMiddleThreshold()`.
    wall_split_middle_threshold: f64,
    /// Threshold (fraction of `optimal_width`) below which a middle bead is
    /// added during bead-count transitions when the current bead count is even.
    /// Matches OrcaSlicer's `getAddMiddleThreshold()`.
    wall_add_middle_threshold: f64,
}

impl DistributedBeadingStrategy {
    /// Creates a new `DistributedBeadingStrategy`. All arguments are in
    /// slicer units (1 unit = 100 nm) except `distribution_count`, which is
    /// a plain bead-count radius.
    pub fn new(
        optimal_width: f64,
        default_transition_length: f64,
        transition_filter_dist: f64,
        distribution_count: usize,
        wall_transition_angle: f64,
        wall_split_middle_threshold: f64,
        wall_add_middle_threshold: f64,
    ) -> Self {
        Self {
            optimal_width,
            default_transition_length,
            transition_filter_dist,
            distribution_count,
            wall_transition_angle,
            wall_split_middle_threshold,
            wall_add_middle_threshold,
        }
    }
}

/// Checks the `Beading` invariant `toolpath_locations.len() ==
/// bead_widths.len()`.
///
/// `compute` calls this on every return path, so the check is genuinely part
/// of `compute`'s execution. It is factored into a standalone `pub` function
/// (rather than inlined `debug_assert_eq!` calls scattered across branches)
/// for two reasons: a single source of truth for the message, and
/// testability — a *correct* `compute` implementation can never actually
/// produce mismatched lengths, so the only way to prove the assertion fires
/// is to call this function directly against a manually malformed `Beading`.
pub fn assert_beading_invariant(beading: &Beading) {
    debug_assert_eq!(
        beading.toolpath_locations.len(),
        beading.bead_widths.len(),
        "Beading invariant violated: toolpath_locations.len() ({}) != bead_widths.len() ({})",
        beading.toolpath_locations.len(),
        beading.bead_widths.len()
    );
}

impl BeadingStrategy for DistributedBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        let beading = match bead_count {
            0 => Beading {
                total_thickness: thickness,
                bead_widths: Vec::new(),
                toolpath_locations: Vec::new(),
                left_over: thickness,
            },
            1 => Beading {
                total_thickness: thickness,
                bead_widths: vec![thickness],
                toolpath_locations: vec![thickness / 2.0],
                left_over: 0.0,
            },
            2 => {
                let outer_width = thickness / 2.0;
                Beading {
                    total_thickness: thickness,
                    bead_widths: vec![outer_width, outer_width],
                    toolpath_locations: vec![outer_width / 2.0, thickness - outer_width / 2.0],
                    left_over: 0.0,
                }
            }
            _ => {
                let to_be_divided = thickness - bead_count as f64 * self.optimal_width;
                let radius = self.distribution_count as f64;
                let one_over_r_sq = if radius >= 2.0 {
                    let inv = 1.0 / (radius - 1.0);
                    inv * inv
                } else {
                    1.0
                };
                let middle = (bead_count - 1) as f64 / 2.0;

                let weights: Vec<f64> = (0..bead_count)
                    .map(|i| {
                        let dev_from_middle = i as f64 - middle;
                        (1.0 - one_over_r_sq * dev_from_middle * dev_from_middle).max(0.0)
                    })
                    .collect();
                let total_weight: f64 = weights.iter().sum();

                let mut bead_widths = Vec::with_capacity(bead_count);
                let mut accumulated = 0.0;
                for (i, &weight) in weights.iter().enumerate() {
                    let width = if i == bead_count - 1 {
                        // The last bead absorbs whatever remains, so the sum
                        // of bead_widths always equals thickness exactly
                        // (modulo floating-point noise well under 1e-4
                        // units), regardless of Gaussian-weight rounding.
                        thickness - accumulated
                    } else {
                        self.optimal_width + to_be_divided * (weight / total_weight)
                    };
                    accumulated += width;
                    bead_widths.push(width);
                }

                let mut toolpath_locations = Vec::with_capacity(bead_count);
                let mut loc = bead_widths[0] / 2.0;
                toolpath_locations.push(loc);
                for i in 1..bead_count {
                    loc += (bead_widths[i - 1] + bead_widths[i]) / 2.0;
                    toolpath_locations.push(loc);
                }

                Beading {
                    total_thickness: thickness,
                    bead_widths,
                    toolpath_locations,
                    left_over: 0.0,
                }
            }
        };

        assert_beading_invariant(&beading);
        beading
    }

    fn optimal_bead_count(&self, thickness: f64) -> usize {
        // Ported from OrcaSlicer DistributedBeadingStrategy.cpp:132-144.
        // Integer truncation (not rounding) of thickness / optimal_width, then
        // a remainder test against a parity-selected minimum line width.
        let naive_count = (thickness / self.optimal_width).trunc();
        let remainder = thickness - naive_count * self.optimal_width;
        let minimum_line_width = self.optimal_width
            * if naive_count as usize % 2 == 1 {
                self.wall_split_middle_threshold
            } else {
                self.wall_add_middle_threshold
            };
        (naive_count as usize) + (remainder >= minimum_line_width) as usize
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        // Ported from OrcaSlicer BeadingStrategy.cpp:90-102.
        let threshold = if lower_bead_count % 2 == 1 {
            self.wall_split_middle_threshold
        } else {
            self.wall_add_middle_threshold
        };
        let lower_optimum = self.optimal_thickness(lower_bead_count);
        let upper_optimum = self.optimal_thickness(lower_bead_count + 1);
        lower_optimum + (upper_optimum - lower_optimum) * threshold
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        bead_count as f64 * self.optimal_width
    }

    fn type_label(&self) -> &'static str {
        "Distributed"
    }

    fn get_split_middle_threshold(&self) -> f64 {
        self.wall_split_middle_threshold
    }

    fn get_add_middle_threshold(&self) -> f64 {
        self.wall_add_middle_threshold
    }

    fn wall_transition_angle(&self) -> f64 {
        self.wall_transition_angle
    }

    fn get_transitioning_length(&self, lower_bead_count: usize) -> f64 {
        if lower_bead_count == 0 {
            // Canonical BeadingStrategy.cpp:54: `scaled<coord_t>(0.01)` = 10µm =
            // 10,000 Orca units. Per docs/08_coordinate_system.md porting rule
            // (PNP_units = Orca_units / 100): 10,000 / 100 = 100 PNP units.
            100.0
        } else {
            self.default_transition_length
        }
    }

    fn get_transition_anchor_pos(&self, lower_bead_count: usize) -> f64 {
        let transition = self.get_transition_thickness(lower_bead_count);
        let lower_optimum = self.optimal_thickness(lower_bead_count);
        let upper_optimum = self.optimal_thickness(lower_bead_count + 1);
        let denominator = upper_optimum - lower_optimum;
        if denominator <= 0.0 {
            return 0.5;
        }
        1.0 - (transition - lower_optimum) / denominator
    }

    fn get_nonlinear_thicknesses(&self, lower_bead_count: usize) -> Vec<f64> {
        let _ = lower_bead_count;
        Vec::new()
    }

    fn get_transition_filter_dist(&self, _lower_bead_count: usize) -> f64 {
        self.transition_filter_dist
    }
}
