//! Host-side fill-polygon partition.
//!
//! Runs as a side effect at `Layer::Perimeters` commit (see `apply` in
//! `layer_executor.rs`). Mutates the per-layer
//! arena's `SliceIR` only — the Blackboard's PrePass-committed
//! `SliceIR` Vec stays canonical (per the slice-prepass-migration invariant
//! that the Blackboard is read-only during Tier 2).
//!
//! For each slice region the helper finds the donor in `arena.perimeter()` by
//! the exact full identity `(object_id, source region_id, ordered
//! variant_chain)` — never the `(object_id, region_id)` pair, because two
//! paint variants can share the pair while carrying different ordered chains.
//! Modifier children look up only their parent's region id (preserving their
//! own object and chain). The effective wall inset is
//! `W = donor.infill_areas ∩ slice_region.polygons`; partitioning is
//! conditional on `W` and on whether the donor has walls:
//!
//! ```text
//! nonempty W:   bridge   = bridge_areas ∩ W
//!               bottom   = (bottom_solid_fill ∩ W) − bridge
//!               top      = (top_solid_fill ∩ W) − (bridge ∪ bottom)
//!               internal = (internal_solid_fill ∩ W) − (bridge ∪ bottom ∪ top)
//!               sparse   = W − (bridge ∪ bottom ∪ top ∪ internal)
//! empty W, walls: all five roles empty (the walls consumed the cross-section)
//! empty W, no walls: bridge = original bridge_areas
//!                    top    = original top_solid_fill − bridge
//!                    bottom = internal = sparse = empty
//! ```
//!
//! Precedence is `bridge > bottom > top > internal > sparse`, mirroring
//! canonical `PrintObject::prepare_infill`. In the nonempty-`W` branch the five
//! roles are pairwise disjoint subsets of `W` whose union is `W`; the
//! empty-`W`/no-wall branch makes no subset claim. Fill claim holders
//! (rectilinear, gyroid, lightning infill modules) emit each role over exactly
//! one polygon with zero polygon math.
//!
//! `internal_solid_fill` carries two contents by construction: the PrePass
//! shell-band marker (`top_solid_fill − exposed seed`), which carves to empty
//! here because it is a subset of `top_solid_fill`, and the converted sparse
//! islands minted by the `minimum_sparse_infill_area` prepass block, which
//! survive as the only polygons in the bucket. Net effect: `internal` is
//! exactly the islands — canonical's `stInternalSolid` fill zone.
//!
//! Missing-donor behaviour: a slice region without a matching donor is skipped
//! (its canonical fill polygons stay at whatever PrePass left them, minus the
//! shell-band marker carve — see the skip arms in
//! `sync_perimeter_infill_areas_into_slice`) and the host emits a structured
//! `log::warn!` naming the offending object/source region (a modifier child's
//! warning also names its parent) so the failure mode is observable in
//! production logs. Real configurations exist where a virtual variant region
//! (region_split work, packets 92–95) is committed to `SliceIR` without a
//! per-variant perimeter entry — the variant's wall geometry is shared with its
//! base region. Treating that as fatal would poison the entire layer; the safer
//! contract is "no donor → no repartition for this region, but log it". The
//! IR-level fatals (`take_slice` / `arena.perimeter()` both `None`) are
//! preserved because those represent a genuine stage-ordering violation, not a
//! per-region absence; a modifier-split rejection is also fatal but restores the
//! entry-state slice before returning.
//!
//! Empty-wall-inset behaviour is conditional, not a universal "preserve":
//!
//! - Empty `infill_areas` **with** donor walls means the walls consumed the
//!   whole cross-section; all five canonical fill roles are empty there (only
//!   walls print).
//! - Empty `infill_areas` **without** donor walls keeps the original
//!   `bridge_areas`, sets `top_solid_fill` to the original top minus that
//!   bridge, and leaves `bottom_solid_fill`, `internal_solid_fill` and
//!   `sparse_infill_area` empty. The
//!   preserved top keeps an exposed surface visible to surface-treatment
//!   stages such as ironing; no role is claimed to be a subset of the empty
//!   wall inset. See `cube_4color_ironing_per_painted_top_color_tdd` in
//!   `tests/executor/` for the regression.

use std::collections::HashMap;

