// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Seam placer module.
//!
//! Implements `LayerModule::run_wall_postprocess` for the `Layer::PerimetersPostProcess` stage.
//! Reads resolved seam from perimeter regions and rotates wall loop geometry
//! so path.points[0] is the seam vertex.
//!
//! Per OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp/cpp.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, SeamReason};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

/// Seam placement mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeamMode {
    /// Select the candidate with the lowest effective score.
    Nearest,
    /// Select the candidate with the highest Y coordinate (rear of print bed).
    Rear,
    /// Select a pseudo-random candidate based on layer index.
    Random,
    /// Align seams vertically across layers (nearest-style scoring seed).
    Aligned,
    /// Align seams vertically across layers, biased to the rear of the bed.
    AlignedBack,
}

/// Seam placer module.
///
/// Selects the best seam candidate from perimeter regions and writes
/// the resolved seam position. Supports nearest, rear, and random modes.
pub struct SeamPlacer {
    /// Seam placement mode.
    mode: SeamMode,
    /// Whether inner-wall seams are offset from the resolved outer seam.
    staggered_inner_seams: bool,
}

impl SeamPlacer {
    /// Returns the seam position mode as a string (for testing).
    pub fn seam_position(&self) -> &str {
        match self.mode {
            SeamMode::Nearest => "nearest",
            SeamMode::Rear => "rear",
            SeamMode::Random => "random",
            SeamMode::Aligned => "aligned",
            SeamMode::AlignedBack => "aligned_back",
        }
    }
}

/// Reason-based priority bonus (lower is better).
/// Concave corners hide seams best, so they get the largest negative bonus.
fn reason_bonus(reason: SeamReason) -> f32 {
    match reason {
        SeamReason::Concave => -0.5,
        SeamReason::Sharp => -0.2,
        SeamReason::UserForced => -1.0,
        SeamReason::Aligned => 0.0,
    }
}

fn effective_score(candidate: &slicer_ir::SeamCandidate) -> f32 {
    candidate.score + reason_bonus(candidate.reason)
}

fn select_seam_candidate(
    mode: SeamMode,
    layer_index: u32,
    candidates: &[slicer_ir::SeamCandidate],
) -> Option<&slicer_ir::SeamCandidate> {
    match mode {
        // Aligned/AlignedBack never reach this function: `run_wall_postprocess`
        // routes them through the host-injected `resolved_seam` snap path
        // (`aligned_seam_location`). The arms below are a defensive fallback.
        SeamMode::Nearest | SeamMode::Aligned | SeamMode::AlignedBack => {
            candidates.iter().min_by(|left, right| {
                effective_score(left)
                    .total_cmp(&effective_score(right))
                    .then_with(|| left.position.y.total_cmp(&right.position.y))
                    .then_with(|| left.position.x.total_cmp(&right.position.x))
            })
        }
        SeamMode::Rear => candidates.iter().max_by(|left, right| {
            left.position
                .y
                .total_cmp(&right.position.y)
                .then_with(|| effective_score(right).total_cmp(&effective_score(left)))
                .then_with(|| left.position.x.total_cmp(&right.position.x))
        }),
        SeamMode::Random => {
            let idx = (layer_index as usize) % candidates.len();
            candidates.get(idx)
        }
    }
}

/// Squared 2D XY distance between two IR points (Z is deliberately ignored:
/// the injected aligned seam carries the planner's layer Z, which may differ
/// slightly from this region's wall-loop Z).
fn dist2_xy(a: &slicer_ir::Point3WithWidth, b: &slicer_ir::Point3WithWidth) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Aligned/AlignedBack seam target (packet 168, TASK-274).
///
/// The seam planner has already chosen the aligned position per layer; the
/// host injects it into `region.resolved_seam()` (ADR-0020 channel) before
/// this module runs. With candidates, this function keeps the candidate snap
/// path. Without candidates, it projects the injected point onto the nearest
/// wall segment. The search radius is deliberately unlimited (packet 168
/// [FWD] note): a far projection is still better than dropping the seam, and
/// the planner's alignment guarantees keep the distance small in practice.
///
/// Returns `None` when there is no injected resolved seam or no wall geometry.
#[derive(Debug, Clone, Copy)]
struct WallSegmentProjection {
    point: slicer_ir::Point3WithWidth,
    wall_index: usize,
    segment_start: usize,
    t: f32,
}

