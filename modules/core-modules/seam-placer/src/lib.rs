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

use slicer_ir::{ConfigValue, ConfigView, SeamReason};
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
}

impl SeamPlacer {
    /// Returns the seam mode as a string (for testing).
    pub fn seam_mode(&self) -> &str {
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
        // (`aligned_seam_target`). The arms below are a defensive fallback.
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
        let is_closed = points.len() > 1 && points.first() == points.last();
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
    let is_closed = points.len() > 1 && points.first() == points.last();
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

fn aligned_seam_target(
    region: &PerimeterRegionView,
    wall_loops: &[slicer_sdk::prelude::WallLoop],
) -> Option<slicer_ir::Point3WithWidth> {
    let injected = region.resolved_seam()?.point;
    let snapped = region
        .seam_candidates()
        .iter()
        .map(|candidate| candidate.position)
        .min_by(|left, right| {
            dist2_xy(left, &injected)
                .total_cmp(&dist2_xy(right, &injected))
                .then_with(|| left.y.total_cmp(&right.y))
                .then_with(|| left.x.total_cmp(&right.x))
        });
    snapped.or_else(|| project_onto_wall_segment(&injected, wall_loops).map(|p| p.point))
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

#[slicer_module]
impl LayerModule for SeamPlacer {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = match config.get("seam_mode") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "nearest" => SeamMode::Nearest,
                "rear" => SeamMode::Rear,
                "random" => SeamMode::Random,
                "aligned" => SeamMode::Aligned,
                "aligned_back" => SeamMode::AlignedBack,
                other => {
                    return Err(ModuleError::fatal(1, format!("unknown seam_mode: {other}")));
                }
            },
            _ => SeamMode::Nearest,
        };

        Ok(Self { mode })
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
            // perimeter-generation time (D-109-SEAM-FATAL-CORRECTED, superseding
            // packet 108's D-108-SEAM-CONSUMED fatal-on-empty). This is NOT
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
                let point = match self.mode {
                    // Aligned modes consume the planner's host-injected
                    // resolved seam and snap it onto real geometry; they do
                    // NOT score candidates. See `aligned_seam_target`.
                    SeamMode::Aligned | SeamMode::AlignedBack => {
                        if region.resolved_seam().is_none() {
                            select_seam_candidate(
                                SeamMode::Nearest,
                                layer_index,
                                region.seam_candidates(),
                            )?
                            .position
                        } else {
                            let point = aligned_seam_target(region, &wall_loops)?;
                            if region.seam_candidates().is_empty() {
                                if let Some(injected) =
                                    region.resolved_seam().map(|seam| seam.point)
                                {
                                    if let Some(projection) =
                                        project_onto_wall_segment(&injected, &wall_loops)
                                    {
                                        if projection.t > 0.0 && projection.t < 1.0 {
                                            wall_loops[projection.wall_index] =
                                                insert_projected_point(
                                                    &wall_loops[projection.wall_index],
                                                    projection,
                                                );
                                        }
                                    }
                                }
                            }
                            point
                        }
                    }
                    // Nearest/rear/random keep the candidate-preference path.
                    SeamMode::Nearest | SeamMode::Rear | SeamMode::Random => {
                        if let Some(candidate) =
                            select_seam_candidate(self.mode, layer_index, region.seam_candidates())
                        {
                            candidate.position
                        } else {
                            region.resolved_seam().as_ref()?.point
                        }
                    }
                };
                let (wall_idx, start_idx) = find_seam_location(&wall_loops, &point)?;
                Some((point, wall_idx, start_idx))
            })();

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
    fn seam_mode_display() {
        let s = SeamPlacer {
            mode: SeamMode::Nearest,
        };
        assert_eq!(s.seam_mode(), "nearest");

        let s = SeamPlacer {
            mode: SeamMode::Rear,
        };
        assert_eq!(s.seam_mode(), "rear");

        let s = SeamPlacer {
            mode: SeamMode::Random,
        };
        assert_eq!(s.seam_mode(), "random");
    }
}
