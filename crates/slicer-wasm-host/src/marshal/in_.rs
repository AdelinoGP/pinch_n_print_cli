//! Pure IR→WIT projection functions (marshal-in surface).
//!
//! AC-2: This file is engine-free — no component-model store, resource table,
//! or resource handle types may appear here.  Functions take IR refs or
//! WIT-typed vecs and return WIT data structs / host backing-data structs.
//!
//! Moved from `host.rs` (packet 113, Step 7 / ADR-0021).

use crate::host::{
    self, ir_to_wit_expolygons, ir_to_wit_paint_layer_view, ir_to_wit_paint_semantic,
    ir_to_wit_paint_value, ir_to_wit_wall_loop, PerimeterRegionData, Point3,
    SegmentAnnotationsEntry, SegmentAnnotationsPolygon, SliceRegionData,
};

// ── Prepass projection helpers ───────────────────────────────────────────────

/// Convert a slicer-ir `ObjectMesh` to a WIT `MeshObjectView` for prepass modules.
///
/// Extracts mesh geometry and paint data from an `ObjectMesh` and produces a
/// read-only WIT view suitable for passing to prepass modules.
pub fn object_mesh_to_wit_mesh_object_view(
    mesh: &slicer_ir::ObjectMesh,
) -> host::prepass::MeshObjectView {
    let vertices: Vec<host::prepass::Point3> = mesh
        .mesh
        .vertices
        .iter()
        .map(|v| host::prepass::Point3 {
            x: v.x,
            y: v.y,
            z: v.z,
        })
        .collect();

    // Convert indexed triangles to list of tuples
    let triangles: Vec<(u32, u32, u32)> = mesh
        .mesh
        .indices
        .chunks(3)
        .map(|chunk| (chunk[0], chunk[1], chunk[2]))
        .collect();

    // Convert paint layers if present
    let paint_layers: Vec<host::prepass::PaintLayerView> =
        if let Some(ref paint_data) = mesh.paint_data {
            paint_data
                .layers
                .iter()
                .map(ir_to_wit_paint_layer_view)
                .collect()
        } else {
            Vec::new()
        };

    host::prepass::MeshObjectView {
        object_id: mesh.id.clone(),
        vertices,
        triangles,
        paint_layers,
    }
}

/// Project `LayerPlanIR` into a deterministic WIT `LayerPlanView`.
///
/// Layers are sorted by `global_layer_index ASC`. The effective layer height
/// per global layer is the maximum across all objects at that layer index
/// (from `object_participation`).
pub fn project_layer_plan_view(
    layer_plan_ir: &slicer_ir::LayerPlanIR,
) -> host::prepass::LayerPlanView {
    let mut entries: Vec<host::prepass::LayerPlanViewEntry> = layer_plan_ir
        .global_layers
        .iter()
        .map(|gl| {
            let effective_layer_height =
                crate::marshal::canonical_effective_layer_height(layer_plan_ir, gl.index);
            host::prepass::LayerPlanViewEntry {
                global_layer_index: gl.index,
                z: gl.z,
                effective_layer_height,
            }
        })
        .collect();
    // Already sorted by index since global_layers is ordered, but sort to be safe.
    entries.sort_by_key(|a| a.global_layer_index);
    host::prepass::LayerPlanView { layers: entries }
}

