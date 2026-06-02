//! Host built-in `PrePass::ShellClassification`.
//!
//! Ports the two-pass OrcaSlicer surface classification:
//! 1. **Pass 1 (depth 0)** — per-region polygon `diff` against the next
//!    active layer (for top exposure) and previous active layer (for bottom
//!    exposure) in that region's own timeline. The diff polygons become the
//!    layer-0 `top_solid_fill` / `bottom_solid_fill` AFTER a morphological
//!    opening (offset(-r) -> offset(+r)) that strips sub-extrusion-width
//!    slivers produced by coincident-edge subtraction.
//! 2. **Pass 2 (depths 1..k-1)** — shrinking-shadow projection. For each
//!    region layer marked as depth-0, walk outward through the region's
//!    timeline (backward for top, forward for bottom) and `intersection` the
//!    accumulated shadow with each neighbor's polygons. Each non-empty
//!    intersection stamps that neighbor with the minimum reached depth and
//!    unions the shadow into its solid-fill.
//!
//! The per-region computation is independent — different `(object, region)`
//! pairs touch disjoint `SlicedRegion`s within each `SliceIR`. The outer loop
//! is kept sequential after benchmarking showed rayon's coordination overhead
//! exceeded the per-region work on realistic fixtures (per-region work runs
//! in microseconds; rayon task scheduling cost dominated). The structural
//! split into `compute_region_updates` returning a `Vec<RegionEdit>` is
//! retained because it isolates per-region logic and remains trivially
//! parallelisable if a future workload shifts the cost balance.
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

use slicer_core::polygon_ops::{difference, intersection, offset, union, OffsetJoinType};
use slicer_ir::{ExPolygon, ObjectId, RegionId, RegionKey, RegionMapIR, SliceIR};

use slicer_ir::BlackboardError;

use crate::blackboard::Blackboard;

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
    // ordering by construction (built per the layer plan), so iteration order
    // is already plan-order.
    let timelines = build_region_timelines(&new_vec);

    // Per-region computation produces a Vec<(slice_idx, RegionUpdate)> tagged
    // with (object_id, region_id). Reads are against the immutable `new_vec`
    // snapshot — Pass 1 and Pass 2 both consume the original per-slice
    // polygons, never the in-flight solid-fill writes, so each region is
    // independent of the others. The outer loop is sequential because
    // benchmarking showed rayon's coordination overhead exceeded the
    // per-region work on realistic fixtures (see benches/shell_classification.rs).
    let per_region_updates: Vec<Vec<RegionEdit>> = timelines
        .iter()
        .map(|((object_id, region_id), timeline)| {
            let (k_top, k_bot) =
                resolve_shell_counts(region_map.as_ref(), object_id, *region_id, timeline);
            let opening_r =
                resolve_opening_radius(region_map.as_ref(), object_id, *region_id, timeline);
            compute_region_updates(
                &new_vec, object_id, *region_id, timeline, k_top, k_bot, opening_r,
            )
        })
        .collect();

    // Apply updates serially. Each update targets a single SlicedRegion (by
    // object_id + region_id within the SliceIR at slice_idx); regions from
    // different timelines never collide.
    for edits in per_region_updates {
        for edit in edits {
            if let Some(region) = find_region_mut(
                &mut new_vec[edit.slice_idx],
                &edit.object_id,
                edit.region_id,
            ) {
                if let Some(idx) = edit.update.top_shell_index {
                    region.top_shell_index = Some(idx);
                }
                if let Some(idx) = edit.update.bottom_shell_index {
                    region.bottom_shell_index = Some(idx);
                }
                if let Some(fill) = edit.update.top_solid_fill {
                    region.top_solid_fill = fill;
                }
                if let Some(fill) = edit.update.bottom_solid_fill {
                    region.bottom_solid_fill = fill;
                }
            }
        }
    }

    blackboard.replace_slice_ir(Arc::new(new_vec))?;
    Ok(())
}

// ============================================================================
// Per-region computation
// ============================================================================

/// Update batched against a single `(object_id, region_id)` at one slice.
struct RegionEdit {
    slice_idx: usize,
    object_id: ObjectId,
    region_id: RegionId,
    update: RegionUpdate,
}

#[derive(Default)]
struct RegionUpdate {
    top_shell_index: Option<u8>,
    bottom_shell_index: Option<u8>,
    top_solid_fill: Option<Vec<ExPolygon>>,
    bottom_solid_fill: Option<Vec<ExPolygon>>,
}