use slicer_core::polygon_ops::{difference, intersection, union};
use slicer_ir::{LayerStageError, MeshIR, StageId};

use crate::LayerArena;

/// Reserved `region_id` flagging a `SlicedRegion` as a modifier footprint staged
/// for `sync_perimeter_infill_areas_into_slice` to consume (packet 132).
///
/// Re-exported from `slicer-ir`, which owns it so that `slicer-wasm-host`'s
/// `push_slice_regions` can filter footprints out of guest views without
/// depending on this crate. The modifier `region_id` namespace stride and the
/// `modifier_sub_region_id` hash live next to it — the Tier-2 mint here and the
/// region-map kernel (`slicer-core::algos::region_mapping`, ticket 18) MUST
/// derive sub-region ids through the same `slicer-ir` function or the ids
/// diverge.
pub use slicer_ir::{
    modifier_sub_region_id, MODIFIER_FOOTPRINT_REGION_ID, MODIFIER_VARIANT_REGION_ID_STRIDE,
};

/// Split every matching parent region by the supplied modifier footprints.
///
/// Footprints are supplied in descending priority order; the parent is reduced
/// after each split, so later footprints receive only the remaining geometry.
/// Keeping this operation shared makes the prepass materialization and the
/// Tier-2 fallback produce the same `SlicedRegion` fields.
fn split_regions_by_modifier_footprints(
    slice: &mut slicer_ir::SliceIR,
    footprints: impl IntoIterator<Item = (String, Vec<slicer_ir::ExPolygon>)>,
) -> Result<(), String> {
    // Work on a copy so overflow and identity failures cannot leave a partially
    // partitioned SliceIR behind.
    let mut working = slice.clone();
    let mut minted = Vec::new();
    let mut minted_identities: Vec<(
        String,
        slicer_ir::RegionId,
        Vec<(String, slicer_ir::PaintValue)>,
    )> = working
        .regions
        .iter()
        .map(|region| {
            (
                region.object_id.clone(),
                region.region_id,
                region.variant_chain.clone(),
            )
        })
        .collect();
    for (object_id, footprint) in footprints {
        if footprint.is_empty() {
            continue;
        }

        let parent_indices: Vec<usize> = working
            .regions
            .iter()
            .enumerate()
            .filter_map(|(index, region)| {
                (region.object_id == object_id
                    && region.region_id != MODIFIER_FOOTPRINT_REGION_ID
                    && !slicer_ir::is_modifier_namespace_id(region.region_id))
                .then_some(index)
            })
            .collect();

        for base_index in parent_indices {
            let sub_polygons = intersection(&working.regions[base_index].polygons, &footprint);
            if sub_polygons.is_empty() {
                continue;
            }

            let base_region_id = working.regions[base_index].region_id;
            if !slicer_ir::modifier_sub_region_id_fits(base_region_id) {
                return Err(format!(
                    "modifier sub-region parent id cannot be encoded for object_id='{}', \
                     parent_region_id='{}', variant_chain={:?}",
                    object_id, base_region_id, working.regions[base_index].variant_chain
                ));
            }

            // Paint segmentation rebuilds regions from their polygons and can
            // leave the pre-perimeter `infill_areas` field empty. Restore the
            // standard prepass default before splitting so the child carries
            // the same raw fill domain as the unsplit path.
            if working.regions[base_index].infill_areas.is_empty() {
                working.regions[base_index].infill_areas =
                    working.regions[base_index].polygons.clone();
            }
            let mut sub = working.regions[base_index].clone();
            sub.region_id = modifier_sub_region_id(base_region_id, &object_id, &footprint);
            sub.segment_annotations.clear();
            sub.polygons = sub_polygons;

            macro_rules! split_field {
                ($field:ident) => {{
                    sub.$field = intersection(&working.regions[base_index].$field, &footprint);
                    working.regions[base_index].$field =
                        difference(&working.regions[base_index].$field, &footprint);
                }};
            }
            split_field!(infill_areas);
            split_field!(bridge_areas);
            split_field!(bottom_solid_fill);
            split_field!(top_solid_fill);
            split_field!(sparse_infill_area);
            split_field!(internal_solid_fill);
            // Internal-bridge polygons are split one at a time so each piece
            // keeps its parent polygon's bridge angle on both sides of the
            // footprint (the carriers are index-aligned).
            {
                let base = &working.regions[base_index];
                let keep_angles = !base.internal_bridge_angles_deg.is_empty();
                let (mut sub_areas, mut sub_angles) = (Vec::new(), Vec::new());
                let (mut base_areas, mut base_angles) = (Vec::new(), Vec::new());
                for (index, polygon) in base.internal_bridge_areas.iter().enumerate() {
                    let angle = base.internal_bridge_angle_deg(index);
                    let polygon = std::slice::from_ref(polygon);
                    for piece in intersection(polygon, &footprint) {
                        sub_areas.push(piece);
                        sub_angles.push(angle);
                    }
                    for piece in difference(polygon, &footprint) {
                        base_areas.push(piece);
                        base_angles.push(angle);
                    }
                }
                if !keep_angles {
                    sub_angles.clear();
                    base_angles.clear();
                }
                sub.internal_bridge_areas = sub_areas;
                sub.internal_bridge_angles_deg = sub_angles;
                let base = &mut working.regions[base_index];
                base.internal_bridge_areas = base_areas;
                base.internal_bridge_angles_deg = base_angles;
            }
            split_field!(raft_fill);
            working.regions[base_index].polygons =
                difference(&working.regions[base_index].polygons, &footprint);
            let identity = (object_id.clone(), sub.region_id, sub.variant_chain.clone());
            if minted_identities
                .iter()
                .any(|existing| existing == &identity)
            {
                return Err(format!(
                    "modifier sub-region identity collision for object_id='{}', region_id='{}', variant_chain={:?}",
                    identity.0, identity.1, identity.2
                ));
            }
            minted_identities.push(identity);
            minted.push(sub);
        }
    }
    working.regions.extend(minted);
    *slice = working;
    Ok(())
}