/// Project active SliceIR regions into the seam planner's whole-print input.
///
/// Only regions with a non-empty supplied boundary are forwarded. The exact
/// RegionMap key, including its variant chain, is used when a map is present
/// so inactive or identity-mismatched regions cannot fabricate a seam plan.
pub fn project_seam_planning_view(
    slices: &[slicer_ir::SliceIR],
    layer_plan: Option<&slicer_ir::LayerPlanIR>,
    region_map: Option<&slicer_ir::RegionMapIR>,
    config_view: &slicer_ir::ConfigView,
) -> host::SeamPlanningViewData {
    let scoring_width = match config_view.get("seam_scoring_width") {
        Some(slicer_ir::ConfigValue::Float(value)) => *value as f32,
        Some(slicer_ir::ConfigValue::Int(value)) => *value as f32,
        _ => 0.4,
    };
    let scoring_width = if scoring_width.is_finite() && scoring_width > 0.0 {
        scoring_width
    } else {
        0.4
    };

    let mut ordered_slices: Vec<&slicer_ir::SliceIR> = slices.iter().collect();
    ordered_slices.sort_by_key(|slice| slice.global_layer_index);

    let mut regions = Vec::new();
    for slice in ordered_slices {
        if layer_plan.is_some_and(|plan| {
            !plan
                .global_layers
                .iter()
                .any(|layer| layer.index == slice.global_layer_index)
        }) {
            continue;
        }

        let mut sliced_regions: Vec<&slicer_ir::SlicedRegion> = slice.regions.iter().collect();
        sliced_regions.sort_by(|left, right| {
            left.object_id
                .cmp(&right.object_id)
                .then(left.region_id.cmp(&right.region_id))
                .then(left.variant_chain.cmp(&right.variant_chain))
        });

        for region in sliced_regions {
            if region.object_id.is_empty() || region.polygons.is_empty() {
                continue;
            }
            if let Some(map) = region_map {
                let active_key = slicer_ir::RegionKey {
                    global_layer_index: slice.global_layer_index,
                    object_id: region.object_id.clone(),
                    region_id: region.region_id,
                    variant_chain: region.variant_chain.clone(),
                };
                if !map.entries.contains_key(&active_key) {
                    continue;
                }
            }

            let mut segment_annotations: Vec<_> = region.segment_annotations.iter().collect();
            segment_annotations.sort_by(|left, right| left.0.cmp(right.0));
            let segment_annotations = segment_annotations
                .into_iter()
                .map(|(semantic, polygons)| host::SegmentAnnotationsEntry {
                    semantic: ir_to_wit_paint_semantic(semantic),
                    polygons: polygons
                        .iter()
                        .map(|values| host::SegmentAnnotationsPolygon {
                            values: values
                                .iter()
                                .map(|value| value.as_ref().map(ir_to_wit_paint_value))
                                .collect(),
                        })
                        .collect(),
                })
                .collect();

            regions.push(host::prepass::SeamPlanningRegionInput {
                global_layer_index: slice.global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id.to_string(),
                variant_chain: region
                    .variant_chain
                    .iter()
                    .map(|(semantic, value)| (semantic.clone(), ir_to_wit_paint_value(value)))
                    .collect(),
                z: slice.z,
                height: region.effective_layer_height,
                ex_polygons: ir_to_wit_expolygons(&region.polygons),
                segment_annotations,
                scoring_width,
            });
        }
    }

    host::SeamPlanningViewData { regions }
}

/// Project `RegionMapIR` into a deterministic WIT `RegionSegmentationView`.
///
/// Entries are sorted by `(global_layer_index ASC, object_id ASC)` with each
/// entry's `region_ids` sorted ASC. This ensures byte-identical projections
/// across consecutive runs.
pub fn project_region_segmentation_view(
    region_map_ir: &slicer_ir::RegionMapIR,
) -> host::prepass::RegionSegmentationView {
    // Group by (global_layer_index, object_id).
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<(u32, String), Vec<String>> = BTreeMap::new();
    for key in region_map_ir.entries.keys() {
        let entry = grouped
            .entry((key.global_layer_index, key.object_id.clone()))
            .or_default();
        entry.push(key.region_id.to_string());
    }
    let mut entries: Vec<host::prepass::RegionSegmentationViewEntry> = grouped
        .into_iter()
        .map(|((layer_index, object_id), mut region_ids)| {
            region_ids.sort(); // ASC by region_id string
            host::prepass::RegionSegmentationViewEntry {
                object_id,
                layer_index,
                region_ids,
            }
        })
        .collect();
    // Already sorted by BTreeMap key order, but explicit sort for clarity.
    entries.sort_by(|a, b| {
        a.layer_index
            .cmp(&b.layer_index)
            .then_with(|| a.object_id.cmp(&b.object_id))
    });
    let mut region_support_configs = Vec::new();
    for key in region_map_ir.entries.keys() {
        let config = region_map_ir.config_for(key).to_config_map();
        let string_value = |name: &str| match config.get(name) {
            Some(slicer_ir::ConfigValue::String(value)) => Some(value.clone()),
            _ => None,
        };
        region_support_configs.push(
            crate::host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::RegionSupportConfig {
            object_id: key.object_id.clone(),
            layer_index: key.global_layer_index,
            region_id: key.region_id.to_string(),
            support_family: string_value("support_family"),
            support_type: string_value("support_type"),
            },
        );
    }
    region_support_configs.sort_by(|a, b| {
        a.layer_index
            .cmp(&b.layer_index)
            .then_with(|| a.object_id.cmp(&b.object_id))
            .then_with(|| a.region_id.cmp(&b.region_id))
    });
    host::prepass::RegionSegmentationView {
        entries,
        region_support_configs,
    }
}