fn interpolate_point(
    start: &slicer_ir::Point3WithWidth,
    end: &slicer_ir::Point3WithWidth,
    t: f32,
) -> slicer_ir::Point3WithWidth {
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    slicer_ir::Point3WithWidth {
        x: lerp(start.x, end.x),
        y: lerp(start.y, end.y),
        z: lerp(start.z, end.z),
        width: lerp(start.width, end.width),
        flow_factor: lerp(start.flow_factor, end.flow_factor),
        overhang_quartile: if t <= 0.5 {
            start.overhang_quartile
        } else {
            end.overhang_quartile
        },
        dist_to_top_mm: lerp(start.dist_to_top_mm, end.dist_to_top_mm),
        overhang_distance_mm: if t <= 0.5 {
            start.overhang_distance_mm
        } else {
            end.overhang_distance_mm
        },
    }
}

fn project_onto_wall_segment(
    target: &slicer_ir::Point3WithWidth,
    wall_loops: &[slicer_sdk::prelude::WallLoop],
) -> Option<WallSegmentProjection> {
    const VERTEX_TOLERANCE: f32 = 0.00001;
    let mut best: Option<(f32, WallSegmentProjection)> = None;

    for (wall_index, loop_) in wall_loops.iter().enumerate() {
        let points = &loop_.path.points;
        if points.is_empty() {
            continue;
        }
        let is_closed = loop_.path.is_closed();
        let effective_len = if is_closed {
            points.len() - 1
        } else {
            points.len()
        };
        if effective_len == 0 {
            continue;
        }

        for segment_start in 0..effective_len {
            let end_index = if segment_start + 1 < effective_len {
                segment_start + 1
            } else {
                0
            };
            let start = points[segment_start];
            let end = points[end_index];
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let length2 = dx * dx + dy * dy;
            let t = if length2 > 0.0 {
                ((target.x - start.x) * dx + (target.y - start.y) * dy) / length2
            } else {
                0.0
            };
            let t = t.clamp(0.0, 1.0);
            let normalized_t = if t.abs() <= VERTEX_TOLERANCE {
                0.0
            } else if (1.0 - t).abs() <= VERTEX_TOLERANCE {
                1.0
            } else {
                t
            };
            let point = match normalized_t {
                0.0 => start,
                1.0 => end,
                _ => interpolate_point(&start, &end, normalized_t),
            };
            let distance2 = dist2_xy(&point, target);
            let projection = WallSegmentProjection {
                point,
                wall_index,
                segment_start,
                t: normalized_t,
            };
            let should_replace = best
                .as_ref()
                .is_none_or(|(best_distance2, best_projection)| {
                    distance2
                        .total_cmp(best_distance2)
                        .then_with(|| projection.wall_index.cmp(&best_projection.wall_index))
                        .then_with(|| projection.segment_start.cmp(&best_projection.segment_start))
                        .is_lt()
                });
            if should_replace {
                best = Some((distance2, projection));
            }
        }
    }

    best.map(|(_, projection)| projection)
}

fn default_wall_feature_flags() -> slicer_ir::WallFeatureFlags {
    slicer_ir::WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: std::collections::HashMap::new(),
    }
}

fn insert_projected_point(
    loop_: &slicer_sdk::prelude::WallLoop,
    projection: WallSegmentProjection,
) -> slicer_sdk::prelude::WallLoop {
    let points = &loop_.path.points;
    let is_closed = loop_.path.is_closed();
    let effective_len = if is_closed {
        points.len() - 1
    } else {
        points.len()
    };
    if effective_len == 0 {
        return loop_.clone();
    }

    let insert_at = projection.segment_start + 1;
    let mut effective_points = points[..effective_len].to_vec();
    effective_points.insert(insert_at, projection.point);

    let width_at = |index: usize| {
        loop_
            .width_profile
            .widths
            .get(index)
            .copied()
            .unwrap_or(points[index].width)
    };
    let start_width = width_at(projection.segment_start);
    let end_index = if projection.segment_start + 1 < effective_len {
        projection.segment_start + 1
    } else {
        0
    };
    let end_width = width_at(end_index);
    let inserted_width = start_width + (end_width - start_width) * projection.t;
    let mut effective_widths: Vec<f32> = (0..effective_len).map(width_at).collect();
    effective_widths.insert(insert_at, inserted_width);

    let flag_at = |index: usize| {
        loop_
            .feature_flags
            .get(index)
            .cloned()
            .or_else(|| loop_.feature_flags.last().cloned())
            .unwrap_or_else(default_wall_feature_flags)
    };
    let mut effective_flags: Vec<_> = (0..effective_len).map(flag_at).collect();
    let inserted_flag = if projection.t <= 0.5 {
        effective_flags[projection.segment_start].clone()
    } else {
        effective_flags[end_index].clone()
    };
    effective_flags.insert(insert_at, inserted_flag);
    if is_closed {
        if let Some(first_flag) = effective_flags.first().cloned() {
            effective_flags.push(first_flag);
        }
    }

    effective_points.push(effective_points[0]);
    effective_widths.push(effective_widths[0]);

    let mut inserted_loop = loop_.clone();
    inserted_loop.path.points = effective_points;
    inserted_loop.width_profile.widths = effective_widths;
    inserted_loop.feature_flags = effective_flags;
    inserted_loop
}

