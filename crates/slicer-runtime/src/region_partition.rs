//! Host-side fill-polygon partition.
//!
//! Runs as a side effect at `Layer::Perimeters` commit (see
//! `commit_layer_outputs` in `layer_executor.rs`). Mutates the per-layer
//! arena's `SliceIR` only — the Blackboard's PrePass-committed
//! `SliceIR` Vec stays canonical (per the slice-prepass-migration invariant
//! that the Blackboard is read-only during Tier 2).
//!
//! For each `(object_id, region_id)` present in `arena.slice()`, the helper
//! finds the matching entry in `arena.perimeter()` and replaces the four
//! canonical fill polygons in place. The wall-inset polygon
//! (`perimeter.infill_areas`) is partitioned by strict precedence
//! `bridge > bottom > top > sparse`, mirroring OrcaSlicer
//! `PrintObject::prepare_infill` (see `OrcaSlicerDocumented/src/libslic3r/
//! PrintObject.cpp:1541-1892` and `:3928-4132`):
//!
//! ```text
//! bridge_final = bridge_areas      ∩ perimeter.infill_areas
//! bottom_final = (bottom_solid_fill ∩ perimeter.infill_areas) − bridge_final
//! top_final    = (top_solid_fill    ∩ perimeter.infill_areas)
//!                  − (bridge_final ∪ bottom_final)
//! sparse       = perimeter.infill_areas
//!                  − (bridge_final ∪ bottom_final ∪ top_final)
//! ```
//!
//! After the hook the four canonical fill polygons are pairwise disjoint
//! subsets of `perimeter.infill_areas`. Fill claim holders (rectilinear,
//! gyroid, lightning infill modules) emit each role over exactly one
//! polygon with zero polygon math.
//!
//! Missing-perimeter behaviour: a `SliceIR` region without a matching
//! `PerimeterIR` entry is skipped (its four canonical fill polygons stay at
//! whatever PrePass left them) and the host emits a structured `log::warn!`
//! naming the offending `(object_id, region_id)` so the failure mode is
//! observable in production logs (`docs/specs/infill-fill-partition-plan.md`
//! Phase B3 / review finding #3). Real configurations exist where a virtual
//! variant region (region_split work, packets 92–95) is committed to
//! `SliceIR` without a per-variant perimeter entry — the variant's wall
//! geometry is shared with its base region. Treating that as fatal would
//! poison the entire layer; the safer contract is "no perimeter → no
//! repartition for this region, but log it". The IR-level fatals
//! (`take_slice` / `arena.perimeter()` both `None`) are preserved because
//! those represent a genuine stage-ordering violation, not a per-region
//! absence.
//!
//! Empty-wall-inset behaviour: a `PerimeterIR` entry whose `infill_areas`
//! is empty (perimeter stage emitted no infill — thin-walled regions or
//! painted regions where the perimeters dispatch produced no
//! `set_infill_areas` call) does NOT collapse `top_solid_fill` /
//! `bottom_solid_fill` to empty. The intersection with an empty wall inset
//! would discard the exposed top surface that the shell-classification
//! step deliberately marked, breaking surface-treatment stages such as
//! ironing. The fallback preserves the original PrePass fill polygons
//! (modulo the bridge / bottom precedence zones) for those regions. The
//! sparse role stays empty by construction (no infill center was produced).
//! See `cube_4color_ironing_per_painted_top_color_tdd` in
//! `tests/executor/` for the regression.

use slicer_core::polygon_ops::{difference, intersection, union};
use slicer_ir::{LayerStageError, SlicedRegion, StageId};

use crate::LayerArena;

/// Reserved `region_id` flagging a `SlicedRegion` as a modifier footprint staged
/// for `sync_perimeter_infill_areas_into_slice` to consume (packet 132).
///
/// Re-exported from `slicer-ir`, which owns it so that `slicer-wasm-host`'s
/// `push_slice_regions` can filter footprints out of guest views without
/// depending on this crate.
pub use slicer_ir::MODIFIER_FOOTPRINT_REGION_ID;

/// Modifier `region_id` namespace stride (next prime above paint's 1_000_000).
/// A minted modifier sub-region id is `base_region_id * STRIDE + hash`
/// (`hash != 0`), so integer division by `STRIDE` inverts to the base id.
pub const MODIFIER_VARIANT_REGION_ID_STRIDE: u64 = 1_000_003;

