//! Host built-in `PrePass::ShellClassification`.
//!
//! Ports the two-pass OrcaSlicer surface classification:
//! 1. **Pass 1 (depth 0)** — per-region polygon `diff` against the next
//!    active layer (for top exposure) and previous active layer (for bottom
//!    exposure) in that region's own timeline. The diff polygons become the
//!    layer-0 `top_solid_fill` / `bottom_solid_fill`.
//! 2. **Pass 2 (depths 1..k-1)** — shrinking-shadow projection. For each
//!    region layer marked as depth-0, walk outward through the region's
//!    timeline (backward for top, forward for bottom) and `intersection` the
//!    accumulated shadow with each neighbor's polygons. Each non-empty
//!    intersection stamps that neighbor with the minimum reached depth and
//!    unions the shadow into its solid-fill.
//!
//! References:
//! - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541-1892`
//!   (`detect_surfaces_type`) — Pass 1 reference.
//! - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:3928-4132`
//!   (`discover_horizontal_shells`) — Pass 2 reference.
//!
//! See `docs/DEVIATION_LOG.md` for documented divergences (hollow-object
//! continue path not ported; `top_solid_fill` flattened across shell sources).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use slicer_core::polygon_ops::{difference, intersection, union};
use slicer_ir::{ExPolygon, ObjectId, RegionId, RegionKey, SliceIR};

use crate::blackboard::{Blackboard, BlackboardError};

/// Structured failures for `PrePass::ShellClassification`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellClassificationError {
    /// `commit_slice_builtin` (PrePass::Slice) must have committed `SliceIR`
    /// before this stage runs.
    SliceIRNotCommitted,
    /// `commit_region_mapping_builtin` must have committed `RegionMapIR`
    /// before this stage runs.
    RegionMapNotCommitted,
    /// Blackboard replace_slice_ir or related slot manipulation failed.
    Blackboard(BlackboardError),
}

impl fmt::Display for ShellClassificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SliceIRNotCommitted => write!(
                f,
                "PrePass::ShellClassification requires PrePass::Slice to commit SliceIR first"
            ),
            Self::RegionMapNotCommitted => write!(
                f,
                "PrePass::ShellClassification requires PrePass::RegionMapping to commit RegionMapIR first"
            ),
            Self::Blackboard(inner) => write!(
                f,
                "PrePass::ShellClassification blackboard error: {inner}"
            ),
        }
    }
}

impl From<BlackboardError> for ShellClassificationError {
    fn from(value: BlackboardError) -> Self {
        Self::Blackboard(value)
    }
}

impl std::error::Error for ShellClassificationError {}

