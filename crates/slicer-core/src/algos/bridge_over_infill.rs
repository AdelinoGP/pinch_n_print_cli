// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintObject.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Deterministic geometry for internal bridges over sparse infill.

use crate::polygon_ops::{
    closing_ex, difference, intersection, offset, opening, union, OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon};
use std::f32::consts::PI;

const SAMPLE_STEP: f64 = 20_000.0;

const SCALED_EPSILON_MM: f64 = 0.0001;
const UNITS_PER_MM: f64 = 10_000.0;

/// Canonical `target_flow_height_factor` fallback captured from PrintObject.cpp.
pub const BRIDGE_FLOW_HEIGHT_FACTOR: f32 = 1.0;

/// Geometry committed for one layer of bridge-depth harvesting.
#[derive(Clone, Debug, Default)]
pub struct BridgeDepthLayer {
    /// Layer print height in millimetres.
    pub print_z: f32,
    /// Lower-layer areas resolved as sparse infill.
    pub sparse_infill: Vec<ExPolygon>,
    /// Lower-layer areas that must mask sparse infill.
    pub not_sparse_infill: Vec<ExPolygon>,
}

/// Candidate geometry committed for one layer of bridge clustering.
#[derive(Clone, Debug, Default)]
pub struct BridgeCandidateLayer {
    /// Layer print height in millimetres.
    pub print_z: f32,
    /// Qualified bridge polygons authored on this layer.
    pub new_polys: Vec<ExPolygon>,
}

