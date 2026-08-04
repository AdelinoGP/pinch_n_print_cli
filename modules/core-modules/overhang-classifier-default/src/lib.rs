// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/SupportSpotsGenerator.cpp
// (curl-height estimation: `get_flow_width`, `estimate_curled_up_height`,
// `estimate_malformations` — the only LIVE code in that file; the
// support-point-placement code the file's name implies is dead/commented-out
// upstream and was not ported).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Consumer of the per-vertex `overhang_quartile` annotation written by the
//! upstream PrePass::OverhangAnnotation pipeline (ADR-0031, packet 106), plus
//! self-contained curled-edge slowdown: applies speed-factor
//! mutations to wall entities on overhangs and near previously-curled wall
//! geometry.
//!
//! Curl estimation and the cross-layer lookup that consumes it are both
//! computed transiently inside [`run_finalization`] — `curled_height` is not
//! a persisted IR/WIT field (unlike `overhang_quartile`), since nothing else
//! in this codebase needs to read it back out. See `CONTEXT.md` for the
//! **overhang quartile** / **curled height** / **artificial curl distance**
//! vocabulary.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigView, ExtrusionRole, Point3WithWidth};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView,
};

/// Core overhang classifier that applies speed-factor mutations to wall entities on overhangs.
pub struct OverhangClassifierDefault;

/// Config float for `key`, defaulting to 0.0.
fn speed(config: &ConfigView, key: &str) -> f32 {
    config.get_float(key).unwrap_or(0.0) as f32
}

/// Base wall speed for `role` (0.0 for non-wall roles).
fn base_speed(role: &ExtrusionRole, config: &ConfigView) -> f32 {
    match role {
        ExtrusionRole::OuterWall => speed(config, "outer_wall_speed"),
        ExtrusionRole::InnerWall => speed(config, "inner_wall_speed"),
        ExtrusionRole::ThinWall => speed(config, "thin_wall_speed"),
        _ => 0.0,
    }
}

/// Overhang speed for `quartile` (1..=4), 0.0 otherwise.
fn overhang_speed(quartile: u8, config: &ConfigView) -> f32 {
    match quartile {
        1 => speed(config, "overhang_1_4_speed"),
        2 => speed(config, "overhang_2_4_speed"),
        3 => speed(config, "overhang_3_4_speed"),
        4 => speed(config, "overhang_4_4_speed"),
        _ => 0.0,
    }
}

/// Line width (mm) used for both overhang-quartile bucketing and curl
/// distance synthesis. Reads `outer_wall_line_width`, falling back to
/// `line_width` (matches the resolution convention documented in
/// `crates/slicer-core/src/algos/overhang_annotation.rs`'s "Config wiring
/// note").
fn line_width(config: &ConfigView) -> f32 {
    config
        .get_float("outer_wall_line_width")
        .or_else(|| config.get_float("line_width"))
        .unwrap_or(0.0) as f32
}

/// Canonical constructs this list as a stack-local `ConfigOptionPercents
/// overhang_overlap_levels({90, 75, 50, 25, 13, 0})` inside `GCode::_extrude`.
/// No config key named `overhang_overlap_levels` exists in canonical
/// `PrintConfig.cpp`.
pub const OVERHANG_OVERLAP_LEVELS: [f32; 6] = [90.0, 75.0, 50.0, 25.0, 13.0, 0.0];

/// Builds the six distance/speed sections used for overhang speed smoothing.
pub fn build_speed_sections(
    ref_speed: f32,
    path_width: f32,
    config: &ConfigView,
) -> Vec<(f32, f32)> {
    let overhang_speed_or_ref = |key: &str| {
        let configured = speed(config, key);
        if configured < 0.5 {
            ref_speed
        } else {
            configured
        }
    };

    let sixth_speed = if config
        .get_bool("slowdown_for_curled_perimeters")
        .unwrap_or(false)
    {
        overhang_speed_or_ref("overhang_4_4_speed")
    } else {
        speed(config, "bridge_speed")
    };
    let speeds = [
        ref_speed,
        overhang_speed_or_ref("overhang_1_4_speed"),
        overhang_speed_or_ref("overhang_2_4_speed"),
        overhang_speed_or_ref("overhang_3_4_speed"),
        overhang_speed_or_ref("overhang_4_4_speed"),
        sixth_speed,
    ];

    let mut sections: Vec<_> = OVERHANG_OVERLAP_LEVELS
        .into_iter()
        .zip(speeds)
        .map(|(overlap, section_speed)| (path_width * (1.0 - overlap / 100.0), section_speed))
        .collect();
    sections.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));

    for i in 1..sections.len() {
        if sections[i].0 == sections[i - 1].0 {
            sections[i].1 = sections[i - 1].1;
        }
    }
    sections
}