fn aligned_seam_location(
    region: &PerimeterRegionView,
    wall_loops: &mut [slicer_sdk::prelude::WallLoop],
) -> Option<(slicer_ir::Point3WithWidth, usize, usize)> {
    let injected = region.resolved_seam()?.point;
    let global_projection = project_onto_wall_segment(&injected, wall_loops);
    let preferred_projection = region.resolved_seam().and_then(|seam| {
        let wall_index = usize::try_from(seam.wall_index).ok()?;
        let loop_ = wall_loops.get(wall_index)?;
        let mut projection = project_onto_wall_segment(&injected, std::slice::from_ref(loop_))?;
        projection.wall_index = wall_index;
        Some(projection)
    });
    let projection = match (global_projection, preferred_projection) {
        (Some(global), Some(preferred)) => {
            let global_distance = dist2_xy(&global.point, &injected);
            let preferred_distance = dist2_xy(&preferred.point, &injected);
            if preferred_distance <= global_distance + 0.000001 {
                preferred
            } else {
                global
            }
        }
        (Some(global), None) => global,
        (None, Some(preferred)) => preferred,
        (None, None) => return None,
    };
    let wall_index = projection.wall_index;
    let loop_ = wall_loops.get(wall_index)?;
    let effective_len = if loop_.path.is_closed() {
        loop_.path.points.len().checked_sub(1)?
    } else {
        loop_.path.points.len()
    };
    if effective_len == 0 {
        return None;
    }

    let start_idx = if projection.t > 0.0 && projection.t < 1.0 {
        let start_idx = projection.segment_start + 1;
        let inserted = insert_projected_point(loop_, projection);
        wall_loops[wall_index] = inserted;
        start_idx
    } else if projection.t <= 0.0 {
        projection.segment_start
    } else if loop_.path.is_closed() {
        (projection.segment_start + 1) % effective_len
    } else {
        projection.segment_start + 1
    };

    Some((projection.point, wall_index, start_idx))
}

fn find_seam_location(
    wall_loops: &[slicer_sdk::prelude::WallLoop],
    seam: &slicer_ir::Point3WithWidth,
) -> Option<(usize, usize)> {
    wall_loops
        .iter()
        .enumerate()
        .find_map(|(wall_index, loop_)| {
            loop_
                .path
                .points
                .iter()
                .position(|point| {
                    (point.x - seam.x).abs() < 0.001
                        && (point.y - seam.y).abs() < 0.001
                        && (point.z - seam.z).abs() < 0.001
                })
                .map(|start_idx| (wall_index, start_idx))
        })
}