/// `PrePass::ShellClassification` host built-in entry point. Reads the
/// committed `Vec<SliceIR>` plus `RegionMapIR`, runs the two-pass
/// classification per region timeline, and atomically replaces the
/// blackboard's SliceIR slot with the annotated Vec.
///
/// Build-immutably + commit-atomically: a mid-pass error leaves the prior
/// (depth-0-only) Vec intact; the new Vec is only published on full success.
pub fn commit_shell_classification_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), ShellClassificationError> {
    let old_arc = blackboard
        .slice_ir()
        .ok_or(ShellClassificationError::SliceIRNotCommitted)?
        .clone();
    let region_map = blackboard
        .region_map()
        .ok_or(ShellClassificationError::RegionMapNotCommitted)?
        .clone();

    let mut new_vec: Vec<SliceIR> = old_arc.as_ref().clone();

    // Build per-region timelines: ordered Vec<usize> of slice indices where the
    // (object, region) pair appears. Slices retain their `global_layer_index`
    // ordering by construction (built per the layer plan), so iteration order is
    // already plan-order.
    let timelines = build_region_timelines(&new_vec);

    for ((object_id, region_id), timeline) in &timelines {
        // Look up shell counts via the first slice's RegionKey. Region planning
        // resolved per-layer configs but shell counts are stable across the
        // timeline by construction.
        let (k_top, k_bot) =
            resolve_shell_counts(region_map.as_ref(), object_id, *region_id, timeline);

        // Pass 1: depth-0 classification per layer in this timeline.
        for (pos, &slice_idx) in timeline.iter().enumerate() {
            let r_polys = clone_region_polys(&new_vec[slice_idx], object_id, *region_id);

            let upper_polys = timeline
                .get(pos + 1)
                .map(|&up_idx| clone_region_polys(&new_vec[up_idx], object_id, *region_id))
                .unwrap_or_default();
            let lower_polys = if pos == 0 {
                Vec::new()
            } else {
                clone_region_polys(&new_vec[timeline[pos - 1]], object_id, *region_id)
            };

            if k_top > 0 {
                let top_diff = difference(&r_polys, &upper_polys);
                if !top_diff.is_empty() {
                    if let Some(region) =
                        find_region_mut(&mut new_vec[slice_idx], object_id, *region_id)
                    {
                        region.top_shell_index = Some(0);
                        region.top_solid_fill = top_diff;
                    }
                }
            }

            if k_bot > 0 {
                let bot_diff = difference(&r_polys, &lower_polys);
                if !bot_diff.is_empty() {
                    if let Some(region) =
                        find_region_mut(&mut new_vec[slice_idx], object_id, *region_id)
                    {
                        region.bottom_shell_index = Some(0);
                        region.bottom_solid_fill = bot_diff;
                    }
                }
            }
        }

        // Pass 2: shrinking-shadow projection for top (walk backward).
        if k_top > 1 {
            for pos in 0..timeline.len() {
                let slice_idx = timeline[pos];
                let region = match find_region(&new_vec[slice_idx], object_id, *region_id) {
                    Some(r) => r,
                    None => continue,
                };
                if region.top_shell_index != Some(0) {
                    continue;
                }
                let mut shadow = region.top_solid_fill.clone();

                for offset_depth in 1..k_top.min((pos + 1) as u8) {
                    let n_pos = pos - offset_depth as usize;
                    let n_slice_idx = timeline[n_pos];
                    let neighbor_polys =
                        clone_region_polys(&new_vec[n_slice_idx], object_id, *region_id);
                    let new_shadow = intersection(&shadow, &neighbor_polys);
                    if new_shadow.is_empty() {
                        break;
                    }
                    if let Some(n_region) =
                        find_region_mut(&mut new_vec[n_slice_idx], object_id, *region_id)
                    {
                        n_region.top_solid_fill = union(&n_region.top_solid_fill, &new_shadow);
                        n_region.top_shell_index = Some(match n_region.top_shell_index {
                            None => offset_depth,
                            Some(existing) => existing.min(offset_depth),
                        });
                    }
                    shadow = new_shadow;
                }
            }
        }

        // Pass 2: shrinking-shadow projection for bottom (walk forward).
        if k_bot > 1 {
            for pos in 0..timeline.len() {
                let slice_idx = timeline[pos];
                let region = match find_region(&new_vec[slice_idx], object_id, *region_id) {
                    Some(r) => r,
                    None => continue,
                };
                if region.bottom_shell_index != Some(0) {
                    continue;
                }
                let mut shadow = region.bottom_solid_fill.clone();

                let remaining = timeline.len() - pos - 1;
                for offset_depth in 1..k_bot.min(remaining.saturating_add(1) as u8) {
                    let n_pos = pos + offset_depth as usize;
                    let n_slice_idx = timeline[n_pos];
                    let neighbor_polys =
                        clone_region_polys(&new_vec[n_slice_idx], object_id, *region_id);
                    let new_shadow = intersection(&shadow, &neighbor_polys);
                    if new_shadow.is_empty() {
                        break;
                    }
                    if let Some(n_region) =
                        find_region_mut(&mut new_vec[n_slice_idx], object_id, *region_id)
                    {
                        n_region.bottom_solid_fill =
                            union(&n_region.bottom_solid_fill, &new_shadow);
                        n_region.bottom_shell_index = Some(match n_region.bottom_shell_index {
                            None => offset_depth,
                            Some(existing) => existing.min(offset_depth),
                        });
                    }
                    shadow = new_shadow;
                }
            }
        }
    }

    blackboard.replace_slice_ir(Arc::new(new_vec))?;
    Ok(())
}

// ============================================================================
// Internal helpers
// ============================================================================

fn build_region_timelines(slices: &[SliceIR]) -> HashMap<(ObjectId, RegionId), Vec<usize>> {
    let mut timelines: HashMap<(ObjectId, RegionId), Vec<usize>> = HashMap::new();
    for (idx, slice) in slices.iter().enumerate() {
        for region in &slice.regions {
            timelines
                .entry((region.object_id.clone(), region.region_id))
                .or_default()
                .push(idx);
        }
    }
    timelines
}

fn resolve_shell_counts(
    region_map: &slicer_ir::RegionMapIR,
    object_id: &ObjectId,
    region_id: RegionId,
    timeline: &[usize],
) -> (u8, u8) {
    // Use the first timeline entry's RegionKey to pick up the per-region
    // resolved config. Saturating cast u32 → u8 captures pathological shell
    // counts > 255 without overflow.
    if let Some(&first_idx) = timeline.first() {
        let key = RegionKey {
            global_layer_index: first_idx as u32,
            object_id: object_id.clone(),
            region_id,
        };
        if let Some(plan) = region_map.entries.get(&key) {
            let k_top: u8 = plan.config.top_shell_layers.try_into().unwrap_or(u8::MAX);
            let k_bot: u8 = plan
                .config
                .bottom_shell_layers
                .try_into()
                .unwrap_or(u8::MAX);
            return (k_top, k_bot);
        }
    }
    // OrcaSlicer default fallback: 3/3 shell layers when no plan entry exists.
    (3, 3)
}

fn find_region<'a>(
    slice: &'a SliceIR,
    object_id: &ObjectId,
    region_id: RegionId,
) -> Option<&'a slicer_ir::SlicedRegion> {
    slice
        .regions
        .iter()
        .find(|r| &r.object_id == object_id && r.region_id == region_id)
}

fn find_region_mut<'a>(
    slice: &'a mut SliceIR,
    object_id: &ObjectId,
    region_id: RegionId,
) -> Option<&'a mut slicer_ir::SlicedRegion> {
    slice
        .regions
        .iter_mut()
        .find(|r| &r.object_id == object_id && r.region_id == region_id)
}

fn clone_region_polys(
    slice: &SliceIR,
    object_id: &ObjectId,
    region_id: RegionId,
) -> Vec<ExPolygon> {
    find_region(slice, object_id, region_id)
        .map(|r| r.polygons.clone())
        .unwrap_or_default()
}
