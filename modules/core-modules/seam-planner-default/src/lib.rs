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
//! Default seam planner for Pinch 'n Print.
//!
//! The planner consumes the host-supplied active `SliceIR` region boundaries.
//! Mesh geometry remains available to the prepass interface for compatibility,
//! but is not a candidate source.

#![warn(missing_docs)]
#![warn(unused_imports)]

#[allow(dead_code)]
mod align;
#[allow(dead_code)]
mod comparator;
#[allow(dead_code)]
mod contours;
// The ray-cast visibility port (`compute_global_visibility`,
// `build_seam_candidates`, and ~20 supporting items) is dormant by design and
// exercised only by tests, so the module keeps a blanket allow.
//
// The paint-classification half is NOT dormant: `candidate_paint_classification`
// is imported below and called from `region_candidates`. Packet 206 originally
// shipped the exact-semantic discriminator here with *no* production caller,
// and this blanket allow is exactly what suppressed the warning that would
// have caught it. The lint cannot distinguish the two halves, so the guard is
// a production-path test instead: `seam_paint_moves_planner_resolved_seam`
// (`tests/seam_region_aware_planning_tdd.rs`) drives
// `run_aligned_planning_entries` and fails if the classifier stops being
// consulted. Do not delete it.
#[allow(dead_code)]
mod visibility;

use slicer_sdk::prelude::*;

use crate::comparator::EnforcedBlockedSeamPoint;
use crate::comparator::SeamSetup;
use crate::visibility::candidate_paint_classification;

/// Default extrusion flow width used for seam scoring. Units: mm.
const DEFAULT_FLOW_WIDTH_MM: f32 = 0.4;

/// Seam planning mode parsed from the `seam_mode` config key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeamPlannerMode {
    /// Score-based nearest selection (default).
    Nearest,
    /// Rear-of-bed bias.
    Rear,
    /// Pseudo-random per-layer selection.
    Random,
    /// Vertically aligned seams.
    Aligned,
    /// Vertically aligned seams biased to the rear.
    AlignedBack,
}

/// Default seam planner that selects seam positions from active region
/// boundaries.
pub struct SeamPlannerDefault {
    /// Seam placement mode.
    mode: SeamPlannerMode,
}

fn region_candidates(region: &SeamPlanningRegionInput) -> Vec<ScoredSeamCandidate> {
    let width = if region.scoring_width.is_finite() && region.scoring_width > 0.0 {
        region.scoring_width
    } else {
        DEFAULT_FLOW_WIDTH_MM
    };
    let paint_annotations: Vec<_> = region
        .segment_annotations
        .iter()
        .map(|(semantic, contours)| (semantic.clone(), contours.as_slice()))
        .collect();
    let paint_annotations = (!paint_annotations.is_empty()).then_some(paint_annotations.as_slice());
    let mut candidates = Vec::new();
    for (contour_idx, polygon) in region.ex_polygons.iter().enumerate() {
        let points = polygon
            .contour
            .points
            .iter()
            .enumerate()
            .map(|(vertex_idx, point)| (point, Some(vertex_idx)))
            .chain(
                polygon
                    .holes
                    .iter()
                    .flat_map(|hole| hole.points.iter())
                    .map(|point| (point, None)),
            );
        for (point, vertex_idx) in points {
            let (point_type, central_enforcer) = vertex_idx
                .map(|vertex_idx| {
                    candidate_paint_classification(paint_annotations, contour_idx, vertex_idx)
                })
                .unwrap_or((EnforcedBlockedSeamPoint::Neutral, false));
            if point_type == EnforcedBlockedSeamPoint::Blocked {
                continue;
            }
            candidates.push(ScoredSeamCandidate {
                position: Point3WithWidth {
                    x: units_to_mm(point.x),
                    y: units_to_mm(point.y),
                    z: region.z,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    overhang_distance_mm: None,
                    dist_to_top_mm: 0.0,
                },
                score: if point_type == EnforcedBlockedSeamPoint::Enforced {
                    if central_enforcer {
                        2.0
                    } else {
                        1.0
                    }
                } else {
                    0.0
                },
                reason: SeamReason {
                    tag: if point_type == EnforcedBlockedSeamPoint::Enforced {
                        "enforced".to_string()
                    } else {
                        "aligned".to_string()
                    },
                },
            });
        }
    }
    candidates
}

