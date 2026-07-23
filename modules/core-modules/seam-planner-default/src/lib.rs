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
#[allow(dead_code)]
mod visibility;

use slicer_sdk::prelude::*;

use crate::comparator::SeamSetup;

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
    let mut candidates = Vec::new();
    for polygon in &region.ex_polygons {
        for point in polygon
            .contour
            .points
            .iter()
            .chain(polygon.holes.iter().flat_map(|hole| hole.points.iter()))
        {
            candidates.push(ScoredSeamCandidate {
                position: Point3WithWidth {
                    x: units_to_mm(point.x),
                    y: units_to_mm(point.y),
                    z: region.z,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                score: 0.0,
                reason: SeamReason {
                    tag: "aligned".to_string(),
                },
            });
        }
    }
    candidates
}

fn choose_region_candidate(
    candidates: &[ScoredSeamCandidate],
    mode: SeamPlannerMode,
    layer_index: u32,
) -> Option<ScoredSeamCandidate> {
    match mode {
        SeamPlannerMode::Aligned | SeamPlannerMode::Nearest => candidates
            .iter()
            .min_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(left.position.x.total_cmp(&right.position.x))
            })
            .cloned(),
        SeamPlannerMode::AlignedBack | SeamPlannerMode::Rear => candidates
            .iter()
            .max_by(|left, right| {
                left.position
                    .y
                    .total_cmp(&right.position.y)
                    .then(right.position.x.total_cmp(&left.position.x))
            })
            .cloned(),
        SeamPlannerMode::Random => candidates
            .get(layer_index as usize % candidates.len())
            .cloned(),
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
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
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