/// Project `SupportGeometryIR` into a deterministic WIT `SupportGeometryView`.
///
/// Entries are sorted by `(global_support_layer_index ASC, object_id ASC, region_id ASC)`.
/// This mirrors the RegionSegmentationView ordering pattern.
pub fn project_support_geometry_view(
    support_geometry_ir: &slicer_ir::SupportGeometryIR,
) -> host::prepass::SupportGeometryView {
    use std::collections::BTreeMap;
    let mut sorted_entries: Vec<host::prepass::SupportGeometryViewEntry> = {
        let mut btree: BTreeMap<(u32, String, String), host::prepass::SupportGeometryViewEntry> =
            BTreeMap::new();
        for (key, polygons) in &support_geometry_ir.entries {
            btree.insert(
                (
                    key.global_support_layer_index,
                    key.object_id.clone(),
                    key.region_id.to_string(),
                ),
                host::prepass::SupportGeometryViewEntry {
                    global_support_layer_index: key.global_support_layer_index,
                    object_id: key.object_id.clone(),
                    region_id: key.region_id.to_string(),
                    outlines: ir_to_wit_expolygons(polygons),
                },
            );
        }
        btree.into_values().collect()
    };
    sorted_entries.sort_by(|a, b| {
        a.global_support_layer_index
            .cmp(&b.global_support_layer_index)
            .then_with(|| a.object_id.cmp(&b.object_id))
            .then_with(|| a.region_id.cmp(&b.region_id))
    });
    host::prepass::SupportGeometryView {
        entries: sorted_entries,
    }
}

/// Project host-owned support analysis into the planner's strategy-neutral view.
pub fn project_support_analysis_view(
    analysis: &slicer_ir::SupportAnalysisIR,
) -> host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportAnalysisView{
    use crate::host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types as types;

    let mut candidates: Vec<_> = analysis
        .candidates
        .iter()
        .map(|candidate| types::SupportAnalysisCandidate {
            id: candidate.id,
            geometry: ir_to_wit_expolygons(&candidate.geometry),
            object_id: candidate.source.object_id.clone(),
            region_id: candidate.source.region_id.to_string(),
            global_layer_index: candidate.source.global_layer_index,
            z_units: candidate.source.z_units,
            enforced: candidate.enforced,
            blocked: candidate.blocked,
        })
        .collect();
    candidates.sort_by(|left, right| {
        left.global_layer_index
            .cmp(&right.global_layer_index)
            .then_with(|| left.object_id.cmp(&right.object_id))
            .then_with(|| left.region_id.cmp(&right.region_id))
            .then_with(|| left.id.cmp(&right.id))
    });
    let project_geometry = |entries: &std::collections::HashMap<
        slicer_ir::SupportGeometryKey,
        Vec<slicer_ir::ExPolygon>,
    >| {
        let mut projected: Vec<_> = entries
            .iter()
            .map(|(key, polygons)| types::SupportAnalysisGeometryEntry {
                global_support_layer_index: key.global_support_layer_index,
                object_id: key.object_id.clone(),
                region_id: key.region_id.to_string(),
                polygons: ir_to_wit_expolygons(polygons),
            })
            .collect();
        projected.sort_by(|left, right| {
            left.global_support_layer_index
                .cmp(&right.global_support_layer_index)
                .then_with(|| left.object_id.cmp(&right.object_id))
                .then_with(|| left.region_id.cmp(&right.region_id))
        });
        projected
    };
    host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportAnalysisView {
        candidates,
        model_occupancy: project_geometry(&analysis.model_occupancy),
        termination_surfaces: project_geometry(&analysis.termination_surfaces),
        shared_settings: analysis
            .shared_settings
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        baseline_feasible_envelope: ir_to_wit_expolygons(&analysis.baseline_feasible_envelope),
        family_assignments: analysis
            .family_assignments
            .iter()
            .map(|((object_id, region_id), family_id)| types::SupportFamilyAssignment {
                object_id: object_id.clone(),
                region_id: region_id.to_string(),
                family_id: family_id.clone(),
            })
            .collect(),
    }
}

// ── Slice / Perimeter region data builders ───────────────────────────────────