/// Reconcile the four canonical fill polygons on every `SliceIR` region
/// against the just-committed `PerimeterIR.infill_areas`. See module docs
/// for the precedence rule and clip-in-place semantics.
///
/// Errors:
/// - `LayerStageError::FatalModule` when a slice region has no matching
///   perimeter region. The message names `(object_id, region_id)`.
/// - `LayerStageError::FatalModule` when neither `SliceIR` nor
///   `PerimeterIR` is staged on the arena (the hook must run after both
///   `Layer::Slice` and `Layer::Perimeters` have committed).
/// - `LayerStageError::ArenaCommit` if the post-mutation `set_slice` fails.
pub fn sync_perimeter_infill_areas_into_slice(
    arena: &mut LayerArena,
    layer_index: u32,
) -> Result<(), LayerStageError> {
    let stage_id: StageId = "Layer::Perimeters".into();
    let module_id = "host:region_partition".to_string();

    let mut slice = arena
        .take_slice()
        .ok_or_else(|| LayerStageError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module_id.clone(),
            message: format!(
                "region_partition at layer {layer_index}: no staged SliceIR \
             (host built-in PrePass::Slice must commit before Layer::Perimeters runs)"
            ),
        })?;

    // Borrow perimeter immutably — we only read infill_areas off it.
    let perimeter = match arena.perimeter() {
        Some(p) => p,
        None => {
            // Re-stage the slice we just took so callers can recover.
            let _ = arena.set_slice(slice);
            return Err(LayerStageError::FatalModule {
                stage_id,
                module_id,
                message: format!(
                    "region_partition at layer {layer_index}: no staged PerimeterIR \
                     (Layer::Perimeters must commit before this hook fires)"
                ),
            });
        }
    };

    // Build a (object_id, region_id) → PerimeterRegion index once before the
    // slice-region loop. Replaces a linear `perimeter.regions.iter().find()`
    // per slice region (review finding #7; O(N×M) → O(N+M)). With
    // variant_chain work (packets 92–95) growing both N and M, the linear
    // scan was real wall-clock cost on multi-color prints.
    // Shared with the Layer::InfillPostProcess dispatch arm's wall-source
    // predicate (ADR-0028 §Amendment): a slice region missing from this index
    // is a virtual variant sharing its base region's walls.
    let perim_index = slicer_wasm_host::dispatch::perimeter_region_index(&perimeter);

    for slice_region in &mut slice.regions {
        // A modifier footprint is never handed to a module, so it never has a
        // perimeter entry. Skip it explicitly rather than let it fall into the
        // virtual-variant branch below, which would log a warning per footprint
        // per layer. `split_modifier_footprints` consumes it further down.
        if slice_region.region_id == MODIFIER_FOOTPRINT_REGION_ID {
            continue;
        }
        let Some(perim) = perim_index
            .get(&(&slice_region.object_id, slice_region.region_id))
            .copied()
        else {
            // No perimeter entry for this slice region — typically a virtual
            // variant region (region_split work, packets 92–95) sharing wall
            // geometry with its base region. Leave the four canonical fill
            // polygons untouched; the base region's partition is canonical
            // for the variant's geometry too. Emit a structured warning so
            // the failure mode is observable in production logs (B3).
            log::warn!(
                "region_partition at layer {layer_index}: no PerimeterIR entry \
                 for SliceIR region (object_id='{}', region_id='{}'); skipping — \
                 variant region with shared base-region wall geometry \
                 (packets 92–95). Top/bottom/bridge fill polygons remain at \
                 PrePass values for this region.",
                slice_region.object_id,
                slice_region.region_id
            );
            continue;
        };

        let wall_inset = &perim.infill_areas;

        // Precedence: bridge > bottom > top > sparse.
        //
        // Edge case (fix): when the perimeter stage produces no infill area
        // for a region (e.g., a thin-walled region whose inset collapses to
        // empty, or a region whose perimeter dispatch never reached
        // `set_infill_areas`), `wall_inset` is the empty set. The naive
        // `intersection(top_solid_fill, wall_inset)` would wipe
        // `top_solid_fill` to empty, discarding an exposed top surface that
        // the shell-classification step deliberately marked. Ironing then
        // skips the region (gate at
        // `modules/core-modules/top-surface-ironing/src/lib.rs:316-327`
        // requires non-empty `top_solid_fill`). The fallback preserves
        // the original `top_solid_fill` / `bottom_solid_fill` polygons
        // (minus the bridge / bottom precedence zones) so that
        // surface-treatment stages still see the exposed top. For the
        // common case where `wall_inset` is non-empty the precedence path
        // is unchanged.
        //
        // Note (cube_4color diagnostic, 2026-06-30): runtime
        // instrumentation on `resources/cube_4color.3mf` showed
        // `wall_inset` is non-empty for the affected region (`rid=0`) at
        // the top layer, so this fallback branch never fires and the
        // remaining ironing-on-one-color symptom is rooted upstream of
        // `region_partition`. The fix is still a defensive correctness
        // improvement; the cube_4color test in
        // `cube_4color_ironing_per_painted_top_color_tdd` is a RED gate
        // tracking the open root cause.
        let bridge = intersection(&slice_region.bridge_areas, wall_inset);
        let bottom = if wall_inset.is_empty() {
            Vec::new()
        } else {
            difference(
                &intersection(&slice_region.bottom_solid_fill, wall_inset),
                &bridge,
            )
        };
        let bridge_or_bottom = union(&bridge, &bottom);
        let top = if wall_inset.is_empty() {
            difference(&slice_region.top_solid_fill, &bridge_or_bottom)
        } else {
            difference(
                &intersection(&slice_region.top_solid_fill, wall_inset),
                &bridge_or_bottom,
            )
        };
        let bridge_or_bottom_or_top = union(&bridge_or_bottom, &top);
        let sparse = difference(wall_inset, &bridge_or_bottom_or_top);

        slice_region.bridge_areas = bridge;
        slice_region.bottom_solid_fill = bottom;
        slice_region.top_solid_fill = top;
        slice_region.sparse_infill_area = sparse;
    }

    // Modifier region split (packet 132): consume any MODIFIER_FOOTPRINT_REGION_ID
    // footprints staged on this layer, minting a sub-region in the modifier
    // `region_id` namespace whose geometry is the intersection of the footprint
    // with the base region's four partitioned fill polygons. The base region's
    // polygons are reduced to the difference. Runs AFTER the existing partition
    // so it composes on already-partitioned polygons.
    split_modifier_footprints(&mut slice);

    arena
        .set_slice(slice)
        .map_err(|source| LayerStageError::ArenaCommit { source })?;

    Ok(())
}

