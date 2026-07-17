// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp (+ SeamPlacer.hpp)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Seam candidate scoring and comparison, ported from canonical `SeamComparator`
//! (`SeamPlacer.cpp` / `SeamPlacer.hpp`).
//!
//! UNIT NOTE: this module's seam data path is f32 **millimetres** (canonical
//! `SeamPlacer` also works in unscaled mm here), angles in **radians**. This is
//! NOT the integer 100 nm coordinate system used elsewhere in the workspace
//! (see `docs/08_coordinate_system.md`); no unit scaling is applied to ported
//! constants.

/// Penalty-difference tolerance used by `is_first_not_much_worse`.
/// Canonical `SeamPlacer::seam_align_score_tolerance` (`SeamPlacer.hpp`).
/// Units: dimensionless penalty units.
const SEAM_ALIGN_SCORE_TOLERANCE: f32 = 0.3;

/// Angle-penalty weight for the Nearest setup.
/// Canonical `SeamPlacer::angle_importance_nearest` (`SeamPlacer.hpp`).
/// Units: dimensionless weight.
const ANGLE_IMPORTANCE_NEAREST: f32 = 1.0;

/// Angle-penalty weight for all non-Nearest setups.
/// Canonical `SeamPlacer::angle_importance_aligned` (`SeamPlacer.hpp`).
/// Units: dimensionless weight.
const ANGLE_IMPORTANCE_ALIGNED: f32 = 0.6;

/// A point deeper than this inside the model counts as "hidden".
/// Canonical `SeamComparator` embedded-distance gate (`SeamPlacer.cpp`).
/// Units: mm (compared against `embedded_distance`, which is negative inside).
const EMBEDDED_HIDDEN_THRESHOLD_MM: f32 = -0.5;

/// Overhang tolerance fraction of flow width in `is_first_not_much_worse`.
/// Canonical `SeamComparator::is_first_not_much_worse` (`SeamPlacer.cpp`).
/// Units: dimensionless fraction (multiplies `flow_width` in mm).
const OVERHANG_TOLERANCE_FLOW_FRACTION: f32 = 0.1;

/// Falloff speed of the distance-penalty gaussian for the Nearest setup.
/// Canonical `SeamComparator::compute_secondary_penalty` distance term
/// (`SeamPlacer.cpp`). Units: 1/mm^2 (applied to squared distance in mm).
const NEAREST_DISTANCE_GAUSS_FALLOFF: f32 = 0.005;

/// Enforced / blocked classification of a seam candidate.
/// Canonical `EnforcedBlockedSeamPoint` (`SeamPlacer.hpp`).
/// Ordering matters: higher discriminant wins in comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnforcedBlockedSeamPoint {
    /// Seam blocked here by the user.
    Blocked = 0,
    /// No user preference.
    Neutral = 1,
    /// Seam enforced here by the user.
    Enforced = 2,
}

/// Per-loop perimeter bookkeeping for seam picking.
/// Canonical `Perimeter` (`SeamPlacer.hpp`).
#[derive(Debug, Clone)]
pub(crate) struct Perimeter {
    /// Index of the first candidate of this loop in the layer candidate list.
    pub start_index: usize,
    /// One-past-last candidate index of this loop in the layer candidate list.
    pub end_index: usize,
    /// Index of the chosen seam candidate for this loop.
    pub seam_index: usize,
    /// True once the seam position has been finalized (e.g. by alignment).
    pub finalized: bool,
    /// Finalized seam position, valid when `finalized`. Units: mm.
    pub final_seam_position: [f32; 3],
}

