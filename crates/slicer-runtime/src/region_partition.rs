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

use std::collections::HashMap;

use slicer_core::polygon_ops::{difference, intersection, union};
use slicer_ir::{LayerStageError, ObjectId, PerimeterRegion, RegionId, StageId};

use crate::LayerArena;

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
    let perim_index: HashMap<(&ObjectId, &RegionId), &PerimeterRegion> = perimeter
        .regions
        .iter()
        .map(|r| ((&r.object_id, &r.region_id), r))
        .collect();

    for slice_region in &mut slice.regions {
        let Some(perim) = perim_index
            .get(&(&slice_region.object_id, &slice_region.region_id))
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
        let bridge = intersection(&slice_region.bridge_areas, wall_inset);
        let bottom = difference(
            &intersection(&slice_region.bottom_solid_fill, wall_inset),
            &bridge,
        );
        let bridge_or_bottom = union(&bridge, &bottom);
        let top = difference(
            &intersection(&slice_region.top_solid_fill, wall_inset),
            &bridge_or_bottom,
        );
        let bridge_or_bottom_or_top = union(&bridge_or_bottom, &top);
        let sparse = difference(wall_inset, &bridge_or_bottom_or_top);

        slice_region.bridge_areas = bridge;
        slice_region.bottom_solid_fill = bottom;
        slice_region.top_solid_fill = top;
        slice_region.sparse_infill_area = sparse;
    }

    arena
        .set_slice(slice)
        .map_err(|source| LayerStageError::ArenaCommit { source })?;

    Ok(())
}
