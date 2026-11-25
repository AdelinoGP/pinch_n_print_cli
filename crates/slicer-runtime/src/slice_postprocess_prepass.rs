//! Host built-in `PrePass::ShellClassification`.
//!
//! Ports the two-pass OrcaSlicer surface classification:
//! 1. **Pass 1 (depth 0)** — per-region polygon `diff` against the next
//!    active layer (for top exposure) and previous active layer (for bottom
//!    exposure) in that region's own timeline. The diff polygons become the
//!    layer-0 `top_solid_fill` / `bottom_solid_fill` AFTER canonical's opening,
//!    `opening_ex(diff, external_perimeter_width / 10)` with `jtMiter` and miter
//!    limit 3, which strips the slivers that coincident-edge subtraction
//!    produces without deleting real narrow surfaces.
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
use slicer_core::algos::prepass_slice::update_external_bridge_orientation;
use slicer_core::polygon_ops::{
    difference, intersection, offset, opening, opening_ex, union, OffsetJoinType,
};
use slicer_ir::{
    ConfigValue, ExPolygon, ObjectId, RegionId, RegionKey, RegionMapIR, ResolvedConfig, SliceIR,
};

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
    let raft_layer_indices: HashSet<u32> = blackboard
        .layer_plan()
        .into_iter()
        .flat_map(|plan| &plan.global_layers)
        .filter(|layer| layer.is_raft)
        .map(|layer| layer.index)
        .collect();

    let mut new_vec: Vec<SliceIR> = old_arc.as_ref().clone();

    // Build per-region timelines: ordered Vec<usize> of slice indices where the
    // (object, region) pair appears. Slices retain their `global_layer_index`
    // ordering by construction (built per the layer plan), so iteration order
    // is already plan-order.
    let timelines = build_region_timelines(&new_vec, &raft_layer_indices);

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
            let params =
                resolve_opening_params(region_map.as_ref(), object_id, *region_id, timeline);
            compute_region_updates(
                &new_vec, object_id, *region_id, timeline, k_top, k_bot, params,
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

    // Bridge classification already ran in `PrePass::Slice` against the raw
    // previous cross-section (canonical `detect_surfaces_type`'s difference).
    // This stage only re-derives the orientation from the classified areas and
    // the committed lower-layer polygons; it must not subtract a second lower
    // layer, which here would be the closed region polygons rather than the
    // raw slices.
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
    // Each slice reads only the immutable lower-layer maps and mutates its own
    // regions, so the axis is embarrassingly parallel.
    new_vec.par_iter_mut().for_each(|slice| {
        let global_layer_index = slice.global_layer_index;
        for region in &mut slice.regions {
            let lower_layer_slices = global_layer_index
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
            // Packet 235 Step 2: orientation derives from the classified
            // geometry + committed lower contours.
            update_external_bridge_orientation(region, lower_layer_slices);
        }
    });

    // Packet 234a: qualify internal-bridge sites against the committed layer
    // below and author anchored bridge centerlines per region. Runs after the
    // shell passes (top/bottom solid fills are populated for every layer
    // above) and strictly after 234's false-site gate. Material exclusion
    // flows through `bridge_areas`: the partition derives `sparse_infill_area`
    // from it at Perimeters commit, so module sparse infill never covers the
    // gated area. (`sparse_infill_area` itself is NOT mutated here — it does
    // not exist yet at this stage and would be overwritten.)
    let gate_stats = gate_internal_bridge_sites(
        &mut new_vec,
        &timelines,
        &region_map,
        &object_layers,
        &lower_layer_polygons,
    );
    log::debug!(
        "internal bridge gate prep_layers={} clusters={} recomputed={} speculation_misses={}",
        gate_stats.prep_layers,
        gate_stats.clusters,
        gate_stats.recomputed,
        gate_stats.speculation_misses
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

/// One layer that survived `gate_internal_bridge_sites`' qualification phase,
/// carrying the geometry that phase needed to compute. Splitting qualification
/// from commit is what lets the former run over the layer axis in parallel.
struct LayerPrep {
    timeline_position: usize,
    slice_idx: usize,
    layer_index: u32,
    print_z: f32,
    target_flow_height: f32,
    qualify_spacing_mm: f32,
    /// `qualify_internal_bridge_surface`'s accepted polygons, per
    /// `internal_solid_fill` surface index, in surface order. Never empty.
    surface_qualifications: Vec<(usize, Vec<ExPolygon>)>,
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

/// Which shell a Pass-2 walk feeds.
#[derive(Clone, Copy)]
enum ShellSide {
    Top,
    Bottom,
}

/// Fold one Pass-2 walk's shadow contributions into `local`, returning the
/// slices that received at least one.
///
/// The sequential fold visited `contributions` seed by seed and, per
/// contribution, replaced the target's fill with `union(fill, shadow)` and its
/// depth with the minimum. Each target slice therefore sees only its own
/// subsequence of contributions, in that global order, and no other slice's.
/// Grouping by target while preserving that order and folding the groups in
/// parallel performs the identical `union` sequence on every slice.
fn merge_shadow_contributions(
    local: &mut HashMap<usize, RegionUpdate>,
    contributions: Vec<Vec<ShadowContribution>>,
    side: ShellSide,
) -> HashSet<usize> {
    let mut grouped: HashMap<usize, Vec<ShadowContribution>> = HashMap::new();
    for contribution in contributions.into_iter().flatten() {
        grouped
            .entry(contribution.slice_idx)
            .or_default()
            .push(contribution);
    }
    let received: HashSet<usize> = grouped.keys().copied().collect();
    let jobs: Vec<(
        usize,
        Option<Vec<ExPolygon>>,
        Option<u8>,
        Vec<ShadowContribution>,
    )> = grouped
        .into_iter()
        .map(|(slice_idx, group)| {
            let entry = local.entry(slice_idx).or_default();
            let (fill, depth) = match side {
                ShellSide::Top => (entry.top_solid_fill.take(), entry.top_shell_index),
                ShellSide::Bottom => (entry.bottom_solid_fill.take(), entry.bottom_shell_index),
            };
            (slice_idx, fill, depth, group)
        })
        .collect();
    let merged: Vec<(usize, Vec<ExPolygon>, Option<u8>)> = jobs
        .into_par_iter()
        .map(|(slice_idx, mut fill, mut depth, group)| {
            for contribution in group {
                let existing_fill = fill.unwrap_or_default();
                fill = Some(union(&existing_fill, &contribution.shadow));
                depth = Some(match depth {
                    None => contribution.depth,
                    Some(prev) => prev.min(contribution.depth),
                });
            }
            (slice_idx, fill.unwrap_or_default(), depth)
        })
        .collect();
    for (slice_idx, fill, depth) in merged {
        let entry = local.entry(slice_idx).or_default();
        match side {
            ShellSide::Top => {
                entry.top_solid_fill = Some(fill);
                entry.top_shell_index = depth;
            }
            ShellSide::Bottom => {
                entry.bottom_solid_fill = Some(fill);
                entry.bottom_shell_index = depth;
            }
        }
    }
    received
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
    params: ShellClassificationParams,
) -> Vec<RegionEdit> {
    // Pass 2 shrinks the shell shadow by one complete extrusion line width.
    // That width and Pass 1's opening radius are two different canonical
    // quantities (see `ShellClassificationParams`) and are resolved
    // separately so this parity fix cannot move the shadow cascade.
    let ShellClassificationParams {
        opening_radius_mm,
        shell_width_mm,
    } = params;
    // Pass 1: depth-0 classification.
    //
    // A pure map over the timeline: position `pos` reads only the immutable
    // `snapshot` at `pos-1..=pos+1` and produces the entry for its own slice,
    // never a neighbour's. Rayon's ordered `collect` keeps the produced order
    // equal to plan order, and each `slice_idx` is produced by exactly one
    // iteration, so the resulting map is identical to the previous sequential
    // build. This pass performs two `difference` calls and one canonical
    // `opening_ex` per exposed side, on full-layer geometry.
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
                let top_diff =
                    apply_opening(&difference(&r_polys, &upper_polys), opening_radius_mm);
                if !top_diff.is_empty() {
                    update.top_shell_index = Some(0);
                    update.top_solid_seed = Some(top_diff.clone());
                    update.top_solid_fill = Some(top_diff);
                    touched = true;
                }
            }

            if k_bot > 0 {
                let bot_diff =
                    apply_opening(&difference(&r_polys, &lower_polys), opening_radius_mm);
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
    // Slices that received at least one top Pass-2 shadow. Every other slice's
    // `top_solid_fill` is still the Pass-1 seed clone.
    let mut top_received: HashSet<usize> = HashSet::new();
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
        // immutable `snapshot`, and writes nothing shared. The merge folds each
        // target slice's contributions in ascending seed order — the order the
        // sequential version visited them — so the `union` / `min` folds see an
        // identical sequence per slice (see `merge_shadow_contributions`).
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
        top_received = merge_shadow_contributions(&mut local, contributions, ShellSide::Top);
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
        merge_shadow_contributions(&mut local, contributions, ShellSide::Bottom);
    }

    // The depth-0 exposed seed remains top solid, not internal solid. Only
    // geometry propagated down from an upper shell layer is an internal-solid
    // bridge candidate.
    //
    // Each entry's difference reads only that entry, so the map is processed
    // in parallel. A slice that no top Pass-2 shadow reached still holds the
    // fill Pass 1 wrote, which is a clone of its seed, and `A \ A` is empty:
    // the clipper call is skipped for it.
    local.par_iter_mut().for_each(|(slice_idx, update)| {
        if let Some(top_fill) = update.top_solid_fill.as_ref() {
            update.internal_solid_fill = Some(if top_received.contains(slice_idx) {
                let top_seed = update.top_solid_seed.as_deref().unwrap_or_default();
                difference(top_fill, top_seed)
            } else {
                Vec::new()
            });
        }
    });

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
// Canonical top/bottom opening
// ============================================================================

/// Fallback extrusion width (mm) when no per-region `line_width` is known.
/// The 0.4 mm nominal width; both the opening radius and the shadow step derive
/// from it.
const FALLBACK_LINE_WIDTH_MM: f32 = 0.4;

/// Canonical `detect_surfaces_type` divides the external-perimeter width by
/// this to get the opening radius: `opening_ex(diff, fw / 10)`.
const OPENING_RADIUS_DIVISOR: f32 = 10.0;

/// Canonical's `DefaultMiterLimit` (`ClipperUtils.hpp`), also used for the
/// bridge opening in `classify_unsupported_bridge_areas`.
const OPENING_MITER_LIMIT: f64 = 3.0;

/// The two widths Pass 1 and Pass 2 consume, both in millimetres.
///
/// They are **not** the same quantity, and the pre-t166 code happened to make
/// them equal (`opening_radius = line_width / 2`, `shell_width = 2 × that`):
///
/// * `opening_radius_mm` is canonical `PrintObject::detect_surfaces_type`'s
///   `opening_ex(diff, flow(frExternalPerimeter).scaled_width() / 10)` — a
///   tenth of the external-perimeter width, i.e. an anti-sliver opening that
///   strips features narrower than a fifth of an extrusion line.
/// * `shell_width_mm` is canonical `discover_horizontal_shells`' shrinking-shadow
///   step, one full extrusion line width.
///
/// Splitting them is deliberate: t166 ports only the Pass-1 opening, so the
/// Pass-2 cascade keeps the width it had (`line_width`).
struct ShellClassificationParams {
    opening_radius_mm: f32,
    shell_width_mm: f32,
}

impl Default for ShellClassificationParams {
    fn default() -> Self {
        Self {
            opening_radius_mm: FALLBACK_LINE_WIDTH_MM / OPENING_RADIUS_DIVISOR,
            shell_width_mm: FALLBACK_LINE_WIDTH_MM,
        }
    }
}

/// Canonical morphological opening, `opening_ex(diff, radius)` with
/// `jtMiter` and miter limit 3 — the same call
/// `classify_unsupported_bridge_areas` makes for canonical's bridge opening,
/// and the same parameters canonical uses for the top/bottom surfaces.
///
/// The pre-t166 code used `offset(-r, Round)` then `offset(+r, Round)` with
/// `r = line_width / 2`, about 5x canonical's radius, which classified narrow
/// top/bottom features as non-solid where canonical keeps them.
fn apply_opening(polys: &[ExPolygon], r: f32) -> Vec<ExPolygon> {
    if polys.is_empty() || !(r > 0.0) {
        return polys.to_vec();
    }
    opening_ex(polys, r as f64, OffsetJoinType::Miter, OPENING_MITER_LIMIT)
}

/// Resolve the external-perimeter extrusion width (mm) that canonical's
/// `flow(frExternalPerimeter).scaled_width()` would report for this region.
///
/// The matched job authors `outer_wall_line_width = 0`, which is canonical's
/// **auto sentinel**, not a zero-width wall: `Flow` resolves it to the
/// profile's `line_width` before either classifier sees it. A resolver that
/// returns the literal `0.0` therefore does not "use canonical's width", it
/// disables the opening entirely — measured: with the literal value the
/// opening is a no-op (5,201,451 points in, 5,201,451 out, 6.8 ms for 974
/// calls) and every layer carries a depth-0 seed.
///
/// The policy lives in one place,
/// [`slicer_core::algos::prepass_slice::resolve_external_perimeter_width_mm`],
/// because `classify_unsupported_bridge_areas` resolves the same canonical
/// quantity for the bridge opening. Two copies drifted once (t168: the bridge
/// copy returned the literal zero and its opening was a no-op); there is one.
fn resolve_shell_external_width_mm(config: &ResolvedConfig) -> f32 {
    slicer_core::algos::prepass_slice::resolve_external_perimeter_width_mm(config)
}

/// Resolve both widths from the region's `ResolvedConfig`. The opening radius
/// is a tenth of the external-perimeter width; the shadow width is the typed
/// `line_width` with the same 0.4 mm fallback. A region with no `RegionPlan`
/// entry takes both fallbacks.
fn resolve_opening_params(
    region_map: &RegionMapIR,
    object_id: &ObjectId,
    region_id: RegionId,
    timeline: &[usize],
) -> ShellClassificationParams {
    let Some(&first_idx) = timeline.first() else {
        return ShellClassificationParams::default();
    };
    let key = RegionKey {
        global_layer_index: first_idx as u32,
        object_id: object_id.clone(),
        region_id,
        variant_chain: Vec::new(),
    };
    if !region_map.entries.contains_key(&key) {
        return ShellClassificationParams::default();
    }
    let config = region_map.config_for(&key);
    let external_width_mm = resolve_shell_external_width_mm(config);
    let shell_width_mm = if config.line_width > 0.0 {
        config.line_width as f32
    } else {
        FALLBACK_LINE_WIDTH_MM
    };
    ShellClassificationParams {
        opening_radius_mm: external_width_mm / OPENING_RADIUS_DIVISOR,
        shell_width_mm,
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Fully computed internal-bridge commit for one layer. The expensive
/// per-layer clipper work depends only on committed geometry and on this
/// layer's own fields, so it can run in the independent qualification phase;
/// only the lower-layer candidate subtraction is order-dependent.
struct InternalBridgeLayerResult {
    qualified: Vec<ExPolygon>,
    angles: Vec<f32>,
}

/// Rectangular clip window for the negative `deep_infill_area` offset: the
/// union bounding box of the layer's internal-bridge candidates, grown by
/// enough to cover the erosion delta plus Clipper's miter reach (`miter_limit`
/// is 2.0 in [`offset`]). Every point whose erosion can reach a candidate lies
/// inside the window, so the eroded set is unchanged wherever a candidate
/// consumes it.
///
/// Only the erosion is windowed. The positive offset stays whole-layer because
/// `internal_bridge_angles` intersects the layer's sparse expansion area with
/// it and derives bridge directions from that whole-layer boundary anchor set;
/// clipping the positive operand moves bridge directions even though every
/// candidate is local (measured: eight benchy layers, up to 22 mm of fill
/// re-routing).
///
/// The window exists because the offset is a local operation: a point farther
/// than `miter_limit x |delta|` from the candidate bounding box cannot change
/// the eroded set inside it. `deep_infill_area` on a large plate extends far
/// beyond the candidate set, and paying to erode that geometry is what the
/// window removes.
fn internal_bridge_offset_window(
    region: &slicer_ir::SlicedRegion,
    qualify_spacing_mm: f32,
) -> Option<ExPolygon> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for surface in &region.internal_solid_fill {
        for ring in std::iter::once(&surface.contour).chain(surface.holes.iter()) {
            for point in &ring.points {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
                any = true;
            }
        }
    }
    if !any {
        return None;
    }
    // 4.5 * spacing is the erosion delta; miter_limit 2.0 doubles its reach.
    // One extra spacing keeps the rectangle clear of the offset boundary so
    // the clipped operand is not itself the clip boundary.
    let margin_mm = (2.0 * 4.5 + 1.0) * qualify_spacing_mm;
    let margin = (f64::from(margin_mm) * slicer_ir::UNITS_PER_MM).ceil() as i64;
    Some(ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 {
                    x: min_x - margin,
                    y: min_y - margin,
                },
                slicer_ir::Point2 {
                    x: max_x + margin,
                    y: min_y - margin,
                },
                slicer_ir::Point2 {
                    x: max_x + margin,
                    y: max_y + margin,
                },
                slicer_ir::Point2 {
                    x: min_x - margin,
                    y: max_y + margin,
                },
            ],
        },
        holes: Vec::new(),
    })
}

/// Compute the exact commit result for one layer from `deep_infill_area`
/// (already lower-candidate-subtracted by the caller when required).
///
/// A surface contributes its qualified polygons when it passes both the
/// deep-sparse clip test and `qualify_internal_bridge_surface`. The two tests
/// are independent filters and the second depends only on the layer's own
/// unsupported set, never on `deep_infill_area`, so phase A runs it once per
/// surface up front (`surface_qualifications`) and this function applies only
/// the first — the same retained polygons, extended in the same surface order.
#[allow(clippy::too_many_arguments)]
fn compute_internal_bridge_layer(
    region: &slicer_ir::SlicedRegion,
    surface_qualifications: &[(usize, Vec<ExPolygon>)],
    qualify_spacing_mm: f32,
    bridge_spacing_mm: f32,
    internal_bridge_angle: f32,
    deep_infill_area: Vec<ExPolygon>,
    object_id: &ObjectId,
    region_id: RegionId,
    layer_index: u32,
    print_z: f32,
) -> Option<InternalBridgeLayerResult> {
    let deep_infill_clip_area = offset(
        &deep_infill_area,
        1.5 * qualify_spacing_mm,
        OffsetJoinType::Miter,
        0.0,
    );
    let mut qualified: Vec<ExPolygon> = Vec::new();
    for (surface_index, polys) in surface_qualifications {
        let surface = &region.internal_solid_fill[*surface_index];
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
        qualified.extend(polys.iter().cloned());
    }
    // Only the surviving candidates are filtered against this erosion,
    // so an empty candidate set skips the offset entirely.
    let internal_unsupported_area = if qualified.is_empty() || deep_infill_area.is_empty() {
        Vec::new()
    } else {
        // Only the surviving candidates consume this erosion, and each is
        // inside the offset window by construction, so the whole-layer operand
        // can be clipped without changing the retained set.
        let window = internal_bridge_offset_window(region, qualify_spacing_mm);
        let clipped;
        let operand: &[ExPolygon] = match &window {
            Some(rect) => {
                clipped = intersection(&deep_infill_area, std::slice::from_ref(rect));
                &clipped
            }
            None => &deep_infill_area,
        };
        offset(
            operand,
            -(4.5 * qualify_spacing_mm),
            OffsetJoinType::Miter,
            0.0,
        )
    };
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
        return None;
    }
    // One bridge direction per qualified polygon (canonical
    // `bridge_over_infill` direction step). The fill module draws each
    // polygon at its own angle; the region-wide
    // `bridge_orientation_deg` stays the external-bridge direction.
    slicer_core::algos::bridge_over_infill::presort_bridge_candidates(&mut qualified);
    let expansion_area = {
        let not_sparse = union(
            &union(&region.top_solid_fill, &region.bottom_solid_fill),
            &region.bridge_areas,
        );
        union(
            &difference(&region.infill_areas, &not_sparse),
            &region.internal_solid_fill,
        )
    };
    let angles = slicer_core::algos::bridge_over_infill::internal_bridge_angles(
        &qualified,
        &slicer_core::algos::bridge_over_infill::InternalBridgeAngleInputs {
            deep_infill_clip_area: &deep_infill_clip_area,
            internal_unsupported_area: &internal_unsupported_area,
            expansion_area: &expansion_area,
            total_fill_area: &region.infill_areas,
            spacing_mm: bridge_spacing_mm,
            override_deg: internal_bridge_angle,
        },
    );
    Some(InternalBridgeLayerResult { qualified, angles })
}