/// One scored seam candidate point on a perimeter loop.
/// Canonical `SeamCandidate` (`SeamPlacer.hpp`).
///
/// Idiomatic deviation from canonical: canonical holds a shared reference to
/// its `Perimeter`; here the only perimeter field the comparator reads
/// (`flow_width`, mm) is denormalized onto the candidate, and `Perimeter`
/// bookkeeping lives beside the candidate list.
#[derive(Debug, Clone)]
pub(crate) struct SeamCandidate {
    /// Candidate position. Units: mm.
    pub position: [f32; 3],
    /// Precomputed visibility score (higher = more visible = worse).
    /// Units: dimensionless penalty units.
    pub visibility: f32,
    /// Overhang amount at this point (0 = fully supported). Units: penalty
    /// units proportional to mm of unsupported distance.
    pub overhang: f32,
    /// Distance to the nearest supported point below. Units: mm.
    // Read only by visibility.rs unit tests asserting the canonical formula.
    #[allow(dead_code)]
    pub unsupported_dist: f32,
    /// Signed distance from the model surface; negative = inside. Units: mm.
    pub embedded_distance: f32,
    /// Local counter-clockwise angle at this point; negative = concave.
    /// Units: radians.
    pub local_ccw_angle: f32,
    /// True for points inside the central region of an enforcer blob.
    pub central_enforcer: bool,
    /// User enforcement classification.
    pub point_type: EnforcedBlockedSeamPoint,
    /// Flow width of the owning perimeter loop (see struct docs). Units: mm.
    pub flow_width: f32,
}

/// Seam position setup, mirroring canonical `spNearest` / `spRear` /
/// `spRandom` / `spAligned` / `spAlignedBack` (`SeamPlacer.hpp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeamSetup {
    /// Prefer proximity to a preferred location.
    Nearest,
    /// Prefer the rear of the bed (max Y).
    Rear,
    /// Deterministic pseudo-random pick along the loop.
    Random,
    /// Vertically aligned seams.
    Aligned,
    /// Vertically aligned seams biased to the rear.
    AlignedBack,
}

/// Gaussian-like falloff used by seam scoring.
/// Canonical `gauss` (`SeamPlacer.cpp`):
/// `mean_value * (exp(1/(falloff_speed*(value-mean_x)^2+1)) - 1) / (e - 1)`.
pub(crate) fn gauss(value: f32, mean_x: f32, mean_value: f32, falloff_speed: f32) -> f32 {
    let shifted = value - mean_x;
    mean_value * ((1.0 / (falloff_speed * shifted * shifted + 1.0)).exp() - 1.0)
        / (std::f32::consts::E - 1.0)
}

/// Angle penalty: concave (negative) angles score lower than convex ones.
/// Canonical `compute_angle_penalty` (`SeamPlacer.cpp`).
/// Input: radians; output: dimensionless penalty units.
pub(crate) fn compute_angle_penalty(ccw_angle: f32) -> f32 {
    gauss(ccw_angle, 0.0, 1.0, 3.0) + 1.0 / (2.0 + (-ccw_angle).exp())
}

/// Comparator over `SeamCandidate`s for a given seam setup.
/// Canonical `SeamComparator` (`SeamPlacer.cpp`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SeamComparator {
    /// Active seam position setup.
    pub setup: SeamSetup,
}

impl SeamComparator {
    /// Create a comparator for the given setup.
    pub fn new(setup: SeamSetup) -> Self {
        Self { setup }
    }

    fn angle_importance(&self) -> f32 {
        // Canonical `SeamComparator` constructor: Nearest uses
        // `angle_importance_nearest`, everything else `angle_importance_aligned`.
        if self.setup == SeamSetup::Nearest {
            ANGLE_IMPORTANCE_NEAREST // dimensionless
        } else {
            ANGLE_IMPORTANCE_ALIGNED // dimensionless
        }
    }

    /// Base penalty of a candidate (lower = better); exposed crate-wide so
    /// the planner can report per-candidate scores in `SeamPlanEntry`.
    pub(crate) fn base_penalty(&self, c: &SeamCandidate) -> f32 {
        // Canonical penalty: overhang + visibility + angle_importance * angle penalty.
        c.overhang
            + c.visibility
            + self.angle_importance() * compute_angle_penalty(c.local_ccw_angle)
    }

    /// Distance penalty toward a preferred XY location (Nearest only).
    /// `1 - gauss(dist_mm, 0, 1, 0.005)`; dist in mm.
    fn distance_penalty(c: &SeamCandidate, preferred: [f32; 2]) -> f32 {
        let dx = c.position[0] - preferred[0]; // mm
        let dy = c.position[1] - preferred[1]; // mm
        let dist = (dx * dx + dy * dy).sqrt(); // mm
        1.0 - gauss(dist, 0.0, 1.0, NEAREST_DISTANCE_GAUSS_FALLOFF)
    }

