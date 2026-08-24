//! Host-owned, strategy-neutral support analysis.

use std::collections::BTreeMap;
use std::sync::Arc;

use rayon::prelude::*;
use slicer_core::algos::overhang_annotation::{detect_support_contacts, SupportContactParams};
use slicer_core::algos::paint_segmentation::modifier_volumes::slice_modifier_volumes;
use slicer_core::polygon_ops::{difference_ex, intersection_ex, offset, union_ex, OffsetJoinType};
use slicer_ir::mm_to_units;
use slicer_ir::slice_ir::{
    ExPolygon, Point2, Polygon, RegionKey, SupportAnalysisIR, SupportCandidate,
    SupportCandidateSource, SupportGeometryKey, SupportType,
};
use slicer_ir::{ConfigValue, PaintSemantic, ResolvedConfig};
use slicer_scheduler::execution_plan::{
    select_support_family, SUPPORT_FAMILY_CONFIG_KEY, SUPPORT_GENERATOR_CONFIG_KEY,
};

use crate::blackboard::Blackboard;
use crate::layer_executor::config_for_region_smallest_chain;

/// Build conservative candidates without propagating support bodies.
///
/// Candidates are **support contacts**, not model cross-sections. Each is the
/// angle-thresholded overhang region produced by
/// [`detect_support_contacts`](slicer_core::algos::overhang_annotation::detect_support_contacts),
/// mirroring canonical `detect_overhangs` (`SupportMaterial.cpp`): a contact
/// appears once, at the overhang's own Z, and geometry with no overhang yields
/// no candidates at all.
///
/// Before packet 224 this stage emitted one candidate per non-empty region per
/// layer, carrying the full model cross-section — no overhang detection of any
/// kind. Downstream planners were left to invent their own contact detection,
/// and any planner that trusted the stream necessarily produced support at
/// every layer of the model. Do not reintroduce an unfiltered candidate stream.
pub fn commit_support_analysis_builtin(
    blackboard: &mut Blackboard,
    config: &ResolvedConfig,
) -> Result<(), crate::BlackboardError> {
    let enable_support = config.support_enabled;
    // Read the typed field directly. `support_threshold_angle` is CLI-bound, so
    // `resolve_*` routes it into this field and never into `extensions` — an
    // extensions lookup here silently ignored every configured value. The macro
    // line in `slicer_ir::resolved_config` owns the default; there is no
    // host-side fallback constant and no `support_angle` fallback (that key is
    // canonical's support *pattern rotation*, not an overhang threshold).
    let threshold_angle_deg = config.support_threshold_angle;
    let mut ir = SupportAnalysisIR::default();
    ir.shared_settings
        .insert("support_enabled".to_string(), enable_support.to_string());
    ir.shared_settings.insert(
        "support_threshold_angle_deg".to_string(),
        threshold_angle_deg.to_string(),
    );
    if enable_support {
        // Unit fixtures may not run region mapping, so preserve their deterministic
        // traditional fallback while production runs consume the committed map.
        let region_map = blackboard.region_map().cloned();
        if let (Some(slices), Some(plan)) = (blackboard.slice_ir(), blackboard.layer_plan()) {
            let mut id = 0_u64;
            let mut object_bounds: BTreeMap<String, (i64, i64, i64, i64)> = BTreeMap::new();
            let mut object_tops: BTreeMap<String, (u32, Vec<ExPolygon>)> = BTreeMap::new();
            // Layer-major contact detection state. Canonical `detect_overhangs`
            // reads `object.layers()[layer_id - 1]->lslices` -- the union of
            // *all* regions of the layer below, at object level -- and diffs
            // each of the current layer's regions against it. So we accumulate
            // (a) the per-(object, layer) polygon set that becomes that union
            // and (b) one work item per (layer, object, region).
            let mut object_layer_polygons: BTreeMap<(String, u32), Vec<ExPolygon>> =
                BTreeMap::new();
            let mut contact_work: Vec<(u32, String, u64, Vec<ExPolygon>)> = Vec::new();
            for slice in slices.iter() {
                for region in &slice.regions {
                    if region.polygons.is_empty() {
                        continue;
                    }
                    let key = SupportGeometryKey {
                        global_support_layer_index: slice.global_layer_index,
                        object_id: region.object_id.clone(),
                        region_id: region.region_id,
                    };
                    ir.model_occupancy.insert(key, region.polygons.clone());
                    let bounds = object_bounds.entry(region.object_id.clone()).or_insert((
                        i64::MAX,
                        i64::MIN,
                        i64::MAX,
                        i64::MIN,
                    ));
                    for polygon in &region.polygons {
                        for point in &polygon.contour.points {
                            bounds.0 = bounds.0.min(point.x);
                            bounds.1 = bounds.1.max(point.x);
                            bounds.2 = bounds.2.min(point.y);
                            bounds.3 = bounds.3.max(point.y);
                        }
                    }
                    let top = object_tops
                        .entry(region.object_id.clone())
                        .or_insert((slice.global_layer_index, region.polygons.clone()));
                    if slice.global_layer_index > top.0 {
                        *top = (slice.global_layer_index, region.polygons.clone());
                    }
                    // Feed the layer-major detection state: this region's
                    // polygons join its object's layer union, and the region
                    // itself becomes one unit of contact-detection work.
                    object_layer_polygons
                        .entry((region.object_id.clone(), slice.global_layer_index))
                        .or_default()
                        .extend(region.polygons.iter().cloned());
                    contact_work.push((
                        slice.global_layer_index,
                        region.object_id.clone(),
                        region.region_id,
                        region.polygons.clone(),
                    ));
                }
            }

            // Angle-thresholded contact detection, layer-major then
            // region-major, mirroring canonical `detect_overhangs`
            // (`SupportMaterial.cpp`). The lower-layer set is the object's
            // whole layer below, unioned once, and every region of the layer
            // above is diffed against that same union. Keying the lower layer
            // per-region instead (the pre-parity shape) made a region that
            // first appears at layer `k` while sitting squarely on a *different*
            // region below emit its entire cross-section as a contact --
            // spurious full-area support on every multi-region object.
            let object_layer_union: BTreeMap<(String, u32), Vec<ExPolygon>> = object_layer_polygons
                .into_iter()
                .map(|(key, polygons)| (key, union_ex(&polygons)))
                .collect();
            let base_params = resolve_contact_params(config, threshold_angle_deg);
            // F-19: sliced support-enforcer / support-blocker modifier volumes.
            // `crates/slicer-core/src/algos/region_mapping.rs` deliberately
            // excludes these two paint subtypes from region splitting, so they
            // never appear as a `variant_chain` entry -- slicing the modifier
            // volumes here is the only way this stage can see them.
            let layer_zs: Vec<f32> = plan.global_layers.iter().map(|layer| layer.z).collect();
            let modifiers =
                ModifierGeometry::slice(blackboard.mesh(), &layer_zs, &plan.global_layers);
            // Deterministic order regardless of `SliceIR` ordering; the
            // parallel pass below reads only shared immutable state and
            // `rayon`'s `collect` into a `Vec` preserves this order, so the
            // committed candidate stream is byte-stable (the same
            // order-independence property the previous per-series `par_iter`
            // had).
            contact_work.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));
            let contacts: Vec<Contact> = contact_work
                .par_iter()
                .filter_map(|(layer_index, object_id, region_id, polygons)| {
                    // Layer 0 rests on the bed and has no layer below it.
                    let lower_index = layer_index.checked_sub(1)?;
                    let empty: Vec<ExPolygon> = Vec::new();
                    let lower = object_layer_union
                        .get(&(object_id.clone(), lower_index))
                        .unwrap_or(&empty);
                    let params = SupportContactParams {
                        // Canonical scales the offset by the *lower* layer's
                        // height.
                        lower_layer_height_mm: layer_height_mm(&plan.global_layers, lower_index),
                        ..base_params
                    };
                    let enforcers = modifiers.enforcers_at(object_id, *layer_index);
                    let blockers = modifiers.blockers_at(object_id, *layer_index);
                    // F-19: the auto/manual axis is a *per-region* setting --
                    // an object's regions may carry different `support_type`
                    // values -- so it is resolved from the region's own config,
                    // never from the run config.
                    let support_type = effective_support_type(
                        region_config(region_map.as_deref(), *layer_index, object_id, *region_id)
                            .unwrap_or(config),
                    );
                    let geometry = if support_type.is_auto() {
                        // Canonical `detect_overhangs` (`SupportMaterial.cpp`)
                        // gates this branch on
                        // `auto_normal_support = support_type == stNormalAuto`.
                        // Blockers are subtracted inside the detector (its step
                        // 3), so they are wired into that parameter rather than
                        // re-subtracted here.
                        detect_support_contacts(polygons, lower, blockers, &params)
                    } else {
                        // Manual: "If Normal (manual) or Tree (manual) is
                        // selected, only support enforcers are generated"
                        // (OrcaSlicer `support_type` tooltip). The
                        // angle-thresholded branch is skipped entirely -- a
                        // region with no enforcer over it yields no candidate
                        // however steep it is.
                        enforcer_contacts(
                            polygons,
                            lower,
                            enforcers,
                            blockers,
                            params.external_perimeter_width_mm,
                        )?
                    };
                    if geometry.is_empty() {
                        return None;
                    }
                    // `enforced` / `blocked` describe how the region relates to
                    // the painted modifier volumes. `blocked` is tested against
                    // the region's own cross-section, not the surviving
                    // geometry, so it stays true for a candidate whose blocked
                    // part has already been subtracted away.
                    let enforced = !intersection_ex(&geometry, enforcers).is_empty();
                    let blocked = !intersection_ex(polygons, blockers).is_empty();
                    Some(Contact {
                        layer_index: *layer_index,
                        object_id: object_id.clone(),
                        region_id: *region_id,
                        geometry,
                        enforced,
                        blocked,
                    })
                })
                .collect();

            for Contact {
                layer_index,
                object_id,
                region_id,
                geometry,
                enforced,
                blocked,
            } in contacts
            {
                let z = plan
                    .global_layers
                    .get(layer_index as usize)
                    .map_or(0.0, |layer| layer.z);
                ir.candidates.push(SupportCandidate {
                    id,
                    geometry,
                    source: SupportCandidateSource {
                        object_id,
                        region_id,
                        global_layer_index: layer_index,
                        z_units: mm_to_units(z),
                    },
                    enforced,
                    blocked,
                });
                id += 1;
            }
            ir.candidates.sort_by_key(|candidate| {
                (
                    candidate.source.global_layer_index,
                    candidate.source.object_id.clone(),
                    candidate.source.region_id,
                    candidate.id,
                )
            });
            let mut candidate_layers: BTreeMap<(String, u64), u32> = BTreeMap::new();
            for candidate in &ir.candidates {
                candidate_layers
                    .entry((
                        candidate.source.object_id.clone(),
                        candidate.source.region_id,
                    ))
                    .or_insert(candidate.source.global_layer_index);
            }
            if let Some(map) = region_map.as_deref() {
                // Mint one assignment for every mapped region, including regions
                // with no support contact. Candidate regions use their first
                // candidate layer to preserve the previous resolution.
                for region_key in map.entries.keys() {
                    let assignment_key = (region_key.object_id.clone(), region_key.region_id);
                    let layer_index = candidate_layers
                        .get(&assignment_key)
                        .copied()
                        .unwrap_or(region_key.global_layer_index);
                    ir.family_assignments
                        .entry(assignment_key)
                        .or_insert_with(|| {
                            // Must use the same two-stage lookup the executor uses to
                            // route the region (`backfill_active_region_configs`), or a
                            // painted region — keyed by a non-empty `variant_chain` —
                            // goes to the tree planner while being recorded here as
                            // "traditional" (F-44).
                            config_for_region_smallest_chain(
                                map,
                                layer_index,
                                &region_key.object_id,
                                region_key.region_id,
                            )
                            .map(|config| map.config_for_raw(config))
                            .map(support_family)
                            .unwrap_or_else(|| "traditional".to_string())
                        });
                }
            } else {
                // Unit fixtures may omit RegionMapIR; retain their candidate-based
                // traditional fallback in that case.
                for candidate in &ir.candidates {
                    ir.family_assignments
                        .entry((
                            candidate.source.object_id.clone(),
                            candidate.source.region_id,
                        ))
                        .or_insert_with(|| "traditional".to_string());
                }
            }
            // SliceIR has no facet classification, so the highest observed
            // cross-section is the narrowest truthful model-termination
            // approximation. The exact-Z service uses the same fallback.
            for (object_id, (top_layer, top_polygons)) in object_tops {
                let Some(plate) = object_bounds
                    .get(&object_id)
                    .and_then(|bounds| rectangle_from_bounds(*bounds))
                else {
                    continue;
                };
                for key in ir.model_occupancy.keys() {
                    if key.object_id == object_id && key.global_support_layer_index == top_layer {
                        ir.termination_surfaces.insert(
                            key.clone(),
                            top_polygons
                                .clone()
                                .into_iter()
                                .chain(std::iter::once(plate.clone()))
                                .collect(),
                        );
                    }
                }
                ir.baseline_feasible_envelope.push(plate);
            }
        }
    }
    blackboard.commit_support_analysis(Arc::new(ir))
}