/// Materialize parameter-modifier sub-regions in the prepass `SliceIR`.
///
/// Region mapping has already minted the matching config entries and active
/// region ids. This pass clips each modifier footprint to the base region,
/// moves the covered geometry into the matching wall-less sub-region, and
/// leaves support enforcer/blocker volumes on their dedicated annotation path.
pub fn split_modifier_sub_regions_for_prepass(
    slice: &mut slicer_ir::SliceIR,
    mesh: &MeshIR,
) -> Result<(), String> {
    let mut working = slice.clone();
    // Remove only raw staging footprints. Any already-materialized modifier
    // child is retained, which makes this helper safe for a slice that has
    // passed through an earlier partitioning seam.
    working
        .regions
        .retain(|region| region.region_id != MODIFIER_FOOTPRINT_REGION_ID);

    let mut footprints = Vec::new();
    for object in &mesh.objects {
        let mut modifier_indices: Vec<usize> = (0..object.modifier_volumes.len()).collect();
        modifier_indices.sort_by_key(|&index| {
            (
                std::cmp::Reverse(object.modifier_volumes[index].priority),
                index,
            )
        });
        for modifier_index in modifier_indices {
            let modifier = &object.modifier_volumes[modifier_index];
            let is_support_modifier = match modifier.kind() {
                slicer_ir::ModifierKind::ParameterModifier => false,
                slicer_ir::ModifierKind::NegativePart => false,
                slicer_ir::ModifierKind::SupportEnforcer => true,
                slicer_ir::ModifierKind::SupportBlocker => true,
            };
            if is_support_modifier
                || modifier.mesh.vertices.is_empty()
                || modifier.mesh.indices.is_empty()
            {
                continue;
            }
            let footprint = slicer_core::slice_mesh_ex(&modifier.mesh, &[working.z])
                .into_iter()
                .next()
                .unwrap_or_default();
            if footprint.is_empty() {
                continue;
            }
            footprints.push((object.id.clone(), footprint));
        }
    }
    split_regions_by_modifier_footprints(&mut working, footprints)?;
    *slice = working;
    Ok(())
}

