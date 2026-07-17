// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
// Original C++ source path: src/libslic3r/Geometry/Curves.hpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Seam alignment across layers, ported from canonical
//! `find_next_seam_in_layer`, `find_seam_string`, and `align_seam_points`
//! (`SeamPlacer.cpp`), plus the weighted cubic B-spline least-squares fit from
//! canonical `fit_curve` with `CubicBSplineKernel` (`Curves.hpp`).
//!
//! UNIT NOTE: this module's seam data path is f32 **millimetres**, angles in
//! **radians** — matching canonical `SeamPlacer`'s unscaled-mm domain, NOT the
//! integer 100 nm system used elsewhere (see `docs/08_coordinate_system.md`).
//!
//! Numerical deviation from canonical: the per-dimension least-squares solve
//! uses normal equations (AᵀA c = Aᵀb) with Gaussian elimination and partial
//! pivoting instead of Eigen's `fullPivHouseholderQr`. The parameter count is
//! tiny, so conditioning is acceptable; results may differ from canonical in
//! the last bits.
//!
//! Behavioral deviation from canonical: `SeamCandidate` here carries no
//! per-layer `layer_angle`, so the curling-influence branch (canonical
//! `align_seam_points`: `-0.8` when the layer angle dominates) always uses
//! `1.0`.

use crate::comparator::{
    compute_angle_penalty, EnforcedBlockedSeamPoint, Perimeter, SeamCandidate, SeamComparator,
    SeamSetup,
};

/// Search radius factor: multiplied by flow width (mm) to get the next-layer
/// search radius in mm.
/// Canonical `SeamPlacer::seam_align_tolerable_dist_factor` (`SeamPlacer.hpp`).
/// Units: dimensionless factor over `flow_width` (mm).
const SEAM_ALIGN_TOLERABLE_DIST_FACTOR: f32 = 4.0;

/// Minimum number of chained seams for a string to be finalized.
/// Canonical `SeamPlacer::seam_align_minimum_string_seams` (`SeamPlacer.hpp`).
/// Units: count.
const SEAM_ALIGN_MINIMUM_STRING_SEAMS: usize = 6;

/// Millimetres of (curl-weighted) string length per B-spline segment.
/// Canonical `SeamPlacer::seam_align_mm_per_segment` (`SeamPlacer.hpp`);
/// canonical truncates the segment count to an integer. Units: mm.
const SEAM_ALIGN_MM_PER_SEGMENT: f32 = 4.0;

/// Angles at least this sharp snap the seam to the observed corner rather
/// than the fitted curve.
/// Canonical `SeamPlacer::sharp_angle_snapping_threshold` (`SeamPlacer.hpp`):
/// 55 degrees. Units: radians.
const SHARP_ANGLE_SNAPPING_THRESHOLD: f32 = 55.0 * std::f32::consts::PI / 180.0;

/// Per-layer seam candidate set: candidate points plus perimeter bookkeeping.
/// Mirrors canonical `PrintObjectSeamData::LayerSeams` (`SeamPlacer.hpp`).
#[derive(Debug, Clone)]
pub(crate) struct LayerCandidates {
    /// All candidates of this layer. Positions in mm.
    pub candidates: Vec<SeamCandidate>,
    /// Perimeter loops indexing into `candidates`.
    pub perimeters: Vec<Perimeter>,
}

/// Reference to one seam choice: layer, owning perimeter, candidate index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SeamRef {
    /// Layer index into the layer list.
    pub layer: usize,
    /// Perimeter index within the layer.
    pub perimeter: usize,
    /// Candidate index within the layer's candidate list.
    pub candidate: usize,
}

fn candidate_of<'a>(layers: &'a [LayerCandidates], r: &SeamRef) -> &'a SeamCandidate {
    &layers[r.layer].candidates[r.candidate]
}

fn dist3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; // mm
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() // mm
}