/// Canonical `SUPPORT_SURFACES_OFFSET_PARAMETERS` is `jtSquare, 0.` — every
/// offset in the support-contact pipeline uses a square join. Mirrors the
/// private constant of the same shape in
/// `slicer_core::algos::overhang_annotation`.
const SUPPORT_SURFACES_JOIN: OffsetJoinType = OffsetJoinType::Square;
/// Arc tolerance for the offsets above; irrelevant for a square join, but the
/// parameter is not optional.
const OFFSET_ARC_TOLERANCE_MM: f32 = 0.01;

/// The *enforcer* half of canonical `detect_contacts` (`SupportMaterial.cpp`):
///
/// ```text
/// enforcer_polygons = diff(intersection(layer.lslices, enforcer_polygons_src),
///                          expand(lower_layer_polygons, 0.05f * no_interface_offset));
/// ```
///
/// "Enforce supports (as if with 90 degrees of slope) for the regions covered
/// by the enforcer meshes" — so no angle threshold applies, but area that
/// already rests on the layer below is still excluded (canonical inflates the
/// lower layer "just a tiny bit to avoid intersection of the overhang areas
/// with the object"). `no_interface_offset` is canonical's minimum external
/// perimeter width, which this stage carries as
/// `SupportContactParams::external_perimeter_width_mm`.
///
/// Canonical intersects the enforcer with the whole layer (`layer.lslices`);
/// this entry point is per-region and the per-region results union to the same
/// area.
///
/// Returns `None` when nothing survives, matching the caller's
/// "no candidate for this region" contract.
fn enforcer_contacts(
    region_polygons: &[ExPolygon],
    lower_layer_polygons: &[ExPolygon],
    enforcers: &[ExPolygon],
    blockers: &[ExPolygon],
    external_perimeter_width_mm: f32,
) -> Option<Vec<ExPolygon>> {
    if enforcers.is_empty() {
        return None;
    }
    let covered = intersection_ex(region_polygons, enforcers);
    if covered.is_empty() {
        return None;
    }
    let grown_lower = offset(
        lower_layer_polygons,
        0.05 * external_perimeter_width_mm,
        SUPPORT_SURFACES_JOIN,
        OFFSET_ARC_TOLERANCE_MM,
    );
    let contacts = difference_ex(&covered, &grown_lower);
    if contacts.is_empty() {
        return None;
    }
    // Canonical applies the blockers to the whole contact set of the layer.
    let contacts = if blockers.is_empty() {
        contacts
    } else {
        difference_ex(&contacts, blockers)
    };
    (!contacts.is_empty()).then(|| union_ex(&contacts))
}