/// Reconcile the five canonical fill polygons on every `SliceIR` region
/// against the just-committed `PerimeterIR.infill_areas`. See module docs
/// for the precedence rule and clip-in-place semantics.
///
/// Errors:
/// - `LayerStageError::FatalModule` when no `SliceIR` is staged on the arena
///   (the hook must run after `Layer::Slice` has committed).
/// - `LayerStageError::FatalModule` when no `PerimeterIR` is staged (the hook
///   must run after `Layer::Perimeters` has committed). The entry-state slice
///   is re-staged before the error returns.
/// - `LayerStageError::FatalModule` when the modifier split rejects its input
///   (identity collision or an unencodable parent id). The ENTRY-STATE slice is
///   re-staged before the error returns, so a rejection cannot abandon the slot
///   taken from the arena; earlier module commits are not rolled back.
/// - `LayerStageError::ArenaCommit` if the post-mutation `set_slice` fails.
///
/// A slice region without a matching perimeter donor is **not** fatal: its fill
/// roles stay at their PrePass values (minus the shell-band marker carve) and a
/// warning names the region.
pub fn sync_perimeter_infill_areas_into_slice(
    arena: &mut LayerArena,
    layer_index: u32,
) -> Result<(), LayerStageError> {
    let stage_id: StageId = "Layer::Perimeters".into();
    let module_id = "host:region_partition".to_string();

    let entry = arena
        .take_slice()
        .ok_or_else(|| LayerStageError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module_id.clone(),
            message: format!(
                "region_partition at layer {layer_index}: no staged SliceIR \
             (host built-in PrePass::Slice must commit before Layer::Perimeters runs)"
            ),
        })?;
    // The entry-state value is the rollback snapshot for every error after the
    // take. Role sync and the modifier split mutate only this working copy, so
    // a rejected split can never leave the arena without the slice it took.
    let mut slice = entry.clone();

    // Borrow perimeter immutably — we only read infill_areas off it.
    let perimeter = match arena.perimeter() {
        Some(p) => p,
        None => {
            // Re-stage the entry-state slice we just took so callers can recover.
            let _ = arena.set_slice(entry);
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

    // Build a `(object_id, region_id, variant_chain) → PerimeterRegion` index
    // once before the slice-region loop — the FULL source identity, not the
    // `(object_id, region_id)` pair. Two paint-minted variants of one base
    // region can share the pair while carrying different ordered chains; a
    // pair-keyed index would collapse them onto whichever perimeter entry was
    // collected last, silently re-attributing wall geometry across sources
    // (AC-R7: guest and partition joins must preserve source identity). Chain
    // equality is exact — never sorted or normalized — and an empty chain is
    // a real value. A slice region missing from this index is a virtual
    // variant sharing its base region's walls (ADR-0028 §Amendment).
    let perim_index: HashMap<
        (
            &slicer_ir::ObjectId,
            slicer_ir::RegionId,
            &[(String, slicer_ir::PaintValue)],
        ),
        &slicer_ir::PerimeterRegion,
    > = perimeter
        .regions
        .iter()
        .map(|r| ((&r.object_id, r.region_id, r.variant_chain.as_slice()), r))
        .collect();

    for slice_region in &mut slice.regions {
        // Raw footprint staging is consumed after this loop and is never a
        // printable region or a perimeter donor.
        if slice_region.region_id == MODIFIER_FOOTPRINT_REGION_ID {
            continue;
        }
        let is_modifier_region = slicer_ir::is_modifier_namespace_id(slice_region.region_id);
        // Modifier children borrow the perimeter entry of the parent region.
        // They are intentionally absent from PerimeterIR, but still need the
        // parent's wall inset to derive their own sparse/solid fill domains.
        let perimeter_region_id = if is_modifier_region {
            slicer_ir::modifier_base_region_id(slice_region.region_id)
                .expect("modifier namespace id must encode a parent region")
        } else {
            slice_region.region_id
        };
        let Some(perim) = perim_index
            .get(&(
                &slice_region.object_id,
                perimeter_region_id,
                slice_region.variant_chain.as_slice(),
            ))
            .copied()
        else {
            if is_modifier_region {
                log::warn!(
                    "region_partition at layer {layer_index}: no PerimeterIR donor for modifier \
                     SliceIR region (object_id='{}', region_id='{}', parent_region_id='{}'); \
                     leaving its prepass fill roles untouched",
                    slice_region.object_id,
                    slice_region.region_id,
                    perimeter_region_id
                );
                // The fill modules emit `internal_solid_fill` as a fill bucket
                // now, so the PrePass shell-band marker (a subset of
                // `top_solid_fill`) must not survive unpartitioned — it would
                // double-fill the exposed top. Converted islands never overlap
                // `top_solid_fill`, so only the marker dies.
                slice_region.internal_solid_fill = difference(
                    &slice_region.internal_solid_fill,
                    &slice_region.top_solid_fill,
                );
                continue;
            }
            // No perimeter entry for this slice region — typically a virtual
            // variant region (region_split work, packets 92–95) sharing wall
            // geometry with its base region. Leave the fill polygons
            // untouched; the base region's partition is canonical
            // for the variant's geometry too. Emit a structured warning so
            // the failure mode is observable in production logs (B3).
            //
            // Same carve as the modifier arm above: the shell-band marker must
            // not survive unpartitioned into a fill-bucket emit.
            log::warn!(
                "region_partition at layer {layer_index}: no PerimeterIR entry \
                 for SliceIR region (object_id='{}', region_id='{}'); skipping — \
                 variant region with shared base-region wall geometry \
                 (packets 92–95). Fill polygons remain at \
                 PrePass values for this region.",
                slice_region.object_id,
                slice_region.region_id
            );
            slice_region.internal_solid_fill = difference(
                &slice_region.internal_solid_fill,
                &slice_region.top_solid_fill,
            );
            continue;
        };

        // Prepass modifier splitting has already removed child geometry from
        // this parent. Perimeter projection restores the full outline only for
        // wall generation, so clip the returned wall inset back to the parent
        // polygon before assigning fill roles. The raw-footprint fallback has
        // an unsplit parent here and therefore remains unchanged.
        let wall_inset = intersection(&perim.infill_areas, &slice_region.polygons);

        // Precedence: bridge > bottom > top > internal > sparse.
        //
        // Edge case (fix): when the perimeter stage produces no infill area
        // for a region (e.g., a thin-walled region whose inset collapses to
        // empty, or a region whose perimeter dispatch never reached
        // `set_infill_areas`), `wall_inset` is the empty set. The naive
        // `intersection(top_solid_fill, wall_inset)` would wipe
        // `top_solid_fill` to empty, discarding an exposed top surface that
        // the shell-classification step deliberately marked. Ironing then
        // skips the region (gate at
        // top-surface-ironing's non-empty top-fill gate
        // requires non-empty `top_solid_fill`). The no-wall fallback preserves
        // the original `top_solid_fill` minus the bridge claim so that
        // surface-treatment stages still see the exposed top; it does NOT
        // preserve `bottom_solid_fill` (that role is empty in this branch) and
        // it makes no subset claim against the empty inset. When the donor has
        // walls, an empty inset means the walls consumed the cross-section and
        // all four roles are empty. For the common non-empty-inset case the
        // precedence path below is unchanged.
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
        // The bridge claim is clipped to `wall_inset` like the other three
        // fills, guarded by the same `wall_inset.is_empty()` escape the
        // `top_solid_fill` arm uses. Packet 234's protected case is exactly
        // the empty case: at a ceiling layer the perimeter module's infill
        // area can be empty (the whole cross-section is top surface), and an
        // unconditional intersection would drop a canonical bridge site
        // (wedge interior-slot ceiling). Removing the clip outright
        // (commit 83180d9e) went too far: with a non-empty `wall_inset` the
        // unclipped bridge claim extended past the outer-wall centerline and
        // bridge extrusion ran over every wall bead.
        //
        // The escape applies only to a region with NO walls. When walls exist
        // but consumed the whole cross-section (thin wheel, window tops,
        // roof ceilings), canonical `LayerRegion.cpp`
        // `slices_to_fill_surfaces_clipped` builds every fill surface as
        // typed slice ∩ the perimeters' fill area, so an empty fill area
        // yields no bridge and no top surface: only walls are printed there.
        let walls_consumed_region = wall_inset.is_empty() && !perim.walls.is_empty();
        let bridge = if walls_consumed_region {
            Vec::new()
        } else if wall_inset.is_empty() {
            slice_region.bridge_areas.clone()
        } else if is_modifier_region
            && difference(&slice_region.bridge_areas, &wall_inset).is_empty()
        {
            // Preserve a prepass-materialized child role byte-for-byte when
            // the parent wall inset already contains it. Boolean intersection
            // can reverse contour winding even when it makes no change.
            slice_region.bridge_areas.clone()
        } else {
            intersection(&slice_region.bridge_areas, &wall_inset)
        };
        let bottom = if wall_inset.is_empty() {
            Vec::new()
        } else {
            difference(
                &intersection(&slice_region.bottom_solid_fill, &wall_inset),
                &bridge,
            )
        };
        let bridge_or_bottom = union(&bridge, &bottom);
        let top = if walls_consumed_region {
            Vec::new()
        } else if wall_inset.is_empty() {
            difference(&slice_region.top_solid_fill, &bridge_or_bottom)
        } else {
            difference(
                &intersection(&slice_region.top_solid_fill, &wall_inset),
                &bridge_or_bottom,
            )
        };
        let bridge_or_bottom_or_top = union(&bridge_or_bottom, &top);
        // Internal solid (converted sparse islands; see the module header) has
        // lower precedence than the three exposed-role fills and higher than
        // sparse. Without the subtraction the sparse claim-holder re-absorbs
        // the converted islands and fills them twice. The PrePass shell-band
        // marker (`top_solid_fill − exposed seed`) is a subset of
        // `top_solid_fill`, so this subtraction also removes it — leaving the
        // converted islands as the bucket's only survivors. An empty
        // `wall_inset` yields an empty bucket, matching the bottom arm: with
        // no wall inset there is no sparse zone, so there is nothing to have
        // converted (canonical's conversion runs on inset surfaces too).
        let internal = difference(
            &intersection(&slice_region.internal_solid_fill, &wall_inset),
            &bridge_or_bottom_or_top,
        );
        let bridge_or_bottom_or_top_or_internal = union(&bridge_or_bottom_or_top, &internal);
        let sparse = difference(&wall_inset, &bridge_or_bottom_or_top_or_internal);

        slice_region.bridge_areas = bridge;
        slice_region.bottom_solid_fill = bottom;
        slice_region.top_solid_fill = top;
        slice_region.internal_solid_fill = internal;
        slice_region.sparse_infill_area = sparse;
    }

    // Modifier region split (packet 132): consume any MODIFIER_FOOTPRINT_REGION_ID
    // footprints staged on this layer, minting a sub-region in the modifier
    // `region_id` namespace whose geometry is the intersection of the footprint
    // with the base region's four partitioned fill polygons. The base region's
    // polygons are reduced to the difference. Runs AFTER the existing partition
    // so it composes on already-partitioned polygons.
    split_modifier_footprints(&mut slice).map_err(|message| {
        // The split is transactional on its own copy; this hook owns the arena
        // slot it took, so it must re-stage the entry-state slice before
        // propagating the rejection.
        let _ = arena.set_slice(entry);
        LayerStageError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module_id.clone(),
            message,
        }
    })?;

    arena
        .set_slice(slice)
        .map_err(|source| LayerStageError::ArenaCommit { source })?;

    Ok(())
}

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
fn split_modifier_footprints(slice: &mut slicer_ir::SliceIR) -> Result<(), String> {
    let mut working = slice.clone();
    let footprints: Vec<(String, Vec<slicer_ir::ExPolygon>)> = working
        .regions
        .iter()
        .filter(|region| region.region_id == MODIFIER_FOOTPRINT_REGION_ID)
        .map(|region| (region.object_id.clone(), region.polygons.clone()))
        .collect();
    if footprints.is_empty() {
        return Ok(());
    }

    working
        .regions
        .retain(|region| region.region_id != MODIFIER_FOOTPRINT_REGION_ID);
    split_regions_by_modifier_footprints(&mut working, footprints)?;
    *slice = working;
    Ok(())
}

/// Deterministic controls and registry-backed properties for this kernel
/// (`H-RUNTIME-MODIFIERS`, `H-RUNTIME-PARTITION`); see
/// `region_partition/property_tests.rs`.
///
/// The attributes are stacked rather than combined into
/// `#[cfg(all(test, feature = "property_testing"))]`: `xtask`'s literal scanner
/// recognizes an exact `#[cfg(test)]` ident and treats the module as test-only
/// code, so the stacked form keeps these fixtures visible to both the
/// struct-literal and test-quality scanners.
#[cfg(test)]
#[cfg(feature = "property_testing")]
mod property_tests;