/// Derive a stable modifier sub-region id (`base_region_id * STRIDE + hash`,
/// `hash != 0`) from the base region id and the modifier footprint geometry.
/// Hashing the footprint polygon points keeps the id stable for a given
/// modifier cross-section and distinct across modifiers within the same base.
fn modifier_sub_region_id(
    base_region_id: u64,
    object_id: &str,
    footprint_geo: &[slicer_ir::ExPolygon],
) -> u64 {
    // FNV-1a over object_id bytes + footprint contour points.
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    for b in object_id.as_bytes() {
        mix(*b);
    }
    for ep in footprint_geo {
        for p in &ep.contour.points {
            for byte in (p.x as u64).to_le_bytes() {
                mix(byte);
            }
            for byte in (p.y as u64).to_le_bytes() {
                mix(byte);
            }
        }
    }
    let hash = (h % (MODIFIER_VARIANT_REGION_ID_STRIDE - 1)) + 1;
    base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE + hash
}

// INVARIANT: modifier footprint regions MUST appear AFTER their corresponding
// base regions in the input `SliceIR.regions` vec. `split_modifier_footprints`
// resolves each footprint to its base via `out.iter().position(...)`, which only
// scans regions already emitted into `out` (the base regions appended earlier in
// this same pass). If a footprint preceded its base, `position()` returns `None`
// and the footprint is silently consumed without minting a sub-region.
//
// Loader-side guarantee: `crates/slicer-model-io/src/loader.rs` (lines 547-628,
// the `ModifierVolume` shape) loads `ObjectMesh.modifier_volumes` into `MeshIR`
// AFTER the base mesh, preserving doc-order so modifier footprints are staged
// after base regions.
//
// Runtime call site that maintains the invariant:
// `crates/slicer-runtime/src/layer_executor.rs::stage_modifier_footprints`
// appends modifier footprints after the base regions.