fn rotate_wall_loop(
    loop_: &slicer_sdk::prelude::WallLoop,
    start_idx: usize,
) -> slicer_sdk::prelude::WallLoop {
    debug_assert_eq!(
        loop_.width_profile.widths.len(),
        loop_.path.points.len(),
        "width_profile.widths must have the same length as path.points"
    );

    // Closure-aware rotation: wall loops carry an explicit closing repeat.
    // Rotate the N effective points, then re-append the new first as the
    // closing repeat. Parallel arrays (feature_flags, width_profile.widths)
    // follow the same shape with closing repeats.
    let total = loop_.path.points.len();
    let is_closed = loop_.path.is_closed();
    let effective = if is_closed { total - 1 } else { total };
    if effective == 0 {
        return loop_.clone();
    }
    let start_idx = start_idx % effective;

    let mut rotated_points = Vec::with_capacity(total);
    for i in 0..effective {
        rotated_points.push(loop_.path.points[(start_idx + i) % effective]);
    }
    if is_closed {
        rotated_points.push(rotated_points[0]);
    }

    let mut rotated_flags = Vec::with_capacity(loop_.feature_flags.len());
    for i in 0..effective {
        rotated_flags.push(loop_.feature_flags[(start_idx + i) % effective].clone());
    }
    if is_closed {
        if let Some(first_flag) = rotated_flags.first().cloned() {
            rotated_flags.push(first_flag);
        }
    }
    let mut rotated_widths = Vec::with_capacity(loop_.width_profile.widths.len());
    for i in 0..effective {
        rotated_widths.push(loop_.width_profile.widths[(start_idx + i) % effective]);
    }
    if is_closed {
        if let Some(first_w) = rotated_widths.first().copied() {
            rotated_widths.push(first_w);
        }
    }

    let mut rotated_loop = loop_.clone();
    rotated_loop.path.points = rotated_points;
    rotated_loop.feature_flags = rotated_flags;
    rotated_loop.width_profile.widths = rotated_widths;
    rotated_loop
}

fn local_corner_angle(
    loop_: &slicer_sdk::prelude::WallLoop,
    seam: &slicer_ir::Point3WithWidth,
) -> Option<(f32, [f32; 2])> {
    let points = &loop_.path.points;
    let effective = points.len().checked_sub(1)?;
    if effective < 3 || !loop_.path.is_closed() {
        return None;
    }
    let index = points[..effective]
        .iter()
        .position(|point| (point.x - seam.x).abs() < 0.001 && (point.y - seam.y).abs() < 0.001);
    let Some(index) = index else {
        let projection = project_onto_wall_segment(seam, std::slice::from_ref(loop_))?;
        if projection.t <= 0.00001
            || 1.0 - projection.t <= 0.00001
            || dist2_xy(&projection.point, seam) > 0.000001
        {
            return None;
        }
        // An aligned seam inserted inside an edge has no candidate-local
        // corner metadata; its deterministic geometry approximation is a
        // straight (zero-angle) point.
        return Some((0.0, [0.0, 0.0]));
    };
    let signed_area = signed_shoelace_area(&loop_.path)?;
    let winding_sign = if signed_area > 0.0 {
        1.0
    } else if signed_area < 0.0 {
        -1.0
    } else {
        return None;
    };
    let previous = points[(index + effective - 1) % effective];
    let current = points[index];
    let next = points[(index + 1) % effective];
    let incoming = [current.x - previous.x, current.y - previous.y];
    let outgoing = [next.x - current.x, next.y - current.y];
    let incoming_len = incoming[0].hypot(incoming[1]);
    let outgoing_len = outgoing[0].hypot(outgoing[1]);
    if incoming_len == 0.0 || outgoing_len == 0.0 {
        return None;
    }
    // Canonical candidate angles are measured after normalizing the contour
    // to counter-clockwise order, regardless of the wall's stored winding.
    let angle = winding_sign
        * (incoming[0] * outgoing[1] - incoming[1] * outgoing[0])
            .atan2(incoming[0] * outgoing[0] + incoming[1] * outgoing[1]);
    let toward_corner = [
        incoming[0] / incoming_len - outgoing[0] / outgoing_len,
        incoming[1] / incoming_len - outgoing[1] / outgoing_len,
    ];
    Some((angle, toward_corner))
}

fn point_in_closed_contour(
    point: &slicer_ir::Point3WithWidth,
    contour: &slicer_ir::ExtrusionPath3D,
) -> bool {
    if !contour.is_closed() || contour.points.len() < 4 {
        return false;
    }

    let points = &contour.points[..contour.points.len() - 1];
    let mut inside = false;
    for index in 0..points.len() {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let px = point.x - start.x;
        let py = point.y - start.y;
        let cross = dx * py - dy * px;
        let cross_scale = (dx * py).abs() + (dy * px).abs();
        if cross.abs() <= f32::EPSILON * cross_scale.max(1.0)
            && point.x >= start.x.min(end.x)
            && point.x <= start.x.max(end.x)
            && point.y >= start.y.min(end.y)
            && point.y <= start.y.max(end.y)
        {
            return true;
        }

        if (start.y > point.y) != (end.y > point.y)
            && point.x < start.x + (point.y - start.y) * dx / dy
        {
            inside = !inside;
        }
    }
    inside
}