/// One detected support contact, before it becomes a [`SupportCandidate`].
struct Contact {
    layer_index: u32,
    object_id: String,
    region_id: u64,
    geometry: Vec<ExPolygon>,
    enforced: bool,
    blocked: bool,
}

/// Sliced support-enforcer / support-blocker modifier volumes, keyed by
/// `(object_id, global_layer_index)`.
///
/// Painted support enforcers and blockers are carried as *modifier volumes* on
/// `ObjectMesh`, not as region variants:
/// `slicer_core::algos::region_mapping` deliberately keeps
/// `PaintSemantic::SupportEnforcer` / `SupportBlocker` out of region splitting,
/// so they never reach this stage through `RegionKey::variant_chain`. Slicing
/// the volumes with
/// [`slice_modifier_volumes`](slicer_core::algos::paint_segmentation::modifier_volumes::slice_modifier_volumes)
/// is the only source.
///
/// The slice is taken **per object** rather than over the whole `MeshIR` at
/// once, because `slice_modifier_volumes` merges every object's volumes into a
/// single per-semantic bucket and would otherwise let one object's enforcer
/// force support on another object that happens to overlap it in XY.
#[derive(Default)]
struct ModifierGeometry {
    enforcers: BTreeMap<(String, u32), Vec<ExPolygon>>,
    blockers: BTreeMap<(String, u32), Vec<ExPolygon>>,
}