/// `committed` returns the internal-bridge polygons phase B has already
/// committed on a slice, if any. A commit appends them to the slice's
/// `bridge_areas`, so they are appended after the slice's own `bridge_areas`
/// here — the exact sequence the committed slice would hold.
#[allow(clippy::too_many_arguments)]
fn build_bridge_depth_layers<'c>(
    slices: &[SliceIR],
    timeline: &[usize],
    depth_start: usize,
    timeline_position: usize,
    object_id: &ObjectId,
    region_id: RegionId,
    density_for: &dyn Fn(&slicer_ir::SlicedRegion, u32) -> f32,
    committed: &dyn Fn(usize) -> Option<&'c [ExPolygon]>,
) -> Vec<slicer_core::algos::bridge_over_infill::BridgeDepthLayer> {
    timeline[depth_start..=timeline_position]
        .iter()
        .filter_map(|&lower_idx| {
            let lower = slices[lower_idx].regions.iter().find(|candidate| {
                candidate.object_id == *object_id && candidate.region_id == region_id
            })?;
            let density = density_for(lower, slices[lower_idx].global_layer_index);
            let mut not_sparse = lower.top_solid_fill.clone();
            not_sparse.extend(lower.bottom_solid_fill.iter().cloned());
            not_sparse.extend(lower.internal_solid_fill.iter().cloned());
            not_sparse.extend(lower.bridge_areas.iter().cloned());
            if let Some(polys) = committed(lower_idx) {
                not_sparse.extend(polys.iter().cloned());
            }
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
        .collect()
}

/// Settle one commit cluster's speculative outcomes, in member order.
///
/// `speculated[k]` is member `k`'s outcome computed under the assumption that
/// every earlier member committed exactly its `precomputed` result (an
/// outcome is `None` = publish the precomputed result, `Some(result)` = a
/// recompute). A member's step reads earlier members only through their
/// committed polygons (`commit`), so while every earlier member's real commit
/// equals its precomputed one, the assumption held for this member and its
/// speculative outcome is the real one. From the first member whose real
/// commit differs, every remaining member is evaluated with `evaluate`
/// against the real commits so far, keyed by slice (`slice_of`) — the plain
/// sequential loop. With `trust_speculation = false` every member is
/// evaluated that way and `speculated` is ignored. Returns the outcomes and
/// whether the speculation missed.
#[allow(clippy::too_many_arguments)]
fn settle_speculation<R>(
    members: std::ops::Range<usize>,
    trust_speculation: bool,
    speculated: Vec<Option<Option<R>>>,
    precomputed: &[Option<R>],
    commit: impl Fn(&R) -> &[ExPolygon],
    slice_of: impl Fn(usize) -> usize,
    evaluate: impl Fn(usize, &HashMap<usize, &[ExPolygon]>) -> Option<Option<R>>,
) -> (Vec<(usize, Option<Option<R>>)>, bool) {
    debug_assert!(!trust_speculation || members.len() == speculated.len());
    // Polygons really committed so far, by slice.
    let mut committed: HashMap<usize, Vec<ExPolygon>> = HashMap::new();
    let mut diverged = !trust_speculation;
    let mut missed = false;
    let mut speculated = speculated.into_iter();
    let mut outcomes = Vec::with_capacity(members.len());
    for member in members {
        let outcome = match speculated.next() {
            Some(speculated) if !diverged => speculated,
            _ => {
                let view: HashMap<usize, &[ExPolygon]> = committed
                    .iter()
                    .map(|(&slice_idx, polys)| (slice_idx, polys.as_slice()))
                    .collect();
                evaluate(member, &view)
            }
        };
        let result = match &outcome {
            Some(recomputed) => recomputed.as_ref(),
            None => precomputed[member].as_ref(),
        };
        let real_commit = result.map(&commit);
        if real_commit != precomputed[member].as_ref().map(&commit) {
            missed |= !diverged;
            diverged = true;
        }
        if let Some(polys) = real_commit {
            committed.insert(slice_of(member), polys.to_vec());
        }
        outcomes.push((member, outcome));
    }
    (outcomes, missed)
}

/// Partition commit-phase layers into independent clusters.
///
/// `positions[i]` is layer `i`'s timeline position (strictly ascending) and
/// `dependency_starts[i]` the first timeline position whose commits layer `i`
/// can read; it reads nothing at or above its own position. Returns
/// contiguous, ascending index ranges into `positions` covering every layer
/// once, such that **every earlier layer `j` with `positions[j] >=
/// dependency_starts[i]` is in layer `i`'s range**. That is the property that
/// makes running each range sequentially, ranges in parallel, equal to one
/// sequential pass: a layer can only observe commits made inside its own
/// range, and inside a range the order is the sequential order.
///
/// The windows' readable layers form a suffix of the earlier layers, so the
/// earliest readable layer decides: a layer whose window reaches back into one
/// or more earlier ranges merges them all with itself. This matters when
/// layer heights vary — a later layer's window can reach past the layer that
/// started the current range, into the range before it.
fn dependent_layer_clusters(
    positions: &[usize],
    dependency_starts: &[usize],
) -> Vec<std::ops::Range<usize>> {
    debug_assert_eq!(positions.len(), dependency_starts.len());
    debug_assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    let mut clusters: Vec<std::ops::Range<usize>> = Vec::new();
    for (index, &dependency_start) in dependency_starts.iter().enumerate() {
        let first_read =
            positions[..index].partition_point(|&position| position < dependency_start);
        if first_read == index {
            clusters.push(index..index + 1);
            continue;
        }
        let first_cluster = clusters.partition_point(|cluster| cluster.end <= first_read);
        let start = clusters[first_cluster].start;
        clusters.truncate(first_cluster);
        clusters.push(start..index + 1);
    }
    clusters
}

/// Packet 234a — canonical `bridge_over_infill` gather port (the support-math
/// functions landed in `slicer_core::algos::bridge_over_infill`). For each
/// region timeline, qualifies the upper layer's `internal_solid_fill` surfaces
/// against the committed layer below and authors the qualified internal-bridge
/// polygons into `SlicedRegion::internal_bridge_areas`, with one bridge
/// direction per polygon in `SlicedRegion::internal_bridge_angles_deg`. This is
/// the single internal-bridge producer: the fill module holding the bridge
/// claim emits each polygon at its own angle. Extends `bridge_areas` with the
/// qualified polygons so the existing partition dataflow (`region_partition`
/// derives `sparse_infill_area = difference(wall_inset, bridge ∪ bottom ∪ top)`
/// at Perimeters commit) keeps module sparse infill out of the gated area.
///
/// Qualification runs in parallel, followed by ordered commits that can read
/// earlier layers' bridge areas; the commits run sequentially inside each
/// cluster of mutually dependent layers and in parallel across clusters (see
/// `dependent_layer_clusters`). Only this prepass legally sees every
/// committed layer; per-layer stage arms run with private arenas. Config
/// resolves through `region_map.config_for(...).extensions`, mirroring how
/// undeclared module keys are routed there by the host resolver.
fn gate_internal_bridge_sites(
    slices: &mut [SliceIR],
    timelines: &HashMap<(ObjectId, RegionId), Vec<usize>>,
    region_map: &RegionMapIR,
    object_layers: &HashMap<ObjectId, HashSet<u32>>,
    lower_layer_polygons: &HashMap<(ObjectId, u32), Vec<ExPolygon>>,
) -> InternalBridgeGateStats {
    gate_internal_bridge_sites_clustered(
        slices,
        timelines,
        region_map,
        object_layers,
        lower_layer_polygons,
        dependent_layer_clusters,
        true,
    )
}

/// Counters for one `gate_internal_bridge_sites` run, summed over timelines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct InternalBridgeGateStats {
    /// Layers that left phase A with at least one qualifying surface.
    prep_layers: usize,
    /// Commit clusters those layers were split into.
    clusters: usize,
    /// Layers recomputed against committed lower layers in phase B.
    recomputed: usize,
    /// Clusters in which some layer committed something other than its
    /// precomputed result, so the speculation was abandoned from there on.
    speculation_misses: usize,
}

/// Signature of the phase-B partitioner (see `dependent_layer_clusters`).
type CommitClustering = fn(&[usize], &[usize]) -> Vec<std::ops::Range<usize>>;

/// `gate_internal_bridge_sites` with the phase-B partitioner and the
/// speculation as parameters, so tests can compare the production
/// configuration against one whole-timeline cluster without speculation —
/// the plain sequential loop — and against no grouping at all.
fn gate_internal_bridge_sites_clustered(
    slices: &mut [SliceIR],
    timelines: &HashMap<(ObjectId, RegionId), Vec<usize>>,
    region_map: &RegionMapIR,
    object_layers: &HashMap<ObjectId, HashSet<u32>>,
    lower_layer_polygons: &HashMap<(ObjectId, u32), Vec<ExPolygon>>,
    clustering: CommitClustering,
    speculate: bool,
) -> InternalBridgeGateStats {
    let mut stats = InternalBridgeGateStats::default();
    // Keep first-pass qualifications separate from carrier-free duplicates so a
    // duplicated area cannot become a new qualification candidate on the next
    // timeline entry.
    // Qualified polygons and their index-aligned bridge angles.
    type QualifiedSites = (Vec<ExPolygon>, Vec<f32>);
    let mut qualified_by_entry: HashMap<(usize, ObjectId, RegionId), QualifiedSites> =
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
                line_width: resolved.map_or(0.0, |config| config.line_width as f32),
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
        // Bridging-flow spacing for the direction step (canonical
        // `bridging_flow(frSolidInfill, true).scaled_spacing()` per candidate).
        let bridge_spacing_mm = slicer_core::flow::canonical_bridging_flow(
            ext_abs("bridge_line_width", nozzle_diameter).unwrap_or(0.0),
            ext_float("internal_bridge_flow", 1.0),
            nozzle_diameter,
        )
        .spacing_mm;
        // Canonical absolute `internal_bridge_angle` override; 0 = automatic.
        let internal_bridge_angle = ext_float("internal_bridge_angle", 0.0);

        // Ascending print_z for this timeline, resolved once. Both bridge-depth
        // windows below are expressed against it, so neither has to materialise
        // the layers it will not read.
        let timeline_zs: Vec<f32> = timeline.iter().map(|&idx| slices[idx].z).collect();

        // Phase A — qualification. Every step below reads only committed
        // geometry (`slices`, and the two lower-layer maps built before this
        // loop), and touches no field that phase B writes, so the layer axis is
        // independent here even though the commit phase that follows is not.
        // `unsupported_span_areas` alone is four full-layer clipper ops per
        // layer and was the stage's largest serial block.
        let slices_ro: &[SliceIR] = slices;
        let preps: Vec<LayerPrep> = timeline
            .par_iter()
            .enumerate()
            .filter_map(|(timeline_position, &slice_idx)| {
            // The first layer has no lower layer — nothing can span it.
            let lower_index = slices_ro[slice_idx]
                .global_layer_index
                .checked_sub(1)
                .filter(|lower_index| {
                    object_layers
                        .get(object_id)
                        .is_some_and(|layers| layers.contains(lower_index))
                })?;
            let lower_fills = lower_layer_polygons.get(&(object_id.clone(), lower_index))?;
            let empty_solids: Vec<ExPolygon> = Vec::new();
            let lower_solids = lower_layer_solids
                .get(&(object_id.clone(), lower_index))
                .unwrap_or(&empty_solids);
            let layer_index = slices_ro[slice_idx].global_layer_index;
            let print_z = slices_ro[slice_idx].z;
            let candidate = slices_ro[slice_idx]
                .regions
                .iter()
                .find(|candidate| {
                    candidate.object_id == *object_id && candidate.region_id == *region_id
                });
            let target_flow_height = candidate
                .and_then(|candidate| {
                    let key = RegionKey {
                        global_layer_index: layer_index,
                        object_id: object_id.clone(),
                        region_id: *region_id,
                        variant_chain: candidate.variant_chain.clone(),
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
                return None;
            };
            // Both early-outs below are hoisted ahead of the depth gather and
            // its offsets. They decide the layer entirely from geometry already
            // in hand, and on ordinary models they reject the large majority of
            // layer-visits (see DEV-148's skip histogram) — running them last
            // meant paying for two full-layer `offset`s per rejected layer and
            // throwing the result away. Guard order is preserved so the skip
            // histogram is unchanged.
            let candidate = match candidate {
                Some(candidate) if !candidate.internal_solid_fill.is_empty() => candidate,
                _ => {
                    log::debug!(
                        "internal bridge skip object={} region={} layer={} print_z={} reason=internal_solid_fill_empty",
                        object_id,
                        region_id,
                        layer_index,
                        print_z
                    );
                    return None;
                }
            };
            // The unsupported set is only ever read through
            // `intersection(surface, unsupported)` below, so lower-layer
            // geometry beyond the surfaces' reach is dropped first (whole
            // polygons and holes only, never clipped; see
            // `unsupported_span_areas_near`).
            let unsupported = slicer_core::algos::bridge_over_infill::unsupported_span_areas_near(
                lower_fills,
                lower_solids,
                qualify_spacing_mm,
                expansion_multiplier,
                &candidate.internal_solid_fill,
            );
            if unsupported.is_empty() {
                log::debug!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=unsupported_empty",
                    object_id,
                    region_id,
                    layer_index,
                    print_z
                );
                return None;
            }
            // `qualify_internal_bridge_surface` reads only the surface, the
            // unsupported set and the layer's spacing — not the deep-infill
            // area the later steps gather — so it is decided here, once per
            // surface. A layer none of whose surfaces qualify can only ever
            // produce an empty candidate set, in the precompute and in any
            // phase-B recompute alike, so it leaves the pipeline now instead
            // of paying for the depth gather and its offsets.
            let surface_qualifications: Vec<(usize, Vec<ExPolygon>)> = candidate
                .internal_solid_fill
                .iter()
                .enumerate()
                .filter_map(|(surface_index, surface)| {
                    slicer_core::algos::bridge_over_infill::qualify_internal_bridge_surface(
                        surface,
                        &unsupported,
                        qualify_spacing_mm,
                        nofilter,
                    )
                    // An empty acceptance extends nothing downstream.
                    .filter(|polys| !polys.is_empty())
                    .map(|polys| (surface_index, polys))
                })
                .collect();
            if surface_qualifications.is_empty() {
                log::debug!(
                    "internal bridge skip object={} region={} layer={} print_z={} reason=qualified_empty_or_deep_sparse_clip surfaces={}",
                    object_id,
                    region_id,
                    layer_index,
                    print_z,
                    candidate.internal_solid_fill.len()
                );
                return None;
            }
            Some(LayerPrep {
                timeline_position,
                slice_idx,
                layer_index,
                print_z,
                target_flow_height,
                qualify_spacing_mm,
                surface_qualifications,
            })
            })
            .collect();

        // Precompute the exact commit result for every qualified layer in
        // parallel. The only order-dependent input is the set of lower-layer
        // bridge polygons subtracted by
        // `remove_filled_polygons_on_lower_layers`; when that set is empty the
        // result below is exact and phase B merely publishes it.
        let no_commits = |_: usize| -> Option<&[ExPolygon]> { None };
        let mut precomputed: Vec<Option<InternalBridgeLayerResult>> = preps
            .par_iter()
            .map(|prep| {
                let region = slices_ro[prep.slice_idx].regions.iter().find(|candidate| {
                    candidate.object_id == *object_id && candidate.region_id == *region_id
                })?;
                let depth_start = slicer_core::algos::bridge_over_infill::depth_window_start(
                    &timeline_zs[..=prep.timeline_position],
                    prep.timeline_position,
                    prep.target_flow_height,
                    slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
                );
                let depth_layers = build_bridge_depth_layers(
                    slices_ro,
                    timeline,
                    depth_start,
                    prep.timeline_position,
                    object_id,
                    *region_id,
                    &density_for,
                    &no_commits,
                );
                let deep_infill_area = slicer_core::algos::bridge_over_infill::gather_areas_w_depth(
                    &depth_layers,
                    depth_layers.len().saturating_sub(1),
                    prep.target_flow_height,
                    slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
                );
                compute_internal_bridge_layer(
                    region,
                    &prep.surface_qualifications,
                    prep.qualify_spacing_mm,
                    bridge_spacing_mm,
                    internal_bridge_angle,
                    deep_infill_area,
                    object_id,
                    *region_id,
                    prep.layer_index,
                    prep.print_z,
                )
            })
            .collect();

        // Phase B — commit. A layer reads the commits of earlier layers of
        // this same pass in exactly two places, both over a window that ends
        // just below it: `lower_candidates` reads `internal_bridge_areas` over
        // the `filled_window_start` window, and the recompute's depth gather
        // reads `bridge_areas` over the `depth_window_start` window. Only a
        // layer in `preps` can commit. `dependent_layer_clusters` groups the
        // layers so that every earlier prep layer inside a layer's window sits
        // in that layer's own cluster; each cluster then runs the sequential
        // commit loop over its own layers in timeline order, seeing its own
        // earlier commits through `committed` and nothing else, which is
        // exactly the state the whole-timeline sequential loop would have
        // shown it. Clusters are independent and run in parallel.
        let dependency_starts: Vec<usize> = preps
            .iter()
            .map(|prep| {
                let depth_start = slicer_core::algos::bridge_over_infill::depth_window_start(
                    &timeline_zs[..=prep.timeline_position],
                    prep.timeline_position,
                    prep.target_flow_height,
                    slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
                );
                let filled_start = slicer_core::algos::bridge_over_infill::filled_window_start(
                    &timeline_zs[..prep.timeline_position],
                    prep.print_z,
                    prep.target_flow_height,
                );
                depth_start.min(filled_start)
            })
            .collect();
        let positions: Vec<usize> = preps.iter().map(|prep| prep.timeline_position).collect();
        let clusters = clustering(&positions, &dependency_starts);
        stats.prep_layers += preps.len();
        stats.clusters += clusters.len();
        let precomputed_ro = &precomputed;
        // One layer's commit-phase step against a view of the commits made
        // below it (`committed_on(slice)` = that slice's committed qualified
        // polygons, if any). `None` publishes the precomputed result;
        // `Some(result)` is a recompute against the committed lower layers.
        let evaluate = |prep_index: usize,
                        committed: &HashMap<usize, &[ExPolygon]>|
         -> Option<Option<InternalBridgeLayerResult>> {
            let prep = &preps[prep_index];
            // The layer's own slice is never committed before its own step.
            let committed_on = |slice_idx: usize| {
                (slice_idx != prep.slice_idx)
                    .then(|| committed.get(&slice_idx).copied())
                    .flatten()
            };
            // Same reasoning as `depth_start`: the callee keeps only layers at
            // or above `print_z - target_flow_height`.
            let filled_start = slicer_core::algos::bridge_over_infill::filled_window_start(
                &timeline_zs[..prep.timeline_position],
                prep.print_z,
                prep.target_flow_height,
            );
            let lower_candidates: Vec<
                slicer_core::algos::bridge_over_infill::BridgeCandidateLayer,
            > = timeline[filled_start..prep.timeline_position]
                .iter()
                .filter_map(|&lower_idx| {
                    let lower = slices_ro[lower_idx].regions.iter().find(|candidate| {
                        candidate.object_id == *object_id && candidate.region_id == *region_id
                    })?;
                    Some(
                        slicer_core::algos::bridge_over_infill::BridgeCandidateLayer {
                            print_z: slices_ro[lower_idx].z,
                            // A commit replaces the slice's
                            // `internal_bridge_areas` with its qualified
                            // polygons.
                            new_polys: committed_on(lower_idx)
                                .map(<[ExPolygon]>::to_vec)
                                .unwrap_or_else(|| lower.internal_bridge_areas.clone()),
                        },
                    )
                })
                .collect();
            // `remove_filled_polygons_on_lower_layers` is the only input that
            // depends on phase B's own earlier writes. When no lower layer
            // carries a bridge polygon the subtraction is a no-op, so the
            // independently precomputed result is exact. Otherwise recompute
            // this layer against the committed lower candidates.
            if !lower_candidates
                .iter()
                .any(|candidate| !candidate.new_polys.is_empty())
            {
                return None;
            }
            let depth_start = slicer_core::algos::bridge_over_infill::depth_window_start(
                &timeline_zs[..=prep.timeline_position],
                prep.timeline_position,
                prep.target_flow_height,
                slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
            );
            let depth_layers = build_bridge_depth_layers(
                slices_ro,
                timeline,
                depth_start,
                prep.timeline_position,
                object_id,
                *region_id,
                &density_for,
                &committed_on,
            );
            let deep_infill_area = slicer_core::algos::bridge_over_infill::gather_areas_w_depth(
                &depth_layers,
                depth_layers.len().saturating_sub(1),
                prep.target_flow_height,
                slicer_core::algos::bridge_over_infill::BRIDGE_FLOW_HEIGHT_FACTOR,
            );
            let deep_infill_area =
                slicer_core::algos::bridge_over_infill::remove_filled_polygons_on_lower_layers(
                    &deep_infill_area,
                    &lower_candidates,
                    prep.print_z,
                    prep.target_flow_height,
                );
            Some(
                slices_ro[prep.slice_idx]
                    .regions
                    .iter()
                    .find(|candidate| {
                        candidate.object_id == *object_id && candidate.region_id == *region_id
                    })
                    .and_then(|region| {
                        compute_internal_bridge_layer(
                            region,
                            &prep.surface_qualifications,
                            prep.qualify_spacing_mm,
                            bridge_spacing_mm,
                            internal_bridge_angle,
                            deep_infill_area,
                            object_id,
                            *region_id,
                            prep.layer_index,
                            prep.print_z,
                        )
                    }),
            )
        };

        // Speculation: evaluate every layer at once, in parallel, under the
        // assumption that each earlier layer commits exactly its precomputed
        // result. The assumption is checked below, in order, against what
        // each layer really commits. The only thing a later layer ever reads
        // from an earlier one is its committed qualified polygons, so a layer
        // whose earlier window members all committed their precomputed
        // polygons saw exactly the state the assumption described, and its
        // speculative outcome is its real one.
        let assumed: HashMap<usize, &[ExPolygon]> = preps
            .iter()
            .zip(precomputed_ro)
            .filter_map(|(prep, result)| {
                result
                    .as_ref()
                    .map(|result| (prep.slice_idx, result.qualified.as_slice()))
            })
            .collect();
        let speculative: Vec<Option<Option<InternalBridgeLayerResult>>> = if speculate {
            (0..preps.len())
                .into_par_iter()
                .map(|prep_index| evaluate(prep_index, &assumed))
                .collect()
        } else {
            Vec::new()
        };

        // Settle each cluster in timeline order. While every commit so far
        // matched its precomputed polygons the speculative outcome stands;
        // from the first mismatch on, the cluster's remaining layers are
        // evaluated one by one against the real commits (`committed`), which
        // is the plain sequential loop. Clusters are independent (see
        // `dependent_layer_clusters`), so they settle in parallel.
        type CommitOutcome = (usize, Option<Option<InternalBridgeLayerResult>>);
        type Speculated = Vec<Option<Option<InternalBridgeLayerResult>>>;
        let mut speculative = speculative.into_iter();
        let jobs: Vec<(std::ops::Range<usize>, Speculated)> = clusters
            .iter()
            .map(|cluster| {
                (
                    cluster.clone(),
                    speculative.by_ref().take(cluster.len()).collect(),
                )
            })
            .collect();
        let settled: Vec<(Vec<CommitOutcome>, bool)> = jobs
            .into_par_iter()
            .map(|(cluster, speculated)| {
                settle_speculation(
                    cluster,
                    speculate,
                    speculated,
                    precomputed_ro,
                    |result: &InternalBridgeLayerResult| result.qualified.as_slice(),
                    |prep_index| preps[prep_index].slice_idx,
                    evaluate,
                )
            })
            .collect();
        let mut outcomes: Vec<Vec<CommitOutcome>> = Vec::with_capacity(settled.len());
        for (cluster_outcomes, diverged) in settled {
            stats.speculation_misses += usize::from(diverged);
            outcomes.push(cluster_outcomes);
        }

        // Publish in timeline order. Each commit writes only its own slice.
        for (prep_index, outcome) in outcomes.into_iter().flatten() {
            stats.recomputed += usize::from(outcome.is_some());
            let slice_idx = preps[prep_index].slice_idx;
            let computed = match outcome {
                Some(recomputed) => recomputed,
                None => precomputed[prep_index].take(),
            };
            let Some(InternalBridgeLayerResult { qualified, angles }) = computed else {
                continue;
            };
            let Some(region) = find_region_mut(&mut slices[slice_idx], object_id, *region_id)
            else {
                continue;
            };
            // The qualified polygons are the final bridge geometry: they join
            // `bridge_areas`, so the Perimeters-commit partition cuts them out
            // of sparse infill (canonical `cut_from_infill`), and the fill
            // module emits them. Canonical anchor-driven growth beyond the
            // candidate is DEV-150.
            region.internal_bridge_areas = qualified.clone();
            region.internal_bridge_angles_deg = angles.clone();
            region.bridge_areas.extend(qualified.iter().cloned());
            qualified_by_entry.insert(
                (slice_idx, object_id.clone(), *region_id),
                (qualified, angles),
            );
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
                let Some((current_areas, current_angles)) =
                    qualified_by_entry.get(&(*current_idx, object_id.clone(), *region_id))
                else {
                    continue;
                };
                // Canonical caches the source layer's stInternalBridge polys
                // with a shrink-expand by the solid-infill extrusion WIDTH
                // (`offset_ex(shrink_ex(cache.polys, offset_distance),
                // offset_distance)` with `offset_distance =
                // flow(frSolidInfill).scaled_width()`); the same cleanup is
                // applied to the overlap below. `opening` is that
                // shrink-then-expand.
                let cleanup_width_mm = f64::from(solid_infill_width);
                let current_cleaned =
                    opening(current_areas, cleanup_width_mm, OffsetJoinType::Miter);
                if current_cleaned.is_empty() {
                    continue;
                }
                let Some(upper) = find_region_mut(&mut slices[*upper_idx], object_id, *region_id)
                else {
                    continue;
                };
                // Canonical converts only the upper layer's stInternal /
                // stInternalSolid surfaces; its own internal bridges stay as
                // they are, so they are excluded here. See
                // `upper_convertible_surfaces` for the class mapping.
                let upper_convertible = upper_convertible_surfaces(upper);
                let mut duplicates = intersection(&upper_convertible, &current_cleaned);
                if !upper.internal_bridge_areas.is_empty() {
                    duplicates = difference(&duplicates, &upper.internal_bridge_areas);
                }
                // Shrink-expand to remove trivial overlap slivers
                // (canonical `overlap = offset_ex(shrink_ex(overlap,
                // offset_distance), offset_distance)`).
                duplicates = opening(&duplicates, cleanup_width_mm, OffsetJoinType::Miter);
                if duplicates.is_empty() {
                    continue;
                }
                // Canonical second internal bridge: the layer's cached bridge
                // angle (the last internal bridge on the layer wins) turned by
                // 90 degrees, so the second layer crosses the first.
                let second_angle = current_angles
                    .last()
                    .map_or(upper.bridge_orientation_deg, |angle| {
                        (angle + 90.0).rem_euclid(180.0)
                    });
                let existing: Vec<f32> = (0..upper.internal_bridge_areas.len())
                    .map(|index| upper.internal_bridge_angle_deg(index))
                    .collect();
                upper.internal_bridge_angles_deg = existing;
                upper
                    .internal_bridge_angles_deg
                    .extend(std::iter::repeat_n(second_angle, duplicates.len()));
                upper
                    .internal_bridge_areas
                    .extend(duplicates.iter().cloned());
                // The duplicates are bridge fill: they join `bridge_areas` so
                // the partition removes them from sparse infill and the fill
                // module emits them.
                upper.bridge_areas.extend(duplicates.iter().cloned());
                // Canonical replaces the converted surfaces: the overlap
                // leaves stInternal/stInternalSolid and becomes the second
                // bridge, while the non-overlapping leftover keeps its
                // original type. `internal_solid_fill` is not part of the
                // partition's bridge cut, so remove the converted area here
                // (`top_solid_fill` too: the dense-interior marker is a subset
                // of it, and the `only_one_wall_top` exposed-top derivation
                // (`top_solid_fill - internal_solid_fill`) runs at the
                // pre-partition perimeters stage — removing from the marker
                // alone would make the converted area read as exposed top).
                if !upper.internal_solid_fill.is_empty() {
                    upper.internal_solid_fill = difference(&upper.internal_solid_fill, &duplicates);
                }
                if !upper.top_solid_fill.is_empty() {
                    upper.top_solid_fill = difference(&upper.top_solid_fill, &duplicates);
                }
            }
        }
    }
    stats
}

