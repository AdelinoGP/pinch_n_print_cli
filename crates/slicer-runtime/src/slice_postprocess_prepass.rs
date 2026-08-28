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
//! # Where the parallelism is, and where it is not
//!
//! **The parallel axis is the timeline (layers), not the set of regions.**
//!
//! The outer loop over `timelines` stays sequential, but not for the reason
//! previously recorded here. That reason — "rayon's coordination overhead
//! exceeded the per-region work; per-region work runs in microseconds" — was an
//! artifact of the old `benches/shell_classification.rs` fixture, which swept
//! the *object* count and gave every layer the same 4-point square (so the
//! `difference` was empty, `apply_opening` short-circuited, and the `offset`
//! calls never ran). The real reason is that **the region axis is degenerate**:
//! `build_region_timelines` keys on `(object_id, region_id)`, and
//! `layer-planner-default`'s `run_layer_planning` emits `region_id: "0"` at
//! every emission site, so a single-material single-object print has exactly
//! one timeline. Measured on a 0.1 mm benchy: `timelines=1 lengths=[480]`, with
//! all 8.46 s of the stage's 8.48 s inside one `compute_region_updates` call.
//! Parallelising that outer loop has a maximum speedup of 1.0x there. The two
//! mechanisms that mint further `RegionId`s (paint variants, modifier
//! sub-regions) both run strictly after this stage.
//!
//! So `compute_region_updates` parallelises internally over the timeline
//! instead — see the per-pass notes on its body.
//!
//! References:
//! - canonical `detect_surfaces_type` (`PrintObject.cpp`) — Pass 1 reference.
//! - canonical `discover_horizontal_shells` (`PrintObject.cpp`) — Pass 2
//!   reference. Note its own comment: "Scattering process is inherently serial,
//!   it is difficult to parallelize without locking." That applies to canonical
//!   because it mutates each neighbour's `fill_surfaces` in place; this port
//!   accumulates into a separate `local` map keyed by slice index, which is
//!   what makes the per-seed chains independent here.
//!
//! See `docs/DEVIATION_LOG.md` for documented divergences (hollow-object
//! continue path not ported; `top_solid_fill` flattened across shell sources).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use rayon::prelude::*;
use slicer_core::algos::prepass_slice::{
    gate_bridge_areas_by_unsupported_span, update_external_bridge_orientation,
};
use slicer_core::polygon_ops::{difference, intersection, offset, union, OffsetJoinType};
use slicer_ir::{ConfigValue, ExPolygon, ObjectId, RegionId, RegionKey, RegionMapIR, SliceIR};

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
    // independent of the others. This loop stays sequential because the axis is
    // degenerate on ordinary prints (one timeline; see the module header);
    // `compute_region_updates` parallelises over the timeline instead.
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
                if let Some(fill) = edit.update.internal_solid_fill {
                    region.internal_solid_fill = fill;
                }
            }
        }
    }

    // Bridge candidates are produced during slicing, but only remain bridges
    // where the committed lower-layer slice leaves an unsupported span. Keep
    // layer presence separate from its polygons: an existing empty layer must
    // retain candidates, while a missing previous layer must clear them.
    let mut object_layers: HashMap<ObjectId, HashSet<u32>> = HashMap::new();
    let mut lower_layer_polygons: HashMap<(ObjectId, u32), Vec<ExPolygon>> = HashMap::new();
    for slice in &new_vec {
        for region in &slice.regions {
            object_layers
                .entry(region.object_id.clone())
                .or_default()
                .insert(slice.global_layer_index);
            lower_layer_polygons
                .entry((region.object_id.clone(), slice.global_layer_index))
                .or_default()
                .extend(region.infill_areas.iter().cloned());
        }
    }
    for slice in &mut new_vec {
        for region in &mut slice.regions {
            let lower_layer_slices = slice
                .global_layer_index
                .checked_sub(1)
                .filter(|lower_index| {
                    object_layers
                        .get(&region.object_id)
                        .is_some_and(|layers| layers.contains(lower_index))
                })
                .and_then(|lower_index| {
                    lower_layer_polygons
                        .get(&(region.object_id.clone(), lower_index))
                        .map(Vec::as_slice)
                });
            gate_bridge_areas_by_unsupported_span(region, lower_layer_slices);
            // Packet 235 Step 2: orientation derives from the GATED geometry
            // + RAW lower contours, overwriting the Slice-stage heuristic.
            update_external_bridge_orientation(region, lower_layer_slices);
        }
    }

    // Packet 234a: qualify internal-bridge sites against the committed layer
    // below and author anchored bridge centerlines per region. Runs after the
    // shell passes (top/bottom solid fills are populated for every layer
    // above) and strictly after 234's false-site gate. Material exclusion
    // flows through `bridge_areas`: the partition derives `sparse_infill_area`
    // from it at Perimeters commit, so module sparse infill never covers the
    // gated area. (`sparse_infill_area` itself is NOT mutated here — it does
    // not exist yet at this stage and would be overwritten.)
    gate_internal_bridge_sites(
        &mut new_vec,
        &timelines,
        &region_map,
        &object_layers,
        &lower_layer_polygons,
    );

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
    top_solid_seed: Option<Vec<ExPolygon>>,
    bottom_solid_fill: Option<Vec<ExPolygon>>,
    internal_solid_fill: Option<Vec<ExPolygon>>,
}