impl ModifierGeometry {
    fn slice(
        mesh: &slicer_ir::MeshIR,
        layer_zs: &[f32],
        global_layers: &[slicer_ir::slice_ir::GlobalLayer],
    ) -> Self {
        let mut geometry = ModifierGeometry::default();
        for object in &mesh.objects {
            if object.modifier_volumes.is_empty() {
                continue;
            }
            // Per-object scoping: hand the slicer exactly one object's volumes.
            let scoped = slicer_ir::MeshIR {
                schema_version: mesh.schema_version,
                objects: vec![slicer_ir::slice_ir::ObjectMesh {
                    id: object.id.clone(),
                    mesh: slicer_ir::slice_ir::IndexedTriangleSet::default(),
                    transform: object.transform,
                    config: object.config.clone(),
                    modifier_volumes: object.modifier_volumes.clone(),
                    paint_data: None,
                    world_z_extent: object.world_z_extent,
                }],
                build_volume: mesh.build_volume,
            };
            for (position, entries) in slice_modifier_volumes(&scoped, layer_zs)
                .into_iter()
                .enumerate()
            {
                // `layer_zs` is built from `global_layers` in order, so the
                // outer index is that slot's position; the candidate stream is
                // keyed by `GlobalLayer::index`, which is what gets stored.
                let Some(layer) = global_layers.get(position) else {
                    continue;
                };
                for entry in entries {
                    if entry.polygons.is_empty() {
                        continue;
                    }
                    let bucket = match entry.semantic {
                        PaintSemantic::SupportEnforcer => &mut geometry.enforcers,
                        PaintSemantic::SupportBlocker => &mut geometry.blockers,
                        _ => continue,
                    };
                    bucket
                        .entry((object.id.clone(), layer.index))
                        .or_default()
                        .extend(entry.polygons);
                }
            }
        }
        for polygons in geometry
            .enforcers
            .values_mut()
            .chain(geometry.blockers.values_mut())
        {
            *polygons = union_ex(polygons);
        }
        geometry
    }

    fn enforcers_at(&self, object_id: &str, layer_index: u32) -> &[ExPolygon] {
        Self::at(&self.enforcers, object_id, layer_index)
    }

    fn blockers_at(&self, object_id: &str, layer_index: u32) -> &[ExPolygon] {
        Self::at(&self.blockers, object_id, layer_index)
    }

    fn at<'a>(
        map: &'a BTreeMap<(String, u32), Vec<ExPolygon>>,
        object_id: &str,
        layer_index: u32,
    ) -> &'a [ExPolygon] {
        map.get(&(object_id.to_string(), layer_index))
            .map_or(&[][..], Vec::as_slice)
    }
}

/// The region's effective `support_type`.
///
/// The raw OrcaSlicer spelling rides in `extensions` (that is the channel a 3MF
/// sidecar's `support_type` reaches pnp through, and it is what
/// `support_family` already consults); the typed field is the fallback. An
/// unrecognised extension string falls back to the typed field rather than
/// silently forcing auto.
fn effective_support_type(config: &ResolvedConfig) -> SupportType {
    config
        .extensions
        .get(SUPPORT_GENERATOR_CONFIG_KEY)
        .and_then(|value| match value {
            ConfigValue::String(value) => SupportType::from_canonical_str(value),
            _ => None,
        })
        .unwrap_or(config.support_type)
}

/// The per-region `ResolvedConfig` for one `(layer, object, region)`, or `None`
/// when no region map is committed (unit fixtures) or the region is absent.
fn region_config<'a>(
    region_map: Option<&'a slicer_ir::RegionMapIR>,
    global_layer_index: u32,
    object_id: &str,
    region_id: u64,
) -> Option<&'a ResolvedConfig> {
    let map = region_map?;
    let key = RegionKey {
        global_layer_index,
        object_id: object_id.to_string(),
        region_id,
        variant_chain: Vec::new(),
    };
    map.entries
        .get(&key)
        .map(|plan| map.config_for_raw(plan.config))
}

/// Resolves the config half of [`SupportContactParams`] once per slice.
///
/// * `fw` -- the external-perimeter extrusion width -- is read as extensions
///   `outer_wall_line_width`, falling back to the typed `line_width` field,
///   falling back to `0.4` mm. This mirrors `resolve_line_width_mm` in
///   `crate::builtins::overhang_annotation_producer`, the resolution
///   `annotate_overhangs`' caller already uses.
/// * `support_threshold_overlap` is canonical
///   `ConfigOptionFloatOrPercent(50., true)`, i.e. 50% of `fw` by default, and
///   resolves against `fw` as its base.
/// * `support_expansion` is canonical `coFloat`, default `0`.
///
/// `lower_layer_height_mm` is per-layer and is filled in by the caller.
fn resolve_contact_params(
    config: &ResolvedConfig,
    threshold_angle_deg: f32,
) -> SupportContactParams {
    let typed_line_width = if config.line_width > 0.0 {
        config.line_width
    } else {
        DEFAULT_LINE_WIDTH_MM
    };
    let external_perimeter_width_mm =
        extension_float(config, "outer_wall_line_width").unwrap_or(typed_line_width);
    let threshold_overlap_mm = extension_abs_value(
        config,
        "support_threshold_overlap",
        external_perimeter_width_mm,
    )
    .unwrap_or(DEFAULT_THRESHOLD_OVERLAP_FRACTION * external_perimeter_width_mm);
    SupportContactParams {
        threshold_angle_deg,
        lower_layer_height_mm: 0.0,
        external_perimeter_width_mm,
        threshold_overlap_mm,
        xy_expansion_mm: extension_float(config, "support_expansion").unwrap_or(0.0),
    }
}

/// Line width used when neither `outer_wall_line_width` nor the typed
/// `line_width` field carries a positive value (the typed field defaults to
/// `0.0`, which would silently disable the tiny-spot filter). Matches the
/// guest-side default used by `classic-perimeters`/`arachne-perimeters`.
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;