    /// True if `a` is a strictly better seam candidate than `b`.
    /// Canonical `SeamComparator::is_first_better` (`SeamPlacer.cpp`).
    pub fn is_first_better(
        &self,
        a: &SeamCandidate,
        b: &SeamCandidate,
        preferred_location: Option<[f32; 2]>,
    ) -> bool {
        // (1) Aligned setups prefer central enforcer points.
        if matches!(self.setup, SeamSetup::Aligned | SeamSetup::AlignedBack)
            && a.central_enforcer != b.central_enforcer
        {
            return a.central_enforcer;
        }

        // (2) Higher enforcement type wins.
        if a.point_type != b.point_type {
            return a.point_type > b.point_type;
        }

        // (3) Lower overhang wins whenever either overhangs.
        if (a.overhang > 0.0 || b.overhang > 0.0) && a.overhang != b.overhang {
            return a.overhang < b.overhang;
        }

        // (4) Prefer hidden points (embedded more than 0.5 mm inside).
        let a_hidden = a.embedded_distance < EMBEDDED_HIDDEN_THRESHOLD_MM;
        let b_hidden = b.embedded_distance < EMBEDDED_HIDDEN_THRESHOLD_MM;
        if a_hidden != b_hidden {
            return a_hidden;
        }

        // (5) Rear setup prefers higher Y.
        if self.setup == SeamSetup::Rear {
            return a.position[1] > b.position[1];
        }

        // (6) Lower penalty wins.
        let mut penalty_a = self.base_penalty(a);
        let mut penalty_b = self.base_penalty(b);
        if self.setup == SeamSetup::Nearest {
            if let Some(preferred) = preferred_location {
                penalty_a += Self::distance_penalty(a, preferred);
                penalty_b += Self::distance_penalty(b, preferred);
            }
        }
        penalty_a < penalty_b
    }

    /// True if `a` is at least almost as good as `b`.
    /// Canonical `SeamComparator::is_first_not_much_worse` (`SeamPlacer.cpp`).
    pub fn is_first_not_much_worse(&self, a: &SeamCandidate, b: &SeamCandidate) -> bool {
        // Aligned setups prefer central enforcer points.
        if matches!(self.setup, SeamSetup::Aligned | SeamSetup::AlignedBack)
            && a.central_enforcer != b.central_enforcer
        {
            return a.central_enforcer;
        }

        // Enforcement gates.
        if a.point_type == EnforcedBlockedSeamPoint::Enforced {
            return true;
        }
        if a.point_type == EnforcedBlockedSeamPoint::Blocked {
            return false;
        }
        if a.point_type != b.point_type {
            return a.point_type > b.point_type;
        }

        // Overhang gate with flow-width-relative tolerance (mm).
        if (a.overhang > 0.0 || b.overhang > 0.0) && a.overhang != b.overhang {
            return a.overhang < b.overhang + OVERHANG_TOLERANCE_FLOW_FRACTION * a.flow_width;
        }

        // Embedded gate at -0.5 mm.
        let a_hidden = a.embedded_distance < EMBEDDED_HIDDEN_THRESHOLD_MM;
        let b_hidden = b.embedded_distance < EMBEDDED_HIDDEN_THRESHOLD_MM;
        if a_hidden != b_hidden {
            return a_hidden;
        }

        // Random setup: everything past the gates is viable.
        if self.setup == SeamSetup::Random {
            return true;
        }

        // Rear setup: tolerant Y comparison (mm on both sides; tolerance is
        // canonical `seam_align_score_tolerance * 5.0`).
        if self.setup == SeamSetup::Rear {
            return a.position[1] + SEAM_ALIGN_SCORE_TOLERANCE * 5.0 > b.position[1];
        }

        // Penalty comparison with tolerance (penalty units).
        self.base_penalty(a) - self.base_penalty(b) < SEAM_ALIGN_SCORE_TOLERANCE
    }
}