/// One layer's worth of shadow projected onto it by a single Pass-2 seed.
struct ShadowContribution {
    slice_idx: usize,
    depth: u8,
    shadow: Vec<ExPolygon>,
}

/// Collect the depth-0 seeds for one Pass-2 walk, in ascending timeline
/// position, snapshotting each seed's own fill.
///
/// Taking this snapshot before any Pass-2 write lands is what keeps seeds
/// independent of one another — and, for the forward-walking bottom pass, is
/// what makes the result match canonical (see the call site).
fn gather_seeds<'a, F>(
    timeline: &[usize],
    local: &'a HashMap<usize, RegionUpdate>,
    select: F,
) -> Vec<(usize, Vec<ExPolygon>)>
where
    F: Fn(&'a RegionUpdate) -> (Option<u8>, Option<&'a Vec<ExPolygon>>),
{
    (0..timeline.len())
        .filter_map(|pos| {
            let update = local.get(&timeline[pos])?;
            let (depth, fill) = select(update);
            // Only project from depth-0 layers (the depth Pass 1 stamped).
            (depth == Some(0)).then(|| (pos, fill.cloned().unwrap_or_default()))
        })
        .collect()
}

/// Walk one seed's shrinking shadow along `steps`, an iterator of
/// `(depth, neighbour_position)` pairs ordered outward from the seed.
///
/// Reads only the immutable `snapshot`, so chains from different seeds are
/// independent and may run concurrently. Stops at the first empty intersection,
/// matching the sequential `break`.
fn project_shadow<I>(
    seed_fill: &[ExPolygon],
    snapshot: &[SliceIR],
    object_id: &ObjectId,
    region_id: RegionId,
    timeline: &[usize],
    shrink_mm: f32,
    steps: I,
) -> Vec<ShadowContribution>
where
    I: Iterator<Item = (u8, usize)>,
{
    let mut shadow = seed_fill.to_vec();
    let mut out = Vec::new();
    for (depth, n_pos) in steps {
        let n_slice_idx = timeline[n_pos];
        let neighbor_polys = clone_region_polys(&snapshot[n_slice_idx], object_id, region_id);
        let intersected = intersection(&shadow, &neighbor_polys);
        let new_shadow = if shrink_mm > 0.0 {
            offset(&intersected, -shrink_mm, OffsetJoinType::Miter, 0.0)
        } else {
            intersected
        };
        if new_shadow.is_empty() {
            break;
        }
        out.push(ShadowContribution {
            slice_idx: n_slice_idx,
            depth,
            shadow: new_shadow.clone(),
        });
        shadow = new_shadow;
    }
    out
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
    // Pass 2 shrinks the shell shadow by one complete extrusion line width;
    // `opening_r` is the half-width used only by Pass 1's anti-sliver opening.
    let shell_width_mm = opening_r * 2.0;
    // Pass 1: depth-0 classification.
    //
    // A pure map over the timeline: position `pos` reads only the immutable
    // `snapshot` at `pos-1..=pos+1` and produces the entry for its own slice,
    // never a neighbour's. Rayon's ordered `collect` keeps the produced order
    // equal to plan order, and each `slice_idx` is produced by exactly one
    // iteration, so the resulting map is identical to the previous sequential
    // build. This is the dominant cost of the stage — two `difference` calls
    // and up to four round-join `offset`s (via `apply_opening`) per layer, on
    // full-layer geometry.
    let pass1: Vec<(usize, RegionUpdate)> = timeline
        .par_iter()
        .enumerate()
        .filter_map(|(pos, &slice_idx)| {
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

            let mut update = RegionUpdate::default();
            let mut touched = false;

            if k_top > 0 {
                let top_diff = apply_opening(&difference(&r_polys, &upper_polys), opening_r);
                if !top_diff.is_empty() {
                    update.top_shell_index = Some(0);
                    update.top_solid_seed = Some(top_diff.clone());
                    update.top_solid_fill = Some(top_diff);
                    touched = true;
                }
            }

            if k_bot > 0 {
                let bot_diff = apply_opening(&difference(&r_polys, &lower_polys), opening_r);
                if !bot_diff.is_empty() {
                    update.bottom_shell_index = Some(0);
                    update.bottom_solid_fill = Some(bot_diff);
                    touched = true;
                }
            }

            touched.then_some((slice_idx, update))
        })
        .collect();
    let mut local: HashMap<usize, RegionUpdate> = pass1.into_iter().collect();
    // Pass 2: shrinking-shadow projection for top (walk backward).
    //
    // Seeds are gathered from `local` *before* any Pass-2 write lands, so each
    // seed's shadow starts from its own Pass-1 depth-0 fill — the layer's own
    // classified top surface. That matches canonical `discover_horizontal_shells`
    // (`PrintObject.cpp`), which builds its `solid` seed from the layer's own
    // `slices`/`fill_surfaces` of the target type rather than from whatever a
    // previous seed projected onto it. See the bottom walk below for why this
    // matters there.
    if k_top > 1 {
        let seeds = gather_seeds(timeline, &local, |u| {
            (u.top_shell_index, u.top_solid_fill.as_ref())
        });
        // Per-seed chains are independent: each reads only its own seed and the
        // immutable `snapshot`, and writes nothing shared. The merge below is
        // serial and walks seeds in ascending timeline position — the same order
        // the sequential version visited them — so the `union` / `min` folds see
        // an identical sequence and produce identical output.
        let contributions: Vec<Vec<ShadowContribution>> = seeds
            .par_iter()
            .map(|(pos, seed_fill)| {
                project_shadow(
                    seed_fill,
                    snapshot,
                    object_id,
                    region_id,
                    timeline,
                    shell_width_mm,
                    (1..k_top.min((*pos + 1) as u8)).map(|d| (d, *pos - d as usize)),
                )
            })
            .collect();
        for contribution in contributions.into_iter().flatten() {
            let existing = local.entry(contribution.slice_idx).or_default();
            let existing_fill = existing.top_solid_fill.clone().unwrap_or_default();
            existing.top_solid_fill = Some(union(&existing_fill, &contribution.shadow));
            existing.top_shell_index = Some(match existing.top_shell_index {
                None => contribution.depth,
                Some(prev) => prev.min(contribution.depth),
            });
        }
    }

    // Pass 2: shrinking-shadow projection for bottom (walk forward).
    //
    // Unlike the top walk, this one projects *forward*, so it writes into
    // positions the sequential version had not visited yet. Seeding each chain
    // from the live `local` therefore fed one seed's projection into the next
    // seed's shadow whenever two depth-0 bottom layers sat within `k_bot` of
    // each other — letting a shadow propagate further than `k_bot` allows.
    // Canonical `discover_horizontal_shells` (`PrintObject.cpp`) does not do
    // this: it rebuilds `solid` for each seed layer from that layer's own
    // `slices` / `fill_surfaces` of the target type, and only the *scatter
    // target* is mutated. Gathering seeds up front restores that, and makes the
    // chains independent as a side effect. See `docs/DEVIATION_LOG.md`.
    if k_bot > 1 {
        let seeds = gather_seeds(timeline, &local, |u| {
            (u.bottom_shell_index, u.bottom_solid_fill.as_ref())
        });
        let contributions: Vec<Vec<ShadowContribution>> = seeds
            .par_iter()
            .map(|(pos, seed_fill)| {
                let remaining = timeline.len() - pos - 1;
                project_shadow(
                    seed_fill,
                    snapshot,
                    object_id,
                    region_id,
                    timeline,
                    shell_width_mm,
                    (1..k_bot.min(remaining.saturating_add(1) as u8))
                        .map(|d| (d, *pos + d as usize)),
                )
            })
            .collect();
        for contribution in contributions.into_iter().flatten() {
            let existing = local.entry(contribution.slice_idx).or_default();
            let existing_fill = existing.bottom_solid_fill.clone().unwrap_or_default();
            existing.bottom_solid_fill = Some(union(&existing_fill, &contribution.shadow));
            existing.bottom_shell_index = Some(match existing.bottom_shell_index {
                None => contribution.depth,
                Some(prev) => prev.min(contribution.depth),
            });
        }
    }

    // The depth-0 exposed seed remains top solid, not internal solid. Only
    // geometry propagated down from an upper shell layer is an internal-solid
    // bridge candidate.
    for update in local.values_mut() {
        if let Some(top_fill) = update.top_solid_fill.as_ref() {
            let top_seed = update.top_solid_seed.as_deref().unwrap_or_default();
            update.internal_solid_fill = Some(difference(top_fill, top_seed));
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
            variant_chain: Vec::new(),
        };
        if region_map.entries.contains_key(&key) {
            let lw = region_map.config_for(&key).line_width;
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

/// Packet 234a — canonical `bridge_over_infill` gather port (the support-math
/// functions landed in `slicer_core::algos::bridge_over_infill`). For each
/// region timeline, qualifies the upper layer's `top_solid_fill` surfaces
/// against the committed layer below and authors anchored internal-bridge
/// polygons into `SlicedRegion::internal_bridge_areas` for the same-layer
/// InfillPostProcess arm to construct and emit. Extends `bridge_areas` with the qualified
/// polygons so the existing partition dataflow (`region_partition` derives
/// `sparse_infill_area = difference(wall_inset, bridge ∪ bottom ∪ top)` at
/// Perimeters commit) keeps module sparse infill out of the gated area.
///
/// Sequential by construction: only this prepass legally sees every committed
/// layer; per-layer stage arms run under rayon with private arenas. Config
/// resolves through `region_map.config_for(...).extensions`, mirroring how
/// undeclared module keys are routed there by the host resolver (same keys the
/// old arm read from the module config view).
fn gate_internal_bridge_sites(
    slices: &mut [SliceIR],
    timelines: &HashMap<(ObjectId, RegionId), Vec<usize>>,
    region_map: &RegionMapIR,
    object_layers: &HashMap<ObjectId, HashSet<u32>>,
    lower_layer_polygons: &HashMap<(ObjectId, u32), Vec<ExPolygon>>,
) {
    // Keep first-pass qualifications separate from carrier-free duplicates so a
    // duplicated area cannot become a new qualification candidate on the next
    // timeline entry.
    let mut qualified_by_entry: HashMap<(usize, ObjectId, RegionId), Vec<ExPolygon>> =
        HashMap::new();
    let density_for = |region: &slicer_ir::SlicedRegion, layer_index: u32| {
        let exact = RegionKey {
            global_layer_index: layer_index,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region.variant_chain.clone(),
        };
        if let Some(plan) = region_map.entries.get(&exact) {
            return region_map.config_for_raw(plan.config).infill_density;
        }
        crate::layer_executor::config_for_region_smallest_chain(
            region_map,
            layer_index,
            &region.object_id,
            region.region_id,
        )
        .map(|config| region_map.config_for_raw(config).infill_density)
        .unwrap_or(0.0)
    };
    // Lower-layer solid support is pooled across regions. Dense regions are
    // wholly solid; sparse regions contribute every persisted solid surface.
    let mut lower_layer_solids: HashMap<(ObjectId, u32), Vec<ExPolygon>> = HashMap::new();
    for slice in slices.iter() {
        for region in &slice.regions {
            let entry = lower_layer_solids
                .entry((region.object_id.clone(), slice.global_layer_index))
                .or_default();
            let density = density_for(region, slice.global_layer_index);
            if density >= 0.999 {
                entry.extend(region.infill_areas.iter().cloned());
            } else {
                entry.extend(region.top_solid_fill.iter().cloned());
                entry.extend(region.bottom_solid_fill.iter().cloned());
                entry.extend(region.internal_solid_fill.iter().cloned());
                entry.extend(region.bridge_areas.iter().cloned());
            }
        }
    }

    for ((object_id, region_id), timeline) in timelines {
        // Resolve this region's flow settings once from its first timeline
        // entry, mirroring `resolve_opening_radius`'s key construction and
        // contains_key guard (`config_for` panics on unknown keys).
        let Some(&first_idx) = timeline.first() else {
            continue;
        };
        let key = RegionKey {
            global_layer_index: first_idx as u32,
            object_id: object_id.clone(),
            region_id: *region_id,
            variant_chain: Vec::new(),
        };
        let resolved = if region_map.entries.contains_key(&key) {
            Some(region_map.config_for(&key))
        } else {
            None
        };
        let ext_float = |name: &str, default: f32| -> f32 {
            resolved
                .and_then(|config| config.extensions.get(name))
                .and_then(|value| match value {
                    ConfigValue::Float(f) => Some(*f as f32),
                    ConfigValue::Int(i) => Some(*i as f32),
                    _ => None,
                })
                .unwrap_or(default)
        };
        let ext_abs = |name: &str, base: f32| -> Option<f32> {
            match resolved?.extensions.get(name)? {
                ConfigValue::Float(value) => Some(*value as f32),
                ConfigValue::Int(value) => Some(*value as f32),
                ConfigValue::Percent(value) => Some(*value as f32 / 100.0 * base),
                ConfigValue::FloatOrPercent { value, is_percent } => Some(if *is_percent {
                    *value as f32 / 100.0 * base
                } else {
                    *value as f32
                }),
                _ => None,
            }
        };
        let nofilter = resolved
            .and_then(|config| config.extensions.get("dont_filter_internal_bridges"))
            .and_then(|value| match value {
                ConfigValue::Bool(flag) => Some(*flag),
                _ => None,
            })
            .unwrap_or(false);
        let extra_bridge_layer = resolved
            .and_then(|config| config.extensions.get("enable_extra_bridge_layer"))
            .is_some_and(|value| match value {
                ConfigValue::Bool(flag) => *flag,
                ConfigValue::Int(value) => *value != 0,
                ConfigValue::Float(value) | ConfigValue::Percent(value) => *value != 0.0,
                ConfigValue::FloatOrPercent { value, .. } => *value != 0.0,
                ConfigValue::String(value) => !value.is_empty() && value != "0" && value != "false",
                ConfigValue::List(values) => !values.is_empty(),
            });
        let nozzle_diameter = ext_float("nozzle_diameter", 0.4);
        let solid_infill_width = slicer_core::flow::resolve_role_width(
            slicer_ir::ExtrusionRole::InternalSolidInfill,
            false,
            false,
            &slicer_core::flow::RoleWidthContext {
                line_width: resolved.map_or(0.0, |config| config.line_width),
                nozzle_diameter,
                internal_solid_infill_line_width: ext_abs(
                    "internal_solid_infill_line_width",
                    nozzle_diameter,
                )
                .unwrap_or(0.0),
                ..Default::default()
            },
        );
        // Canonical: expansion_multiplier 3 under strict filtering, 1 when the
        // filter is relaxed in any way (ibfLimited / ibfNofilter).
        let expansion_multiplier: f64 = if nofilter { 1.0 } else { 3.0 };

        for &slice_idx in timeline {
            // The first layer has no lower layer — nothing can span it.
            let Some(lower_index) =
                slices[slice_idx]
                    .global_layer_index
                    .checked_sub(1)
                    .filter(|lower_index| {
                        object_layers
                            .get(object_id)
                            .is_some_and(|layers| layers.contains(lower_index))
                    })
            else {
                continue;
            };
            let Some(lower_fills) = lower_layer_polygons.get(&(object_id.clone(), lower_index))
            else {
                continue;
            };
            let empty_solids: Vec<ExPolygon> = Vec::new();
            let lower_solids = lower_layer_solids
                .get(&(object_id.clone(), lower_index))
                .unwrap_or(&empty_solids);
            let layer_index = slices[slice_idx].global_layer_index;
            let print_z = slices[slice_idx].z;
            let timeline_position = timeline
                .iter()
                .position(|&candidate| candidate == slice_idx)
                .unwrap_or(0);
            let target_flow_height = slices[slice_idx]
                .regions
                .iter()
                .find(|candidate| {
                    candidate.object_id == *object_id && candidate.region_id == *region_id
                })
                .and_then(|_candidate| {
                    let key = RegionKey {
                        global_layer_index: layer_index,
                        object_id: object_id.clone(),
                        region_id: *region_id,
                        variant_chain: _candidate.variant_chain.clone(),
                    };
                    let key = if region_map.entries.contains_key(&key) {
                        key
                    } else {
                        RegionKey {
                            variant_chain: Vec::new(),
                            ..key
                        }
                    };
                    region_map.entries.contains_key(&key).then(|| {
                        region_map
                            .config_for(&key)
                            .extensions
                            .get("layer_height")
                            .and_then(|value| match value {
                                ConfigValue::Float(value) => Some(*value as f32),
                                ConfigValue::Int(value) => Some(*value as f32),
                                _ => None,
                            })
                    })
                })
                .flatten()
                .unwrap_or(0.2);
            let Ok(qualify_spacing_mm) =
                slicer_core::flow::line_width_to_spacing(solid_infill_width, target_flow_height)
            else {
                log::error!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=invalid_solid_infill_flow",
                    object_id,
                    region_id,
                    layer_index,
                    print_z
                );
                continue;
            };
            let depth_layers: Vec<slicer_core::algos::bridge_over_infill::BridgeDepthLayer> =
                timeline[..=timeline_position]
                    .iter()
                    .filter_map(|&lower_idx| {
                        let lower = slices[lower_idx].regions.iter().find(|candidate| {
                            candidate.object_id == *object_id && candidate.region_id == *region_id
                        })?;
                        let density = density_for(lower, slices[lower_idx].global_layer_index);
                        let mut not_sparse = lower.top_solid_fill.clone();
                        not_sparse.extend(lower.bottom_solid_fill.iter().cloned());
                        not_sparse.extend(lower.internal_solid_fill.iter().cloned());
                        not_sparse.extend(lower.bridge_areas.iter().cloned());
                        Some(slicer_core::algos::bridge_over_infill::BridgeDepthLayer {
                            print_z: slices[lower_idx].z,
                            sparse_infill: if density < 0.999 {
                                lower.infill_areas.clone()
                            } else {
                                Vec::new()
                            },
                            not_sparse_infill: if density >= 0.999 {
                                let mut dense = lower.infill_areas.clone();
                                dense.extend(not_sparse);
                                dense
                            } else {
                                not_sparse
                            },
                        })
                    })
                    .collect();
            let deep_infill_area = slicer_core::algos::bridge_over_infill::gather_areas_w_depth(
                &depth_layers,
                depth_layers.len().saturating_sub(1),
                target_flow_height,
                slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
            );
            let lower_candidates: Vec<
                slicer_core::algos::bridge_over_infill::BridgeCandidateLayer,
            > = timeline[..timeline_position]
                .iter()
                .filter_map(|&lower_idx| {
                    let lower = slices[lower_idx].regions.iter().find(|candidate| {
                        candidate.object_id == *object_id && candidate.region_id == *region_id
                    })?;
                    Some(
                        slicer_core::algos::bridge_over_infill::BridgeCandidateLayer {
                            print_z: slices[lower_idx].z,
                            new_polys: lower.internal_bridge_areas.clone(),
                        },
                    )
                })
                .collect();
            let deep_infill_area =
                slicer_core::algos::bridge_over_infill::remove_filled_polygons_on_lower_layers(
                    &deep_infill_area,
                    &lower_candidates,
                    print_z,
                    target_flow_height,
                );
            let deep_infill_clip_area = offset(
                &deep_infill_area,
                1.5 * qualify_spacing_mm,
                OffsetJoinType::Miter,
                0.0,
            );
            let internal_unsupported_area = if deep_infill_area.is_empty() {
                Vec::new()
            } else {
                offset(
                    &deep_infill_area,
                    -(4.5 * qualify_spacing_mm),
                    OffsetJoinType::Miter,
                    0.0,
                )
            };
            let Some(region) = find_region_mut(&mut slices[slice_idx], object_id, *region_id)
            else {
                continue;
            };
            if region.internal_solid_fill.is_empty() {
                log::debug!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=internal_solid_fill_empty",
                    object_id,
                    region_id,
                    layer_index,
                    print_z
                );
                continue;
            }
            // Canonical gather arithmetic: closing of lower fills shrunk by
            // mult*spacing minus grown lower solids (Step 1 port), then the
            // per-surface gates with the expand(4*spacing) clip.
            let unsupported = slicer_core::algos::bridge_over_infill::unsupported_span_areas(
                lower_fills,
                lower_solids,
                qualify_spacing_mm,
                expansion_multiplier,
            );
            if unsupported.is_empty() {
                log::debug!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=unsupported_empty",
                    object_id,
                    region_id,
                    layer_index,
                    print_z
                );
                continue;
            }
            let mut qualified: Vec<ExPolygon> = Vec::new();
            for surface in &region.internal_solid_fill {
                let expanded_candidate = offset(
                    std::slice::from_ref(surface),
                    qualify_spacing_mm,
                    OffsetJoinType::Miter,
                    0.0,
                );
                if intersection(&expanded_candidate, &deep_infill_clip_area).is_empty() {
                    log::debug!(
                        "internal bridge skip object={} region={} layer={} print_z={} reason=deep_sparse_clip_empty",
                        object_id, region_id, layer_index, print_z
                    );
                    continue;
                }
                if let Some(polys) =
                    slicer_core::algos::bridge_over_infill::qualify_internal_bridge_surface(
                        surface,
                        &unsupported,
                        qualify_spacing_mm,
                        nofilter,
                    )
                {
                    qualified.extend(polys);
                }
            }
            qualified.retain(|polygon| {
                !intersection(std::slice::from_ref(polygon), &internal_unsupported_area).is_empty()
            });
            if qualified.is_empty() {
                log::debug!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=qualified_empty_or_deep_sparse_clip surfaces={}",
                    object_id,
                    region_id,
                    layer_index,
                    print_z,
                    region.internal_solid_fill.len()
                );
                continue;
            }
            // Persist the qualified carrier independently of downstream
            // construction; Step 4 consumes this area after real walls exist.
            region.internal_bridge_areas = qualified.clone();
            region.bridge_areas.extend(qualified.clone());
            qualified_by_entry.insert((slice_idx, object_id.clone(), *region_id), qualified);
        }

        if extra_bridge_layer {
            for pair in timeline.windows(2) {
                let [current_idx, upper_idx] = pair else {
                    continue;
                };
                if slices[*current_idx].global_layer_index.checked_add(1)
                    != Some(slices[*upper_idx].global_layer_index)
                {
                    continue;
                }
                let Some(current_areas) =
                    qualified_by_entry.get(&(*current_idx, object_id.clone(), *region_id))
                else {
                    continue;
                };
                let Some(upper) = find_region_mut(&mut slices[*upper_idx], object_id, *region_id)
                else {
                    continue;
                };
                let duplicates = intersection(&upper.internal_solid_fill, current_areas);
                upper.internal_bridge_areas.extend(duplicates);
            }
        }
    }
}

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
            variant_chain: Vec::new(),
        };
        if region_map.entries.contains_key(&key) {
            let resolved = region_map.config_for(&key);
            let k_top: u8 = resolved.top_shell_layers.try_into().unwrap_or(u8::MAX);
            let k_bot: u8 = resolved.bottom_shell_layers.try_into().unwrap_or(u8::MAX);
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

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{Point2, Polygon, SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION};

    fn rect(min_x: f32, max_x: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min_x, 0.0),
                    Point2::from_mm(max_x, 0.0),
                    Point2::from_mm(max_x, 10.0),
                    Point2::from_mm(min_x, 10.0),
                ],
            },
            holes: vec![],
        }
    }

    fn slice_with(index: u32, polygons: Vec<ExPolygon>) -> SliceIR {
        SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: index,
            z: 0.2 * (index + 1) as f32,
            regions: vec![SlicedRegion {
                object_id: String::from("o"),
                region_id: 0,
                polygons: polygons.clone(),
                infill_areas: polygons,
                ..Default::default()
            }],
        }
    }

    fn min_x_mm(polys: &[ExPolygon]) -> f32 {
        let min_unit = polys
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .map(|p| p.x)
            .min()
            .expect("non-empty fill");
        min_unit as f32 / 10_000.0
    }

    /// A bottom seed must project its *own* depth-0 surface, not whatever an
    /// earlier seed already unioned onto it.
    ///
    /// Layers 0..3 form a staircase: L0 covers x∈[0,10], L1..L3 cover x∈[0,20].
    /// That makes L0 a bottom seed (nothing below it) and L1 a bottom seed too
    /// (its x∈[10,20] strip overhangs L0). With `k_bot = 3` each seed reaches
    /// two layers up, so L0 may influence L1 and L2 — never L3.
    ///
    /// Seeding L1's chain from the live accumulator let L0's [0,10] surface ride
    /// along on L1's projection and reach L3, i.e. four solid layers from a
    /// three-layer setting. Canonical `discover_horizontal_shells`
    /// (`PrintObject.cpp`) rebuilds its `solid` seed from the seed layer's own
    /// classified surfaces each time, so the strip never accumulates.
    #[test]
    fn bottom_shadow_does_not_propagate_past_k_bot_via_a_later_seed() {
        let snapshot = vec![
            slice_with(0, vec![rect(0.0, 10.0)]),
            slice_with(1, vec![rect(0.0, 20.0)]),
            slice_with(2, vec![rect(0.0, 20.0)]),
            slice_with(3, vec![rect(0.0, 20.0)]),
        ];
        let object_id = String::from("o");

        let edits = compute_region_updates(&snapshot, &object_id, 0, &[0, 1, 2, 3], 0, 3, 0.0);

        let l3 = edits
            .iter()
            .find(|e| e.slice_idx == 3)
            .expect("L3 receives a projection from the L1 seed");
        let fill = l3
            .update
            .bottom_solid_fill
            .as_ref()
            .expect("L3 has bottom solid fill");

        // Only L1's own overhang strip may reach L3. If L0's surface leaked
        // through L1's seed, this fill would start at x = 0.
        assert!(
            min_x_mm(fill) >= 9.99,
            "L0's bottom surface propagated past k_bot into L3: fill starts at x = {} mm, expected x >= 10 mm",
            min_x_mm(fill)
        );
    }

    /// The parallel passes must not let worker scheduling reach the output.
    #[test]
    fn repeated_runs_are_identical() {
        let snapshot: Vec<SliceIR> = (0..24u32)
            .map(|i| {
                let inset = (i % 5) as f32;
                slice_with(i, vec![rect(inset, 20.0 - inset)])
            })
            .collect();
        let object_id = String::from("o");
        let timeline: Vec<usize> = (0..snapshot.len()).collect();

        let normalize = |mut edits: Vec<RegionEdit>| {
            edits.sort_by_key(|e| e.slice_idx);
            edits
                .into_iter()
                .map(|e| {
                    (
                        e.slice_idx,
                        e.update.top_shell_index,
                        e.update.bottom_shell_index,
                        e.update.top_solid_fill,
                        e.update.bottom_solid_fill,
                    )
                })
                .collect::<Vec<_>>()
        };

        let baseline = normalize(compute_region_updates(
            &snapshot, &object_id, 0, &timeline, 3, 3, 0.0,
        ));
        for _ in 0..8 {
            let again = normalize(compute_region_updates(
                &snapshot, &object_id, 0, &timeline, 3, 3, 0.0,
            ));
            assert_eq!(baseline, again, "shell classification output is not stable");
        }
    }
}
