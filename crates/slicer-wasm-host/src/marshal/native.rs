//! Native SDK transport for the marshalling boundary.
//!
//! This module deliberately feeds the same collected-output accumulators used
//! by the WASM transport.  Consequently validation, origin bucketing, and IR
//! conversion remain single-sourced in `out.rs`.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;

use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder, SupportOutputBuilder};
use slicer_sdk::native::{
    NativeFinalizationRequest, NativeFinalizationResponse, NativeLayerRequest, NativeLayerResponse,
    NativePostpassInput, NativePostpassRequest, NativePostpassResponse, NativePrepassRequest,
    NativePrepassResponse,
};
use slicer_sdk::postpass_types::GcodeOutputCommand;
use slicer_sdk::prepass_types::{
    LayerPlanView, MeshObjectView, RegionSegmentationView, SupportAnalysisView, SupportGeometryView,
};
use slicer_sdk::traits::LayerCollectionView;
use slicer_sdk::traits::PaintRegionLayerView;
use slicer_sdk::views::{PerimeterRegionView, SliceRegionView};

use crate::binding::{
    CompiledModuleLive, FinalizationStageInput, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};
use crate::marshal::{
    canonical_effective_layer_height, convert_infill_output, convert_perimeter_output,
    convert_support_output_with_plan, ir_to_wit_expolygons, ir_to_wit_extrusion_path,
    ir_to_wit_extrusion_role, ir_to_wit_wall_loop, GcodeCommandCollected, InfillOutputCollected,
    OriginId, PerimeterOutputCollected, SupportOutputCollected,
};

pub(crate) fn native_support_plan_roles(
    roles: &[slicer_ir::SupportPlanRoleRegion],
) -> Vec<slicer_ir::SupportPlanRoleRegion> {
    roles
        .iter()
        .map(|role| slicer_ir::SupportPlanRoleRegion {
            role: match role.role {
                slicer_ir::SupportPlanRole::SupportBody => slicer_ir::SupportPlanRole::SupportBody,
                slicer_ir::SupportPlanRole::TopInterface => {
                    slicer_ir::SupportPlanRole::TopInterface
                }
                slicer_ir::SupportPlanRole::BaseInterface => {
                    slicer_ir::SupportPlanRole::BaseInterface
                }
                slicer_ir::SupportPlanRole::BottomInterface => {
                    slicer_ir::SupportPlanRole::BottomInterface
                }
                slicer_ir::SupportPlanRole::RaftRelated => slicer_ir::SupportPlanRole::RaftRelated,
            },
            regions: role.regions.clone(),
        })
        .collect()
}

fn origin(value: &Option<slicer_sdk::builders::RegionOrigin>) -> Option<OriginId> {
    value.as_ref().map(|origin| OriginId {
        object_id: origin.object_id.clone(),
        region_id: origin.region_id,
    })
}

fn native_paint_layers(
    paint_data: &slicer_ir::FacetPaintData,
) -> Vec<slicer_sdk::prepass_types::PaintLayerView> {
    paint_data
        .layers
        .iter()
        .map(crate::marshal::ir_to_wit_paint_layer_view)
        .map(|layer| slicer_sdk::prepass_types::PaintLayerView {
            semantic: layer.semantic,
            facet_values: layer
                .facet_values
                .into_iter()
                .map(|value| value.map(native_paint_value))
                .collect(),
            strokes: layer
                .strokes
                .into_iter()
                .map(|stroke| slicer_sdk::prepass_types::PaintStrokeView {
                    triangles: stroke
                        .triangles
                        .chunks_exact(3)
                        .map(|points| {
                            [
                                [points[0].x, points[0].y, points[0].z],
                                [points[1].x, points[1].y, points[1].z],
                                [points[2].x, points[2].y, points[2].z],
                            ]
                        })
                        .collect(),
                    semantic: stroke.semantic,
                    value: native_paint_value(stroke.value),
                })
                .collect(),
        })
        .collect()
}

fn native_paint_value(
    value: crate::host::prepass::PaintValueView,
) -> slicer_sdk::prepass_types::PaintValueView {
    match value {
        crate::host::prepass::PaintValueView::Flag(value) => {
            slicer_sdk::prepass_types::PaintValueView {
                kind: "flag".to_string(),
                flag: Some(value),
                scalar: None,
                tool_index: None,
            }
        }
        crate::host::prepass::PaintValueView::Scalar(value) => {
            slicer_sdk::prepass_types::PaintValueView {
                kind: "scalar".to_string(),
                flag: None,
                scalar: Some(value),
                tool_index: None,
            }
        }
        crate::host::prepass::PaintValueView::ToolIndex(value) => {
            slicer_sdk::prepass_types::PaintValueView {
                kind: "tool_index".to_string(),
                flag: None,
                scalar: None,
                tool_index: Some(value),
            }
        }
    }
}

/// Axis-aligned bounding box in slice-space units (1 unit = 100 nm). `None`
/// when the polygon set is empty (e.g. an unsliced region).
type Bbox = (i64, i64, i64, i64);

/// Bounding box (min_x, min_y, max_x, max_y) over every contour/hole vertex
/// across the given polygons. Returns `None` for an empty slice.
///
/// IR-side twin of the wasm leg's `expolygons_bbox`
/// (`crates/slicer-wasm-host/src/marshal/in_.rs`); duplicated rather than
/// shared because the wasm leg's copy is private to that module.
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
/// candidate overhang polygon. Mirrors the wasm leg's `bbox_overlaps`.
fn bbox_overlaps(region_bbox: Bbox, poly: &slicer_ir::ExPolygon) -> bool {
    let Some(poly_bbox) = expolygons_bbox(std::slice::from_ref(poly)) else {
        return false;
    };
    let (r_min_x, r_min_y, r_max_x, r_max_y) = region_bbox;
    let (p_min_x, p_min_y, p_max_x, p_max_y) = poly_bbox;
    r_min_x <= p_max_x && p_min_x <= r_max_x && r_min_y <= p_max_y && p_min_y <= r_max_y
}