/// Packet 132 modifier region split.
///
/// For every `SlicedRegion` flagged with `MODIFIER_FOOTPRINT_REGION_ID`, find
/// the matching base region (same `object_id`, non-footprint), intersect the
/// footprint geometry with the base region's four partitioned fill polygons,
/// and mint a sub-region carrying those intersections. The base region's four
/// polygons are reduced to the difference (base ∖ footprint). A footprint whose
/// intersection with the base is empty (degenerate / out-of-layer) mints no
/// sub-region. The footprint region is always consumed (removed) and the
/// sub-region carries no own `PerimeterIR` entry — it borrows the base walls.
fn split_modifier_footprints(slice: &mut slicer_ir::SliceIR) {
    let has_footprint = slice
        .regions
        .iter()
        .any(|r| r.region_id == MODIFIER_FOOTPRINT_REGION_ID);
    if !has_footprint {
        return;
    }

    let regions = std::mem::take(&mut slice.regions);
    let mut out: Vec<SlicedRegion> = Vec::with_capacity(regions.len());
    let mut minted: Vec<SlicedRegion> = Vec::new();

    for r in regions {
        if r.region_id == MODIFIER_FOOTPRINT_REGION_ID {
            let obj = r.object_id.clone();
            // Locate the matching base region (same object_id, not a footprint).
            if let Some(bi) = out
                .iter()
                .position(|x| x.object_id == obj && x.region_id != MODIFIER_FOOTPRINT_REGION_ID)
            {
                let base_region_id = out[bi].region_id;
                let eff = out[bi].effective_layer_height;
                let fp_geo = r.polygons.clone();

                let sub_bridge = intersection(&out[bi].bridge_areas, &fp_geo);
                let sub_bottom = intersection(&out[bi].bottom_solid_fill, &fp_geo);
                let sub_top = intersection(&out[bi].top_solid_fill, &fp_geo);
                let sub_sparse = intersection(&out[bi].sparse_infill_area, &fp_geo);

                let has_geo = !sub_bridge.is_empty()
                    || !sub_bottom.is_empty()
                    || !sub_top.is_empty()
                    || !sub_sparse.is_empty();

                if has_geo {
                    out[bi].bridge_areas = difference(&out[bi].bridge_areas, &fp_geo);
                    out[bi].bottom_solid_fill = difference(&out[bi].bottom_solid_fill, &fp_geo);
                    out[bi].top_solid_fill = difference(&out[bi].top_solid_fill, &fp_geo);
                    out[bi].sparse_infill_area = difference(&out[bi].sparse_infill_area, &fp_geo);

                    let sub_polygons = intersection(&out[bi].polygons, &fp_geo);
                    let sub_id = modifier_sub_region_id(base_region_id, &obj, &r.polygons);
                    minted.push(SlicedRegion {
                        object_id: obj.clone(),
                        region_id: sub_id,
                        polygons: sub_polygons,
                        infill_areas: sub_sparse.clone(),
                        effective_layer_height: eff,
                        variant_chain: Vec::new(),
                        bridge_areas: sub_bridge,
                        bottom_solid_fill: sub_bottom,
                        top_solid_fill: sub_top,
                        sparse_infill_area: sub_sparse,
                        // Inherit the base region's shell-classification fields.
                        // The sub-region's polygons are subsets of the base's by
                        // construction (sub_top ⊆ base.top_solid_fill, sub_bridge
                        // ⊆ base.bridge_areas), so depth/orientation are
                        // geometrically identical. Precedent: paint segmentation's
                        // Phase 6/7 fix at
                        // crates/slicer-core/src/algos/paint_segmentation/mod.rs:920-942.
                        top_shell_index: out[bi].top_shell_index,
                        bottom_shell_index: out[bi].bottom_shell_index,
                        is_bridge: out[bi].is_bridge,
                        bridge_orientation_deg: out[bi].bridge_orientation_deg,
                        ..Default::default()
                    });
                }
            }
            // Footprint region is consumed (removed); never re-pushed.
        } else {
            out.push(r);
        }
    }

    out.extend(minted);
    slice.regions = out;
}