/// Interpolates a smoothed speed from sorted distance/speed sections.
pub fn calculate_speed(distance: f32, sections: &[(f32, f32)], original_speed: f32) -> f32 {
    if sections.is_empty() {
        return original_speed;
    }
    if distance <= sections[0].0 {
        return original_speed;
    }
    if distance >= sections[sections.len() - 1].0 {
        return sections[sections.len() - 1].1;
    }

    let pair = sections
        .windows(2)
        .find(|pair| distance <= pair[1].0)
        .expect("distance must be bracketed by sorted sections");
    let (d0, s0) = pair[0];
    let (d1, s1) = pair[1];
    let t = ((distance - d0) / (d1 - d0)).clamp(0.0, 1.0);
    let extrusion_speed = ((1.0 - t) * s0 + t * s1).round();
    extrusion_speed.min(original_speed)
}

const EPSILON: f32 = 1e-4;

/// Returns the intersections between a segment and boundary segments, ordered
/// from the first endpoint of `seg` to the second.
pub fn segment_intersections(
    seg: ((f32, f32), (f32, f32)),
    boundary: &[(f32, f32, f32, f32)],
) -> Vec<(f32, f32)> {
    let (start, end) = seg;
    let direction = (end.0 - start.0, end.1 - start.1);
    let direction_length_squared = direction.0 * direction.0 + direction.1 * direction.1;
    let cross = |a: (f32, f32), b: (f32, f32)| a.0 * b.1 - a.1 * b.0;
    let point_distance_squared = |a: (f32, f32), b: (f32, f32)| {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        dx * dx + dy * dy
    };
    let parameter_on_segment = |point: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        let delta = (b.0 - a.0, b.1 - a.1);
        let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
        if length_squared <= EPSILON * EPSILON {
            return (point_distance_squared(point, a) <= EPSILON * EPSILON).then_some(0.0);
        }
        let t = ((point.0 - a.0) * delta.0 + (point.1 - a.1) * delta.1) / length_squared;
        if !(-EPSILON..=1.0 + EPSILON).contains(&t) {
            return None;
        }
        let projected = (a.0 + t * delta.0, a.1 + t * delta.1);
        (point_distance_squared(point, projected) <= EPSILON * EPSILON).then_some(t.clamp(0.0, 1.0))
    };

    let mut intersections = Vec::new();
    let mut add_unique = |point: (f32, f32)| {
        if !intersections
            .iter()
            .any(|existing| point_distance_squared(*existing, point) <= EPSILON * EPSILON)
        {
            intersections.push(point);
        }
    };
    for &(x0, y0, x1, y1) in boundary {
        let boundary_start = (x0, y0);
        let boundary_end = (x1, y1);
        let boundary_direction = (x1 - x0, y1 - y0);
        let boundary_length_squared = boundary_direction.0 * boundary_direction.0
            + boundary_direction.1 * boundary_direction.1;

        if direction_length_squared <= EPSILON * EPSILON {
            if boundary_length_squared <= EPSILON * EPSILON {
                if point_distance_squared(start, boundary_start) <= EPSILON * EPSILON {
                    add_unique(start);
                }
            } else if parameter_on_segment(start, boundary_start, boundary_end).is_some() {
                add_unique(start);
            }
            continue;
        }

        if boundary_length_squared <= EPSILON * EPSILON {
            if parameter_on_segment(boundary_start, start, end).is_some() {
                add_unique(boundary_start);
            }
            continue;
        }

        let from_start = (boundary_start.0 - start.0, boundary_start.1 - start.1);
        let denominator = cross(direction, boundary_direction);
        if denominator.abs() > EPSILON {
            let segment_t = cross(from_start, boundary_direction) / denominator;
            let boundary_t = cross(from_start, direction) / denominator;
            if (-EPSILON..=1.0 + EPSILON).contains(&segment_t)
                && (-EPSILON..=1.0 + EPSILON).contains(&boundary_t)
            {
                let segment_t = segment_t.clamp(0.0, 1.0);
                add_unique((
                    start.0 + segment_t * direction.0,
                    start.1 + segment_t * direction.1,
                ));
            }
        } else if cross(from_start, direction).abs() <= EPSILON {
            if parameter_on_segment(boundary_start, start, end).is_some() {
                add_unique(boundary_start);
            }
            if parameter_on_segment(boundary_end, start, end).is_some() {
                add_unique(boundary_end);
            }
            if parameter_on_segment(start, boundary_start, boundary_end).is_some() {
                add_unique(start);
            }
            if parameter_on_segment(end, boundary_start, boundary_end).is_some() {
                add_unique(end);
            }
        }
    }

    intersections.sort_by(|a, b| {
        let parameter = |point: &(f32, f32)| {
            ((point.0 - start.0) * direction.0 + (point.1 - start.1) * direction.1)
                / direction_length_squared
        };
        parameter(a)
            .total_cmp(&parameter(b))
            .then_with(|| a.0.total_cmp(&b.0))
            .then_with(|| a.1.total_cmp(&b.1))
    });
    intersections
}