/// Find the seam in `layer_idx` that continues the string ending at
/// `prev_seam`. Canonical `find_next_seam_in_layer` (`SeamPlacer.cpp`),
/// with a linear scan replacing the canonical KD-tree radius query.
pub(crate) fn find_next_seam_in_layer(
    layers: &[LayerCandidates],
    prev_seam: &SeamCandidate,
    layer_idx: usize,
    comparator: &SeamComparator,
) -> Option<SeamRef> {
    let layer = &layers[layer_idx];
    // Search radius in mm: canonical `max_distance`.
    let max_distance = SEAM_ALIGN_TOLERABLE_DIST_FACTOR * prev_seam.flow_width; // mm
    let max_distance_sq = max_distance * max_distance; // mm^2
    let enforcer_distance_sq = (3.0 * max_distance) * (3.0 * max_distance); // mm^2

    // Previous seam projected onto this layer: same xy, layer z (z drops out
    // of the xy distance below).
    let (px, py) = (prev_seam.position[0], prev_seam.position[1]); // mm

    let mut best_nearby: Option<SeamRef> = None;
    let mut nearest: Option<(SeamRef, f32)> = None;

    for (pi, perimeter) in layer.perimeters.iter().enumerate() {
        if perimeter.finalized {
            continue;
        }
        for ci in perimeter.start_index..perimeter.end_index {
            let c = &layer.candidates[ci];
            let dx = c.position[0] - px; // mm
            let dy = c.position[1] - py; // mm
            let dist_sq = dx * dx + dy * dy; // mm^2
            let r = SeamRef {
                layer: layer_idx,
                perimeter: pi,
                candidate: ci,
            };
            // A central enforcer close enough wins immediately.
            if c.central_enforcer && dist_sq < enforcer_distance_sq {
                return Some(r);
            }
            if dist_sq >= max_distance_sq {
                continue;
            }
            match &nearest {
                Some((_, best_sq)) if *best_sq <= dist_sq => {}
                _ => nearest = Some((r, dist_sq)),
            }
            match &best_nearby {
                Some(b) if !comparator.is_first_better(c, candidate_of(layers, b), None) => {}
                _ => best_nearby = Some(r),
            }
        }
    }

    if let Some((nearest_ref, _)) = nearest {
        if comparator.is_first_not_much_worse(candidate_of(layers, &nearest_ref), prev_seam) {
            return Some(nearest_ref);
        }
    }
    if let Some(best_ref) = best_nearby {
        if comparator.is_first_not_much_worse(candidate_of(layers, &best_ref), prev_seam) {
            return Some(best_ref);
        }
    }
    None
}

/// Build a vertical string of chained seams through `start`.
/// Canonical `find_seam_string` (`SeamPlacer.cpp`): walk up from
/// `start.layer + 1`, then reverse from `start` and walk down to layer 0,
/// stopping at the first failed step in each direction.
pub(crate) fn find_seam_string(
    layers: &[LayerCandidates],
    start: SeamRef,
    comparator: &SeamComparator,
) -> Vec<SeamRef> {
    let mut string = vec![start];

    // Walk up.
    let mut last = start;
    for layer_idx in (start.layer + 1)..layers.len() {
        match find_next_seam_in_layer(layers, candidate_of(layers, &last), layer_idx, comparator) {
            Some(next) => {
                string.push(next);
                last = next;
            }
            None => break,
        }
    }

    // Walk down from the start.
    let mut last = start;
    for layer_idx in (0..start.layer).rev() {
        match find_next_seam_in_layer(layers, candidate_of(layers, &last), layer_idx, comparator) {
            Some(next) => {
                string.push(next);
                last = next;
            }
            None => break,
        }
    }

    string
}