fn signed_shoelace_area(contour: &slicer_ir::ExtrusionPath3D) -> Option<f64> {
    if !contour.is_closed() || contour.points.len() < 4 {
        return None;
    }

    let points = &contour.points[..contour.points.len() - 1];
    let twice_area = (0..points.len()).fold(0.0_f64, |area, index| {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        area + f64::from(start.x) * f64::from(end.y) - f64::from(end.x) * f64::from(start.y)
    });
    Some(twice_area * 0.5)
}

fn absolute_shoelace_area(contour: &slicer_ir::ExtrusionPath3D) -> Option<f64> {
    signed_shoelace_area(contour).map(f64::abs)
}

fn stagger_inner_wall(
    loop_: &slicer_sdk::prelude::WallLoop,
    outer_seam: &slicer_ir::Point3WithWidth,
    outer_corner: Option<(f32, [f32; 2])>,
) -> slicer_sdk::prelude::WallLoop {
    let points = &loop_.path.points;
    if loop_.path.role != ExtrusionRole::InnerWall || points.len() < 3 || !loop_.path.is_closed() {
        return loop_.clone();
    }

    let mut projection = match project_onto_wall_segment(outer_seam, std::slice::from_ref(loop_)) {
        Some(projection) => projection,
        None => return loop_.clone(),
    };
    let mut depth = dist2_xy(&projection.point, outer_seam).sqrt();

    if let Some((angle, toward_corner)) = outer_corner {
        let beta = (angle / 2.0).cos().abs();
        if angle < -f32::EPSILON && beta > f32::EPSILON {
            // Match Orca's concave-corner correction by overshooting along the
            // outer vertex bisector and projecting that target onto this wall.
            let corrected_depth = std::f32::consts::SQRT_2 * depth / beta;
            let target = slicer_ir::Point3WithWidth {
                x: outer_seam.x + corrected_depth * toward_corner[0] * 0.5,
                y: outer_seam.y + corrected_depth * toward_corner[1] * 0.5,
                ..*outer_seam
            };
            if let Some(corrected) = project_onto_wall_segment(&target, std::slice::from_ref(loop_))
            {
                projection = corrected;
            }
            depth = corrected_depth;
        } else {
            depth = depth * beta / std::f32::consts::SQRT_2;
        }
    }
    // PnP IR has no canonical candidate-local angle metadata, so geometry
    // supplies a deterministic approximation; the minimum-width clamp remains.
    let effective = points.len() - 1;
    let width_at = |index: usize| {
        loop_
            .width_profile
            .widths
            .get(index)
            .copied()
            .unwrap_or(points[index].width)
    };
    let segment_end = (projection.segment_start + 1) % effective;
    let projected_width = width_at(projection.segment_start)
        + (width_at(segment_end) - width_at(projection.segment_start)) * projection.t;
    depth = depth.max(projected_width);

    let segment_length = |start: usize| {
        let end = (start + 1) % effective;
        (points[end].x - points[start].x).hypot(points[end].y - points[start].y)
    };
    let circumference: f32 = (0..effective).map(segment_length).sum();
    if circumference <= 0.0 {
        return loop_.clone();
    }
    let mut remaining = depth % circumference;
    let mut segment = projection.segment_start;
    let mut t = projection.t;
    loop {
        let length = segment_length(segment);
        let available = length * (1.0 - t);
        if length > 0.0 && remaining < available {
            t += remaining / length;
            break;
        }
        remaining -= available;
        segment = (segment + 1) % effective;
        t = 0.0;
        if remaining <= f32::EPSILON {
            break;
        }
    }

    const VERTEX_TOLERANCE: f32 = 0.00001;
    if t <= VERTEX_TOLERANCE {
        return rotate_wall_loop(loop_, segment);
    }
    if 1.0 - t <= VERTEX_TOLERANCE {
        return rotate_wall_loop(loop_, (segment + 1) % effective);
    }
    let end = (segment + 1) % effective;
    let staggered_projection = WallSegmentProjection {
        point: interpolate_point(&points[segment], &points[end], t),
        wall_index: 0,
        segment_start: segment,
        t,
    };
    let inserted = insert_projected_point(loop_, staggered_projection);
    rotate_wall_loop(&inserted, segment + 1)
}