/// Finds the smallest distance at which the section speed is no faster than
/// `original_speed`, or `-1.0` when no section slows the path.
pub fn min_distance_from_sections(sections: &[(f32, f32)], original_speed: f32) -> f32 {
    sections
        .iter()
        .filter(|(_, section_speed)| *section_speed <= original_speed)
        .map(|(distance, _)| *distance)
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(-1.0)
}

/// Port of canonical `estimate_points_properties` (`GCode/ExtrusionProcessor.hpp`)
/// and `ExtrusionQualityEstimator::estimate_extrusion_quality` insertion logic.
pub fn insert_extended_points(
    points: &[Point3WithWidth],
    distances: &[Option<f32>],
    boundary: &[(f32, f32, f32, f32)],
    flow_width: f32,
    min_distance: f32,
) -> (Vec<Point3WithWidth>, Vec<Option<f32>>) {
    let min_distances = vec![min_distance; points.len()];
    insert_extended_points_with_point_widths(
        points,
        distances,
        boundary,
        flow_width,
        &min_distances,
    )
}

fn insert_extended_points_with_point_widths(
    points: &[Point3WithWidth],
    distances: &[Option<f32>],
    boundary: &[(f32, f32, f32, f32)],
    fallback_width: f32,
    min_distances: &[f32],
) -> (Vec<Point3WithWidth>, Vec<Option<f32>>) {
    if points.len() != distances.len() || points.len() != min_distances.len() {
        return (points.to_vec(), distances.to_vec());
    }
    if points.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let width_for = |point: Point3WithWidth| {
        if point.width >= 0.0 {
            point.width
        } else {
            fallback_width
        }
    };
    let distance_3d = |a: Point3WithWidth, b: Point3WithWidth| {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    let interpolate = |curr: Point3WithWidth,
                       next: Point3WithWidth,
                       t: f32,
                       curr_distance: f32,
                       next_distance: f32,
                       distance: f32| {
        let curr_is_closer = (curr_distance - distance).abs() <= (next_distance - distance).abs();
        Point3WithWidth {
            x: curr.x + t * (next.x - curr.x),
            y: curr.y + t * (next.y - curr.y),
            z: curr.z + t * (next.z - curr.z),
            width: curr.width + t * (next.width - curr.width),
            flow_factor: curr.flow_factor + t * (next.flow_factor - curr.flow_factor),
            overhang_quartile: if curr_is_closer {
                curr.overhang_quartile
            } else {
                next.overhang_quartile
            },
            dist_to_top_mm: curr.dist_to_top_mm + t * (next.dist_to_top_mm - curr.dist_to_top_mm),
            overhang_distance_mm: Some(distance),
        }
    };

    let mut crossing_points = Vec::with_capacity(points.len() + boundary.len());
    crossing_points.push((points[0], distances[0], min_distances[0]));
    for index in 1..points.len() {
        let (curr, curr_distance, curr_min_distance) = crossing_points
            .last()
            .copied()
            .expect("crossing points always contains the current point");
        let next = points[index];
        let next_distance = distances[index];
        let next_min_distance = min_distances[index];
        let curr_boundary_offset = 0.5 * width_for(curr);
        let next_boundary_offset = 0.5 * width_for(next);

        if let (Some(curr_distance), Some(next_distance)) = (curr_distance, next_distance) {
            if (curr_distance > curr_boundary_offset + EPSILON)
                != (next_distance > next_boundary_offset + EPSILON)
            {
                let curr_min_spacing = width_for(curr) * 0.25;
                let next_min_spacing = width_for(next) * 0.25;
                let intersections =
                    segment_intersections(((curr.x, curr.y), (next.x, next.y)), boundary);
                for (x, y) in intersections {
                    let dx = next.x - curr.x;
                    let dy = next.y - curr.y;
                    let length_squared = dx * dx + dy * dy;
                    let t = if length_squared > EPSILON * EPSILON {
                        (((x - curr.x) * dx + (y - curr.y) * dy) / length_squared).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let interpolated_distance = curr_distance + t * (next_distance - curr_distance);
                    let mut candidate = interpolate(
                        curr,
                        next,
                        t,
                        curr_distance,
                        next_distance,
                        interpolated_distance,
                    );
                    let candidate_boundary_offset = 0.5 * width_for(candidate);
                    candidate.overhang_distance_mm = Some(candidate_boundary_offset);
                    if distance_3d(candidate, curr) > curr_min_spacing
                        && distance_3d(next, candidate) > next_min_spacing
                    {
                        let candidate_min_distance =
                            curr_min_distance + t * (next_min_distance - curr_min_distance);
                        crossing_points.push((
                            candidate,
                            Some(candidate_boundary_offset),
                            candidate_min_distance,
                        ));
                    }
                }
            }
        }
        crossing_points.push((next, next_distance, next_min_distance));
    }

    let mut extended = Vec::with_capacity(crossing_points.len() * 2);
    extended.push(crossing_points[0]);
    for index in 0..crossing_points.len() - 1 {
        let (curr, curr_distance, curr_min_distance) = crossing_points[index];
        let (next, next_distance, next_min_distance) = crossing_points[index + 1];

        if let (Some(curr_distance), Some(next_distance)) = (curr_distance, next_distance) {
            let curr_boundary_offset = 0.5 * width_for(curr);
            let next_boundary_offset = 0.5 * width_for(next);
            let curr_min_spacing = width_for(curr) * 0.25;
            let next_min_spacing = width_for(next) * 0.25;
            let near_boundary = (curr_distance > -curr_boundary_offset
                && curr_distance < curr_boundary_offset + 2.0)
                || (next_distance > -next_boundary_offset
                    && next_distance < next_boundary_offset + 2.0);
            if near_boundary {
                let line_len = distance_3d(curr, next);
                let gate_open = ((curr_min_distance > 0.0
                    && curr_distance.abs() > curr_min_distance)
                    || (next_min_distance > 0.0 && next_distance.abs() > next_min_distance))
                    && line_len >= 2.0
                    || (curr_min_distance <= 0.0 && next_min_distance <= 0.0 && line_len > 4.0);
                if gate_open {
                    let a0 =
                        ((curr_distance + 3.0 * curr_boundary_offset) / line_len).clamp(0.0, 1.0);
                    let a1 = (1.0 - (next_distance + 3.0 * next_boundary_offset) / line_len)
                        .clamp(0.0, 1.0);
                    let t0 = a0.min(a1);
                    let t1 = a0.max(a1);

                    let mut add_candidate = |t: f32| {
                        if !(0.0 < t && t < 1.0) {
                            return;
                        }
                        let candidate_distance =
                            curr_distance + t * (next_distance - curr_distance);
                        let candidate = interpolate(
                            curr,
                            next,
                            t,
                            curr_distance,
                            next_distance,
                            candidate_distance,
                        );
                        if distance_3d(candidate, curr) > curr_min_spacing
                            && distance_3d(next, candidate) > next_min_spacing
                        {
                            let candidate_min_distance =
                                curr_min_distance + t * (next_min_distance - curr_min_distance);
                            extended.push((
                                candidate,
                                Some(candidate_distance),
                                candidate_min_distance,
                            ));
                        }
                    };

                    add_candidate(t0);
                    if t1 != t0 {
                        add_candidate(t1);
                    }
                }
            }
        }
        extended.push((next, next_distance, next_min_distance));
    }

    extended
        .into_iter()
        .map(|(point, distance, _)| (point, distance))
        .unzip()
}

// Deleted with `BAND_BOUNDARY_MULTIPLIERS` and `quartile_for_distance`: the
// "Keep these two lists numerically identical" invariant no longer has a
// module-side list to keep in sync.

/// Curl-height estimation, ported from OrcaSlicer's `estimate_curled_up_height`
/// (`SupportSpotsGenerator.cpp:199-236`). `distance` is the unsigned distance
/// (mm) from this point to the nearest reference point on the layer below;
/// `curvature` is signed discrete curvature (1/mm, see [`discrete_curvature`]);
/// `prev_line_curled_height` seeds decay from the nearest reference point's
/// own curled height. Upstream's `malformation_distance_factors` (0.2, 1.1)
/// and `max_curled_height_factor` (10.0) are inlined as named locals — this
/// codebase has no equivalent tunable `Params` struct for this feature yet.
fn estimate_curled_up_height(
    distance: f32,
    curvature: f32,
    layer_height: f32,
    flow_width: f32,
    prev_line_curled_height: f32,
) -> f32 {
    const MALFORMATION_DISTANCE_FACTORS: (f32, f32) = (0.2, 1.1);
    const MAX_CURLED_HEIGHT_FACTOR: f32 = 10.0;

    let mut curled_up_height = 0.0f32;
    if distance.abs() < 3.0 * flow_width {
        curled_up_height = (prev_line_curled_height - layer_height * 0.75).max(0.0);
    }
    if distance > MALFORMATION_DISTANCE_FACTORS.0 * flow_width
        && distance < MALFORMATION_DISTANCE_FACTORS.1 * flow_width
    {
        let curling_section = distance;
        let swelling_radius = (layer_height + curling_section) / 2.0;
        curled_up_height += ((swelling_radius - layer_height) / 2.0).max(0.0);
        if curvature > 0.01 {
            let radius = 1.0 / curvature;
            let curling_t = (radius / 100.0).sqrt();
            let b = curling_t * flow_width;
            let a = curling_section;
            let c = (a * a - b * b).max(0.0).sqrt();
            curled_up_height += c;
        }
        curled_up_height = curled_up_height.min(MAX_CURLED_HEIGHT_FACTOR * layer_height);
    }
    curled_up_height
}

/// Signed discrete curvature (1/mm) at `curr`, given its polyline neighbors.
/// Not a verbatim port of OrcaSlicer's `estimate_points_properties` (that
/// function lives outside the ~150 live lines this port scoped from
/// `SupportSpotsGenerator.cpp` and has its own AABB-tree-based distance
/// annotation infrastructure) — this is a standard angle-over-arc-length
/// discrete curvature estimate, functionally equivalent for the purpose of
/// [`estimate_curled_up_height`]'s convex-turn bonus term.
fn discrete_curvature(prev: (f32, f32), curr: (f32, f32), next: (f32, f32)) -> f32 {
    let d1 = (curr.0 - prev.0, curr.1 - prev.1);
    let d2 = (next.0 - curr.0, next.1 - curr.1);
    let len1 = (d1.0 * d1.0 + d1.1 * d1.1).sqrt();
    let len2 = (d2.0 * d2.0 + d2.1 * d2.1).sqrt();
    if len1 < 1e-6 || len2 < 1e-6 {
        return 0.0;
    }
    let cross = d1.0 * d2.1 - d1.1 * d2.0;
    let dot = d1.0 * d2.0 + d1.1 * d2.1;
    let angle = cross.atan2(dot);
    angle.abs() / ((len1 + len2) / 2.0)
}

/// Nearest reference point to `(x, y)` in `points` (each `(x, y, curled_height)`),
/// returning `(distance_mm, that_point's_curled_height)`. `None` if `points`
/// is empty.
fn nearest_reference_point(points: &[(f32, f32, f32)], x: f32, y: f32) -> Option<(f32, f32)> {
    points
        .iter()
        .map(|&(px, py, ch)| (((x - px).powi(2) + (y - py).powi(2)).sqrt(), ch))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

#[slicer_module]
impl FinalizationModule for OverhangClassifierDefault {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(OverhangClassifierDefault)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !config.get_bool("enable_overhang_speed").unwrap_or(true) {
            return Ok(());
        }

        // Curl reuses the overhang speed table (no separate curl-specific
        // config keys — see the module doc-comment), so "all overhang bands
        // are zero" already means the whole feature family is off; skipping
        // here also avoids the wasted cross-layer point scan below.
        if (1..=4).all(|q| overhang_speed(q, config) == 0.0) {
            return Ok(());
        }
        let flow_width = line_width(config);
        let dist_limit = 10.0 * flow_width;

        // Reference geometry for curl: the previous layer's own OuterWall
        // points, each carrying its own curled_height. Empty for layer 0
        // (no layer below to reference) and stays that way until the first
        // layer with OuterWall geometry has been walked.
        let mut prev_wall_points: Vec<(f32, f32, f32)> = Vec::new();

        for (idx, layer) in layers.iter().enumerate() {
            let layer_height = if idx == 0 {
                None
            } else {
                Some((layer.z() - layers[idx - 1].z()).max(0.001))
            };
            let boundary: Vec<(f32, f32, f32, f32)> =
                if idx == 0 || flow_width <= 0.0 {
                    Vec::new()
                } else {
                    let mut boundary = Vec::new();
                    for entity in layers[idx - 1].ordered_entities() {
                        if entity.role != ExtrusionRole::OuterWall {
                            continue;
                        }
                        let points = &entity.path.points;
                        boundary.extend(points.windows(2).map(|segment| {
                            (segment[0].x, segment[0].y, segment[1].x, segment[1].y)
                        }));

                        // Most closed loops repeat their first point, so windows(2)
                        // already contains the closing edge. Add it explicitly for
                        // the equivalent closed representation without that repeat.
                        if points.len() >= 2 {
                            let first = points[0];
                            let last = points[points.len() - 1];
                            if first.x != last.x || first.y != last.y {
                                boundary.push((last.x, last.y, first.x, first.y));
                            }
                        }
                    }
                    boundary
                };

            // (1) Consumption: speed is resolved per point from the stamped
            // signed distances, then curl is applied to that already-clamped
            // point speed. Layer 0 is still produced as curl reference geometry
            // below, but never enters this speed path.
            if idx > 0 {
                for entity in layer.ordered_entities() {
                    let base = base_speed(&entity.role, config);
                    if base <= 0.0 || entity.path.points.is_empty() {
                        continue;
                    }

                    let original_points = &entity.path.points;
                    let distances: Vec<Option<f32>> = original_points
                        .iter()
                        .map(|point| point.overhang_distance_mm)
                        .collect();
                    let min_distances: Vec<f32> = original_points
                        .iter()
                        .map(|point| {
                            let sections = build_speed_sections(base, point.width, config);
                            min_distance_from_sections(&sections, base)
                        })
                        .collect();
                    let (new_points, new_distances) = insert_extended_points_with_point_widths(
                        original_points,
                        &distances,
                        &boundary,
                        flow_width,
                        &min_distances,
                    );
                    let points_grew = new_points.len() > original_points.len();
                    let points = &new_points;
                    let mut speeds = Vec::with_capacity(points.len());
                    let mut has_distance = false;
                    let mut has_curl = false;
                    for (point_idx, point) in points.iter().enumerate() {
                        let mut extrusion_speed = base;
                        if point.overhang_quartile.is_some() {
                            let sections = build_speed_sections(base, point.width, config);
                            if let Some(distance) = new_distances[point_idx] {
                                has_distance = true;
                                let current_speed = calculate_speed(distance, &sections, base);
                                let next_speed = points
                                    .get(point_idx + 1)
                                    .filter(|next| next.overhang_quartile.is_some())
                                    .and_then(|_| {
                                        new_distances.get(point_idx + 1).copied().flatten()
                                    })
                                    .map(|next_distance| {
                                        let next = &points[point_idx + 1];
                                        let next_sections =
                                            build_speed_sections(base, next.width, config);
                                        calculate_speed(next_distance, &next_sections, base)
                                    })
                                    .unwrap_or(base);
                                extrusion_speed = current_speed.min(next_speed).min(base);
                            }

                            if flow_width > 0.0 && !prev_wall_points.is_empty() {
                                // Only reachable once `prev_wall_points` has been
                                // seeded by a prior iteration (idx >= 1), where
                                // `layer_height` is always `Some`; the `unwrap_or`
                                // is a defensive fallback, not the expected path.
                                let lh = layer_height.unwrap_or(flow_width);
                                if let Some((distance, curled_height)) =
                                    nearest_reference_point(&prev_wall_points, point.x, point.y)
                                {
                                    if distance < dist_limit && curled_height > 0.0 {
                                        // Ported shape from ExtrusionProcessor.hpp's
                                        // artificial_distance_to_curled_lines formula.
                                        let artificial = flow_width
                                            * (1.0 - distance / dist_limit).powi(2)
                                            * (curled_height / (lh * 10.0));
                                        if artificial > 0.0 {
                                            has_curl = true;
                                            let curled_speed =
                                                calculate_speed(artificial, &sections, base);
                                            // Curl is applied after the original-speed
                                            // clamp, matching canonical ordering.
                                            extrusion_speed = curled_speed.min(extrusion_speed);
                                        }
                                    }
                                }
                            }
                        }
                        speeds.push(extrusion_speed);
                    }

                    if !has_distance && !has_curl {
                        continue;
                    }

                    let factors: Vec<f32> = speeds.into_iter().map(|speed| speed / base).collect();
                    if points_grew {
                        output
                            .modify_entity(
                                layer.layer_index(),
                                entity.entity_id,
                                EntityMutation::SetPathPoints(new_points),
                            )
                            .map_err(ModuleError::from_str)?;
                    }
                    let mutation = EntityMutation::SetPointSpeedFactors(factors);
                    output
                        .modify_entity(layer.layer_index(), entity.entity_id, mutation)
                        .map_err(ModuleError::from_str)?;
                }
            }

            // (2) Production: record this layer's own OuterWall points as
            // reference geometry for the NEXT layer's curl lookup. Always
            // collect positions (layer 0 included, so layer 1 has something
            // to reference); only estimate a nonzero height when a lower
            // layer exists to measure distance/decay against — matches this
            // codebase's "no previous layer ⇒ no signal" precedent already
            // used for `overhang_quartile` at layer 0.
            let mut this_layer_points: Vec<(f32, f32, f32)> = Vec::new();
            if flow_width > 0.0 {
                for entity in layer.ordered_entities() {
                    if entity.role != ExtrusionRole::OuterWall {
                        continue;
                    }
                    let pts = &entity.path.points;
                    let n = pts.len();
                    for i in 0..n {
                        let curr = (pts[i].x, pts[i].y);
                        let curled_height = match layer_height {
                            Some(lh) => {
                                let prev_pt = pts[if i == 0 { n - 1 } else { i - 1 }];
                                let next_pt = pts[(i + 1) % n];
                                let curvature = discrete_curvature(
                                    (prev_pt.x, prev_pt.y),
                                    curr,
                                    (next_pt.x, next_pt.y),
                                );
                                let (distance, prev_h) =
                                    nearest_reference_point(&prev_wall_points, curr.0, curr.1)
                                        .unwrap_or((f32::MAX, 0.0));
                                estimate_curled_up_height(
                                    distance, curvature, lh, flow_width, prev_h,
                                )
                            }
                            None => 0.0,
                        };
                        this_layer_points.push((curr.0, curr.1, curled_height));
                    }
                }
            }
            prev_wall_points = this_layer_points;
        }
        Ok(())
    }
}