/// Pick the best seam candidate index in `perimeter_range` via
/// `is_first_better`. Canonical `pick_seam_point` (`SeamPlacer.cpp`), except
/// the chosen index is returned rather than stored on the perimeter.
pub(crate) fn pick_seam_point(
    candidates: &[SeamCandidate],
    perimeter_range: std::ops::Range<usize>,
    comparator: &SeamComparator,
) -> usize {
    let mut best = perimeter_range.start;
    for i in perimeter_range {
        if comparator.is_first_better(&candidates[i], &candidates[best], None) {
            best = i;
        }
    }
    best
}

/// Pick the candidate index nearest-best w.r.t. `preferred_location` (mm)
/// using a Nearest-setup comparator.
/// Canonical `pick_nearest_seam_point_index` (`SeamPlacer.cpp`).
// Canonical port exercised only by this file's unit tests; not wired into
// `run_seam_planning` (nearest mode is handled per-layer by `seam-placer`).
#[allow(dead_code)]
pub(crate) fn pick_nearest_seam_point_index(
    candidates: &[SeamCandidate],
    perimeter_range: std::ops::Range<usize>,
    preferred_location: [f32; 2],
) -> usize {
    let comparator = SeamComparator::new(SeamSetup::Nearest);
    let mut best = perimeter_range.start;
    for i in perimeter_range {
        if comparator.is_first_better(&candidates[i], &candidates[best], Some(preferred_location)) {
            best = i;
        }
    }
    best
}

/// Stateless hash-based RNG in [0, 1) seeded from a position (mm).
/// Canonical hash used by `pick_random_seam_point` (`SeamPlacer.cpp`):
/// `frac(|sin(dot(pos, (12.9898, 78.233, 133.3333))) * 43758.5453|)`.
// The canonical hash literals are kept verbatim; f32 rounds them identically.
// Reachable only from `pick_random_seam_point`, which is test-only (see below).
#[allow(dead_code)]
#[allow(clippy::excessive_precision)]
fn position_hash_rand(pos: [f32; 3]) -> f32 {
    // Constants are dimensionless hash coefficients (not lengths).
    let dot = pos[0] * 12.9898 + pos[1] * 78.233 + pos[2] * 133.3333;
    let v = (dot.sin() * 43758.5453).abs();
    v - v.floor()
}