#[slicer_module]
impl LayerModule for SeamPlacer {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = match config.get("seam_position") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "nearest" => SeamMode::Nearest,
                "rear" => SeamMode::Rear,
                "random" => SeamMode::Random,
                "aligned" => SeamMode::Aligned,
                "aligned_back" => SeamMode::AlignedBack,
                other => {
                    return Err(ModuleError::fatal(
                        1,
                        format!("unknown seam_position: {other}"),
                    ));
                }
            },
            _ => SeamMode::Nearest,
        };
        let staggered_inner_seams = match config.get("staggered_inner_seams") {
            Some(ConfigValue::Bool(value)) => *value,
            _ => false,
        };

        Ok(Self {
            mode,
            staggered_inner_seams,
        })
    }

    fn run_wall_postprocess(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Contract: every region's wall loops MUST reach the output. Seam
        // rotation is a best-effort optimisation that no-ops when (a) the
        // region has no seam information at all, or (b) the source seam's
        // coordinates don't match any wall-loop vertex within tolerance
        // (`seam-planner-default` currently emits mesh-corner coords while
        // walls live on the inset boundary — a known pre-existing gap).
        //
        // Dropping a region's walls here would propagate through
        // `convert_perimeter_output` (no bucket → no PerimeterRegion entry)
        // and corrupt the `(object_id, region_id)` pairing in
        // `layer_executor::commit_layer_outputs` for multi-region prints.
        let mut degraded_error = None;
        let mut empty_wall_loop_error = None;
        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            if matches!(self.mode, SeamMode::Aligned | SeamMode::AlignedBack)
                && region.resolved_seam().is_none()
                && degraded_error.is_none()
            {
                degraded_error = Some(ModuleError::non_fatal(
                    6,
                    format!(
                        "missing seam plan entry (layer={}, object={}, region_id={}, variant_chain=[])",
                        layer_index,
                        region.object_id(),
                        region.region_id(),
                    ),
                ));
            }
            let mut wall_loops = region.wall_loops().to_vec();
            if wall_loops.is_empty() {
                continue;
            }

            // A region with an empty `seam_candidates` list AND no
            // `resolved_seam` has no usable seam information — most commonly a
            // `seam_blocker` paint region excluded every corner candidate at
            // perimeter-generation time (superseding packet 108's fatal-on-empty
            // behavior). This is NOT
            // fatal: the upstream sharpest-vertex fallback in `slicer_core`
            // normally guarantees a candidate exists, and OrcaSlicer degrades
            // rather than aborting the slice. Above all, the HIGH-2
            // wall-preservation invariant requires every region's walls to reach
            // the output — dropping them (or failing the layer) corrupts the
            // `(object_id, region_id)` pairing in `commit_layer_outputs` for
            // multi-region prints. The graceful path below emits the walls
            // pristine with no resolved seam.

            // Compute the optional seam target. `None` → emit walls pristine
            // (no rotation, no `set_resolved_seam` call).
            let seam_target: Option<(slicer_ir::Point3WithWidth, usize, usize)> = (|| {
                match self.mode {
                    // Aligned modes consume the planner's host-injected
                    // resolved seam and snap it onto real geometry; they do
                    // NOT score candidates. See `aligned_seam_location`.
                    SeamMode::Aligned | SeamMode::AlignedBack => {
                        if region.resolved_seam().is_none() {
                            let point = select_seam_candidate(
                                SeamMode::Nearest,
                                layer_index,
                                region.seam_candidates(),
                            )?
                            .position;
                            let (wall_idx, start_idx) = find_seam_location(&wall_loops, &point)?;
                            Some((point, wall_idx, start_idx))
                        } else {
                            aligned_seam_location(region, &mut wall_loops)
                        }
                    }
                    // Nearest/rear/random keep the candidate-preference path.
                    SeamMode::Nearest | SeamMode::Rear | SeamMode::Random => {
                        let point = if let Some(candidate) =
                            select_seam_candidate(self.mode, layer_index, region.seam_candidates())
                        {
                            candidate.position
                        } else {
                            region.resolved_seam().as_ref()?.point
                        };
                        let (wall_idx, start_idx) = find_seam_location(&wall_loops, &point)?;
                        Some((point, wall_idx, start_idx))
                    }
                }
            })();

            if self.staggered_inner_seams {
                if let Some((point, outer_wall_index, _)) = seam_target.filter(|(_, index, _)| {
                    let target = &wall_loops[*index];
                    target.path.role == ExtrusionRole::OuterWall
                        && target.loop_type == slicer_ir::LoopType::Outer
                }) {
                    let outer_corner = local_corner_angle(&wall_loops[outer_wall_index], &point);
                    let outer_contour = wall_loops[outer_wall_index].path.clone();
                    let selected_outer_area = absolute_shoelace_area(&outer_contour);
                    let associated_inner_indices: Vec<_> = wall_loops
                        .iter()
                        .enumerate()
                        .filter_map(|(inner_index, inner)| {
                            let representative = inner.path.points.first()?;
                            let selected_outer_area = selected_outer_area?;
                            if inner.path.role != ExtrusionRole::InnerWall
                                || inner.loop_type != slicer_ir::LoopType::Inner
                                || !inner.path.is_closed()
                                || !point_in_closed_contour(representative, &outer_contour)
                            {
                                return None;
                            }
                            let inside_smaller_outer =
                                wall_loops.iter().enumerate().any(|(outer_index, outer)| {
                                    outer_index != outer_wall_index
                                        && outer.path.role == ExtrusionRole::OuterWall
                                        && outer.loop_type == slicer_ir::LoopType::Outer
                                        && absolute_shoelace_area(&outer.path)
                                            .is_some_and(|area| area < selected_outer_area)
                                        && point_in_closed_contour(representative, &outer.path)
                                });
                            (!inside_smaller_outer).then_some(inner_index)
                        })
                        .collect();
                    for inner_index in associated_inner_indices {
                        wall_loops[inner_index] =
                            stagger_inner_wall(&wall_loops[inner_index], &point, outer_corner);
                    }
                }
            }

            if let Some((point, wall_idx, _)) = &seam_target {
                output
                    .set_resolved_seam(*point, *wall_idx as u32)
                    .map_err(|e| ModuleError::fatal(3, e))?;
            }

            for (wall_index, loop_) in wall_loops.iter().enumerate() {
                let emitted_loop = match seam_target {
                    Some((_, target_wall_index, start_idx)) if wall_index == target_wall_index => {
                        rotate_wall_loop(loop_, start_idx)
                    }
                    _ => loop_.clone(),
                };

                if emitted_loop.path.points.is_empty() && empty_wall_loop_error.is_none() {
                    empty_wall_loop_error = Some(ModuleError::non_fatal(
                        7,
                        format!(
                            "degenerate empty wall loop (no points) at wall_index={wall_index}"
                        ),
                    ));
                }
                let emitted_point = emitted_loop
                    .path
                    .points
                    .first()
                    .copied()
                    .or_else(|| region.resolved_seam().map(|seam| seam.point))
                    .unwrap_or(slicer_ir::Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        width: 0.0,
                        flow_factor: 0.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    });

                output
                    .push_reordered_wall_loop(emitted_point, wall_index as u32, emitted_loop)
                    .map_err(|e| ModuleError::fatal(5, e))?;
            }
        }

        empty_wall_loop_error.or(degraded_error).map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_bonus_concave_is_lowest() {
        assert!(reason_bonus(SeamReason::Concave) < reason_bonus(SeamReason::Sharp));
        assert!(reason_bonus(SeamReason::Sharp) < reason_bonus(SeamReason::Aligned));
    }

    #[test]
    fn reason_bonus_user_forced_wins() {
        assert!(reason_bonus(SeamReason::UserForced) < reason_bonus(SeamReason::Concave));
    }

    #[test]
    fn seam_position_display() {
        let s = SeamPlacer {
            mode: SeamMode::Nearest,
            staggered_inner_seams: false,
        };
        assert_eq!(s.seam_position(), "nearest");

        let s = SeamPlacer {
            mode: SeamMode::Rear,
            staggered_inner_seams: false,
        };
        assert_eq!(s.seam_position(), "rear");

        let s = SeamPlacer {
            mode: SeamMode::Random,
            staggered_inner_seams: false,
        };
        assert_eq!(s.seam_position(), "random");
    }
}