/// Canonical `support_threshold_overlap` default: `ConfigOptionFloatOrPercent(50., true)`.
const DEFAULT_THRESHOLD_OVERLAP_FRACTION: f32 = 0.5;

/// Absolute (non-percent) float read from `extensions`.
fn extension_float(config: &ResolvedConfig, key: &str) -> Option<f32> {
    match config.extensions.get(key)? {
        ConfigValue::Float(value) => Some(*value as f32),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => Some(*value as f32),
        _ => None,
    }
}

/// `extensions` read mirroring `ConfigOptionFloatOrPercent::get_abs_value`:
/// a percent resolves against `base`, an absolute value is returned unchanged.
fn extension_abs_value(config: &ResolvedConfig, key: &str, base: f32) -> Option<f32> {
    match config.extensions.get(key)? {
        ConfigValue::Percent(percent) => (base > 0.0).then(|| *percent as f32 / 100.0 * base),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                (base > 0.0).then(|| *value as f32 / 100.0 * base)
            } else {
                Some(*value as f32)
            }
        }
        ConfigValue::Float(value) => Some(*value as f32),
        _ => None,
    }
}

fn support_family(config: &ResolvedConfig) -> String {
    let support_family = config
        .extensions
        .get(SUPPORT_FAMILY_CONFIG_KEY)
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        });
    let support_type = config
        .extensions
        .get(SUPPORT_GENERATOR_CONFIG_KEY)
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .or(config.support_type.family_claim());
    select_support_family(support_family, support_type).to_string()
}

/// Effective printed height of `layer_index`, derived as the Z delta from the
/// layer below. Canonical scales the contact offset by the *lower* layer's
/// height, so this is the value the detector needs per entry. Layer 0 has no
/// predecessor and takes its own Z as its height.
fn layer_height_mm(global_layers: &[slicer_ir::slice_ir::GlobalLayer], layer_index: u32) -> f32 {
    let Some(layer) = global_layers.iter().find(|l| l.index == layer_index) else {
        return 0.0;
    };
    if layer_index == 0 {
        return layer.z;
    }
    global_layers
        .iter()
        .find(|l| l.index == layer_index - 1)
        .map_or(layer.z, |below| layer.z - below.z)
}