/// Align seams vertically across layers by fitting weighted cubic B-splines
/// to strings of chained seams. Canonical `align_seam_points`
/// (`SeamPlacer.cpp`), simple-retry variant: strings shorter than
/// `SEAM_ALIGN_MINIMUM_STRING_SEAMS` are skipped (their perimeters stay
/// unfinalized) rather than retried from alternative starts.
pub(crate) fn align_seam_points(layers: &mut [LayerCandidates], comparator: &SeamComparator) {
    // Gather one seam ref per perimeter across all layers.
    let mut refs: Vec<SeamRef> = Vec::new();
    for (li, layer) in layers.iter().enumerate() {
        for (pi, perimeter) in layer.perimeters.iter().enumerate() {
            refs.push(SeamRef {
                layer: li,
                perimeter: pi,
                candidate: perimeter.seam_index,
            });
        }
    }

    // Stable sort best-first via the comparator.
    let layers_ro: &[LayerCandidates] = layers;
    refs.sort_by(|a, b| {
        let ca = candidate_of(layers_ro, a);
        let cb = candidate_of(layers_ro, b);
        if comparator.is_first_better(ca, cb, None) {
            std::cmp::Ordering::Less
        } else if comparator.is_first_better(cb, ca, None) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    for start in refs {
        if layers[start.layer].perimeters[start.perimeter].finalized {
            continue;
        }
        let mut string = find_seam_string(layers, start, comparator);
        if string.len() < SEAM_ALIGN_MINIMUM_STRING_SEAMS {
            // Too short to be worth aligning; leave perimeters unfinalized.
            continue;
        }
        string.sort_by_key(|r| r.layer);

        // Build weighted observations.
        let n = string.len();
        let mut observations: Vec<[f32; 2]> = Vec::with_capacity(n); // xy, mm
        let mut observation_points: Vec<f32> = Vec::with_capacity(n); // z, mm
        let mut weights: Vec<f32> = Vec::with_capacity(n); // dimensionless
        let mut total_length: f32 = 0.0; // curl-weighted mm
        let mut last_position: Option<[f32; 3]> = None;
        for r in &string {
            let c = candidate_of(layers, r);
            observations.push([c.position[0], c.position[1]]);
            observation_points.push(c.position[2]);
            // Weight in 1/penalty-units; canonical adds 0.1 to avoid blowup.
            let mut weight = 1.0 / (0.1 + compute_angle_penalty(c.local_ccw_angle));
            let curling_influence = if c.point_type == EnforcedBlockedSeamPoint::Enforced {
                weight += 3.0;
                1.0
            } else {
                // Canonical: -0.8 when `layer_angle > 2*|local_ccw_angle|`;
                // our candidates carry no layer_angle, so always 1.0
                // (documented deviation, see module docs).
                1.0
            };
            weights.push(weight);
            if let Some(prev) = last_position {
                total_length += curling_influence * dist3d(prev, c.position); // mm
            }
            last_position = Some(c.position);
        }
        if comparator.setup == SeamSetup::Rear {
            total_length *= 0.3;
        }
        // Canonical truncates to an integer segment count, minimum 1.
        let segments = ((total_length.max(0.0) / SEAM_ALIGN_MM_PER_SEGMENT) as usize).max(1); // count

        let fit = fit_cubic_bspline(&observation_points, &observations, &weights, segments);

        // Blend each seam between its observed corner and the fitted curve,
        // then finalize.
        for r in &string {
            let c = candidate_of(layers, r);
            let z = c.position[2]; // mm
            let fitted = fit.get_fitted_value(z); // mm
                                                  // Sharpness blend factor, dimensionless in [0, 1].
            let mut t = (c.local_ccw_angle.abs() / SHARP_ANGLE_SNAPPING_THRESHOLD)
                .powi(3)
                .min(1.0);
            if c.point_type == EnforcedBlockedSeamPoint::Enforced {
                t = t.max(0.4);
            }
            let final_xy = [
                t * c.position[0] + (1.0 - t) * fitted[0], // mm
                t * c.position[1] + (1.0 - t) * fitted[1], // mm
            ];
            let perimeter = &mut layers[r.layer].perimeters[r.perimeter];
            perimeter.seam_index = r.candidate;
            perimeter.final_seam_position = [final_xy[0], final_xy[1], z];
            perimeter.finalized = true;
        }
    }
}

/// Partition-of-unity cubic B-spline kernel.
/// Canonical `CubicBSplineKernel` (`Curves.hpp`). Input: normalized distance
/// in segment units (dimensionless); output: dimensionless basis value.
fn bspline_kernel(x: f32) -> f32 {
    let x = x.abs();
    if x >= 2.0 {
        0.0
    } else if x <= 1.0 {
        4.0 / 6.0 - x * x + 0.5 * x * x * x
    } else {
        let x = x - 1.0;
        1.0 / 6.0 - 0.5 * x + 0.5 * x * x - x * x * x / 6.0
    }
}

/// Fitted cubic B-spline over a scalar parameter (z, mm) mapping to xy (mm).
#[derive(Debug, Clone)]
pub(crate) struct CubicBSplineFit {
    /// First observation parameter (mm).
    start: f32,
    /// Knot spacing (mm per segment).
    segment_size: f32,
    /// Control-point coefficients, one xy pair per parameter. Units: mm.
    coefficients: Vec<[f32; 2]>,
}

impl CubicBSplineFit {
    /// Evaluate the fitted curve at parameter `op` (mm) → xy (mm).
    /// Canonical `PolynomialCurve`-style `get_fitted_value` for the B-spline
    /// kernel (`Curves.hpp`): sum the same 4-segment clamped kernel window
    /// used to build the design matrix.
    pub fn get_fitted_value(&self, op: f32) -> [f32; 2] {
        let params = self.coefficients.len() as isize;
        let mid = ((op - self.start) / self.segment_size).floor() as isize;
        let mut result = [0.0f32; 2];
        for seg in (mid - 1)..(mid + 3) {
            // Normalized distance in segment units (dimensionless).
            let nd = (self.start + seg as f32 * self.segment_size - op) / self.segment_size;
            let p = seg.clamp(0, params - 1) as usize;
            let k = bspline_kernel(nd);
            result[0] += self.coefficients[p][0] * k;
            result[1] += self.coefficients[p][1] * k;
        }
        result
    }
}

/// Weighted least-squares cubic B-spline fit.
/// Canonical `fit_curve` with `CubicBSplineKernel` (`Curves.hpp`).
///
/// `observation_points` must be sorted ascending (z in mm); `observations`
/// are the xy values (mm); `weights` are dimensionless.
///
/// Numerical deviation: solved via normal equations + Gaussian elimination
/// with partial pivoting instead of canonical `fullPivHouseholderQr`.
pub(crate) fn fit_cubic_bspline(
    observation_points: &[f32],
    observations: &[[f32; 2]],
    weights: &[f32],
    segments: usize,
) -> CubicBSplineFit {
    assert_eq!(observation_points.len(), observations.len());
    assert_eq!(observation_points.len(), weights.len());
    let n = observation_points.len();
    let start = observation_points[0]; // mm
    let valid_length = observation_points[n - 1] - start; // mm

    if valid_length <= 0.0 {
        // Degenerate parameter span: constant fit at the weighted mean.
        let wsum: f32 = weights.iter().sum();
        let mut mean = [0.0f32; 2];
        for (obs, w) in observations.iter().zip(weights) {
            mean[0] += obs[0] * w;
            mean[1] += obs[1] * w;
        }
        if wsum > 0.0 {
            mean[0] /= wsum;
            mean[1] /= wsum;
        }
        return CubicBSplineFit {
            start,
            segment_size: 1.0, // mm; arbitrary, curve is constant
            coefficients: vec![mean; 2],
        };
    }

    let segment_size = valid_length / segments as f32; // mm
    let parameters_count = segments + 1;

    // Design matrix rows scaled by sqrt(weight) (canonical `fit_curve`).
    let mut design: Vec<Vec<f64>> = vec![vec![0.0; parameters_count]; n];
    let mut data: Vec<[f64; 2]> = vec![[0.0; 2]; n];
    for i in 0..n {
        let sqrt_w = (weights[i] as f64).sqrt();
        let op = observation_points[i];
        let mid = ((op - start) / segment_size).floor() as isize;
        for seg in (mid - 1)..(mid + 3) {
            let nd = (start + seg as f32 * segment_size - op) / segment_size;
            let p = seg.clamp(0, parameters_count as isize - 1) as usize;
            design[i][p] += bspline_kernel(nd) as f64 * sqrt_w;
        }
        data[i][0] = observations[i][0] as f64 * sqrt_w;
        data[i][1] = observations[i][1] as f64 * sqrt_w;
    }

    // Normal equations: AtA c = Atb, per dimension.
    let p = parameters_count;
    let mut ata = vec![vec![0.0f64; p]; p];
    let mut atb = vec![[0.0f64; 2]; p];
    for i in 0..n {
        for j in 0..p {
            let dij = design[i][j];
            if dij == 0.0 {
                continue;
            }
            for k in 0..p {
                ata[j][k] += dij * design[i][k];
            }
            atb[j][0] += dij * data[i][0];
            atb[j][1] += dij * data[i][1];
        }
    }

    let cx = solve_gaussian(&ata, &atb.iter().map(|b| b[0]).collect::<Vec<_>>());
    let cy = solve_gaussian(&ata, &atb.iter().map(|b| b[1]).collect::<Vec<_>>());

    CubicBSplineFit {
        start,
        segment_size,
        coefficients: cx
            .into_iter()
            .zip(cy)
            .map(|(x, y)| [x as f32, y as f32])
            .collect(),
    }
}

/// Solve `a * x = b` by Gaussian elimination with partial pivoting.
/// Near-singular pivots (rank deficiency from unconstrained control points)
/// leave the corresponding solution component at 0.
fn solve_gaussian(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    let mut rhs = b.to_vec();

    for col in 0..n {
        // Partial pivot.
        let mut pivot_row = col;
        for row in (col + 1)..n {
            if m[row][col].abs() > m[pivot_row][col].abs() {
                pivot_row = row;
            }
        }
        if pivot_row != col {
            m.swap(col, pivot_row);
            rhs.swap(col, pivot_row);
        }
        let pivot = m[col][col];
        if pivot.abs() < 1e-12 {
            continue; // rank-deficient column; solution component stays 0
        }
        for row in (col + 1)..n {
            let factor = m[row][col] / pivot;
            if factor == 0.0 {
                continue;
            }
            for k in col..n {
                m[row][k] -= factor * m[col][k];
            }
            rhs[row] -= factor * rhs[col];
        }
    }

    // Back substitution.
    let mut x = vec![0.0f64; n];
    for col in (0..n).rev() {
        let pivot = m[col][col];
        if pivot.abs() < 1e-12 {
            x[col] = 0.0;
            continue;
        }
        let mut sum = rhs[col];
        for k in (col + 1)..n {
            sum -= m[col][k] * x[k];
        }
        x[col] = sum / pivot;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparator::pick_seam_point;

    fn candidate(position: [f32; 3], visibility: f32) -> SeamCandidate {
        SeamCandidate {
            position,
            visibility,
            overhang: 0.0,
            unsupported_dist: 0.0,
            embedded_distance: 0.0,
            local_ccw_angle: -0.5, // rad; mildly concave corner
            central_enforcer: false,
            point_type: EnforcedBlockedSeamPoint::Neutral,
            flow_width: 0.4, // mm
        }
    }

    /// A stack of layers, each a 10 mm square with candidates at the four
    /// corners. `corner_visibility[k]` sets the visibility penalty of corner
    /// k, biasing the comparator's pick. Layer z spacing 0.2 mm.
    fn square_layers(num_layers: usize, corner_visibility: [f32; 4]) -> Vec<LayerCandidates> {
        let corners = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]; // mm
        (0..num_layers)
            .map(|li| {
                let z = li as f32 * 0.2; // mm
                let candidates: Vec<SeamCandidate> = corners
                    .iter()
                    .zip(corner_visibility.iter())
                    .map(|(&(x, y), &vis)| candidate([x, y, z], vis))
                    .collect();
                let comparator = SeamComparator::new(SeamSetup::Aligned);
                let seam_index = pick_seam_point(&candidates, 0..4, &comparator);
                LayerCandidates {
                    candidates,
                    perimeters: vec![Perimeter {
                        start_index: 0,
                        end_index: 4,
                        seam_index,
                        finalized: false,
                        final_seam_position: [0.0; 3],
                    }],
                }
            })
            .collect()
    }

    #[test]
    fn bspline_fit_recovers_straight_line() {
        // Noisy points on x = 0.5z + 1, y = -0.2z + 3 (mm). Slopes kept
        // gentle: the clamped B-spline basis (canonical behavior) biases
        // steep-slope endpoints, which is not what this test measures.
        let n = 25;
        let op: Vec<f32> = (0..n).map(|i| i as f32 * 0.5).collect();
        let obs: Vec<[f32; 2]> = op
            .iter()
            .enumerate()
            .map(|(i, &z)| {
                let noise = 0.05 * ((i as f32) * 1.7).sin(); // deterministic, mm
                [0.5 * z + 1.0 + noise, -0.2 * z + 3.0 - noise]
            })
            .collect();
        let weights = vec![1.0f32; n];
        let fit = fit_cubic_bspline(&op, &obs, &weights, 3);
        for &z in &op {
            let v = fit.get_fitted_value(z);
            assert!(
                (v[0] - (0.5 * z + 1.0)).abs() < 0.2,
                "x off at z={z}: {}",
                v[0]
            );
            assert!(
                (v[1] - (-0.2 * z + 3.0)).abs() < 0.2,
                "y off at z={z}: {}",
                v[1]
            );
        }
    }

    #[test]
    fn bspline_fit_constant_input_gives_constant_output() {
        let n = 12;
        let op: Vec<f32> = (0..n).map(|i| i as f32 * 0.2).collect();
        let obs = vec![[3.0f32, 4.0f32]; n];
        let weights = vec![1.0f32; n];
        let fit = fit_cubic_bspline(&op, &obs, &weights, 2);
        for &z in &op {
            let v = fit.get_fitted_value(z);
            assert!((v[0] - 3.0).abs() < 1e-3, "x at z={z}: {}", v[0]);
            assert!((v[1] - 4.0).abs() < 1e-3, "y at z={z}: {}", v[1]);
        }
    }

    #[test]
    fn aligned_chains_finalize_around_favored_corner() {
        // Corner 2 (10, 10) favored via lowest visibility.
        let mut layers = square_layers(20, [0.5, 0.5, 0.0, 0.5]);
        let comparator = SeamComparator::new(SeamSetup::Aligned);
        align_seam_points(&mut layers, &comparator);
        for (li, layer) in layers.iter().enumerate() {
            let p = &layer.perimeters[0];
            assert!(p.finalized, "layer {li} not finalized");
            let pos = p.final_seam_position;
            assert!(
                (pos[0] - 10.0).abs() <= 0.5 && (pos[1] - 10.0).abs() <= 0.5,
                "layer {li} seam {pos:?} not within 0.5 mm of favored corner"
            );
        }
    }

    #[test]
    fn aligned_back_finalizes_near_max_y() {
        // Rear-visibility bias baked into the fixture: rear corners (y = 10)
        // score better than front ones; corner 2 best.
        let mut layers = square_layers(20, [0.8, 0.8, 0.0, 0.3]);
        let comparator = SeamComparator::new(SeamSetup::AlignedBack);
        align_seam_points(&mut layers, &comparator);
        for (li, layer) in layers.iter().enumerate() {
            let p = &layer.perimeters[0];
            assert!(p.finalized, "layer {li} not finalized");
            assert!(
                (p.final_seam_position[1] - 10.0).abs() <= 0.5,
                "layer {li} y = {} not within 0.5 mm of max-Y",
                p.final_seam_position[1]
            );
        }
    }

    #[test]
    fn short_string_leaves_perimeters_unfinalized() {
        // 3 layers < SEAM_ALIGN_MINIMUM_STRING_SEAMS (6).
        let mut layers = square_layers(3, [0.5, 0.5, 0.0, 0.5]);
        let comparator = SeamComparator::new(SeamSetup::Aligned);
        align_seam_points(&mut layers, &comparator);
        for (li, layer) in layers.iter().enumerate() {
            assert!(
                !layer.perimeters[0].finalized,
                "layer {li} unexpectedly finalized"
            );
        }
    }

    #[test]
    fn alignment_is_deterministic() {
        let comparator = SeamComparator::new(SeamSetup::Aligned);
        let mut a = square_layers(20, [0.5, 0.5, 0.0, 0.5]);
        let mut b = square_layers(20, [0.5, 0.5, 0.0, 0.5]);
        align_seam_points(&mut a, &comparator);
        align_seam_points(&mut b, &comparator);
        for (la, lb) in a.iter().zip(&b) {
            for (pa, pb) in la.perimeters.iter().zip(&lb.perimeters) {
                assert_eq!(pa.finalized, pb.finalized);
                assert_eq!(pa.seam_index, pb.seam_index);
                assert_eq!(pa.final_seam_position, pb.final_seam_position);
            }
        }
    }

    #[test]
    fn find_seam_string_walks_both_directions() {
        let layers = square_layers(10, [0.5, 0.5, 0.0, 0.5]);
        let comparator = SeamComparator::new(SeamSetup::Aligned);
        let start = SeamRef {
            layer: 5,
            perimeter: 0,
            candidate: layers[5].perimeters[0].seam_index,
        };
        let string = find_seam_string(&layers, start, &comparator);
        assert_eq!(string.len(), 10);
        let mut seen: Vec<usize> = string.iter().map(|r| r.layer).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..10).collect::<Vec<_>>());
    }
}