/// Convert a `SlicedRegion` from the IR into a `SliceRegionData` for the WIT resource.
///
/// `held_claims` is the resolved fill-role claim set for this module on this
/// region, computed by `validation::resolve_held_claims` against the region's
/// `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`. The dispatcher
/// builds the `(ObjectId, RegionId) -> Vec<String>` map on
/// `HostExecutionContext.held_claims_per_region` before the WIT call;
/// `push_slice_regions` looks up each region and passes the slice in here.
///
/// `surface_classification` is used to resolve `region.nonplanar_surface` to a
/// `SurfaceGroup` record. Pass `None` when the IR is unavailable (e.g. in tests
/// that construct regions directly); the `surface_group` field will be `None`.
/// `global_layer_index` is the layer this region was sliced at; used to key
/// `SurfaceClassificationIR.overhang_quartile_polygons` (host-only aggregation
/// populated by `PrePass::OverhangAnnotation`, packet 106/107).
pub fn sliced_region_to_data(
    region: &slicer_ir::SlicedRegion,
    z: f32,
    held_claims: Vec<String>,
    surface_classification: Option<&slicer_ir::SurfaceClassificationIR>,
    global_layer_index: u32,
) -> SliceRegionData {
    let view = slicer_sdk::views::SliceRegionView::from_ir(region, z, held_claims);
    let segment_annotations: Vec<SegmentAnnotationsEntry> = region
        .segment_annotations
        .iter()
        .map(|(semantic, poly_values)| SegmentAnnotationsEntry {
            semantic: ir_to_wit_paint_semantic(semantic),
            polygons: poly_values
                .iter()
                .map(|point_values| SegmentAnnotationsPolygon {
                    values: point_values
                        .iter()
                        .map(|opt| opt.as_ref().map(ir_to_wit_paint_value))
                        .collect(),
                })
                .collect(),
        })
        .collect();

    // Project the region's paint variant chain (carries the painted FuzzySkin
    // signal) so the guest's `variant-chain()` accessor can enable per-vertex
    // jitter without routing FuzzySkin through segment_annotations (D14).
    let variant_chain: Vec<(String, _)> = view
        .variant_chain()
        .iter()
        .map(|(name, value)| (name.clone(), ir_to_wit_paint_value(value)))
        .collect();

    // Resolve the surface group from SurfaceClassificationIR if available.
    let surface_group: Option<crate::host::layer::slicer::ir_handles::ir_handles::SurfaceGroup> =
        region.nonplanar_surface.and_then(|sg_id| {
            surface_classification
                .and_then(|sc| sc.per_object.get(&region.object_id))
                .and_then(|obj| obj.surface_groups.iter().find(|g| g.id == sg_id))
                .map(
                    |g| crate::host::layer::slicer::ir_handles::ir_handles::SurfaceGroup {
                        id: g.id,
                        facet_indices: g.facet_indices.clone(),
                        z_min: g.z_min,
                        z_max: g.z_max,
                        area_mm2: g.area_mm2,
                        printable: g.printable,
                        shell_count: g.shell_count,
                    },
                )
        });

    // Clip this layer's overhang quartile bands to this region's own polygon
    // area. AC-1 requires bands to be exactly pre-filtered to the region's
    // polygon footprint, not merely AABB-adjacent to it. The AABB check below
    // is retained only as a cheap prefilter (fast-reject candidates whose
    // bounding boxes don't even overlap) — it is NOT the source of truth for
    // inclusion. Bands surviving the prefilter are exactly intersected against
    // `region.polygons` via `slicer_core::polygon_ops::intersection_ex`; any
    // band whose clipped result is empty is dropped entirely. `overhang_areas`
    // is then the union of the clipped bands, i.e. exactly the overhang
    // footprint that actually covers this region (AC-2).
    let region_bbox = expolygons_bbox(&region.polygons);
    let overhang_quartile_polygons: Vec<
        crate::host::layer::slicer::ir_handles::ir_handles::QuartileBand,
    > = surface_classification
        .and_then(|sc| {
            sc.overhang_quartile_polygons
                .get(&region.object_id)
                .and_then(|by_layer| by_layer.get(&global_layer_index))
        })
        .map(|bands| {
            bands
                .iter()
                .filter_map(|band| {
                    let prefiltered: Vec<slicer_ir::ExPolygon> = band
                        .polygons
                        .iter()
                        .filter(|poly| match region_bbox {
                            Some(rb) => bbox_overlaps(rb, poly),
                            None => false,
                        })
                        .cloned()
                        .collect();
                    if prefiltered.is_empty() {
                        return None;
                    }
                    let clipped: Vec<slicer_ir::ExPolygon> =
                        slicer_core::polygon_ops::intersection_ex(&prefiltered, &region.polygons);
                    if clipped.is_empty() {
                        None
                    } else {
                        Some(
                            crate::host::layer::slicer::ir_handles::ir_handles::QuartileBand {
                                quartile: band.quartile,
                                polygons: ir_to_wit_expolygons(&clipped),
                            },
                        )
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let overhang_areas: Vec<crate::host::layer::slicer::types::geometry::ExPolygon> =
        overhang_quartile_polygons
            .iter()
            .flat_map(|band| band.polygons.clone())
            .collect();
    let prev_layer_boundary: Vec<crate::host::layer::slicer::types::geometry::ExPolygon> =
        surface_classification
            .and_then(|sc| {
                sc.prev_layer_boundaries
                    .get(&region.object_id)
                    .and_then(|by_layer| by_layer.get(&global_layer_index))
            })
            .map(|polygons| ir_to_wit_expolygons(polygons))
            .unwrap_or_default();

    SliceRegionData {
        object_id: view.object_id().clone(),
        region_id: view.region_id().to_string(),
        polygons: ir_to_wit_expolygons(view.polygons()),
        infill_areas: ir_to_wit_expolygons(view.infill_areas()),
        effective_layer_height: view.effective_layer_height(),
        z: view.z(),
        variant_chain,
        has_nonplanar: view.has_nonplanar(),
        segment_annotations,
        needs_support: view.derive_needs_support(surface_classification),
        top_shell_index: view.top_shell_index(),
        bottom_shell_index: view.bottom_shell_index(),
        top_solid_fill: ir_to_wit_expolygons(view.top_solid_fill()),
        bottom_solid_fill: ir_to_wit_expolygons(view.bottom_solid_fill()),
        internal_solid_fill: ir_to_wit_expolygons(view.internal_solid_fill()),
        is_bridge: view.is_bridge(),
        bridge_areas: ir_to_wit_expolygons(view.bridge_areas()),
        internal_bridge_areas: ir_to_wit_expolygons(view.internal_bridge_areas()),
        bridge_orientation_deg: view.bridge_orientation_deg(),
        sparse_infill_area: ir_to_wit_expolygons(view.sparse_infill_area()),
        held_claims: view.held_claims().to_vec(),
        overhang_areas,
        overhang_quartile_polygons,
        prev_layer_boundary,
        surface_group,
    }
}

/// Axis-aligned bounding box in slice-space units (1 unit = 100 nm). `None`
/// when the polygon set is empty (e.g. an unsliced region).
type Bbox = (i64, i64, i64, i64);

/// Bounding box (min_x, min_y, max_x, max_y) over every contour/hole vertex
/// across the given polygons. Returns `None` for an empty slice.
fn expolygons_bbox(polys: &[slicer_ir::ExPolygon]) -> Option<Bbox> {
    let mut acc: Option<Bbox> = None;
    for poly in polys {
        for pt in poly
            .contour
            .points
            .iter()
            .chain(poly.holes.iter().flat_map(|h| h.points.iter()))
        {
            acc = Some(match acc {
                None => (pt.x, pt.y, pt.x, pt.y),
                Some((min_x, min_y, max_x, max_y)) => (
                    min_x.min(pt.x),
                    min_y.min(pt.y),
                    max_x.max(pt.x),
                    max_y.max(pt.y),
                ),
            });
        }
    }
    acc
}

/// Cheap AABB-overlap prefilter between a region's bounding box and a
/// candidate overhang polygon. Not an exact boolean intersection — sufficient
/// for gating which overhang quartile polygons are relevant to a region
/// (AC-1, packet 107).
fn bbox_overlaps(region_bbox: Bbox, poly: &slicer_ir::ExPolygon) -> bool {
    let Some(poly_bbox) = expolygons_bbox(std::slice::from_ref(poly)) else {
        return false;
    };
    let (r_min_x, r_min_y, r_max_x, r_max_y) = region_bbox;
    let (p_min_x, p_min_y, p_max_x, p_max_y) = poly_bbox;
    r_min_x <= p_max_x && p_min_x <= r_max_x && r_min_y <= p_max_y && p_min_y <= r_max_y
}

/// Convert a `PerimeterRegion` from the IR into a `PerimeterRegionData` WIT resource.
pub fn perimeter_region_to_data(region: &slicer_ir::PerimeterRegion) -> PerimeterRegionData {
    let view = slicer_sdk::views::PerimeterRegionView::from_ir(region);
    PerimeterRegionData {
        object_id: view.object_id().clone(),
        region_id: view.region_id().to_string(),
        wall_loops: view.wall_loops().iter().map(ir_to_wit_wall_loop).collect(),
        infill_areas: ir_to_wit_expolygons(view.infill_areas()),
        // ADR-0028 fields default empty here; the Layer::InfillPostProcess
        // dispatch arm enriches them from SliceIR/RegionMapIR after
        // construction (crate::dispatch).
        sparse_infill_area: Vec::new(),
        top_solid_fill: Vec::new(),
        bottom_solid_fill: Vec::new(),
        bridge_areas: Vec::new(),
        tool_index: 0,
        wall_source_region_id: None,
        // Note: width and flow_factor are intentionally discarded here;
        // SeamPosition.point is used for diagnostics only.
        resolved_seam: view.resolved_seam().cloned().map(|sp| {
            (
                Point3 {
                    x: sp.point.x,
                    y: sp.point.y,
                    z: sp.point.z,
                },
                sp.wall_index,
            )
        }),
        // Note: width/flow_factor/overhang_quartile and `reason` are
        // intentionally discarded here, mirroring the resolved_seam
        // conversion above — the WIT `push-seam-candidate` write contract
        // never carried them (only `pos: point3, score: f32`), so there is
        // nothing to round-trip.
        seam_candidates: view
            .seam_candidates()
            .iter()
            .map(|sc| {
                (
                    Point3 {
                        x: sc.position.x,
                        y: sc.position.y,
                        z: sc.position.z,
                    },
                    sc.score,
                )
            })
            .collect(),
    }
}

// ── Harvest cores (WIT output → IR) ─────────────────────────────────────────
//
// These are the pure projection cores extracted from dispatch.rs's harvest_*
// wrapper functions (which keep the HostExecutionContext unwrapping).

/// Pure core of `harvest_layer_plan_ir`: `LayerProposal`s → `LayerPlanIR`.
pub(crate) fn harvest_layer_plan_ir_from(
    proposals: Vec<host::prepass::LayerProposal>,
) -> Result<slicer_ir::LayerPlanIR, String> {
    use slicer_ir::{ActiveRegion, GlobalLayer, LayerPlanIR, ObjectLayerRef, ResolvedConfig};
    use std::collections::HashMap;

    const MAX_LAYERS: u32 = 100_000;

    let mut global_layers: Vec<GlobalLayer> = Vec::with_capacity(proposals.len());
    let mut object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::new();

    for (idx, proposal) in proposals.into_iter().enumerate() {
        let index = idx as u32;
        if index >= MAX_LAYERS {
            return Err(format!(
                "layer-plan-output: layer count exceeded maximum budget of {MAX_LAYERS}"
            ));
        }

        let mut active_regions: Vec<ActiveRegion> = Vec::new();

        for region_prop in proposal.active_regions {
            let region_id =
                host::parse_canonical_region_id(&region_prop.region_id).map_err(|reason| {
                    format!(
                        "layer-plan-output: region '{}'/'{}' has invalid region-id: {reason}",
                        region_prop.object_id, region_prop.region_id
                    )
                })?;

            active_regions.push(ActiveRegion {
                object_id: region_prop.object_id.clone(),
                region_id,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: region_prop.effective_layer_height,
                nonplanar_shell: None,
                is_catchup_layer: region_prop.is_catchup,
                catchup_z_bottom: region_prop.catchup_z_bottom,
                tool_index: 0,
            });

            let obj_refs = object_participation
                .entry(region_prop.object_id.clone())
                .or_default();
            let already_referenced = obj_refs.iter().any(|r| r.global_layer_index == index);

            if !already_referenced {
                obj_refs.push(ObjectLayerRef {
                    local_layer_index: obj_refs.len() as u32,
                    global_layer_index: index,
                    effective_layer_height: region_prop.effective_layer_height,
                });
            }
        }

        global_layers.push(GlobalLayer {
            index,
            z: proposal.z,
            active_regions,
            has_nonplanar: false,
            is_sync_layer: false,
        });
    }

    Ok(LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    })
}

/// Restore host-only per-region configuration after a WASM layer-plan harvest.
pub(crate) fn restore_layer_plan_configs(
    plan: &mut slicer_ir::LayerPlanIR,
    input_layer_plan: Option<&slicer_ir::LayerPlanIR>,
    region_map: Option<&slicer_ir::RegionMapIR>,
    module_config: Option<&slicer_ir::ConfigView>,
) {
    for layer in &mut plan.global_layers {
        for region in &mut layer.active_regions {
            let resolved_config = region_map
                .and_then(|map| {
                    map.entries.iter().find_map(|(key, _)| {
                        (key.object_id == region.object_id && key.region_id == region.region_id)
                            .then(|| map.config_for(key).clone())
                    })
                })
                .or_else(|| {
                    input_layer_plan.and_then(|input| {
                        input.global_layers.iter().find_map(|input_layer| {
                            input_layer.active_regions.iter().find_map(|candidate| {
                                (candidate.object_id == region.object_id
                                    && candidate.region_id == region.region_id)
                                    .then(|| candidate.resolved_config.clone())
                            })
                        })
                    })
                })
                .or_else(|| {
                    input_layer_plan.and_then(|input| {
                        input.global_layers.iter().find_map(|input_layer| {
                            input_layer
                                .active_regions
                                .iter()
                                .find(|candidate| candidate.region_id == region.region_id)
                                .map(|candidate| candidate.resolved_config.clone())
                        })
                    })
                })
                .or_else(|| {
                    region_map.and_then(|map| {
                        map.entries.iter().find_map(|(key, _)| {
                            (key.region_id == region.region_id).then(|| map.config_for(key).clone())
                        })
                    })
                })
                .or_else(|| {
                    let mut config = slicer_ir::ResolvedConfig::default();
                    if let Some(value) = module_config.and_then(|view| view.get("support_type")) {
                        config
                            .extensions
                            .insert("support_type".to_string(), value.clone());
                    }
                    if let Some(value) = module_config.and_then(|view| view.get("support_family")) {
                        config
                            .extensions
                            .insert("support_family".to_string(), value.clone());
                    }
                    (!config.extensions.is_empty()).then_some(config)
                });
            if let Some(resolved_config) = resolved_config {
                region.resolved_config = resolved_config;
            }
        }
    }
}

#[allow(unreachable_patterns)]
fn convert_variant_chain(
    wit: &[(String, host::prepass::PaintValue)],
) -> Result<Vec<(String, slicer_ir::PaintValue)>, String> {
    wit.iter()
        .map(|(name, value)| {
            let value = match value {
                host::prepass::PaintValue::Flag(value) => slicer_ir::PaintValue::Flag(*value),
                host::prepass::PaintValue::Scalar(value) => slicer_ir::PaintValue::Scalar(*value),
                host::prepass::PaintValue::ToolIndex(value) => {
                    slicer_ir::PaintValue::ToolIndex(*value)
                }
                _ => {
                    return Err(format!(
                        "unsupported variant-chain paint value for '{name}'"
                    ))
                }
            };
            Ok((name.clone(), value))
        })
        .collect()
}

/// Pure core of `harvest_seam_plan_ir`: WIT `SeamPlanEntry`s → `SeamPlanIR`.
pub fn harvest_seam_plan_ir_from(
    seam_plan_entries: Vec<host::prepass::SeamPlanEntry>,
) -> Result<slicer_ir::SeamPlanIR, String> {
    use slicer_ir::SeamPosition;
    use slicer_ir::{RegionKey, ScoredSeamCandidate, SeamPlanEntry, SeamPlanIR};
    use std::collections::HashMap;

    let mut seen: HashMap<RegionKey, ()> = HashMap::new();
    let mut entries: Vec<SeamPlanEntry> = Vec::with_capacity(seam_plan_entries.len());

    for entry in seam_plan_entries.into_iter() {
        let region_id = host::parse_canonical_region_id(&entry.region_id).map_err(|reason| {
            format!(
                "seam-planning-output: region '{}'/'{}' has invalid identity: invalid region-id: {reason}",
                entry.object_id, entry.region_id
            )
        })?;

        let variant_chain = convert_variant_chain(&entry.variant_chain).map_err(|reason| {
            format!(
                "seam-planning-output: region '{}'/'{}' has invalid identity: {reason}",
                entry.object_id, entry.region_id
            )
        })?;

        let region_key = RegionKey {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id.clone(),
            region_id,
            variant_chain,
        };

        let is_duplicate = seen.contains_key(&region_key);
        seen.insert(region_key.clone(), ());
        if is_duplicate {
            continue;
        }

        let scored_candidates: Vec<ScoredSeamCandidate> = entry
            .scored_candidates
            .iter()
            .map(|sc| ScoredSeamCandidate {
                position: slicer_ir::Point3WithWidth {
                    x: sc.position.x,
                    y: sc.position.y,
                    z: sc.position.z,
                    width: sc.position.width,
                    flow_factor: sc.position.flow_factor,
                    overhang_quartile: sc.position.overhang_quartile,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                score: sc.score,
                reason: match sc.reason.tag.as_str() {
                    "concave" => slicer_ir::SeamReason::Concave,
                    "sharp" => slicer_ir::SeamReason::Sharp,
                    "user_forced" => slicer_ir::SeamReason::UserForced,
                    _ => slicer_ir::SeamReason::Aligned,
                },
            })
            .collect();

        let chosen_candidate = SeamPosition {
            point: slicer_ir::Point3WithWidth {
                x: entry.chosen_position.x,
                y: entry.chosen_position.y,
                z: entry.chosen_position.z,
                width: entry.chosen_position.width,
                flow_factor: entry.chosen_position.flow_factor,
                overhang_quartile: entry.chosen_position.overhang_quartile,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
            wall_index: entry.chosen_wall_index,
        };

        entries.push(SeamPlanEntry {
            region_key,
            chosen_candidate,
            scored_candidates,
        });
    }

    Ok(SeamPlanIR {
        entries,
        ..Default::default()
    })
}

/// Pure core of `harvest_support_plan_ir`: WIT `SupportPlanEntry`s → `SupportPlanIR`.
pub(crate) fn harvest_support_plan_ir_from(
    support_plan_entries: Vec<host::prepass::SupportPlanEntry>,
    raft_plan: Option<host::prepass::RaftPlan>,
) -> Result<slicer_ir::SupportPlanIR, String> {
    use slicer_ir::{
        RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
        SupportPlanRoleRegion, SupportPlanSkeleton,
    };

    let mut entries: Vec<SupportPlanEntry> = Vec::with_capacity(support_plan_entries.len());

    for entry in support_plan_entries.into_iter() {
        let region_id = host::parse_canonical_region_id(&entry.region_id).map_err(|reason| {
            format!(
                "support-generation-output: region '{}'/'{}' has invalid region-id: {reason}",
                entry.object_id, entry.region_id
            )
        })?;

        if let Some(skeleton) = entry.skeleton.as_ref() {
            if skeleton.wall_counts.len() != skeleton.points.len() {
                return Err(format!(
                    "support-generation-output: skeleton wall-counts length {} does not match points length {}",
                    skeleton.wall_counts.len(),
                    skeleton.points.len()
                ));
            }
        }

        entries.push(SupportPlanEntry {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id,
            region_id,
            family_id: entry.family_id,
            demand_ids: entry.demand_ids,
            body_ids: entry.body_ids,
            anchor_layer_index: entry.anchor_layer_index,
            anchor_z: entry.anchor_z,
            roles: entry
                .roles
                .into_iter()
                .map(|r| SupportPlanRoleRegion {
                    role: match r.role {
                        host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanRole::SupportBody => SupportPlanRole::SupportBody,
                        host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanRole::TopInterface => {
                            SupportPlanRole::TopInterface
                        }
                        host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanRole::BaseInterface => {
                            SupportPlanRole::BaseInterface
                        }
                        host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanRole::BottomInterface => {
                            SupportPlanRole::BottomInterface
                        }
                        host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanRole::RaftRelated => SupportPlanRole::RaftRelated,
                    },
                    regions: crate::marshal::wit_to_ir_expolygons(&r.regions),
                })
                .collect(),
            skeleton: entry.skeleton.map(|s| SupportPlanSkeleton {
                points: s
                    .points
                    .into_iter()
                    .map(|p| slicer_ir::Point3 {
                        x: p.x,
                        y: p.y,
                        z: p.z,
                    })
                    .collect(),
                wall_counts: s.wall_counts,
            }),
            capabilities: entry.capabilities,
            provenance: entry.provenance,
            decline_reason: entry.decline_reason.map(|r| match r {
                host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanDeclineReason::DeclinedPolicy => {
                    SupportPlanDeclineReason::DeclinedPolicy
                }
                host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanDeclineReason::NoRoute => {
                    SupportPlanDeclineReason::NoRoute
                }
                host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanDeclineReason::Blocked => {
                    SupportPlanDeclineReason::Blocked
                }
                host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::SupportPlanDeclineReason::UnsupportedMode => {
                    SupportPlanDeclineReason::UnsupportedMode
                }
            }),
        });
    }

    Ok(SupportPlanIR {
        entries,
        raft_plan: raft_plan.map(|plan| RaftPlan {
            raft_layers: plan.raft_layers,
            raft_first_layer_density: plan.raft_first_layer_density,
            base_raft_layers: plan.base_raft_layers,
            interface_raft_layers: plan.interface_raft_layers,
        }),
        ..Default::default()
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::harvest_support_plan_ir_from;
    use crate::host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::{
        SupportPlanEntry, SupportPlanRole, SupportPlanRoleRegion,
    };

    #[test]
    fn base_interface_role_round_trips_both_legs() {
        let wit_entry = SupportPlanEntry {
            global_layer_index: 3,
            object_id: "object-a".into(),
            region_id: "7".into(),
            family_id: "tree".into(),
            demand_ids: vec![],
            body_ids: vec![],
            anchor_layer_index: 3,
            anchor_z: 42_000,
            roles: vec![SupportPlanRoleRegion {
                role: SupportPlanRole::BaseInterface,
                regions: vec![],
            }],
            skeleton: None,
            capabilities: vec![],
            provenance: vec![],
            decline_reason: None,
        };

        let ir = harvest_support_plan_ir_from(vec![wit_entry], None).expect("WIT to IR");
        assert_eq!(
            ir.entries[0].roles[0].role,
            slicer_ir::SupportPlanRole::BaseInterface
        );

        let native_roles = crate::marshal::native::native_support_plan_roles(&ir.entries[0].roles);
        assert_eq!(
            native_roles[0].role,
            slicer_ir::SupportPlanRole::BaseInterface
        );
    }
}

/// Pure core of `harvest_mesh_analysis_auxiliary`.
pub(crate) fn harvest_mesh_analysis_auxiliary_from(
    mesh_analysis_annotations: Vec<(String, host::prepass::FacetAnnotation)>,
    mesh_analysis_surface_groups: Vec<(String, host::prepass::SurfaceGroupProposal)>,
) -> slicer_core::MeshAnalysisAuxiliary {
    use host::prepass as pm;
    use slicer_core::{
        FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, SurfaceGroupRecord,
    };

    let facet_annotations = mesh_analysis_annotations
        .into_iter()
        .map(|(obj, ann)| {
            let classification = match ann.classification {
                pm::FacetClass::Normal => FacetClassRecord::Normal,
                pm::FacetClass::NearHorizontal => FacetClassRecord::NearHorizontal,
                pm::FacetClass::Overhang => FacetClassRecord::Overhang,
                pm::FacetClass::Bridge => FacetClassRecord::Bridge,
                pm::FacetClass::TopSurface => FacetClassRecord::TopSurface,
                pm::FacetClass::BottomSurface => FacetClassRecord::BottomSurface,
            };
            (
                obj,
                FacetAnnotationRecord {
                    facet_index: ann.facet_index,
                    slope_angle_deg: ann.slope_angle_deg,
                    classification,
                },
            )
        })
        .collect();

    let surface_groups = mesh_analysis_surface_groups
        .into_iter()
        .map(|(obj, grp)| {
            (
                obj,
                SurfaceGroupRecord {
                    facet_indices: grp.facet_indices,
                    z_min: grp.z_min,
                    z_max: grp.z_max,
                    shell_count: grp.shell_count,
                },
            )
        })
        .collect();

    MeshAnalysisAuxiliary {
        facet_annotations,
        surface_groups,
    }
}