/// Populate the four `SurfaceClassificationIR`-derived fields on a native
/// `SliceRegionView`, mirroring the wasm leg's `sliced_region_to_data`
/// (`crates/slicer-wasm-host/src/marshal/in_.rs`) exactly — same surface-group
/// resolution by id, same AABB prefilter + exact `intersection_ex` clip with
/// empty-band drop, same flatten for `overhang_areas`, same
/// `prev_layer_boundaries` lookup. Without this the native/integrated leg hands
/// every module empty anchors while the wasm leg sees real data.
///
/// `global_layer_index` must be `SliceIR::global_layer_index` (the key used by
/// `SurfaceClassificationIR.overhang_quartile_polygons` /
/// `prev_layer_boundaries`), not the per-object layer index.
fn populate_surface_classification_fields(
    view: &mut SliceRegionView,
    region: &slicer_ir::SlicedRegion,
    surface_classification: Option<&slicer_ir::SurfaceClassificationIR>,
    global_layer_index: u32,
) {
    let surface_group = region.nonplanar_surface.and_then(|sg_id| {
        surface_classification
            .and_then(|sc| sc.per_object.get(&region.object_id))
            .and_then(|obj| obj.surface_groups.iter().find(|g| g.id == sg_id))
            .cloned()
    });
    view.set_surface_group(surface_group);

    let region_bbox = expolygons_bbox(&region.polygons);
    let overhang_quartile_polygons: Vec<slicer_ir::slice_ir::QuartileBand> = surface_classification
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
                        Some(slicer_ir::slice_ir::QuartileBand {
                            quartile: band.quartile,
                            polygons: clipped,
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let overhang_areas: Vec<slicer_ir::ExPolygon> = overhang_quartile_polygons
        .iter()
        .flat_map(|band| band.polygons.clone())
        .collect();
    view.set_overhang_quartile_polygons(overhang_quartile_polygons);
    view.set_overhang_areas(overhang_areas);

    let prev_layer_boundary: Vec<slicer_ir::ExPolygon> = surface_classification
        .and_then(|sc| {
            sc.prev_layer_boundaries
                .get(&region.object_id)
                .and_then(|by_layer| by_layer.get(&global_layer_index))
        })
        .cloned()
        .unwrap_or_default();
    view.set_prev_layer_boundary(prev_layer_boundary);
}

/// Build a native layer request without passing any wasm-host type across the
/// SDK boundary.
pub fn build_native_layer_request(
    stage_export: &'static str,
    layer_index: u32,
    input: &LayerStageInput<'_>,
    module: &CompiledModuleLive<'_>,
    held_claims_map: &HashMap<(String, String), Vec<String>>,
) -> NativeLayerRequest {
    let regions = input
        .slice
        .map(|slice| {
            slice
                .regions
                .iter()
                .map(|region| {
                    let mut view = SliceRegionView::from_ir(
                        region,
                        slice.z,
                        held_claims_map
                            .get(&(region.object_id.clone(), region.region_id.to_string()))
                            .cloned()
                            .unwrap_or_default(),
                    );
                    view.set_needs_support(view.derive_needs_support(input.surface_classification));
                    view.set_config((*module.config_view).clone());
                    populate_surface_classification_fields(
                        &mut view,
                        region,
                        input.surface_classification,
                        slice.global_layer_index,
                    );
                    view
                })
                .collect()
        })
        .unwrap_or_default();

    // Mirror the wasm leg (`push_perimeter_regions`): a layer with no committed
    // `PerimeterIR` yields an empty region list, never `None`. The postprocess
    // native entries (`run_wall_postprocess` / `run_infill_postprocess` /
    // `run_path_optimization`) require `perimeter_regions` to be `Some`; the
    // wasm leg tolerates a missing perimeter by pushing zero regions, so the
    // native leg must too (native/wasm leg parity, cf. 9685cd03).
    let perimeter_regions = Some(
        input
            .perimeter
            .map(|perimeter| {
                perimeter
                    .regions
                    .iter()
                    .map(|region| {
                        let mut view = PerimeterRegionView::from_ir(region);
                        view.set_config((*module.config_view).clone());
                        view
                    })
                    .collect()
            })
            .unwrap_or_default(),
    );

    let mut paint = input
        .paint_regions
        .as_ref()
        .map(|_| PaintRegionLayerView::with_paint_regions(layer_index, std::sync::Arc::new(())))
        .unwrap_or_else(|| PaintRegionLayerView::new(layer_index));
    if let Some(ir) = input.lightning_tree_ir.as_ref() {
        paint = paint.with_lightning_tree_ir(std::sync::Arc::clone(ir));
    }
    // Mirror the wasm leg's paint-view construction. `dispatch_layer_call`
    // indexes the committed `SupportPlanIR` into `PaintRegionLayerData` via
    // `build_paint_layer_data_with_plan` for exactly two stages (`Layer::Infill`
    // and `Layer::Support`); the guest shim then rebuilds an SDK
    // `PaintRegionLayerView` with `with_support_plan`. Until this call the
    // native leg handed the module a plan-less view, so any renderer that keys
    // off `support_plan_entries_for` (traditional-support and
    // tree-support-family, since packet 222 removed their plan-less fallback)
    // emitted nothing and `commit_native_layer_response` returned `Ok(None)`.
    if matches!(stage_export, "Layer::Infill" | "Layer::Support") {
        paint = paint.with_support_plan(
            input
                .support_plan
                .as_ref()
                .map(std::sync::Arc::clone)
                .unwrap_or_default(),
        );
    }
    // `build_layer_support_glue` (crates/slicer-macros) attaches a SliceIR
    // rebuilt from the region views on the support stage only, so that
    // `paint_policy_for` can surface enforcer/blocker annotations. Same scope
    // here: attaching it unconditionally would give native stages a view the
    // wasm leg does not have.
    if stage_export == "Layer::Support" {
        if let Some(slice) = input.slice {
            paint = paint.with_slice_ir(std::sync::Arc::new(slice.clone()));
        }
    }

    NativeLayerRequest {
        layer_index,
        regions,
        perimeter_regions,
        paint: Some(paint),
        prior_infill: input.infill.map(|infill| infill.regions.clone()),
        config: (*module.config_view).clone(),
        stage_export,
    }
}

/// Build the unified native prepass envelope.  The selected stage consumes only
/// the corresponding optional field, matching the SDK envelope contract.
pub fn build_native_prepass_request(
    stage_export: &'static str,
    input: &PrepassStageInput<'_>,
    module: &CompiledModuleLive<'_>,
) -> NativePrepassRequest {
    let mesh_objects: Vec<MeshObjectView> = input
        .mesh
        .objects
        .iter()
        .map(|mesh| MeshObjectView {
            object_id: mesh.id.clone(),
            vertices: mesh.mesh.vertices.iter().map(|v| [v.x, v.y, v.z]).collect(),
            triangles: mesh
                .mesh
                .indices
                .chunks_exact(3)
                .map(|v| [v[0], v[1], v[2]])
                .collect(),
            paint_layers: mesh
                .paint_data
                .as_ref()
                .map(native_paint_layers)
                .unwrap_or_default(),
        })
        .collect();
    let paint_objects = input
        .mesh
        .objects
        .iter()
        .map(
            |mesh| slicer_sdk::prepass_types::PaintSegmentationObjectView {
                object_id: mesh.id.clone(),
                vertices: mesh.mesh.vertices.iter().map(|v| [v.x, v.y, v.z]).collect(),
                triangles: mesh
                    .mesh
                    .indices
                    .chunks_exact(3)
                    .map(|v| [v[0], v[1], v[2]])
                    .collect(),
                paint_layers: mesh
                    .paint_data
                    .as_ref()
                    .map(native_paint_layers)
                    .unwrap_or_default(),
                transform_matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                participating_layer_indices: input
                    .layer_plan
                    .as_deref()
                    .and_then(|plan| plan.object_participation.get(&mesh.id))
                    .map(|refs| {
                        refs.iter()
                            .map(|reference| reference.global_layer_index)
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        )
        .collect();
    let layer_plan = input.layer_plan.as_deref().map(|plan| LayerPlanView {
        layers: plan
            .global_layers
            .iter()
            .map(|layer| slicer_sdk::prepass_types::LayerPlanViewEntry {
                global_layer_index: layer.index,
                z: layer.z,
                effective_layer_height: canonical_effective_layer_height(plan, layer.index),
            })
            .collect(),
    });
    let region_segmentation = input.region_map.as_deref().map(|map| {
        let mut entries = std::collections::BTreeMap::<(u32, String), Vec<String>>::new();
        for key in map.entries.keys() {
            entries
                .entry((key.global_layer_index, key.object_id.clone()))
                .or_default()
                .push(key.region_id.to_string());
        }
        RegionSegmentationView {
            entries: entries
                .into_iter()
                .map(|((layer_index, object_id), region_ids)| {
                    slicer_sdk::prepass_types::RegionSegmentationViewEntry {
                        object_id,
                        layer_index,
                        region_ids,
                    }
                })
                .collect(),
            region_support_configs: map
                .entries
                .iter()
                .map(|(key, plan)| {
                    let config = map.config_for_raw(plan.config).to_config_map();
                    let string_value = |name: &str| match config.get(name) {
                        Some(slicer_ir::ConfigValue::String(value)) => Some(value.clone()),
                        _ => None,
                    };
                    slicer_sdk::prepass_types::RegionSupportConfig {
                        object_id: key.object_id.clone(),
                        layer_index: key.global_layer_index,
                        region_id: key.region_id.to_string(),
                        support_family: string_value("support_family"),
                        support_type: string_value("support_type"),
                    }
                })
                .collect(),
        }
    });
    let support_geometry = input
        .support_geometry
        .as_deref()
        .map(|geometry| SupportGeometryView {
            entries: geometry
                .entries
                .iter()
                .map(
                    |(key, polygons)| slicer_sdk::prepass_types::SupportGeometryViewEntry {
                        global_support_layer_index: key.global_support_layer_index,
                        object_id: key.object_id.clone(),
                        region_id: key.region_id.to_string(),
                        outlines: polygons.clone(),
                    },
                )
                .collect(),
        });
    let support_analysis = input.support_analysis.as_deref().map(|analysis| {
        let mut candidates: Vec<_> = analysis
            .candidates
            .iter()
            .map(
                |candidate| slicer_sdk::prepass_types::SupportAnalysisCandidate {
                    id: candidate.id,
                    geometry: candidate.geometry.clone(),
                    object_id: candidate.source.object_id.clone(),
                    region_id: candidate.source.region_id.to_string(),
                    global_layer_index: candidate.source.global_layer_index,
                    z_units: candidate.source.z_units,
                    enforced: candidate.enforced,
                    blocked: candidate.blocked,
                },
            )
            .collect();
        candidates.sort_by_key(|candidate| {
            (
                candidate.global_layer_index,
                candidate.object_id.clone(),
                candidate.region_id.clone(),
                candidate.id,
            )
        });
        let project_geometry = |entries: &std::collections::HashMap<
            slicer_ir::SupportGeometryKey,
            Vec<slicer_ir::ExPolygon>,
        >| {
            let mut entries: Vec<_> = entries
                .iter()
                .map(
                    |(key, polygons)| slicer_sdk::prepass_types::SupportAnalysisGeometryEntry {
                        global_support_layer_index: key.global_support_layer_index,
                        object_id: key.object_id.clone(),
                        region_id: key.region_id.to_string(),
                        polygons: polygons.clone(),
                    },
                )
                .collect();
            entries.sort_by_key(|entry| {
                (
                    entry.global_support_layer_index,
                    entry.object_id.clone(),
                    entry.region_id.clone(),
                )
            });
            entries
        };
        SupportAnalysisView {
            candidates,
            model_occupancy: project_geometry(&analysis.model_occupancy),
            termination_surfaces: project_geometry(&analysis.termination_surfaces),
            shared_settings: analysis
                .shared_settings
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            baseline_feasible_envelope: analysis.baseline_feasible_envelope.clone(),
            family_assignments: analysis
                .family_assignments
                .iter()
                .map(|((object_id, region_id), family_id)| {
                    slicer_sdk::prepass_types::SupportFamilyAssignment {
                        object_id: object_id.clone(),
                        region_id: region_id.to_string(),
                        family_id: family_id.clone(),
                    }
                })
                .collect(),
        }
    });
    let seam_regions =
        input
            .slice_ir
            .as_deref()
            .map(|slices| slicer_sdk::prepass_types::SeamPlanningView {
                regions: slices
                    .iter()
                    .flat_map(|slice| {
                        slice.regions.iter().map(|region| {
                            slicer_sdk::prepass_types::SeamPlanningRegionInput {
                                global_layer_index: slice.global_layer_index,
                                object_id: region.object_id.clone(),
                                region_id: region.region_id.to_string(),
                                variant_chain: region.variant_chain.clone(),
                                z: slice.z,
                                height: region.effective_layer_height,
                                ex_polygons: region.polygons.clone(),
                                segment_annotations: region
                                    .segment_annotations
                                    .iter()
                                    .map(|(semantic, polygons)| {
                                        (semantic.clone(), polygons.clone())
                                    })
                                    .collect(),
                                scoring_width: 0.4,
                            }
                        })
                    })
                    .collect(),
            });
    NativePrepassRequest {
        mesh_objects: Some(mesh_objects),
        object_ids: Some(
            input
                .mesh
                .objects
                .iter()
                .map(|mesh| mesh.id.clone())
                .collect(),
        ),
        layer_plan,
        region_segmentation,
        support_analysis,
        support_geometry,
        // Unlike the mesh-analysis view, paint segmentation needs the richer
        // object view: facet paint, identity transform, and layer participation.
        paint_objects: Some(paint_objects),
        seam_regions,
        config: (*module.config_view).clone(),
        stage_export,
    }
}

/// Commit a native prepass response using the same stage-selected IR outputs as WASM.
pub fn commit_native_prepass_response(
    response: &NativePrepassResponse,
    stage_export: &str,
) -> Result<slicer_core::PrepassStageOutput, String> {
    commit_native_prepass_response_with_inputs(response, stage_export, None, None, None)
}

/// Commit a native prepass response while retaining the per-region config from
/// the already committed plan or region map. Native layer proposals do not
/// carry this host-only field themselves.
pub fn commit_native_prepass_response_with_inputs(
    response: &NativePrepassResponse,
    stage_export: &str,
    input_layer_plan: Option<&slicer_ir::LayerPlanIR>,
    region_map: Option<&slicer_ir::RegionMapIR>,
    module_config: Option<&slicer_ir::ConfigView>,
) -> Result<slicer_core::PrepassStageOutput, String> {
    use std::sync::Arc;
    match stage_export {
        "PrePass::LayerPlanning" => {
            let Some(output) = response.layer_plan.as_ref() else {
                return Err(
                    "native prepass response missing layer-plan output for stage PrePass::LayerPlanning"
                        .to_string(),
                );
            };
            let mut global_layers = Vec::new();
            let mut participation: std::collections::HashMap<
                String,
                Vec<slicer_ir::ObjectLayerRef>,
            > = std::collections::HashMap::new();
            for (index, proposal) in output.layers().iter().enumerate() {
                let active_regions: Result<Vec<_>, String> = proposal
                    .active_regions
                    .iter()
                    .map(|region| {
                        let region_id = region
                            .region_id
                            .parse()
                            .map_err(|e| format!("invalid region id: {e}"))?;
                        let resolved_config = input_layer_plan
                            .and_then(|plan| plan.global_layers.get(index))
                            .and_then(|layer| {
                                layer.active_regions.iter().find(|candidate| {
                                    candidate.object_id == region.object_id
                                        && candidate.region_id == region_id
                                })
                            })
                            .map(|candidate| candidate.resolved_config.clone())
                            .or_else(|| {
                                region_map.and_then(|map| {
                                    map.entries.iter().find_map(|(key, _)| {
                                        (key.global_layer_index == index as u32
                                            && key.object_id == region.object_id
                                            && key.region_id == region_id)
                                            .then(|| map.config_for(key).clone())
                                    })
                                })
                            })
                            .unwrap_or_else(|| {
                                let mut config = slicer_ir::ResolvedConfig::default();
                                if let Some(slicer_ir::ConfigValue::String(value)) =
                                    module_config.and_then(|view| view.get("support_type"))
                                {
                                    config.extensions.insert(
                                        "support_type".to_string(),
                                        slicer_ir::ConfigValue::String(value.clone()),
                                    );
                                }
                                config
                            });
                        Ok(slicer_ir::ActiveRegion {
                            object_id: region.object_id.clone(),
                            region_id,
                            resolved_config,
                            effective_layer_height: region.effective_layer_height,
                            is_catchup_layer: region.is_catchup,
                            catchup_z_bottom: region.catchup_z_bottom,
                            ..Default::default()
                        })
                    })
                    .collect();
                let active_regions = active_regions?;
                for region in &proposal.active_regions {
                    let obj_refs = participation.entry(region.object_id.clone()).or_default();
                    let already_referenced = obj_refs
                        .iter()
                        .any(|r| r.global_layer_index == index as u32);
                    if !already_referenced {
                        obj_refs.push(slicer_ir::ObjectLayerRef {
                            local_layer_index: obj_refs.len() as u32,
                            global_layer_index: index as u32,
                            effective_layer_height: region.effective_layer_height,
                        });
                    }
                }
                global_layers.push(slicer_ir::GlobalLayer {
                    index: index as u32,
                    z: proposal.z,
                    active_regions,
                    ..Default::default()
                });
            }
            Ok(slicer_core::PrepassStageOutput::LayerPlan(Arc::new(
                slicer_ir::LayerPlanIR {
                    global_layers,
                    object_participation: participation,
                    ..Default::default()
                },
            )))
        }
        "PrePass::SeamPlanning" => {
            let Some(output) = response.seam_planning.as_ref() else {
                return Err(
                    "native prepass response missing seam-planning output for stage PrePass::SeamPlanning"
                        .to_string(),
                );
            };
            Ok(slicer_core::PrepassStageOutput::SeamPlan(Arc::new(
                slicer_ir::SeamPlanIR {
                    entries: output
                        .entries()
                        .iter()
                        .map(|entry| -> Result<_, String> {
                            Ok(slicer_ir::SeamPlanEntry {
                                region_key: slicer_ir::RegionKey {
                                    global_layer_index: entry.global_layer_index,
                                    object_id: entry.object_id.clone(),
                                    region_id: entry.region_id.parse().map_err(|e| {
                                        format!("invalid seam region id '{}': {e}", entry.region_id)
                                    })?,
                                    variant_chain: entry.variant_chain.clone(),
                                },
                                chosen_candidate: slicer_ir::SeamPosition {
                                    point: entry.chosen_position,
                                    wall_index: entry.chosen_wall_index,
                                },
                                scored_candidates: entry
                                    .scored_candidates
                                    .iter()
                                    .map(|candidate| slicer_ir::ScoredSeamCandidate {
                                        position: candidate.position,
                                        score: candidate.score,
                                        reason: match candidate.reason.tag.as_str() {
                                            "concave" => slicer_ir::SeamReason::Concave,
                                            "sharp" => slicer_ir::SeamReason::Sharp,
                                            "user_forced" => slicer_ir::SeamReason::UserForced,
                                            _ => slicer_ir::SeamReason::Aligned,
                                        },
                                    })
                                    .collect(),
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                    ..Default::default()
                },
            )))
        }
        "PrePass::SupportGeometry" => {
            let Some(output) = response.support_geometry.as_ref() else {
                return Err(
                    "native prepass response missing support-geometry output for stage PrePass::SupportGeometry"
                        .to_string(),
                );
            };
            Ok(slicer_core::PrepassStageOutput::SupportPlan(Arc::new(
                slicer_ir::SupportPlanIR {
                    entries: output
                        .entries()
                        .iter()
                        .map(|entry| -> Result<_, String> {
                            if let Some(skeleton) = entry.skeleton.as_ref() {
                                if skeleton.wall_counts.len() != skeleton.points.len() {
                                    return Err(format!(
                                        "native support-generation-output: skeleton wall_counts length {} does not match points length {}",
                                        skeleton.wall_counts.len(),
                                        skeleton.points.len()
                                    ));
                                }
                            }
                            Ok(slicer_ir::SupportPlanEntry {
                                global_layer_index: entry.global_layer_index,
                                object_id: entry.object_id.clone(),
                                region_id: entry.region_id.parse().map_err(|e| {
                                    format!("invalid support region id '{}': {e}", entry.region_id)
                                })?,
                                family_id: entry.family_id.clone(),
                                demand_ids: entry.demand_ids.clone(),
                                body_ids: entry.body_ids.clone(),
                                anchor_layer_index: entry.anchor_layer_index,
                                anchor_z: entry.anchor_z,
                                roles: native_support_plan_roles(&entry.roles),
                                skeleton: entry.skeleton.clone(),
                                capabilities: entry.capabilities.clone(),
                                provenance: entry.provenance.clone(),
                                decline_reason: entry.decline_reason,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                    raft_plan: output.raft_plan().cloned().map(|plan| slicer_ir::RaftPlan {
                        raft_layers: plan.raft_layers,
                        raft_first_layer_density: plan.raft_first_layer_density,
                        base_raft_layers: plan.base_raft_layers,
                        interface_raft_layers: plan.interface_raft_layers,
                    }),
                    ..Default::default()
                },
            )))
        }
        "PrePass::MeshAnalysis" => {
            let Some(output) = response.mesh_analysis.as_ref() else {
                return Err(
                    "native prepass response missing mesh-analysis output for stage PrePass::MeshAnalysis"
                        .to_string(),
                );
            };
            Ok(slicer_core::PrepassStageOutput::MeshAnalysisAuxiliary(
                Arc::new(slicer_core::MeshAnalysisAuxiliary {
                    facet_annotations: output
                        .facet_annotations()
                        .iter()
                        .map(|(id, annotation)| {
                            (
                                id.clone(),
                                slicer_core::FacetAnnotationRecord {
                                    facet_index: annotation.facet_index,
                                    slope_angle_deg: annotation.slope_angle_deg,
                                    classification: match annotation.classification {
                                        slicer_sdk::prepass_types::FacetClass::Normal => {
                                            slicer_core::FacetClassRecord::Normal
                                        }
                                        slicer_sdk::prepass_types::FacetClass::NearHorizontal => {
                                            slicer_core::FacetClassRecord::NearHorizontal
                                        }
                                        slicer_sdk::prepass_types::FacetClass::Overhang => {
                                            slicer_core::FacetClassRecord::Overhang
                                        }
                                        slicer_sdk::prepass_types::FacetClass::Bridge => {
                                            slicer_core::FacetClassRecord::Bridge
                                        }
                                        slicer_sdk::prepass_types::FacetClass::TopSurface => {
                                            slicer_core::FacetClassRecord::TopSurface
                                        }
                                        slicer_sdk::prepass_types::FacetClass::BottomSurface => {
                                            slicer_core::FacetClassRecord::BottomSurface
                                        }
                                    },
                                },
                            )
                        })
                        .collect(),
                    surface_groups: output
                        .surface_groups()
                        .iter()
                        .map(|(id, group)| {
                            (
                                id.clone(),
                                slicer_core::SurfaceGroupRecord {
                                    facet_indices: group.facet_indices.clone(),
                                    z_min: group.z_min,
                                    z_max: group.z_max,
                                    shell_count: group.shell_count,
                                },
                            )
                        })
                        .collect(),
                }),
            ))
        }
        "PrePass::PaintSegmentation" => Ok(slicer_core::PrepassStageOutput::None),
        _ => Err(format!(
            "native prepass response has no output commit for stage {stage_export}"
        )),
    }
}

/// Build a native postpass request from the stage payload.
pub fn build_native_postpass_request(
    stage_export: &'static str,
    input: &PostpassStageInput<'_>,
    module: &CompiledModuleLive<'_>,
    payload: NativePostpassInput,
) -> NativePostpassRequest {
    let _ = input;
    NativePostpassRequest {
        input: payload,
        config: (*module.config_view).clone(),
        stage_export,
    }
}

/// Commit native postpass output; text bypasses the gcode accumulator just like WASM.
pub fn commit_native_postpass_response(
    response: NativePostpassResponse,
    commands: Option<&mut Vec<slicer_ir::GCodeCommand>>,
) -> Result<slicer_ir::PostpassOutput, String> {
    match response {
        NativePostpassResponse::Text(text) => Ok(slicer_ir::PostpassOutput::TextSuccess { text }),
        NativePostpassResponse::Gcode(output_commands) => {
            let collected = output_commands
                .into_iter()
                .map(|command| Ok::<_, String>(match command {
                    GcodeOutputCommand::Command(command) => match command {
                        slicer_ir::GCodeCommand::Move {
                            x,
                            y,
                            z,
                            e,
                            f,
                            role,
                        } => GcodeCommandCollected::Move(crate::host::GcodeMoveCmd {
                            x,
                            y,
                            z,
                            e,
                            f,
                            role: ir_to_wit_extrusion_role(&role),
                        }),
                        slicer_ir::GCodeCommand::Retract {
                            length,
                            speed,
                            mode,
                        } => GcodeCommandCollected::Retract {
                            length,
                            speed,
                            mode,
                        },
                        slicer_ir::GCodeCommand::Unretract {
                            length,
                            speed,
                            mode,
                        } => GcodeCommandCollected::Unretract {
                            length,
                            speed,
                            mode,
                        },
                        slicer_ir::GCodeCommand::FanSpeed { value } => {
                            GcodeCommandCollected::FanSpeed(value)
                        }
                        slicer_ir::GCodeCommand::Temperature {
                            tool,
                            celsius,
                            wait,
                        } => GcodeCommandCollected::Temperature {
                            tool,
                            celsius,
                            wait,
                        },
                        slicer_ir::GCodeCommand::ToolChange {
                            after_entity_index,
                            from,
                            to,
                        } => GcodeCommandCollected::ToolChange {
                            after_entity_index,
                            from_tool: from,
                            to_tool: to,
                        },
                        slicer_ir::GCodeCommand::Comment { text } => {
                            GcodeCommandCollected::Comment(text)
                        }
                        slicer_ir::GCodeCommand::Raw { text } => GcodeCommandCollected::Raw(text),
                        slicer_ir::GCodeCommand::ExtrusionMode { .. } => {
                            return Err("postpass gcode output extrusion-mode has no collected-command variant".to_string());
                        }
                    },
                    GcodeOutputCommand::ZHop {
                        after_entity_index,
                        hop_height,
                    } => GcodeCommandCollected::ZHop {
                        after_entity_index,
                        hop_height,
                    },
                }))
                .collect::<Result<Vec<_>, String>>()?;
            match crate::marshal::collect_postpass_output(&collected)? {
                None => Ok(slicer_ir::PostpassOutput::GCodeSuccess),
                Some(new_commands) => {
                    if let Some(commands) = commands {
                        *commands = new_commands;
                        Ok(slicer_ir::PostpassOutput::GCodeSuccess)
                    } else {
                        Err(
                            "native gcode postpass emitted commands without a gcode accumulator"
                                .to_string(),
                        )
                    }
                }
            }
        }
    }
}

/// Build a native finalization request from completed layers.
pub fn build_native_finalization_request(
    stage_export: &'static str,
    input: &FinalizationStageInput<'_>,
    module: &CompiledModuleLive<'_>,
    layers: &[slicer_ir::LayerCollectionIR],
) -> NativeFinalizationRequest {
    let _ = input;
    NativeFinalizationRequest {
        layers: layers
            .iter()
            .cloned()
            .map(LayerCollectionView::new)
            .collect(),
        config: (*module.config_view).clone(),
        stage_export,
    }
}

/// Commit finalization through the SDK builder's full merge applier.
pub fn commit_native_finalization_response(
    response: NativeFinalizationResponse,
    layers: &mut Vec<slicer_ir::LayerCollectionIR>,
) -> Result<slicer_ir::FinalizationOutput, String> {
    response
        .output
        .apply_to(layers)
        .map_err(|message| format!("finalization merge failed: {message}"))?;
    Ok(slicer_ir::FinalizationOutput::Success)
}

fn collect_infill(builder: &InfillOutputBuilder) -> InfillOutputCollected {
    InfillOutputCollected {
        sparse_paths: builder
            .sparse_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        solid_paths: builder
            .solid_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        ironing_paths: builder
            .ironing_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        sparse_path_origins: builder.sparse_path_origins().iter().map(origin).collect(),
        solid_path_origins: builder.solid_path_origins().iter().map(origin).collect(),
        ironing_path_origins: builder.ironing_path_origins().iter().map(origin).collect(),
    }
}

fn collect_perimeter(builder: &PerimeterOutputBuilder) -> PerimeterOutputCollected {
    PerimeterOutputCollected {
        wall_loops: builder
            .wall_loops()
            .iter()
            .map(ir_to_wit_wall_loop)
            .collect(),
        rotated_wall_loops: builder
            .rotated_wall_loops()
            .iter()
            .map(|(_, _, wall)| ir_to_wit_wall_loop(wall))
            .collect(),
        rotated_wall_loop_origins: builder
            .rotated_wall_loop_origins()
            .iter()
            .map(origin)
            .collect(),
        wall_loop_origins: builder.wall_loop_origins().iter().map(origin).collect(),
        infill_areas: builder
            .infill_areas()
            .iter()
            .map(|areas| ir_to_wit_expolygons(areas))
            .collect(),
        infill_areas_origins: builder.infill_areas_origins().iter().map(origin).collect(),
        seam_candidates: builder
            .seam_candidates()
            .iter()
            .map(|(point, score)| {
                (
                    crate::host::Point3 {
                        x: point.x,
                        y: point.y,
                        z: point.z,
                    },
                    *score,
                )
            })
            .collect(),
        seam_candidate_origins: builder
            .seam_candidate_origins()
            .iter()
            .map(origin)
            .collect(),
        resolved_seam: builder.resolved_seam().map(|seam| {
            (
                crate::host::Point3 {
                    x: seam.point.x,
                    y: seam.point.y,
                    z: seam.point.z,
                },
                seam.wall_index,
            )
        }),
        resolved_seam_origin: builder.resolved_seam_origin().map(|origin| OriginId {
            object_id: origin.object_id.clone(),
            region_id: origin.region_id,
        }),
    }
}

fn collect_support(builder: &SupportOutputBuilder) -> SupportOutputCollected {
    SupportOutputCollected {
        support_paths: builder
            .support_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        interface_paths: builder
            .interface_paths()
            .iter()
            .map(|(path, top)| (ir_to_wit_extrusion_path(path), *top))
            .collect(),
        raft_paths: builder
            .raft_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        support_path_origins: builder.support_path_origins().iter().map(origin).collect(),
        interface_path_origins: builder
            .interface_path_origins()
            .iter()
            .map(origin)
            .collect(),
        raft_path_origins: builder.raft_path_origins().iter().map(origin).collect(),
    }
}

/// Commit a native layer response through the existing output converters.
///
/// `support_plan` is the committed `SupportPlanIR` for this slice, threaded in
/// from `dispatch_layer_call`'s `input.support_plan`. It exists so the native
/// path consumes the **same** plan the wasm path does: `deconstruct_layer_ctx`
/// forwards it to `convert_support_output_with_plan`, and until packet 224 the
/// native branch passed `None` there, silently discarding plan-derived
/// identity (family_id / body_id / demand_ids / object_id) on every
/// native-dispatched support stage.
pub fn commit_native_layer_response(
    response: &NativeLayerResponse,
    stage_export: &str,
    layer_index: u32,
    support_plan: Option<&slicer_ir::SupportPlanIR>,
) -> Result<Option<slicer_ir::LayerStageCommit>, String> {
    use slicer_ir::LayerStageCommit;
    match stage_export {
        "Layer::Infill" | "Layer::InfillPostProcess" => {
            let Some(builder) = response.infill.as_ref() else {
                return Ok(None);
            };
            let collected = collect_infill(builder);
            if collected.sparse_paths.is_empty()
                && collected.solid_paths.is_empty()
                && collected.ironing_paths.is_empty()
            {
                return Ok(None);
            }
            // Native modules author no per-path tool today; pass `None` so the
            // authored-coloring grant defaults to deny at this boundary too.
            let ir = convert_infill_output(&collected, layer_index, None)?;
            Ok(Some(if stage_export.ends_with("PostProcess") {
                LayerStageCommit::InfillPostProcess(ir)
            } else {
                LayerStageCommit::Infill(ir)
            }))
        }
        "Layer::Support" => {
            let Some(support) = response.support.as_ref() else {
                return Ok(None);
            };
            let collected = collect_support(&support.output);
            if collected.support_paths.is_empty()
                && collected.interface_paths.is_empty()
                && collected.raft_paths.is_empty()
            {
                let Some(collection) = support.collection.anchored_proposal() else {
                    return Ok(None);
                };
                for entity in &collection.events {
                    crate::marshal::validate_anchored_entity_geometry(entity)?;
                }
                return Ok(Some(LayerStageCommit::AnchoredEvents(vec![
                    collection.clone()
                ])));
            }
            let ir = convert_support_output_with_plan(&collected, layer_index, support_plan)?;
            Ok(Some(LayerStageCommit::Support(ir)))
        }
        "Layer::SupportPostProcess" => {
            let Some(builder) = response.support.as_ref() else {
                return Ok(None);
            };
            let collected = collect_support(&builder.output);
            if collected.support_paths.is_empty()
                && collected.interface_paths.is_empty()
                && collected.raft_paths.is_empty()
            {
                return Ok(None);
            }
            let ir = convert_support_output_with_plan(&collected, layer_index, support_plan)?;
            Ok(Some(LayerStageCommit::SupportPostProcess(ir)))
        }
        "Layer::AnchoredEvents" => {
            let Some(builder) = response.anchored_events.as_ref() else {
                return Ok(None);
            };
            let Some(collection) = builder.anchored_proposal() else {
                return Ok(None);
            };
            for entity in &collection.events {
                crate::marshal::validate_anchored_entity_geometry(entity)?;
            }
            Ok(Some(LayerStageCommit::AnchoredEvents(vec![
                collection.clone()
            ])))
        }
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => {
            let Some(builder) = response.perimeters.as_ref() else {
                return Ok(None);
            };
            let collected = collect_perimeter(builder);
            let any = !collected.wall_loops.is_empty()
                || !collected.rotated_wall_loops.is_empty()
                || !collected.infill_areas.is_empty()
                || !collected.seam_candidates.is_empty();
            let ir = if any {
                Some(convert_perimeter_output(&collected, layer_index)?)
            } else {
                None
            };
            if stage_export.ends_with("PostProcess") {
                Ok(Some(LayerStageCommit::PerimetersPostProcess(ir)))
            } else {
                Ok(ir.map(|value| LayerStageCommit::Perimeters(value)))
            }
        }
        "Layer::SlicePostProcess" => {
            let Some(builder) = response.slice_postprocess.as_ref() else {
                return Ok(None);
            };
            if builder.polygon_updates().is_empty() && builder.path_z_updates().is_empty() {
                return Ok(None);
            }
            let polygon_updates = builder
                .polygon_updates()
                .iter()
                .map(|(key, polys)| {
                    let region_id = key.region_id;
                    Ok((
                        slicer_ir::RegionKey {
                            global_layer_index: layer_index,
                            object_id: key.object_id.clone(),
                            region_id,
                            variant_chain: Vec::new(),
                        },
                        polys.clone(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let path_z_updates = builder
                .path_z_updates()
                .iter()
                .map(|(key, path_idx, vertex_idx, z)| {
                    let region_id = key.region_id;
                    Ok((
                        slicer_ir::RegionKey {
                            global_layer_index: layer_index,
                            object_id: key.object_id.clone(),
                            region_id,
                            variant_chain: Vec::new(),
                        },
                        *path_idx,
                        *vertex_idx,
                        *z,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Some(LayerStageCommit::SlicePostProcess {
                polygon_updates,
                path_z_updates,
            }))
        }
        "Layer::PathOptimization" => {
            let Some(path) = response.path_optimization.as_ref() else {
                return Ok(None);
            };
            let mut commit = slicer_ir::PathOptimizationCommit::default();
            for (i, command) in path.output.commands().iter().enumerate() {
                match command {
                    GcodeOutputCommand::Command(command) => match command {
                        slicer_ir::GCodeCommand::ToolChange { after_entity_index, from, to } => {
                            commit.tool_changes.push(slicer_ir::ToolChange {
                                after_entity_index: *after_entity_index,
                                from_tool: *from,
                                to_tool: *to,
                            });
                        }
                        slicer_ir::GCodeCommand::Comment { text } => commit.annotations.push(
                            slicer_ir::LayerAnnotationKind::Comment(text.clone()),
                        ),
                        slicer_ir::GCodeCommand::Raw { text } => commit
                            .annotations
                            .push(slicer_ir::LayerAnnotationKind::Raw(text.clone())),
                        slicer_ir::GCodeCommand::Move { x, y, z, f, .. } => {
                            commit.travel_moves.push(slicer_ir::TravelMoveDest {
                                x: *x, y: *y, z: *z, f: *f,
                            });
                        }
                        slicer_ir::GCodeCommand::Retract { length, speed, mode } => {
                            commit.retracts.push(slicer_ir::RetractSpec {
                                length: *length, speed: *speed, is_unretract: false, mode: *mode,
                            });
                        }
                        slicer_ir::GCodeCommand::Unretract { length, speed, mode } => {
                            commit.retracts.push(slicer_ir::RetractSpec {
                                length: *length, speed: *speed, is_unretract: true, mode: *mode,
                            });
                        }
                        slicer_ir::GCodeCommand::FanSpeed { .. } => return Err(format!(
                            "native Layer::PathOptimization emitted unsupported GCode command FanSpeed at index {i}"
                        )),
                        slicer_ir::GCodeCommand::Temperature { .. } => return Err(format!(
                            "native Layer::PathOptimization emitted unsupported GCode command Temperature at index {i}"
                        )),
                        slicer_ir::GCodeCommand::ExtrusionMode { .. } => return Err(format!(
                            "native Layer::PathOptimization emitted unsupported GCode command ExtrusionMode at index {i}"
                        )),
                    },
                    GcodeOutputCommand::ZHop { hop_height, .. } => {
                        if !hop_height.is_finite() || *hop_height <= 0.0 {
                            return Err(format!(
                                "native Layer::PathOptimization push-z-hop at index {i} rejected: hop-height={hop_height} is not finite and strictly positive"
                            ));
                        }
                        commit.z_hops.push(*hop_height);
                    }
                }
            }
            commit.order_proposal = path.collection.proposal().map(|proposal| proposal.to_vec());
            Ok(Some(LayerStageCommit::PathOptimization(commit)))
        }
        _ => Err(format!(
            "native layer response has no output commit for stage {stage_export}"
        )),
    }
}
