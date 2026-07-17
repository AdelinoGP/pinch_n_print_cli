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
/// this module runs. This function snaps that injected point to the nearest
/// `seam_candidates()` position by 2D XY distance so the seam lands on an
/// actual wall vertex. The snap radius is deliberately unlimited (packet
/// 168 [FWD] note): a far snap is still better than dropping the seam, and
/// the planner's alignment guarantees keep the distance small in practice.
///
/// Fallback when the candidate list is empty: nearest wall-loop vertex.
/// Returns `None` (→ pristine wall emission) when there is no injected
/// resolved seam or no wall vertex exists.
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
    snapped.or_else(|| {
        // Empty candidate list → snap directly to the nearest wall vertex.
        wall_loops
            .iter()
            .flat_map(|loop_| loop_.path.points.iter())
            .min_by(|left, right| {
                dist2_xy(left, &injected)
                    .total_cmp(&dist2_xy(right, &injected))
                    .then_with(|| left.y.total_cmp(&right.y))
                    .then_with(|| left.x.total_cmp(&right.x))
            })
            .copied()
    })
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

    // Closure-aware rotation: wall loops carry an explicit closing repeat
    // (OrcaSlicer `ExtrusionPath::is_closed()` convention; see
    // `crates/slicer-ir/src/slice_ir.rs::ExtrusionPath3D::is_closed`). A naïve
    // modular rotation over the full N+1 points produces an invalid loop
    // (start point appears twice in the middle). Rotate the N effective
    // points, then re-append the new first as the closing repeat. Parallel
    // arrays (feature_flags, width_profile.widths) follow the same shape.
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
        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            let wall_loops = region.wall_loops();
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
                        aligned_seam_target(region, wall_loops)?
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
                let (wall_idx, start_idx) = find_seam_location(wall_loops, &point)?;
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

                let emitted_point = emitted_loop.path.points.first().copied().ok_or_else(|| {
                    ModuleError::fatal(4, "wall loop must contain at least one point")
                })?;

                output
                    .push_reordered_wall_loop(emitted_point, wall_index as u32, emitted_loop)
                    .map_err(|e| ModuleError::fatal(5, e))?;
            }
        }

        Ok(())
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