/// Pick one candidate for a region.
///
/// Paint priority is applied first and applies to **every** mode: enforced
/// vertices score above neutral ones (`region_candidates`), so restricting to
/// the maximum score selects the enforced set whenever seam paint is present
/// and is a no-op otherwise (every neutral candidate scores 0.0). The mode
/// then only breaks ties *within* that set — an enforcer must not be
/// overridden by a geometric preference.
///
/// Note this narrows `Random`'s pool: `layer_index % candidates.len()` cycles
/// over the enforced set rather than the whole contour on painted models.
/// That is intended — random seam placement inside a painted enforcer region
/// is still enforced — and is pinned by
/// `random_mode_cycles_only_enforced_candidates` below.
fn choose_region_candidate(
    candidates: &[ScoredSeamCandidate],
    mode: SeamPlannerMode,
    layer_index: u32,
) -> Option<ScoredSeamCandidate> {
    let max_score = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::NEG_INFINITY, f32::max);
    let candidates = candidates
        .iter()
        .filter(|candidate| candidate.score == max_score)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    match mode {
        SeamPlannerMode::Aligned | SeamPlannerMode::Nearest => candidates
            .iter()
            .min_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(left.position.x.total_cmp(&right.position.x))
            })
            .map(|candidate| (*candidate).clone()),
        SeamPlannerMode::AlignedBack | SeamPlannerMode::Rear => candidates
            .iter()
            .max_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(right.position.x.total_cmp(&left.position.x))
            })
            .map(|candidate| (*candidate).clone()),
        SeamPlannerMode::Random => candidates
            .get(layer_index as usize % candidates.len())
            .map(|candidate| (*candidate).clone()),
    }
}

fn run_region_planning_entries(
    region_input: &SeamPlanningView,
    mode: SeamPlannerMode,
) -> Vec<SeamPlanEntry> {
    let mut regions: Vec<&SeamPlanningRegionInput> = region_input.regions.iter().collect();
    regions.sort_by(|left, right| {
        left.global_layer_index
            .cmp(&right.global_layer_index)
            .then(left.object_id.cmp(&right.object_id))
            .then(left.region_id.cmp(&right.region_id))
            .then(left.variant_chain.cmp(&right.variant_chain))
    });

    let mut entries = Vec::new();
    let mut previous_key: Option<(u32, String, String, Vec<(String, slicer_ir::PaintValue)>)> =
        None;
    for region in regions {
        let key = (
            region.global_layer_index,
            region.object_id.clone(),
            region.region_id.clone(),
            region.variant_chain.clone(),
        );
        if previous_key.as_ref() == Some(&key) {
            continue;
        }
        previous_key = Some(key);

        let scored_candidates = region_candidates(region);
        let Some(chosen) =
            choose_region_candidate(&scored_candidates, mode, region.global_layer_index)
        else {
            continue;
        };
        entries.push(SeamPlanEntry {
            global_layer_index: region.global_layer_index,
            object_id: region.object_id.clone(),
            region_id: region.region_id.clone(),
            variant_chain: region.variant_chain.clone(),
            chosen_position: chosen.position,
            chosen_wall_index: 0,
            scored_candidates,
        });
    }
    entries
}

/// Build seam-plan entries directly from supplied active region polygons.
///
/// This pure entry point is used by the per-region contract tests. Mesh
/// vertices and layer-plan Z values are deliberately absent from this path.
pub fn run_aligned_planning_entries(
    region_input: &SeamPlanningView,
    aligned_back: bool,
) -> Vec<SeamPlanEntry> {
    run_region_planning_entries(
        region_input,
        if aligned_back {
            SeamPlannerMode::AlignedBack
        } else {
            SeamPlannerMode::Aligned
        },
    )
}

fn run_aligned_planning(
    setup: SeamSetup,
    _objects: &[MeshObjectView],
    _layer_plan: &LayerPlanView,
    region_input: &SeamPlanningView,
    output: &mut SeamPlanningOutput,
) -> Result<(), ModuleError> {
    let mode = match setup {
        SeamSetup::Aligned => SeamPlannerMode::Aligned,
        SeamSetup::AlignedBack => SeamPlannerMode::AlignedBack,
        _ => unreachable!("aligned planning only accepts aligned setups"),
    };
    for entry in run_region_planning_entries(region_input, mode) {
        output
            .push_seam_plan(entry)
            .map_err(|e| ModuleError::fatal(1, format!("push_seam_plan failed: {e}")))?;
    }
    Ok(())
}