/// Run Pass 1 + Pass 2 for a single `(object, region)` timeline against the
/// read-only `snapshot`. Returns one `RegionEdit` per slice that the region
/// touched. The closure tracks all state in `local`, keyed by slice index.
fn compute_region_updates(
    snapshot: &[SliceIR],
    object_id: &ObjectId,
    region_id: RegionId,
    timeline: &[usize],
    k_top: u8,
    k_bot: u8,
    opening_r: f32,
) -> Vec<RegionEdit> {
    let mut local: HashMap<usize, RegionUpdate> = HashMap::new();

    // Pass 1: depth-0 classification.
    for (pos, &slice_idx) in timeline.iter().enumerate() {
        let r_polys = clone_region_polys(&snapshot[slice_idx], object_id, region_id);

        let upper_polys = timeline
            .get(pos + 1)
            .map(|&up_idx| clone_region_polys(&snapshot[up_idx], object_id, region_id))
            .unwrap_or_default();
        let lower_polys = if pos == 0 {
            Vec::new()
        } else {
            clone_region_polys(&snapshot[timeline[pos - 1]], object_id, region_id)
        };

        if k_top > 0 {
            let top_diff = apply_opening(&difference(&r_polys, &upper_polys), opening_r);
            if !top_diff.is_empty() {
                let entry = local.entry(slice_idx).or_default();
                entry.top_shell_index = Some(0);
                entry.top_solid_fill = Some(top_diff);
            }
        }

        if k_bot > 0 {
            let bot_diff = apply_opening(&difference(&r_polys, &lower_polys), opening_r);
            if !bot_diff.is_empty() {
                let entry = local.entry(slice_idx).or_default();
                entry.bottom_shell_index = Some(0);
                entry.bottom_solid_fill = Some(bot_diff);
            }
        }
    }

    // Pass 2: shrinking-shadow projection for top (walk backward).
    if k_top > 1 {
        for pos in 0..timeline.len() {
            let slice_idx = timeline[pos];
            // Only project from depth-0 layers (the depth that Pass 1 stamped).
            let local_top_idx = local.get(&slice_idx).and_then(|u| u.top_shell_index);
            if local_top_idx != Some(0) {
                continue;
            }
            let mut shadow = local
                .get(&slice_idx)
                .and_then(|u| u.top_solid_fill.clone())
                .unwrap_or_default();

            for offset_depth in 1..k_top.min((pos + 1) as u8) {
                let n_pos = pos - offset_depth as usize;
                let n_slice_idx = timeline[n_pos];
                let neighbor_polys =
                    clone_region_polys(&snapshot[n_slice_idx], object_id, region_id);
                let new_shadow = intersection(&shadow, &neighbor_polys);
                if new_shadow.is_empty() {
                    break;
                }
                let existing = local.entry(n_slice_idx).or_default();
                let existing_fill = existing.top_solid_fill.clone().unwrap_or_default();
                existing.top_solid_fill = Some(union(&existing_fill, &new_shadow));
                existing.top_shell_index = Some(match existing.top_shell_index {
                    None => offset_depth,
                    Some(prev) => prev.min(offset_depth),
                });
                shadow = new_shadow;
            }
        }
    }

    // Pass 2: shrinking-shadow projection for bottom (walk forward).
    if k_bot > 1 {
        for pos in 0..timeline.len() {
            let slice_idx = timeline[pos];
            let local_bot_idx = local.get(&slice_idx).and_then(|u| u.bottom_shell_index);
            if local_bot_idx != Some(0) {
                continue;
            }
            let mut shadow = local
                .get(&slice_idx)
                .and_then(|u| u.bottom_solid_fill.clone())
                .unwrap_or_default();

            let remaining = timeline.len() - pos - 1;
            for offset_depth in 1..k_bot.min(remaining.saturating_add(1) as u8) {
                let n_pos = pos + offset_depth as usize;
                let n_slice_idx = timeline[n_pos];
                let neighbor_polys =
                    clone_region_polys(&snapshot[n_slice_idx], object_id, region_id);
                let new_shadow = intersection(&shadow, &neighbor_polys);
                if new_shadow.is_empty() {
                    break;
                }
                let existing = local.entry(n_slice_idx).or_default();
                let existing_fill = existing.bottom_solid_fill.clone().unwrap_or_default();
                existing.bottom_solid_fill = Some(union(&existing_fill, &new_shadow));
                existing.bottom_shell_index = Some(match existing.bottom_shell_index {
                    None => offset_depth,
                    Some(prev) => prev.min(offset_depth),
                });
                shadow = new_shadow;
            }
        }
    }

    local
        .into_iter()
        .map(|(slice_idx, update)| RegionEdit {
            slice_idx,
            object_id: object_id.clone(),
            region_id,
            update,
        })
        .collect()
}

// ============================================================================
// Anti-sliver opening
// ============================================================================

/// OrcaSlicer fallback radius (mm) when no per-region `line_width` is known.
/// Half of the 0.4 mm nominal extrusion width.
const FALLBACK_OPENING_RADIUS_MM: f32 = 0.2;

/// Morphological opening: `offset(-r)` followed by `offset(+r)`. Removes
/// features narrower than `2r` (sub-extrusion-width slivers) while leaving
/// wider geometry essentially unchanged. Mirrors
/// `slicer_core::triangle_mesh_slicer::apply_slice_closing_radius` but with
/// reversed offset order.
fn apply_opening(polys: &[ExPolygon], r: f32) -> Vec<ExPolygon> {
    if polys.is_empty() || r <= 0.0 {
        return polys.to_vec();
    }
    let eroded = offset(polys, -r, OffsetJoinType::Round, 0.0);
    offset(&eroded, r, OffsetJoinType::Round, 0.0)
}

/// Resolve the opening radius from the region's `line_width` (half-width =
/// removes any feature narrower than one extrusion line). Falls back to the
/// 0.2 mm constant when no `RegionPlan` entry exists for this region.
fn resolve_opening_radius(
    region_map: &RegionMapIR,
    object_id: &ObjectId,
    region_id: RegionId,
    timeline: &[usize],
) -> f32 {
    if let Some(&first_idx) = timeline.first() {
        let key = RegionKey {
            global_layer_index: first_idx as u32,
            object_id: object_id.clone(),
            region_id,
        };
        if let Some(plan) = region_map.entries.get(&key) {
            let lw = plan.config.line_width;
            if lw > 0.0 {
                return lw * 0.5;
            }
        }
    }
    FALLBACK_OPENING_RADIUS_MM
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
    slice
        .regions
        .iter()
        .find(|r| &r.object_id == object_id && r.region_id == region_id)
        .map(|r| r.polygons.clone())
        .unwrap_or_default()
}