fn rectangle_from_bounds((min_x, max_x, min_y, max_y): (i64, i64, i64, i64)) -> Option<ExPolygon> {
    if min_x >= max_x || min_y >= max_y {
        return None;
    }
    Some(ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{
        ConfigValue, GlobalLayer, LayerPlanIR, MeshIR, RegionMapIR, RegionPlan, ResolvedConfig,
        SliceIR, SlicedRegion,
    };

    /// Axis-aligned square in **millimetres**.
    ///
    /// Fixtures here must be mm-scale: contact detection now runs canonical's
    /// `-0.1 * fw` tiny-spot filter, so the raw-unit squares this module used
    /// before (~30 units ~= 0.003mm) are far below one line width and are
    /// filtered away entirely. The geometry below is sized in whole
    /// millimetres for the same reason.
    fn square(x_mm: f32, y_mm: f32, size_mm: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x_mm, y_mm),
                    Point2::from_mm(x_mm + size_mm, y_mm),
                    Point2::from_mm(x_mm + size_mm, y_mm + size_mm),
                    Point2::from_mm(x_mm, y_mm + size_mm),
                ],
            },
            holes: Vec::new(),
        }
    }

    /// Two 0.2mm-thick layers at realistic Z. `GlobalLayer::default()` has
    /// `z == 0`, which makes every layer height 0 and therefore every
    /// `lower_layer_offset` 0 — a plain difference, never the angle-thresholded
    /// path these fixtures mean to exercise.
    fn global_layers(count: u32) -> Vec<GlobalLayer> {
        (0..count)
            .map(|index| GlobalLayer {
                index,
                z: (index + 1) as f32 * 0.2,
                ..GlobalLayer::default()
            })
            .collect()
    }

    fn support_enabled_config() -> ResolvedConfig {
        ResolvedConfig {
            support_enabled: true,
            ..ResolvedConfig::default()
        }
    }

    /// Commits a two-layer, single-region slice stack for object `"object"`,
    /// region 3, with the given lower and upper footprints.
    fn blackboard_with_stack(lower: &ExPolygon, upper: &ExPolygon) -> Blackboard {
        blackboard_with_stack_and_mesh(lower, upper, MeshIR::default())
    }

    /// As [`blackboard_with_stack`], but with a caller-supplied `MeshIR` so a
    /// fixture can carry support-enforcer / support-blocker modifier volumes.
    fn blackboard_with_stack_and_mesh(
        lower: &ExPolygon,
        upper: &ExPolygon,
        mesh: MeshIR,
    ) -> Blackboard {
        let mut blackboard = Blackboard::new(Arc::new(mesh), 2);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: global_layers(2),
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            .commit_slice_ir(Arc::new(vec![
                SliceIR {
                    global_layer_index: 0,
                    regions: vec![SlicedRegion {
                        object_id: "object".to_string(),
                        region_id: 3,
                        polygons: vec![lower.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
                SliceIR {
                    global_layer_index: 1,
                    regions: vec![SlicedRegion {
                        object_id: "object".to_string(),
                        region_id: 3,
                        polygons: vec![upper.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
            ]))
            .unwrap();
        blackboard
    }

    #[test]
    fn support_analysis_populates_all_derivable_inputs() {
        // Layer 1 is a 5mm square overhanging a 3mm one by 1mm on every side,
        // so layer 1 genuinely overhangs at the default 30-degree threshold
        // (0.2mm layer / tan(31 deg) = 0.33mm of required overlap).
        let lower = square(1.0, 2.0, 3.0);
        let upper = square(0.0, 1.0, 5.0);
        let mut blackboard = blackboard_with_stack(&lower, &upper);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();
        let key = SupportGeometryKey {
            global_support_layer_index: 0,
            object_id: "object".to_string(),
            region_id: 3,
        };
        assert_eq!(
            analysis.shared_settings.get("support_enabled"),
            Some(&"true".to_string())
        );
        assert_eq!(
            analysis.model_occupancy.get(&key),
            Some(&vec![lower.clone()])
        );

        // Candidates are support contacts, not cross-sections: exactly one, at
        // the overhanging layer, and never at the supported layer below it.
        assert_eq!(analysis.candidates.len(), 1);
        assert_eq!(analysis.candidates[0].source.global_layer_index, 1);
        assert_eq!(analysis.candidates[0].source.object_id, "object");
        assert_eq!(analysis.candidates[0].source.region_id, 3);
        assert!(!analysis.candidates[0].enforced);
        assert!(!analysis.candidates[0].blocked);
        // The contact is the overhanging remainder, strictly smaller than the
        // upper cross-section it came from.
        assert!(!analysis.candidates[0].geometry.is_empty());
        assert_ne!(analysis.candidates[0].geometry, vec![upper.clone()]);

        let termination_key = SupportGeometryKey {
            global_support_layer_index: 1,
            object_id: "object".to_string(),
            region_id: 3,
        };
        assert_eq!(analysis.termination_surfaces.len(), 1);
        assert_eq!(analysis.termination_surfaces[&termination_key].len(), 2);
        assert_eq!(analysis.termination_surfaces[&termination_key][0], upper);
        assert!(!analysis.termination_surfaces.contains_key(&key));
        assert_eq!(analysis.baseline_feasible_envelope.len(), 1);
        assert_eq!(
            analysis.baseline_feasible_envelope[0].contour.points.len(),
            4
        );
        assert_eq!(
            analysis.family_assignments.get(&("object".to_string(), 3)),
            Some(&"traditional".to_string())
        );
    }

    #[test]
    fn region_covered_by_the_layer_above_is_not_a_candidate() {
        // A region fully covered from above is not an overhang, so no contact
        // may be emitted for it. This rule lives here, not in a planner: the
        // planners receive finished contacts and own routing, not the judgement
        // of what counts as an overhang.
        //
        // Inherited from the traditional planner's former
        // `fully_covered_candidate_is_declined`, which asserted this shape but
        // actually passed on an unrelated empty-mesh path.
        let wide = square(0.0, 0.0, 4.0);
        let narrow = square(1.0, 1.0, 1.0);
        // Layer 0 is wide, layer 1 is narrow and sits entirely within it.
        let mut blackboard = blackboard_with_stack(&wide, &narrow);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();

        assert!(
            analysis.candidates.is_empty(),
            "a region wholly covered by the layer below it overhangs nothing, so it \
             must yield no candidates; got {:?}",
            analysis.candidates
        );
    }

    #[test]
    fn straight_column_yields_no_support_candidates() {
        // Regression pin for packet 224 RC-0: this stage previously emitted one
        // candidate per non-empty region per layer with no overhang detection
        // whatsoever, so a straight column produced support candidates at every
        // layer. Identical footprints must now produce none.
        let polygon = square(1.0, 2.0, 3.0);
        let mut blackboard = blackboard_with_stack(&polygon, &polygon);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();

        assert!(
            analysis.candidates.is_empty(),
            "a straight column has no overhang and must yield no candidates, got {:?}",
            analysis.candidates
        );
        // Occupancy and termination are independent of contact detection and
        // must still be populated.
        assert_eq!(analysis.model_occupancy.len(), 2);
        assert_eq!(analysis.termination_surfaces.len(), 1);
    }

    /// F-2 regression pin. `support_threshold_angle` is CLI-bound, so
    /// `resolve_*` routes it to the typed field and never to `extensions`. This
    /// stage used to read `extensions` only, so it fell through to a hardcoded
    /// 45.0 on every slice and the user's configured angle was never applied.
    ///
    /// Asserts both halves: the default is the canonical 30.0 (OrcaSlicer
    /// `PrintConfig.cpp` `support_threshold_angle`, `ConfigOptionInt(30)`), and
    /// a configured value reaches the detector rather than the default.
    #[test]
    fn configured_threshold_angle_reaches_detection() {
        let polygon = square(1.0, 2.0, 3.0);

        let mut blackboard = blackboard_with_stack(&polygon, &polygon);
        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        assert_eq!(
            blackboard
                .support_analysis()
                .unwrap()
                .shared_settings
                .get("support_threshold_angle_deg"),
            Some(&"30".to_string()),
            "default must be the canonical 30 deg, owned by the ResolvedConfig macro line"
        );

        let mut blackboard = blackboard_with_stack(&polygon, &polygon);
        let config = ResolvedConfig {
            support_enabled: true,
            support_threshold_angle: 12.5,
            ..ResolvedConfig::default()
        };
        commit_support_analysis_builtin(&mut blackboard, &config).unwrap();
        assert_eq!(
            blackboard
                .support_analysis()
                .unwrap()
                .shared_settings
                .get("support_threshold_angle_deg"),
            Some(&"12.5".to_string()),
            "the configured typed field must reach detection; reading `extensions`              instead silently pinned this to the default"
        );
    }

    #[test]
    fn support_analysis_uses_region_map_family_precedence() {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: global_layers(2),
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            // Family assignment is keyed off candidates, and candidates are now
            // support contacts, so each region needs a genuine overhang: layer 1
            // is wider than layer 0.
            .commit_slice_ir(Arc::new(
                [(0_u32, 1.0_f32), (1_u32, 2.0_f32)]
                    .into_iter()
                    .map(|(global_layer_index, size)| SliceIR {
                        global_layer_index,
                        regions: (3..=5)
                            .map(|region_id| SlicedRegion {
                                object_id: "object".to_string(),
                                region_id,
                                polygons: vec![square(0.0, 0.0, size)],
                                ..SlicedRegion::default()
                            })
                            .collect(),
                        ..SliceIR::default()
                    })
                    .collect::<Vec<_>>(),
            ))
            .unwrap();

        let mut region_map = RegionMapIR::default();
        let canonical_tree = ResolvedConfig {
            extensions: [(
                "support_family".to_string(),
                ConfigValue::String("tree".to_string()),
            )]
            .into_iter()
            .collect(),
            ..ResolvedConfig::default()
        };
        let alias_override = ResolvedConfig {
            extensions: [
                (
                    "support_family".to_string(),
                    ConfigValue::String("tree".to_string()),
                ),
                (
                    "support_type".to_string(),
                    ConfigValue::String("normal(auto)".to_string()),
                ),
            ]
            .into_iter()
            .collect(),
            ..ResolvedConfig::default()
        };
        let enum_tree = ResolvedConfig {
            support_type: slicer_ir::SupportType::TreeAuto,
            ..ResolvedConfig::default()
        };
        for (region_id, config) in [(3, canonical_tree), (4, alias_override), (5, enum_tree)] {
            let config_id = region_map.intern_config(config);
            // Family is resolved at the contact layer, so the map must carry the
            // region on every layer it exists on — as a production region map does.
            for global_layer_index in 0..=1 {
                region_map.entries.insert(
                    RegionKey {
                        global_layer_index,
                        object_id: "object".to_string(),
                        region_id,
                        variant_chain: Vec::new(),
                    },
                    RegionPlan {
                        config: config_id,
                        ..RegionPlan::default()
                    },
                );
            }
        }
        blackboard.commit_region_map(Arc::new(region_map)).unwrap();

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let assignments = &blackboard.support_analysis().unwrap().family_assignments;
        assert_eq!(assignments[&(String::from("object"), 3)], "tree");
        assert_eq!(assignments[&(String::from("object"), 4)], "traditional");
        assert_eq!(assignments[&(String::from("object"), 5)], "tree");
    }

    /// F-44: a painted region is keyed in the region map by a **non-empty**
    /// `variant_chain`, so the exact empty-chain `RegionKey` lookup misses it.
    /// The executor's `backfill_active_region_configs` routes such a region to
    /// the tree planner via its smallest-chain fallback; before the fix this
    /// stage recorded it as "traditional", so the routing decision and the
    /// recorded `family_assignments` string disagreed.
    #[test]
    fn support_analysis_resolves_painted_region_family_via_variant_chain() {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: global_layers(2),
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            .commit_slice_ir(Arc::new(
                [(0_u32, 1.0_f32), (1_u32, 2.0_f32)]
                    .into_iter()
                    .map(|(global_layer_index, size)| SliceIR {
                        global_layer_index,
                        regions: vec![SlicedRegion {
                            object_id: "object".to_string(),
                            region_id: 7,
                            polygons: vec![square(0.0, 0.0, size)],
                            ..SlicedRegion::default()
                        }],
                        ..SliceIR::default()
                    })
                    .collect::<Vec<_>>(),
            ))
            .unwrap();

        let mut region_map = RegionMapIR::default();
        let config_id = region_map.intern_config(ResolvedConfig {
            support_type: slicer_ir::SupportType::TreeAuto,
            ..ResolvedConfig::default()
        });
        for global_layer_index in 0..=1 {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index,
                    object_id: "object".to_string(),
                    region_id: 7,
                    // Painted: the only entry for this region carries a
                    // non-empty chain, exactly as RegionMapping emits it.
                    variant_chain: vec![(
                        "support_paint".to_string(),
                        slicer_ir::slice_ir::PaintValue::Flag(true),
                    )],
                },
                RegionPlan {
                    config: config_id,
                    ..RegionPlan::default()
                },
            );
        }
        blackboard.commit_region_map(Arc::new(region_map)).unwrap();

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let assignments = &blackboard.support_analysis().unwrap().family_assignments;
        assert_eq!(assignments[&(String::from("object"), 7)], "tree");
    }

    // ---------------------------------------------------------------------
    // F-19: the auto/manual axis of canonical `support_type`.
    // ---------------------------------------------------------------------

    /// Axis-aligned box spanning `[x0,x1] x [y0,y1]` in mm over the full Z of
    /// both fixture layers (`global_layers(2)` puts them at 0.2 and 0.4 mm).
    fn modifier_box(x0: f32, y0: f32, x1: f32, y1: f32) -> slicer_ir::IndexedTriangleSet {
        use slicer_ir::{IndexedTriangleSet, Point3};
        let (z0, z1) = (-1.0_f32, 1.0_f32);
        let vertices = [
            (x0, y0, z0),
            (x1, y0, z0),
            (x1, y1, z0),
            (x0, y1, z0),
            (x0, y0, z1),
            (x1, y0, z1),
            (x1, y1, z1),
            (x0, y1, z1),
        ]
        .into_iter()
        .map(|(x, y, z)| Point3 { x, y, z })
        .collect();
        IndexedTriangleSet {
            vertices,
            indices: vec![
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0,
                7, 3, 1, 2, 6, 1, 6, 5,
            ],
        }
    }

    /// A `MeshIR` for object `"object"` carrying the given `(subtype, box)`
    /// modifier volumes.
    fn mesh_with_modifiers(volumes: &[(&str, slicer_ir::IndexedTriangleSet)]) -> MeshIR {
        use slicer_ir::{ConfigDelta, ModifierScope, ModifierVolume, ObjectMesh};
        MeshIR {
            objects: vec![ObjectMesh {
                id: "object".to_string(),
                modifier_volumes: volumes
                    .iter()
                    .enumerate()
                    // exhaustive: ModifierVolume has no Default and every field is load-bearing here
                    .map(|(index, (subtype, mesh))| ModifierVolume {
                        id: format!("mv-{index}"),
                        mesh: mesh.clone(),
                        config_delta: ConfigDelta {
                            fields: [(
                                "subtype".to_string(),
                                ConfigValue::String((*subtype).to_string()),
                            )]
                            .into_iter()
                            .collect(),
                        },
                        priority: 0,
                        applies_to: ModifierScope::AllFeatures,
                    })
                    .collect(),
                ..ObjectMesh::default()
            }],
            ..MeshIR::default()
        }
    }

    fn config_with_support_type(spelling: &str) -> ResolvedConfig {
        ResolvedConfig {
            extensions: [(
                "support_type".to_string(),
                ConfigValue::String(spelling.to_string()),
            )]
            .into_iter()
            .collect(),
            ..support_enabled_config()
        }
    }

    /// The overhanging stack from `support_analysis_populates_all_derivable_inputs`:
    /// layer 1 is a 5mm square overhanging a 3mm one on every side.
    fn overhang_stack() -> (ExPolygon, ExPolygon) {
        (square(1.0, 2.0, 3.0), square(0.0, 1.0, 5.0))
    }

    fn candidates_for(config: &ResolvedConfig, mesh: MeshIR) -> Vec<SupportCandidate> {
        let (lower, upper) = overhang_stack();
        let mut blackboard = blackboard_with_stack_and_mesh(&lower, &upper, mesh);
        commit_support_analysis_builtin(&mut blackboard, config).unwrap();
        blackboard.support_analysis().unwrap().candidates.clone()
    }

    /// Canonical `detect_overhangs` (`SupportMaterial.cpp`) gates its
    /// angle-thresholded branch on `support_type == stNormalAuto`, and the Orca
    /// `support_type` tooltip is explicit: "If Normal (manual) or Tree (manual)
    /// is selected, only support enforcers are generated."
    ///
    /// Before F-19 `SupportType` had no auto/manual axis at all, so this
    /// genuinely-overhanging stack produced an auto-detected candidate under
    /// every `support_type` value.
    #[test]
    fn manual_support_type_emits_no_auto_detected_candidate() {
        // Control: the same fixture under `normal(auto)` does overhang.
        assert!(
            !candidates_for(&config_with_support_type("normal(auto)"), MeshIR::default())
                .is_empty(),
            "fixture must overhang under normal(auto), or the manual assertion proves nothing"
        );
        for manual in ["normal(manual)", "tree(manual)"] {
            assert!(
                candidates_for(&config_with_support_type(manual), MeshIR::default()).is_empty(),
                "{manual} must emit no auto-detected candidate without an enforcer"
            );
        }
    }

    /// Manual mode still supports enforcer-covered geometry, and only that
    /// geometry.
    #[test]
    fn manual_support_type_emits_enforcer_driven_candidate() {
        let enforcer = square(0.0, 1.0, 2.0);
        let candidates = candidates_for(
            &config_with_support_type("tree(manual)"),
            mesh_with_modifiers(&[("support_enforcer", modifier_box(0.0, 1.0, 2.0, 3.0))]),
        );
        assert_eq!(
            candidates.len(),
            1,
            "one enforcer over one overhanging layer yields one candidate"
        );
        let candidate = &candidates[0];
        assert_eq!(candidate.source.global_layer_index, 1);
        assert!(
            candidate.enforced,
            "enforcer-driven candidate must be flagged"
        );
        assert!(!candidate.blocked);
        assert!(
            difference_ex(&candidate.geometry, &[enforcer]).is_empty(),
            "manual candidates must not extend beyond the enforcer footprint"
        );
    }

    /// Canonical `detect_overhangs` step 3 subtracts the blocker set from the
    /// contact region. The producer used to pass `&[]` for that parameter.
    #[test]
    fn auto_support_type_subtracts_blockers() {
        let candidates = candidates_for(
            &config_with_support_type("normal(auto)"),
            // Covers the whole layer-1 footprint, so nothing survives.
            mesh_with_modifiers(&[("support_blocker", modifier_box(-1.0, 0.0, 6.0, 7.0))]),
        );
        assert!(
            candidates.is_empty(),
            "a blocker covering the whole overhang must remove every candidate"
        );
    }

    /// A blocker that only clips part of the overhang leaves a candidate, and
    /// that candidate is flagged `blocked`.
    #[test]
    fn auto_support_type_flags_partially_blocked_candidate() {
        let candidates = candidates_for(
            &config_with_support_type("normal(auto)"),
            mesh_with_modifiers(&[("support_blocker", modifier_box(-1.0, 0.0, 1.0, 7.0))]),
        );
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0].blocked,
            "candidate overlapping a blocker must be flagged"
        );
        assert!(!candidates[0].enforced);
        assert!(
            difference_ex(&candidates[0].geometry, &[square(1.0, 1.0, 5.0)]).is_empty(),
            "the blocked strip (x < 1mm) must be subtracted from the contact"
        );
    }

    /// The axis is a per-region setting: a region map may give two regions of
    /// the same object different `support_type` values, and the run config must
    /// not override them.
    #[test]
    fn auto_manual_axis_is_resolved_per_region() {
        let (lower, upper) = overhang_stack();
        let mut blackboard = blackboard_with_stack_and_mesh(&lower, &upper, MeshIR::default());
        let mut region_map = RegionMapIR::default();
        let manual = region_map.intern_config(config_with_support_type("tree(manual)"));
        for global_layer_index in 0..=1 {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index,
                    object_id: "object".to_string(),
                    region_id: 3,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: manual,
                    ..RegionPlan::default()
                },
            );
        }
        blackboard.commit_region_map(Arc::new(region_map)).unwrap();
        // The *run* config is auto; the region's own config is manual and wins.
        commit_support_analysis_builtin(&mut blackboard, &config_with_support_type("normal(auto)"))
            .unwrap();
        assert!(
            blackboard.support_analysis().unwrap().candidates.is_empty(),
            "a manual region config must suppress auto detection even under an auto run config"
        );
    }
}
