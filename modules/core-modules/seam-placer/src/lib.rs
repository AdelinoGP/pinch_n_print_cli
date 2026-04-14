//! Seam placer module.
//!
//! Implements `LayerModule::run_wall_postprocess` for the `Layer::WallPostProcess` stage.
//! Reads seam candidates from perimeter regions and selects the best candidate
//! based on configurable seam placement mode (nearest, rear, random).
//!
//! Per OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp/cpp.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, SeamReason};
use slicer_sdk::builders::PerimeterOutputBuilder;
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

impl LayerModule for SeamPlacer {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = match config.fields.get("seam_mode") {
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
        // Collect all candidates from all regions
        let mut all_candidates = Vec::new();
        for region in regions {
            for c in region.seam_candidates() {
                all_candidates.push(c);
            }
        }

        if all_candidates.is_empty() {
            return Ok(());
        }

        let best = match self.mode {
            SeamMode::Nearest => {
                // Select candidate with lowest effective score (score + reason bonus)
                all_candidates
                    .iter()
                    .min_by(|a, b| {
                        let score_a = a.score + reason_bonus(a.reason);
                        let score_b = b.score + reason_bonus(b.reason);
                        score_a
                            .partial_cmp(&score_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .expect("non-empty")
            }
            SeamMode::Rear => {
                // Select candidate with highest Y coordinate
                all_candidates
                    .iter()
                    .max_by(|a, b| {
                        a.position
                            .y
                            .partial_cmp(&b.position.y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .expect("non-empty")
            }
            SeamMode::Random => {
                // Deterministic pseudo-random selection based on layer index
                let idx = (layer_index as usize) % all_candidates.len();
                all_candidates[idx]
            }
        };

        let _ = output.set_resolved_seam(best.position, 0);

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