/// Gather sparse lower-layer area within the canonical depth window.
pub fn gather_areas_w_depth(
    layers: &[BridgeDepthLayer],
    layer_index: usize,
    target_flow_height: f32,
    target_flow_height_factor: f32,
) -> Vec<ExPolygon> {
    let Some(layer) = layers.get(layer_index) else {
        return Vec::new();
    };
    let bottom_z =
        layer.print_z - target_flow_height * target_flow_height_factor - SCALED_EPSILON_MM as f32;
    let mut sparse = Vec::new();
    let mut solid = Vec::new();
    for i in (0..layer_index).rev() {
        let lower = &layers[i];
        if lower.print_z < bottom_z && i < layer_index - 1 {
            break;
        }
        sparse.extend(lower.sparse_infill.iter().cloned());
        solid.extend(lower.not_sparse_infill.iter().cloned());
    }
    if sparse.is_empty() {
        return Vec::new();
    }
    let sparse = closing_ex(
        &union(&sparse, &[]),
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    let solid = closing_ex(
        &union(&solid, &[]),
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    difference(&sparse, &solid)
}

/// Group consecutive candidate layers when both their Z gap and area overlap
/// satisfy the canonical bridge-thread conditions.
pub fn cluster_candidate_layers(
    layers: &[BridgeCandidateLayer],
    bridging_flow_height: f32,
    target_flow_height_factor: f32,
) -> Vec<Vec<usize>> {
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for (index, candidate) in layers.iter().enumerate() {
        let joins = clusters.last().and_then(|cluster| {
            let previous = cluster.last().copied()?;
            let gap = candidate.print_z - layers[previous].print_z;
            (gap < bridging_flow_height * target_flow_height_factor - SCALED_EPSILON_MM as f32
                && !intersection(&candidate.new_polys, &layers[previous].new_polys).is_empty())
            .then_some(())
        });
        if joins.is_some() {
            clusters.last_mut().expect("cluster exists").push(index);
        } else {
            clusters.push(vec![index]);
        }
    }
    clusters
}

/// Remove candidate polygons already filled on lower layers within the depth.
pub fn remove_filled_polygons_on_lower_layers(
    deep_infill_area: &[ExPolygon],
    lower_candidates: &[BridgeCandidateLayer],
    current_print_z: f32,
    target_flow_height: f32,
) -> Vec<ExPolygon> {
    let bottom_z = current_print_z - target_flow_height - SCALED_EPSILON_MM as f32;
    let filled: Vec<ExPolygon> = lower_candidates
        .iter()
        .filter(|candidate| candidate.print_z >= bottom_z)
        .flat_map(|candidate| candidate.new_polys.iter().cloned())
        .collect();
    if filled.is_empty() {
        deep_infill_area.to_vec()
    } else {
        difference(deep_infill_area, &union(&filled, &[]))
    }
}

/// Compute the unsupported portion left after accounting for lower-layer fills
/// and solids. Distances follow the workspace's 100 nm coordinate units.
pub fn unsupported_span_areas(
    lower_fills: &[ExPolygon],
    lower_solids: &[ExPolygon],
    spacing_mm: f32,
    expansion_multiplier: f64,
) -> Vec<ExPolygon> {
    if lower_fills.is_empty() || spacing_mm <= 0.0 {
        return Vec::new();
    }
    let spacing = spacing_mm as f64;
    // Canonical: initially consider the whole layer unsupported — the carrier
    // is the lower fills themselves (closing(SCALED_EPSILON)), NOT the bbox
    // complement of the fills. RC-A fix.
    let mut unsupported = closing_ex(lower_fills, SCALED_EPSILON_MM, OffsetJoinType::Miter);
    unsupported = offset(
        &unsupported,
        -(expansion_multiplier * spacing) as f32,
        OffsetJoinType::Miter,
        0.0,
    );
    if lower_solids.is_empty() || unsupported.is_empty() {
        return unsupported;
    }
    let solids_shrunk = offset(lower_solids, -spacing as f32, OffsetJoinType::Miter, 0.0);
    let solids_grown = offset(
        &solids_shrunk,
        ((1.0 + expansion_multiplier) * spacing) as f32,
        OffsetJoinType::Miter,
        0.0,
    );
    difference(&unsupported, &solids_grown)
}

/// Qualify the genuinely unsupported part of one internal bridge surface.
pub fn qualify_internal_bridge_surface(
    surface: &ExPolygon,
    unsupported: &[ExPolygon],
    spacing_mm: f32,
    nofilter: bool,
) -> Option<Vec<ExPolygon>> {
    if spacing_mm <= 0.0 {
        return None;
    }
    let unsupported_surface = intersection(std::slice::from_ref(surface), unsupported);
    if unsupported_surface.is_empty() {
        return None;
    }
    let spacing_units = spacing_mm as f64 * UNITS_PER_MM;
    let unsupported_area = expolygons_area(&unsupported_surface);
    let surface_area = expolygon_area(surface);
    let partially_supported = unsupported_area < surface_area - 1.0;
    if !nofilter && partially_supported && unsupported_area <= 9.0 * spacing_units * spacing_units {
        return None;
    }

    let mut worth = intersection(
        std::slice::from_ref(surface),
        &offset(
            &unsupported_surface,
            4.0 * spacing_mm,
            OffsetJoinType::Miter,
            0.0,
        ),
    );
    let expanded_worth = offset(&worth, spacing_mm, OffsetJoinType::Miter, 0.0);
    for polygon in difference(std::slice::from_ref(surface), &expanded_worth) {
        let area = expolygon_area(&polygon);
        if area > spacing_units * spacing_units && area < spacing_units * 120_000.0 {
            worth.push(polygon);
        }
    }
    let closed = closing_ex(&worth, SCALED_EPSILON_MM, OffsetJoinType::Miter);
    Some(intersection(&closed, std::slice::from_ref(surface)))
}

fn expolygons_area(polygons: &[ExPolygon]) -> f64 {
    polygons.iter().map(expolygon_area).sum()
}

fn expolygon_area(polygon: &ExPolygon) -> f64 {
    fn ring_area(ring: &Polygon) -> f64 {
        ring.points
            .iter()
            .zip(ring.points.iter().cycle().skip(1))
            .take(ring.points.len())
            .map(|(a, b)| a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64)
            .sum::<f64>()
            .abs()
            / 2.0
    }
    ring_area(&polygon.contour) - polygon.holes.iter().map(ring_area).sum::<f64>()
}

// -----------------------------------------------------------------------------
// Portions of this helper are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintObject.cpp
//
// This is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
/// Apply the canonical expansion-zone treatment to one bridge candidate.
///
/// `deep_infill_area` is the sparse-infill footprint at this construction site;
/// `provenance` is the qualified internal-bridge set that produced the candidate.
pub fn expand_candidate_area(
    candidate: &[ExPolygon],
    expansion_source: &[ExPolygon],
    deep_infill_area: &[ExPolygon],
    provenance: &[ExPolygon],
    spacing_mm: f32,
) -> Vec<ExPolygon> {
    if candidate.is_empty() || spacing_mm <= 0.0 {
        return Vec::new();
    }
    let limiting_area = union(candidate, expansion_source);
    let grown = offset(&limiting_area, spacing_mm, OffsetJoinType::Miter, 0.0);
    let clipped = intersection(&grown, deep_infill_area);
    let qualified: Vec<ExPolygon> = clipped
        .into_iter()
        .filter(|area| !intersection(std::slice::from_ref(area), provenance).is_empty())
        .collect();
    if qualified.is_empty() {
        return Vec::new();
    }
    let opened = opening(
        &qualified,
        f64::from(spacing_mm) * 0.75,
        OffsetJoinType::Miter,
    );
    closing_ex(&opened, f64::from(spacing_mm), OffsetJoinType::Miter)
}

/// Select a bridge direction from arc-length-weighted nearest anchor samples.
pub fn determine_bridging_angle(
    anchors: &[Vec<Point2>],
    area_edges: &[Vec<Point2>],
    override_deg: f32,
) -> f32 {
    if override_deg > 0.0 {
        return override_deg;
    }
    let mut samples = Vec::new();
    for edge in area_edges {
        for pair in edge.windows(2) {
            let dx = (pair[1].x - pair[0].x) as f64;
            let dy = (pair[1].y - pair[0].y) as f64;
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            let count = (length / SAMPLE_STEP).ceil().max(1.0) as usize;
            for n in 0..count {
                let t = ((n as f64 + 0.5) / count as f64).min(1.0);
                let p = (pair[0].x as f64 + dx * t, pair[0].y as f64 + dy * t);
                if let Some(angle) = nearest_anchor_angle(anchors, p) {
                    samples.push(angle);
                }
            }
        }
    }
    if samples.is_empty() {
        return 0.001;
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    let window = 18.0_f64.to_radians();
    let mut best_score = 0usize;
    let mut best_angle = 0.0;
    for &candidate in &samples {
        let mut score = 0usize;
        let mut sum = 0.0;
        for &direction in &samples {
            let mut delta = (direction - candidate).rem_euclid(PI as f64);
            if delta > PI as f64 / 2.0 {
                delta -= PI as f64;
            }
            if delta.abs() <= window {
                score += 1;
                sum += candidate + delta;
            }
        }
        if score > best_score {
            best_score = score;
            best_angle = (sum / score as f64).rem_euclid(PI as f64);
        }
    }
    (best_angle.to_degrees() as f32)
        .rem_euclid(180.0)
        .max(0.001)
}

fn nearest_anchor_angle(anchors: &[Vec<Point2>], p: (f64, f64)) -> Option<f64> {
    let mut best = None;
    for line in anchors {
        for pair in line.windows(2) {
            let ax = pair[0].x as f64;
            let ay = pair[0].y as f64;
            let dx = (pair[1].x - pair[0].x) as f64;
            let dy = (pair[1].y - pair[0].y) as f64;
            let len2 = dx * dx + dy * dy;
            if len2 == 0.0 {
                continue;
            }
            let t = (((p.0 - ax) * dx + (p.1 - ay) * dy) / len2).clamp(0.0, 1.0);
            let distance2 = (p.0 - ax - t * dx).powi(2) + (p.1 - ay - t * dy).powi(2);
            let angle = (dy.atan2(dx) + PI as f64 / 2.0).rem_euclid(PI as f64);
            if best.is_none_or(|(d, _, _): (f64, f64, f64)| distance2 < d) {
                best = Some((distance2, angle, len2));
            }
        }
    }
    best.map(|(_, angle, _)| angle)
}

/// Construct scan strips whose endpoints are supported by anchor geometry.
pub fn construct_anchored_polygon(
    anchors: &[Vec<Point2>],
    voids: &[ExPolygon],
    angle_deg: f32,
    spacing_mm: f32,
    thread_width_mm: f32,
) -> (Vec<ExPolygon>, Vec<Vec<Point2>>) {
    if voids.is_empty() || spacing_mm <= 0.0 {
        return (Vec::new(), Vec::new());
    }
    let theta = (-angle_deg as f64 + 90.0_f64).to_radians();
    let (sin, cos) = theta.sin_cos();
    let spacing = (spacing_mm as f64 * 10_000.0).round();
    // Sparse infill can leave a small gap before the void wall; the canonical
    // construction extends a section across that gap to the flanking anchor.
    let tolerance = (thread_width_mm as f64 * 10_000.0 * 5.0).max(1.0);
    let mut result = Vec::new();
    let mut lines = Vec::new();
    for void in voids {
        let rotated: Vec<(f64, f64)> = void
            .contour
            .points
            .iter()
            .map(|p| rotate(p, sin, cos))
            .collect();
        if rotated.is_empty() {
            continue;
        }
        let min_x = rotated.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = rotated
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = rotated.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = rotated
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let rotated_anchors: Vec<Vec<(f64, f64)>> = anchors
            .iter()
            .map(|a| a.iter().map(|p| rotate(p, sin, cos)).collect())
            .collect();
        let mut x = min_x + spacing / 2.0;
        while x < max_x {
            let mut sections = vertical_sections(&rotated, x);
            let supported = !anchors.is_empty();
            for section in &mut sections {
                let mut low = section.0;
                let mut high = section.1;
                let mut low_anchor = None;
                let mut high_anchor = None;
                for line in &rotated_anchors {
                    for pair in line.windows(2) {
                        if let Some(y) = crossing_y(pair, x) {
                            if (y - low).abs() <= tolerance {
                                low_anchor = Some(y);
                            }
                            if (y - high).abs() <= tolerance {
                                high_anchor = Some(y);
                            }
                        }
                    }
                }
                if let Some(y) = low_anchor {
                    low = y;
                }
                if let Some(y) = high_anchor {
                    high = y;
                }
                *section = (low.max(min_y), high.min(max_y));
            }
            if supported {
                sections.retain(|(lo, hi)| {
                    (hi - lo) >= 1.0
                        && has_anchor(&rotated_anchors, x, *lo, tolerance)
                        && has_anchor(&rotated_anchors, x, *hi, tolerance)
                });
            }
            for (lo, hi) in sections {
                if hi <= lo {
                    continue;
                }
                let a = unrotate(x, lo, sin, cos);
                let b = unrotate(x, hi, sin, cos);
                lines.push(vec![a, b]);
                result.extend(intersection(
                    &[strip(a, b, (tolerance / 2.0).round() as i64)],
                    std::slice::from_ref(void),
                ));
            }
            x += spacing;
        }
    }
    (result, lines)
}

fn rotate(p: &Point2, sin: f64, cos: f64) -> (f64, f64) {
    (
        p.x as f64 * cos + p.y as f64 * sin,
        -p.x as f64 * sin + p.y as f64 * cos,
    )
}
fn unrotate(x: f64, y: f64, sin: f64, cos: f64) -> Point2 {
    Point2 {
        x: (x * cos - y * sin).round() as i64,
        y: (x * sin + y * cos).round() as i64,
    }
}
fn crossing_y(pair: &[(f64, f64)], x: f64) -> Option<f64> {
    let (a, b) = (pair[0], pair[1]);
    if (a.0 <= x && x < b.0) || (b.0 <= x && x < a.0) {
        Some(a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0))
    } else {
        None
    }
}
fn vertical_sections(poly: &[(f64, f64)], x: f64) -> Vec<(f64, f64)> {
    let mut ys = Vec::new();
    for pair in poly
        .windows(2)
        .chain(std::iter::once(&[poly[poly.len() - 1], poly[0]][..]))
    {
        if let Some(y) = crossing_y(pair, x) {
            ys.push(y);
        }
    }
    ys.sort_by(|a, b| a.total_cmp(b));
    ys.chunks_exact(2).map(|v| (v[0], v[1])).collect()
}
fn has_anchor(anchors: &[Vec<(f64, f64)>], x: f64, y: f64, tolerance: f64) -> bool {
    anchors.iter().flat_map(|a| a.windows(2)).any(|p| {
        let (a, b) = (p[0], p[1]);
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let t =
            (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy).max(1e-12)).clamp(0.0, 1.0);
        (x - a.0 - t * dx).hypot(y - a.1 - t * dy) <= tolerance
    })
}
fn strip(a: Point2, b: Point2, half_width: i64) -> ExPolygon {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    let len = dx.hypot(dy).max(1.0);
    let nx = (-dy / len * half_width as f64).round() as i64;
    let ny = (dx / len * half_width as f64).round() as i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: a.x + nx,
                    y: a.y + ny,
                },
                Point2 {
                    x: b.x + nx,
                    y: b.y + ny,
                },
                Point2 {
                    x: b.x - nx,
                    y: b.y - ny,
                },
                Point2 {
                    x: a.x - nx,
                    y: a.y - ny,
                },
            ],
        },
        holes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(min: f32, max: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min, 0.0),
                    Point2::from_mm(max, 0.0),
                    Point2::from_mm(max, 5.0),
                    Point2::from_mm(min, 5.0),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn gather_depth_subtracts_solids_and_stops_at_cutoff() {
        let sparse = square(0.0, 5.0);
        let layers = vec![
            BridgeDepthLayer {
                print_z: 0.0,
                sparse_infill: vec![sparse.clone()],
                ..Default::default()
            },
            BridgeDepthLayer {
                print_z: 0.2,
                sparse_infill: vec![sparse.clone()],
                not_sparse_infill: vec![square(0.0, 2.0)],
            },
            BridgeDepthLayer {
                print_z: 0.4,
                ..Default::default()
            },
        ];
        let result = gather_areas_w_depth(&layers, 2, 0.2, 1.0);
        assert!(result
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .all(|p| p.x >= Point2::from_mm(2.0, 0.0).x));
    }

    #[test]
    fn clustering_requires_z_gap_and_intersection() {
        let layers = vec![
            BridgeCandidateLayer {
                print_z: 0.0,
                new_polys: vec![square(0.0, 2.0)],
            },
            BridgeCandidateLayer {
                print_z: 0.1,
                new_polys: vec![square(1.0, 3.0)],
            },
            BridgeCandidateLayer {
                print_z: 0.3,
                new_polys: vec![square(1.0, 3.0)],
            },
        ];
        assert_eq!(
            cluster_candidate_layers(&layers, 0.2, 1.0),
            vec![vec![0, 1], vec![2]]
        );
    }

    #[test]
    fn filled_lower_candidates_are_removed() {
        let result = remove_filled_polygons_on_lower_layers(
            &[square(0.0, 5.0)],
            &[BridgeCandidateLayer {
                print_z: 0.2,
                new_polys: vec![square(0.0, 2.0)],
            }],
            0.4,
            0.2,
        );
        assert!(result
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .all(|p| p.x >= Point2::from_mm(2.0, 0.0).x));
    }
}