#[slicer_module]
impl PrepassModule for SeamPlannerDefault {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = match config.get("seam_mode") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "nearest" => SeamPlannerMode::Nearest,
                "rear" => SeamPlannerMode::Rear,
                "random" => SeamPlannerMode::Random,
                "aligned" => SeamPlannerMode::Aligned,
                "aligned_back" => SeamPlannerMode::AlignedBack,
                other => {
                    return Err(ModuleError::fatal(1, format!("unknown seam_mode: {other}")));
                }
            },
            _ => SeamPlannerMode::Nearest,
        };

        Ok(Self { mode })
    }

    fn run_seam_planning(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        output: &mut SeamPlanningOutput,
        _config: &ConfigView,
        region_input: &SeamPlanningView,
    ) -> Result<(), ModuleError> {
        match self.mode {
            SeamPlannerMode::Aligned => run_aligned_planning(
                SeamSetup::Aligned,
                objects,
                layer_plan,
                region_input,
                output,
            ),
            SeamPlannerMode::AlignedBack => run_aligned_planning(
                SeamSetup::AlignedBack,
                objects,
                layer_plan,
                region_input,
                output,
            ),
            mode => {
                for entry in run_region_planning_entries(region_input, mode) {
                    output.push_seam_plan(entry).map_err(|e| {
                        ModuleError::fatal(1, format!("push_seam_plan failed: {e}"))
                    })?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(x: f32, y: f32, score: f32) -> ScoredSeamCandidate {
        ScoredSeamCandidate {
            position: Point3WithWidth {
                x,
                y,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                overhang_distance_mm: None,
                dist_to_top_mm: 0.0,
            },
            score,
            reason: SeamReason {
                tag: "aligned".to_string(),
            },
        }
    }

    /// Paint priority must gate `Random` like every other mode: the pool it
    /// cycles is the enforced set, not the whole contour.
    #[test]
    fn random_mode_cycles_only_enforced_candidates() {
        let candidates = vec![
            candidate(0.0, 0.0, 0.0),
            candidate(10.0, 0.0, 1.0),
            candidate(10.0, 10.0, 0.0),
            candidate(0.0, 10.0, 1.0),
        ];

        // Two enforced candidates, so the layer index cycles over 2, not 4.
        for layer_index in 0..6u32 {
            let chosen = choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
                .expect("a candidate");
            assert_eq!(chosen.score, 1.0, "Random must not pick a neutral vertex");
        }
        let first = choose_region_candidate(&candidates, SeamPlannerMode::Random, 0).unwrap();
        let second = choose_region_candidate(&candidates, SeamPlannerMode::Random, 1).unwrap();
        let third = choose_region_candidate(&candidates, SeamPlannerMode::Random, 2).unwrap();
        assert_ne!((first.position.x, first.position.y), (second.position.x, second.position.y));
        assert_eq!((first.position.x, first.position.y), (third.position.x, third.position.y));
    }

    /// With no paint every candidate scores 0.0, so the filter is a no-op and
    /// `Random` still cycles the full contour.
    #[test]
    fn random_mode_cycles_all_candidates_when_unpainted() {
        let candidates = vec![
            candidate(0.0, 0.0, 0.0),
            candidate(10.0, 0.0, 0.0),
            candidate(10.0, 10.0, 0.0),
            candidate(0.0, 10.0, 0.0),
        ];

        let picks: Vec<(f32, f32)> = (0..4u32)
            .map(|layer_index| {
                let chosen =
                    choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
                        .expect("a candidate");
                (chosen.position.x, chosen.position.y)
            })
            .collect();

        assert_eq!(picks.len(), 4);
        let mut unique = picks.clone();
        unique.dedup();
        assert_eq!(unique.len(), 4, "all four vertices must be reachable");
    }

    /// An empty candidate list must yield `None` rather than panicking on the
    /// `NEG_INFINITY` fold or the `% candidates.len()` modulo.
    #[test]
    fn empty_candidate_list_yields_none_in_every_mode() {
        for mode in [
            SeamPlannerMode::Aligned,
            SeamPlannerMode::AlignedBack,
            SeamPlannerMode::Nearest,
            SeamPlannerMode::Rear,
            SeamPlannerMode::Random,
        ] {
            assert!(choose_region_candidate(&[], mode, 0).is_none());
        }
    }
}