fn build_region_timelines(
    slices: &[SliceIR],
    raft_layer_indices: &HashSet<u32>,
) -> HashMap<(ObjectId, RegionId), Vec<usize>> {
    let mut timelines: HashMap<(ObjectId, RegionId), Vec<usize>> = HashMap::new();
    for (idx, slice) in slices.iter().enumerate() {
        // Raft layers may reference the model's regions so the raft module can
        // key its output, but they are not part of the model's shell timeline.
        // In particular, they must not consume the bottom-shell depth budget.
        if raft_layer_indices.contains(&slice.global_layer_index) {
            continue;
        }
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

/// Upper-layer surfaces canonical's extra-bridge-layer pass may convert:
/// `next_region->fill_surfaces.filter_by_types({stInternal, stInternalSolid})`.
///
/// `internal_solid_fill` is this IR's `stInternalSolid` carrier. The
/// `stInternal` half is the sparse claim — `infill_areas` minus the denser
/// claims (top, bottom, external bridge) — the same derivation the
/// qualification step uses for its expansion area. Internal-bridge surfaces
/// are excluded by the caller (canonical keeps them as they are).
fn upper_convertible_surfaces(region: &slicer_ir::SlicedRegion) -> Vec<ExPolygon> {
    let not_sparse = union(
        &union(&region.top_solid_fill, &region.bottom_solid_fill),
        &region.bridge_areas,
    );
    let st_internal = difference(&region.infill_areas, &not_sparse);
    union(&st_internal, &region.internal_solid_fill)
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

        // Zero widths keep the fixture about propagation depth, not geometry:
        // no opening is applied and the shadow never shrinks.
        let edits = compute_region_updates(
            &snapshot,
            &object_id,
            0,
            &[0, 1, 2, 3],
            0,
            3,
            ShellClassificationParams {
                opening_radius_mm: 0.0,
                shell_width_mm: 0.0,
            },
        );

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
    fn internal_bridge_gate_is_thread_count_independent_with_mixed_skips() {
        let mut snapshot: Vec<_> = (0..8)
            .map(|i| slice_with(i, vec![rect(0.0, 20.0)]))
            .collect();
        // L1, L3 and L7 bridge over sparse layers. L4 is solid but fully
        // supported by L3; L2 has neither a solid candidate nor unsupported
        // area, exercising the overlapping early-out conditions as well.
        for index in [1, 3, 4, 7] {
            snapshot[index].regions[0].internal_solid_fill = vec![rect(0.0, 20.0)];
        }
        let mut region_map = RegionMapIR::default();
        let config = region_map.intern_config(slicer_ir::ResolvedConfig {
            infill_density: 0.2,
            line_width: 0.4,
            ..Default::default()
        });
        for slice in &snapshot {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: slice.global_layer_index,
                    object_id: "o".into(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                slicer_ir::RegionPlan {
                    config,
                    ..Default::default()
                },
            );
        }
        let timelines = build_region_timelines(&snapshot, &HashSet::new());
        let object_layers = HashMap::from([("o".into(), (0..8).collect())]);
        let lower_fills = snapshot
            .iter()
            .map(|slice| {
                (
                    ("o".into(), slice.global_layer_index),
                    slice.regions[0].infill_areas.clone(),
                )
            })
            .collect();
        let run = |threads| {
            let mut slices = snapshot.clone();
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    gate_internal_bridge_sites(
                        &mut slices,
                        &timelines,
                        &region_map,
                        &object_layers,
                        &lower_fills,
                    );
                });
            let qualified: Vec<_> = slices
                .iter()
                .filter(|slice| !slice.regions[0].internal_bridge_areas.is_empty())
                .map(|slice| slice.global_layer_index)
                .collect();
            assert_eq!(
                qualified,
                vec![1, 3, 7],
                "qualification must not be vacuous"
            );
            serde_json::to_vec(&slices).expect("serialize gated slices")
        };
        let serial = run(1);
        for _ in 0..4 {
            assert_eq!(serial, run(4), "worker scheduling changed bridge output");
        }
    }

    /// The parallel shell passes must not let worker scheduling reach the output.
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

        // Production widths: the canonical opening runs on every layer here, so
        // a scheduling-dependent result would show up.
        let baseline = normalize(compute_region_updates(
            &snapshot,
            &object_id,
            0,
            &timeline,
            3,
            3,
            ShellClassificationParams::default(),
        ));
        for _ in 0..8 {
            let again = normalize(compute_region_updates(
                &snapshot,
                &object_id,
                0,
                &timeline,
                3,
                3,
                ShellClassificationParams::default(),
            ));
            assert_eq!(baseline, again, "shell classification output is not stable");
        }
    }

    /// Canonical's extra-bridge-layer pass converts
    /// `fill_surfaces.filter_by_types({stInternal, stInternalSolid})`, i.e.
    /// BOTH the sparse and the dense-interior surfaces of the layer above.
    /// This IR carries the dense class as `internal_solid_fill`; the sparse
    /// class is derived as `infill_areas − (top ∪ bottom ∪ external bridge)`.
    ///
    /// A fixture where the upper layer's sparse claim sits over a bridged
    /// lower layer must therefore be offered to the conversion — the previous
    /// operand (`internal_solid_fill` alone) never saw a sparse-only layer and
    /// produced no second bridge there.
    #[test]
    fn upper_convertible_surfaces_include_the_sparse_class() {
        let mut region = slicer_ir::SlicedRegion {
            object_id: String::from("o"),
            region_id: 0,
            polygons: vec![rect(0.0, 20.0)],
            infill_areas: vec![rect(0.0, 20.0)],
            ..Default::default()
        };
        // Sparse-only layer: no dense interior, a top claim covering the left
        // half. The convertible set must be the sparse remainder, [10,20].
        region.top_solid_fill = vec![rect(0.0, 10.0)];

        let convertible = upper_convertible_surfaces(&region);
        assert!(
            !convertible.is_empty(),
            "a sparse-only layer is still stInternal and must be convertible"
        );
        assert_eq!(
            min_x_mm(&convertible),
            10.0,
            "the top claim is not convertible (canonical keeps non-stInternal/\
             stInternalSolid surfaces as they are); the sparse remainder is"
        );

        // Dense-interior layer: `internal_solid_fill` is offered as well.
        region.internal_solid_fill = vec![rect(10.0, 20.0)];
        let convertible = upper_convertible_surfaces(&region);
        assert!(
            !convertible.is_empty(),
            "stInternalSolid must be convertible even where the sparse \
             remainder is empty"
        );
        assert_eq!(
            min_x_mm(&convertible),
            10.0,
            "the dense-interior class is stInternalSolid and must be included"
        );

        // A layer whose whole fill is top surface offers nothing.
        let mut all_top = slicer_ir::SlicedRegion {
            object_id: String::from("o"),
            region_id: 0,
            polygons: vec![rect(0.0, 20.0)],
            infill_areas: vec![rect(0.0, 20.0)],
            ..Default::default()
        };
        all_top.top_solid_fill = vec![rect(0.0, 20.0)];
        assert!(
            upper_convertible_surfaces(&all_top).is_empty(),
            "no stInternal/stInternalSolid surface means nothing to convert"
        );
    }

    /// Canonical's opening keeps a surface that the pre-t166 `line_width / 2`
    /// round opening deleted. This is the parity statement the stage now
    /// implements: the strip is 0.3 mm wide, so `2 × 0.04 mm` cannot remove it
    /// while the legacy `2 × 0.2 mm` must.
    #[test]
    fn canonical_opening_keeps_a_sub_line_width_surface_the_legacy_opening_deleted() {
        let strip = vec![rect(0.0, 0.3)];
        let kept = apply_opening(&strip, 0.04);
        assert!(
            !kept.is_empty(),
            "canonical opening (radius = external_perimeter_width / 10) must keep a \
             0.3 mm wide surface; the pre-t166 radius deleted it"
        );

        // The control: the legacy radius really does delete this fixture, or the
        // assertion above proves nothing.
        let eroded = offset(&strip, -0.2, OffsetJoinType::Round, 0.0);
        let legacy = offset(&eroded, 0.2, OffsetJoinType::Round, 0.0);
        assert!(
            legacy.is_empty(),
            "the fixture must fail under the legacy 0.2 mm round opening"
        );
    }

    /// `outer_wall_line_width = 0` is canonical's **auto** sentinel, not a
    /// zero-width wall. Resolving it literally disables the opening, which was
    /// the t166 defect: measured as a no-op (5,201,451 points in and out).
    #[test]
    fn opening_widths_resolve_the_auto_sentinel_and_split_the_two_widths() {
        let mut region_map = RegionMapIR::default();
        let mut auto = slicer_ir::ResolvedConfig {
            line_width: 0.4,
            ..Default::default()
        };
        auto.extensions
            .insert("outer_wall_line_width".into(), ConfigValue::Float(0.0));
        let auto_id = region_map.intern_config(auto);
        let mut explicit = slicer_ir::ResolvedConfig {
            line_width: 0.4,
            ..Default::default()
        };
        explicit
            .extensions
            .insert("outer_wall_line_width".into(), ConfigValue::Float(0.62));
        let explicit_id = region_map.intern_config(explicit);

        for (object, config_id) in [("auto", auto_id), ("explicit", explicit_id)] {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: 0,
                    object_id: object.into(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                slicer_ir::RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }

        let auto_params = resolve_opening_params(&region_map, &"auto".into(), 0, &[0]);
        assert!(
            (auto_params.opening_radius_mm - 0.04).abs() < 1e-6,
            "auto sentinel must resolve to line_width / 10 = 0.04, got {}",
            auto_params.opening_radius_mm
        );
        assert!(
            (auto_params.shell_width_mm - 0.4).abs() < 1e-6,
            "Pass 2's shadow width is line_width, got {}",
            auto_params.shell_width_mm
        );

        let explicit_params = resolve_opening_params(&region_map, &"explicit".into(), 0, &[0]);
        assert!(
            (explicit_params.opening_radius_mm - 0.062).abs() < 1e-6,
            "an absolute outer_wall_line_width must win, got {}",
            explicit_params.opening_radius_mm
        );
        assert!(
            (explicit_params.shell_width_mm - 0.4).abs() < 1e-6,
            "the opening radius must not leak into Pass 2, got {}",
            explicit_params.shell_width_mm
        );
    }

    /// The erosion window is a locality bound, not an approximation: clipping
    /// the deep-infill operand to the candidate bbox plus the miter reach must
    /// leave the eroded set identical *inside the candidate bbox*, even when
    /// the operand carries a separate island far outside the window.
    #[test]
    fn internal_bridge_window_preserves_the_erosion_inside_the_candidate_bbox() {
        let mut region = slicer_ir::SlicedRegion {
            object_id: String::from("o"),
            region_id: 0,
            ..Default::default()
        };
        region.internal_solid_fill = vec![rect(20.0, 30.0)];
        let deep_infill_area = vec![rect(0.0, 10.0), rect(20.0, 30.0)];
        let qualify_spacing_mm = 0.4;
        let window = internal_bridge_offset_window(&region, qualify_spacing_mm)
            .expect("a non-empty candidate set has a window");
        let clipped = intersection(&deep_infill_area, std::slice::from_ref(&window));
        assert_ne!(
            clipped, deep_infill_area,
            "the far island must be clipped away or the fixture proves nothing"
        );
        let candidate_bbox = vec![rect(20.0, 30.0)];
        let delta = -4.5 * qualify_spacing_mm;
        let whole = offset(&deep_infill_area, delta, OffsetJoinType::Miter, 0.0);
        let windowed = offset(&clipped, delta, OffsetJoinType::Miter, 0.0);
        let whole_inside = intersection(&whole, &candidate_bbox);
        let windowed_inside = intersection(&windowed, &candidate_bbox);
        assert_eq!(
            whole_inside, windowed_inside,
            "the erosion moved inside the candidate bbox"
        );
    }

    /// Index of the cluster holding layer `index`.
    fn cluster_of(clusters: &[std::ops::Range<usize>], index: usize) -> usize {
        clusters
            .iter()
            .position(|cluster| cluster.contains(&index))
            .expect("every layer is in a cluster")
    }

    /// Every earlier layer a layer can read shares its cluster.
    fn respects_dependencies(
        clusters: &[std::ops::Range<usize>],
        positions: &[usize],
        dependency_starts: &[usize],
    ) -> bool {
        (0..positions.len()).all(|reader| {
            (0..reader)
                .filter(|&earlier| positions[earlier] >= dependency_starts[reader])
                .all(|earlier| cluster_of(clusters, earlier) == cluster_of(clusters, reader))
        })
    }

    /// The pre-fix rule this partition replaced in the prototype: chain a layer
    /// to the current cluster only when its immediate predecessor is readable.
    fn previous_layer_clusters(
        positions: &[usize],
        dependency_starts: &[usize],
    ) -> Vec<std::ops::Range<usize>> {
        let mut clusters: Vec<std::ops::Range<usize>> = Vec::new();
        for index in 0..positions.len() {
            let joins = index > 0 && positions[index - 1] >= dependency_starts[index];
            match clusters.last_mut() {
                Some(cluster) if joins => cluster.end = index + 1,
                _ => clusters.push(index..index + 1),
            }
        }
        clusters
    }

    /// Commit clusters must contain every layer a member can read, and must not
    /// merge layers that no dependency joins (that would only cost parallelism).
    #[test]
    fn dependent_layer_clusters_hold_every_readable_layer_and_nothing_more() {
        // Variable layer heights: layer 1's window stops above layer 0, but
        // layer 2's longer window reaches past layer 1 into layer 0.
        let positions = [3, 6, 7];
        let dependency_starts = [2, 5, 3];
        assert_eq!(
            dependent_layer_clusters(&positions, &dependency_starts),
            vec![0..3]
        );
        let naive = previous_layer_clusters(&positions, &dependency_starts);
        assert!(
            !respects_dependencies(&naive, &positions, &dependency_starts),
            "negative control: chaining to the predecessor alone splits layer 2 from layer 0"
        );

        let mut state = 0x0001_84c1_u64;
        let mut next = |bound: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % bound as u64) as usize
        };
        let mut split_cases = 0;
        let mut merge_cases = 0;
        for _ in 0..4_000 {
            let count = 1 + next(12);
            let mut positions = Vec::with_capacity(count);
            let mut position = next(3);
            for _ in 0..count {
                positions.push(position);
                position += 1 + next(4);
            }
            let dependency_starts: Vec<usize> = positions
                .iter()
                .map(|&position| position.saturating_sub(next(7)))
                .collect();
            let clusters = dependent_layer_clusters(&positions, &dependency_starts);
            // A contiguous, ascending cover of every layer.
            assert_eq!(clusters.first().map(|c| c.start), Some(0));
            assert_eq!(clusters.last().map(|c| c.end), Some(count));
            assert!(clusters.windows(2).all(|pair| pair[0].end == pair[1].start));
            assert!(clusters.iter().all(|cluster| !cluster.is_empty()));
            assert!(
                respects_dependencies(&clusters, &positions, &dependency_starts),
                "positions {positions:?} starts {dependency_starts:?} -> {clusters:?}"
            );
            // Minimal: every internal boundary of a cluster is crossed by a
            // dependency, so no cluster could be split further.
            for cluster in &clusters {
                for boundary in cluster.start + 1..cluster.end {
                    let crossed = (boundary..cluster.end).any(|reader| {
                        (cluster.start..boundary)
                            .any(|earlier| positions[earlier] >= dependency_starts[reader])
                    });
                    assert!(crossed, "{clusters:?} over-merges at {boundary} for {positions:?} / {dependency_starts:?}");
                }
            }
            split_cases += usize::from(clusters.len() > 1);
            merge_cases += usize::from(!respects_dependencies(
                &previous_layer_clusters(&positions, &dependency_starts),
                &positions,
                &dependency_starts,
            ));
        }
        assert!(
            split_cases > 1_000,
            "too few fixtures split into parallel clusters"
        );
        assert!(
            merge_cases > 0,
            "no fixture needed a merge past the predecessor"
        );
    }

    /// Fixture for the commit-phase tests: L1 bridges over sparse L0 on the
    /// left, L2 bridges over sparse L1 on the right — and L2 sits inside L1's
    /// one-flow-height window, so its commit must see L1's. L6 bridges over
    /// sparse L5 far above, independently.
    fn dependent_bridge_fixture() -> (Vec<SliceIR>, RegionMapIR) {
        let mut snapshot: Vec<_> = (0..9)
            .map(|i| slice_with(i, vec![rect(0.0, 20.0)]))
            .collect();
        snapshot[1].regions[0].internal_solid_fill = vec![rect(0.0, 8.0)];
        snapshot[2].regions[0].internal_solid_fill = vec![rect(10.0, 20.0)];
        snapshot[6].regions[0].internal_solid_fill = vec![rect(0.0, 20.0)];
        let mut region_map = RegionMapIR::default();
        let config = region_map.intern_config(slicer_ir::ResolvedConfig {
            infill_density: 0.2,
            line_width: 0.4,
            ..Default::default()
        });
        for slice in &snapshot {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: slice.global_layer_index,
                    object_id: "o".into(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                slicer_ir::RegionPlan {
                    config,
                    ..Default::default()
                },
            );
        }
        (snapshot, region_map)
    }

    fn run_gate_with(
        snapshot: &[SliceIR],
        region_map: &RegionMapIR,
        clustering: CommitClustering,
        speculate: bool,
    ) -> (Vec<u8>, InternalBridgeGateStats, Vec<u32>) {
        let timelines = build_region_timelines(snapshot, &HashSet::new());
        let object_layers = HashMap::from([(
            "o".into(),
            snapshot
                .iter()
                .map(|slice| slice.global_layer_index)
                .collect(),
        )]);
        let lower_fills = snapshot
            .iter()
            .map(|slice| {
                (
                    ("o".into(), slice.global_layer_index),
                    slice.regions[0].infill_areas.clone(),
                )
            })
            .collect();
        let mut slices = snapshot.to_vec();
        let stats = gate_internal_bridge_sites_clustered(
            &mut slices,
            &timelines,
            region_map,
            &object_layers,
            &lower_fills,
            clustering,
            speculate,
        );
        let bridged = slices
            .iter()
            .filter(|slice| !slice.regions[0].internal_bridge_areas.is_empty())
            .map(|slice| slice.global_layer_index)
            .collect();
        (
            serde_json::to_vec(&slices).expect("serialize gated slices"),
            stats,
            bridged,
        )
    }

    /// The production commit phase (dependency clusters in parallel,
    /// speculative recomputes) must reproduce the plain sequential loop — one
    /// whole-timeline cluster, no speculation — exactly. The singleton
    /// partition without speculation is the negative control: it hides L1's
    /// commit from L2, which shows the fixture really has a cross-layer read.
    #[test]
    fn clustered_commits_equal_the_sequential_commit_loop() {
        let (snapshot, region_map) = dependent_bridge_fixture();
        let sequential: CommitClustering = |positions, _| {
            (!positions.is_empty())
                .then_some(0..positions.len())
                .into_iter()
                .collect()
        };
        let singletons: CommitClustering =
            |positions, _| (0..positions.len()).map(|i| i..i + 1).collect();

        let (clustered, clustered_stats, bridged) =
            run_gate_with(&snapshot, &region_map, dependent_layer_clusters, true);
        let (serial, serial_stats, _) = run_gate_with(&snapshot, &region_map, sequential, false);
        let (unspeculated, _, _) =
            run_gate_with(&snapshot, &region_map, dependent_layer_clusters, false);
        let (_, singleton_stats, _) = run_gate_with(&snapshot, &region_map, singletons, false);

        assert!(
            bridged.contains(&1) && bridged.contains(&6),
            "bridged layers: {bridged:?}"
        );
        assert!(
            clustered_stats.clusters > 1,
            "the fixture must split into parallel clusters: {clustered_stats:?}"
        );
        assert!(
            singleton_stats.recomputed < serial_stats.recomputed,
            "negative control: no layer read a lower commit ({singleton_stats:?} vs {serial_stats:?})"
        );
        assert_eq!(clustered_stats.recomputed, serial_stats.recomputed);
        assert_eq!(
            clustered, serial,
            "clustered commits diverged from the sequential loop"
        );
        assert_eq!(
            unspeculated, serial,
            "unspeculated clusters diverged from the sequential loop"
        );
    }

    /// Pass 2's per-target parallel merge must equal the sequential union fold
    /// bit for bit, and the skipped `internal_solid_fill` differences must be
    /// exactly the empty ones. (The per-target fold order is preserved by
    /// construction; clipper's unions on this fixture happen to be
    /// order-insensitive, so the test pins the result, not the order.)
    #[test]
    fn pass2_merge_matches_the_sequential_union_fold_and_skips_only_empty_diffs() {
        // Three top seeds (L2's right strip, L4's middle strip, L5 whole),
        // two of them projecting onto L3 and L2.
        let widths = [30.0, 30.0, 30.0, 20.0, 20.0, 10.0];
        let snapshot: Vec<SliceIR> = widths
            .iter()
            .enumerate()
            .map(|(i, &width)| slice_with(i as u32, vec![rect(0.0, width)]))
            .collect();
        let object_id = String::from("o");
        let timeline: Vec<usize> = (0..snapshot.len()).collect();
        let k_top = 4u8;
        let edits = compute_region_updates(
            &snapshot,
            &object_id,
            0,
            &timeline,
            k_top,
            0,
            ShellClassificationParams {
                opening_radius_mm: 0.0,
                shell_width_mm: 0.0,
            },
        );

        // Sequential oracle: seeds from the immutable snapshot, then every
        // seed's chain folded in ascending seed order.
        let polys: Vec<Vec<ExPolygon>> = snapshot
            .iter()
            .map(|s| s.regions[0].polygons.clone())
            .collect();
        let mut seeds: HashMap<usize, Vec<ExPolygon>> = HashMap::new();
        for pos in 0..polys.len() {
            let upper = polys.get(pos + 1).cloned().unwrap_or_default();
            let seed = difference(&polys[pos], &upper);
            if !seed.is_empty() {
                seeds.insert(pos, seed);
            }
        }
        let mut fills: HashMap<usize, Vec<ExPolygon>> = seeds.clone();
        let mut contributions: HashMap<usize, usize> = HashMap::new();
        let mut seed_positions: Vec<usize> = seeds.keys().copied().collect();
        seed_positions.sort_unstable();
        for &pos in &seed_positions {
            let mut shadow = seeds[&pos].clone();
            for depth in 1..k_top.min((pos + 1) as u8) {
                let target = pos - depth as usize;
                let projected = intersection(&shadow, &polys[target]);
                if projected.is_empty() {
                    break;
                }
                let existing = fills.remove(&target).unwrap_or_default();
                fills.insert(target, union(&existing, &projected));
                *contributions.entry(target).or_default() += 1;
                shadow = projected;
            }
        }
        assert!(
            contributions.values().any(|&count| count >= 2),
            "no slice received two shadows, so merge order went untested"
        );
        assert!(
            seeds.keys().any(|pos| !contributions.contains_key(pos)),
            "no seed slice went without a shadow, so the skipped difference went untested"
        );

        for edit in &edits {
            let want_fill = fills.get(&edit.slice_idx);
            assert_eq!(
                edit.update.top_solid_fill.as_ref(),
                want_fill,
                "slice {}: top fill differs from the sequential fold",
                edit.slice_idx
            );
            if let Some(fill) = want_fill {
                let seed = seeds.get(&edit.slice_idx).cloned().unwrap_or_default();
                assert_eq!(
                    edit.update.internal_solid_fill.as_ref(),
                    Some(&difference(fill, &seed)),
                    "slice {}: internal solid fill differs from fill \\ seed",
                    edit.slice_idx
                );
            }
        }
    }

    /// Minimal commit result for driving `settle_speculation` directly.
    #[derive(Clone, Debug, PartialEq)]
    struct Commit(Vec<ExPolygon>);

    fn commit_of(width: f32) -> Option<Commit> {
        Some(Commit(vec![rect(0.0, width)]))
    }

    /// While every real commit equals its precomputed one, speculative
    /// outcomes are published as they are and nothing is re-evaluated.
    #[test]
    fn settled_speculation_keeps_outcomes_while_commits_match() {
        let precomputed = vec![commit_of(1.0), None, commit_of(3.0)];
        // Member 0 publishes its precomputed result, member 1 recomputes to
        // the same nothing, member 2 recomputes to the same polygons.
        let speculated = vec![None, Some(None), Some(commit_of(3.0))];
        let (outcomes, missed) = settle_speculation(
            0..3,
            true,
            speculated.clone(),
            &precomputed,
            |commit: &Commit| commit.0.as_slice(),
            |member| 10 + member,
            |member, _| panic!("member {member} must not be re-evaluated"),
        );
        assert!(!missed);
        assert_eq!(
            outcomes,
            vec![
                (0, speculated[0].clone()),
                (1, speculated[1].clone()),
                (2, speculated[2].clone())
            ]
        );
    }

    /// The first commit that differs from its precomputed one invalidates the
    /// speculation for every later member, which is then evaluated against
    /// the real commits; the differing member's own outcome stands, since its
    /// inputs were still as assumed.
    #[test]
    fn settled_speculation_reevaluates_every_member_after_a_miss() {
        let precomputed = vec![commit_of(1.0), commit_of(2.0), commit_of(3.0), None];
        let speculated = vec![
            None,
            Some(commit_of(2.5)),
            Some(commit_of(99.0)),
            Some(commit_of(98.0)),
        ];
        let seen = std::cell::RefCell::new(Vec::new());
        let (outcomes, missed) = settle_speculation(
            0..4,
            true,
            speculated,
            &precomputed,
            |commit: &Commit| commit.0.as_slice(),
            |member| 10 + member,
            |member, committed| {
                let mut view: Vec<(usize, Vec<ExPolygon>)> = committed
                    .iter()
                    .map(|(&slice, polys)| (slice, polys.to_vec()))
                    .collect();
                view.sort_by_key(|(slice, _)| *slice);
                seen.borrow_mut().push((member, view));
                Some(commit_of(7.0 + member as f32))
            },
        );
        assert!(missed);
        assert_eq!(
            outcomes,
            vec![
                (0, None),
                (1, Some(commit_of(2.5))),
                (2, Some(commit_of(9.0))),
                (3, Some(commit_of(10.0))),
            ]
        );
        let seen = seen.into_inner();
        assert_eq!(
            seen.len(),
            2,
            "exactly the members after the miss are re-evaluated"
        );
        assert_eq!(
            seen[0],
            (
                2,
                vec![(10, vec![rect(0.0, 1.0)]), (11, vec![rect(0.0, 2.5)])]
            ),
            "member 2 sees member 0's precomputed and member 1's real commit"
        );
        assert_eq!(seen[1].1.last(), Some(&(12, vec![rect(0.0, 9.0)])));
    }

    /// Without trusting the speculation every member is evaluated in order.
    #[test]
    fn untrusted_speculation_is_the_sequential_loop() {
        let precomputed = vec![commit_of(1.0), commit_of(2.0)];
        let calls = std::cell::Cell::new(0);
        let (outcomes, missed) = settle_speculation(
            0..2,
            false,
            Vec::new(),
            &precomputed,
            |commit: &Commit| commit.0.as_slice(),
            |member| member,
            |_, _| {
                calls.set(calls.get() + 1);
                None
            },
        );
        assert!(!missed, "nothing was speculated, so nothing can miss");
        assert_eq!(calls.get(), 2);
        assert_eq!(outcomes, vec![(0, None), (1, None)]);
    }
}
