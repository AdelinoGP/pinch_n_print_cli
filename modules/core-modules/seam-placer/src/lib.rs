//! Seam placer module.
//!
//! Implements `LayerModule::run_wall_postprocess` for the `Layer::WallPostProcess` stage.
//! Reads resolved seam from perimeter regions and rotates wall loop geometry
//! so path.points[0] is the seam vertex.
//!
//! Per OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp/cpp.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![allow(dead_code)]

use slicer_ir::{ConfigValue, ConfigView, SeamReason};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::error::ModuleError;
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
        SeamMode::Nearest => candidates.iter().min_by(|left, right| {
            effective_score(left)
                .total_cmp(&effective_score(right))
                .then_with(|| left.position.y.total_cmp(&right.position.y))
                .then_with(|| left.position.x.total_cmp(&right.position.x))
        }),
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

fn find_seam_location(
    wall_loops: &[slicer_sdk::prelude::WallLoop],
    seam: &slicer_ir::Point3WithWidth,
) -> Option<(usize, usize)> {
    wall_loops.iter().enumerate().find_map(|(wall_index, loop_)| {
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
    let point_count = loop_.path.points.len();
    let mut rotated_points = Vec::with_capacity(point_count);
    for i in 0..point_count {
        rotated_points.push(loop_.path.points[(start_idx + i) % point_count]);
    }

    let mut rotated_flags = Vec::with_capacity(loop_.feature_flags.len());
    for i in 0..point_count {
        rotated_flags.push(loop_.feature_flags[(start_idx + i) % point_count].clone());
    }

    let mut rotated_widths = Vec::with_capacity(loop_.width_profile.widths.len());
    for i in 0..point_count {
        rotated_widths.push(loop_.width_profile.widths[(start_idx + i) % point_count]);
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
        for region in regions {
            let wall_loops = region.wall_loops();
            if wall_loops.is_empty() {
                continue;
            }

            let seam_position = if let Some(candidate) =
                select_seam_candidate(self.mode, layer_index, region.seam_candidates())
            {
                // wall_index is set to 0 here as a placeholder; the actual wall index
                // is determined below by `find_seam_location` which searches all walls
                // to locate the point and returns the correct (wall_index, start_idx) pair.
                slicer_ir::SeamPosition {
                    point: candidate.position,
                    wall_index: 0,
                }
            } else if let Some(seam) = region.resolved_seam() {
                seam.clone()
            } else {
                continue;
            };

            let Some((target_wall_index, start_idx)) =
                find_seam_location(wall_loops, &seam_position.point)
            else {
                return Err(ModuleError::fatal(
                    2,
                    format!(
                        "resolved seam ({:.3}, {:.3}, {:.3}) was not found in any wall loop",
                        seam_position.point.x, seam_position.point.y, seam_position.point.z
                    ),
                ));
            };

            output
                .set_resolved_seam(seam_position.point, target_wall_index as u32)
                .map_err(|e| ModuleError::fatal(3, e))?;

            for (wall_index, loop_) in wall_loops.iter().enumerate() {
                let emitted_loop = if wall_index == target_wall_index {
                    rotate_wall_loop(loop_, start_idx)
                } else {
                    loop_.clone()
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