/// Deterministically pick a pseudo-random seam point along the loop.
/// Canonical `pick_random_seam_point` (`SeamPlacer.cpp`).
///
/// Builds the list of viable edges (candidates where both-way
/// `is_first_not_much_worse` holds against the current best example,
/// restarting the list whenever a strictly better point appears), then does an
/// edge-length-weighted selection with the stateless position-hash RNG and
/// interpolates along the chosen edge.
///
/// Returns `(chosen_candidate_index, interpolated_position_mm)`.
// Canonical port exercised only by this file's unit tests; not wired into
// `run_seam_planning` (random mode is handled per-layer by `seam-placer`).
#[allow(dead_code)]
pub(crate) fn pick_random_seam_point(
    candidates: &[SeamCandidate],
    perimeter_range: std::ops::Range<usize>,
) -> (usize, [f32; 3]) {
    struct Viable {
        index: usize,
        edge_length: f32, // mm
        edge: [f32; 3],   // mm
    }

    let comparator = SeamComparator::new(SeamSetup::Random);
    let start = perimeter_range.start;
    let end = perimeter_range.end;
    let mut viable_example_index = start;
    let mut viables: Vec<Viable> = Vec::new();

    let edge_of = |i: usize| -> ([f32; 3], f32) {
        let next = if i + 1 >= end { start } else { i + 1 };
        let a = candidates[i].position;
        let b = candidates[next].position;
        let edge = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]; // mm
        let len = (edge[0] * edge[0] + edge[1] * edge[1] + edge[2] * edge[2]).sqrt(); // mm
        (edge, len)
    };

    for i in start..end {
        if comparator.is_first_not_much_worse(&candidates[i], &candidates[viable_example_index])
            && comparator.is_first_not_much_worse(&candidates[viable_example_index], &candidates[i])
        {
            // Comparable to the current best: viable.
            let (edge, edge_length) = edge_of(i);
            viables.push(Viable {
                index: i,
                edge_length,
                edge,
            });
        } else if comparator.is_first_better(
            &candidates[i],
            &candidates[viable_example_index],
            None,
        ) {
            // Strictly better point found: restart the viable list.
            viable_example_index = i;
            viables.clear();
            let (edge, edge_length) = edge_of(i);
            viables.push(Viable {
                index: i,
                edge_length,
                edge,
            });
        }
        // else: strictly worse, skip.
    }

    if viables.is_empty() {
        // Degenerate range; fall back to the example point.
        return (
            viable_example_index,
            candidates[viable_example_index].position,
        );
    }

    let len_sum: f32 = viables.iter().map(|v| v.edge_length).sum(); // mm
    let rand = position_hash_rand(candidates[start].position);
    let mut picked_len = len_sum * rand; // mm

    let mut chosen = viables.len() - 1;
    for (vi, viable) in viables.iter().enumerate() {
        if picked_len < viable.edge_length {
            chosen = vi;
            break;
        }
        picked_len -= viable.edge_length;
    }

    let viable = &viables[chosen];
    let t = if viable.edge_length > 0.0 {
        picked_len / viable.edge_length
    } else {
        0.0
    };
    let p = candidates[viable.index].position;
    (
        viable.index,
        [
            p[0] + viable.edge[0] * t,
            p[1] + viable.edge[1] * t,
            p[2] + viable.edge[2] * t,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(position: [f32; 3]) -> SeamCandidate {
        SeamCandidate {
            position,
            visibility: 0.0,
            overhang: 0.0,
            unsupported_dist: 0.0,
            embedded_distance: 0.0,
            local_ccw_angle: 0.0,
            central_enforcer: false,
            point_type: EnforcedBlockedSeamPoint::Neutral,
            flow_width: 0.4, // mm
        }
    }

    #[test]
    fn concave_angle_scores_lower_penalty_than_convex() {
        // Concave = negative ccw angle; must be preferred (lower penalty).
        assert!(compute_angle_penalty(-1.0) < compute_angle_penalty(1.0));
        assert!(compute_angle_penalty(-0.3) < compute_angle_penalty(0.3));
        assert!(compute_angle_penalty(-2.5) < compute_angle_penalty(-0.1));
    }

    #[test]
    fn rear_prefers_max_y() {
        let comparator = SeamComparator::new(SeamSetup::Rear);
        let front = candidate([0.0, 1.0, 0.0]);
        let rear = candidate([0.0, 50.0, 0.0]);
        assert!(comparator.is_first_better(&rear, &front, None));
        assert!(!comparator.is_first_better(&front, &rear, None));

        let best = pick_seam_point(
            &[front.clone(), rear.clone(), candidate([0.0, 10.0, 0.0])],
            0..3,
            &comparator,
        );
        assert_eq!(best, 1);
    }

    #[test]
    fn aligned_setups_prefer_central_enforcer() {
        for setup in [SeamSetup::Aligned, SeamSetup::AlignedBack] {
            let comparator = SeamComparator::new(setup);
            let mut enforcer = candidate([0.0, 0.0, 0.0]);
            enforcer.central_enforcer = true;
            // Give the plain candidate an otherwise winning profile.
            let mut plain = candidate([0.0, 100.0, 0.0]);
            plain.local_ccw_angle = -2.0; // strongly concave (rad)
            assert!(comparator.is_first_better(&enforcer, &plain, None));
            assert!(!comparator.is_first_better(&plain, &enforcer, None));
            assert!(comparator.is_first_not_much_worse(&enforcer, &plain));
            assert!(!comparator.is_first_not_much_worse(&plain, &enforcer));
        }
        // Nearest must NOT apply the enforcer preference branch.
        let comparator = SeamComparator::new(SeamSetup::Nearest);
        let mut enforcer = candidate([0.0, 0.0, 0.0]);
        enforcer.central_enforcer = true;
        enforcer.local_ccw_angle = 2.0; // convex: high penalty
        let mut plain = candidate([0.0, 0.0, 0.0]);
        plain.local_ccw_angle = -2.0; // concave: low penalty
        assert!(comparator.is_first_better(&plain, &enforcer, None));
    }

    #[test]
    fn overhang_gate_prefers_lower_overhang() {
        let comparator = SeamComparator::new(SeamSetup::Nearest);
        let mut hanging = candidate([0.0, 0.0, 0.0]);
        hanging.overhang = 1.5;
        hanging.local_ccw_angle = -2.0; // otherwise excellent
        let supported = candidate([0.0, 0.0, 0.0]);
        assert!(comparator.is_first_better(&supported, &hanging, None));
        assert!(!comparator.is_first_better(&hanging, &supported, None));
        // Not-much-worse: big overhang gap exceeds 0.1 * flow_width tolerance.
        assert!(!comparator.is_first_not_much_worse(&hanging, &supported));
        assert!(comparator.is_first_not_much_worse(&supported, &hanging));
    }

    #[test]
    fn embedded_gate_prefers_hidden_points() {
        let comparator = SeamComparator::new(SeamSetup::Nearest);
        let mut hidden = candidate([0.0, 0.0, 0.0]);
        hidden.embedded_distance = -1.0; // mm, > 0.5 mm inside
        hidden.local_ccw_angle = 2.0; // otherwise terrible
        let mut exposed = candidate([0.0, 0.0, 0.0]);
        exposed.embedded_distance = 0.0;
        exposed.local_ccw_angle = -2.0; // otherwise excellent
        assert!(comparator.is_first_better(&hidden, &exposed, None));
        assert!(!comparator.is_first_better(&exposed, &hidden, None));
        assert!(comparator.is_first_not_much_worse(&hidden, &exposed));
        assert!(!comparator.is_first_not_much_worse(&exposed, &hidden));
    }

    #[test]
    fn enforced_type_beats_neutral_and_blocked_loses() {
        let comparator = SeamComparator::new(SeamSetup::Nearest);
        let mut enforced = candidate([0.0, 0.0, 0.0]);
        enforced.point_type = EnforcedBlockedSeamPoint::Enforced;
        enforced.local_ccw_angle = 2.0;
        let mut blocked = candidate([0.0, 0.0, 0.0]);
        blocked.point_type = EnforcedBlockedSeamPoint::Blocked;
        blocked.local_ccw_angle = -2.0;
        let neutral = candidate([0.0, 0.0, 0.0]);
        assert!(comparator.is_first_better(&enforced, &neutral, None));
        assert!(comparator.is_first_better(&neutral, &blocked, None));
        assert!(comparator.is_first_not_much_worse(&enforced, &neutral));
        assert!(!comparator.is_first_not_much_worse(&blocked, &neutral));
    }

    #[test]
    fn nearest_distance_penalty_pulls_toward_preferred_location() {
        let near = candidate([1.0, 0.0, 0.0]);
        let far = candidate([80.0, 0.0, 0.0]);
        let idx = pick_nearest_seam_point_index(&[far, near], 0..2, [0.0, 0.0]);
        assert_eq!(idx, 1);
    }

    #[test]
    fn random_pick_is_deterministic() {
        // A square loop of equally-good candidates (mm).
        let candidates: Vec<SeamCandidate> = vec![
            candidate([0.0, 0.0, 0.3]),
            candidate([10.0, 0.0, 0.3]),
            candidate([10.0, 10.0, 0.3]),
            candidate([0.0, 10.0, 0.3]),
        ];
        let (i1, p1) = pick_random_seam_point(&candidates, 0..4);
        let (i2, p2) = pick_random_seam_point(&candidates, 0..4);
        assert_eq!(i1, i2);
        assert_eq!(p1, p2);
        // The interpolated point must sit within the loop's bounding box.
        assert!(p1[0] >= 0.0 && p1[0] <= 10.0);
        assert!(p1[1] >= 0.0 && p1[1] <= 10.0);
    }

    #[test]
    fn random_pick_restarts_viables_on_strictly_better_point() {
        // One blocked point, one enforced point: enforced must win the pick.
        let mut blocked = candidate([0.0, 0.0, 0.0]);
        blocked.point_type = EnforcedBlockedSeamPoint::Blocked;
        let mut enforced = candidate([5.0, 0.0, 0.0]);
        enforced.point_type = EnforcedBlockedSeamPoint::Enforced;
        let plain = candidate([10.0, 0.0, 0.0]);
        let (idx, _) = pick_random_seam_point(&[blocked, enforced, plain], 0..3);
        assert_eq!(idx, 1);
    }
}
