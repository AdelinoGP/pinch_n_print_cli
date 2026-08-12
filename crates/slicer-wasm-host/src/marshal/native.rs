//! Native SDK transport for the marshalling boundary.
//!
//! This module deliberately feeds the same collected-output accumulators used
//! by the WASM transport.  Consequently validation, origin bucketing, and IR
//! conversion remain single-sourced in `out.rs`.

#![cfg(not(target_arch = "wasm32"))]

use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder, SupportOutputBuilder};
use slicer_sdk::native::{
    NativeFinalizationRequest, NativeFinalizationResponse, NativeLayerRequest, NativeLayerResponse,
    NativePostpassInput, NativePostpassRequest, NativePostpassResponse, NativePrepassRequest,
    NativePrepassResponse,
};
use slicer_sdk::postpass_types::GcodeOutputCommand;
use slicer_sdk::prepass_types::{
    LayerPlanView, MeshObjectView, RegionSegmentationView, SupportGeometryView,
};
use slicer_sdk::traits::LayerCollectionView;
use slicer_sdk::traits::PaintRegionLayerView;
use slicer_sdk::views::{PerimeterRegionView, SliceRegionView};

use crate::binding::{
    CompiledModuleLive, FinalizationStageInput, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};
use crate::marshal::{
    convert_infill_output, convert_perimeter_output, convert_support_output, ir_to_wit_expolygons,
    ir_to_wit_extrusion_path, ir_to_wit_extrusion_role, ir_to_wit_wall_loop, GcodeCommandCollected,
    InfillOutputCollected, OriginId, PerimeterOutputCollected, SupportOutputCollected,
};

fn origin(value: &Option<(String, u64)>) -> Option<OriginId> {
    value.as_ref().map(|(object_id, region_id)| OriginId {
        object_id: object_id.clone(),
        region_id: *region_id,
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

/// Build a native layer request without passing any wasm-host type across the
/// SDK boundary.
pub fn build_native_layer_request(
    stage_export: &'static str,
    layer_index: u32,
    input: &LayerStageInput<'_>,
    module: &CompiledModuleLive<'_>,
) -> NativeLayerRequest {
    // Completeness mirror (the wasm leg is `sliced_region_to_data`):
    // | SDK field                 | IR source                         | wasm backer       |
    // | object_id                 | SlicedRegion.object_id            | object-id         |
    // | region_id                 | SlicedRegion.region_id            | region-id         |
    // | polygons                  | SlicedRegion.polygons             | polygons          |
    // | infill_areas              | SlicedRegion.infill_areas         | infill-areas      |
    // | nonplanar_surface         | is_some()                          | has-nonplanar     |
    // | effective_layer_height    | SlicedRegion.effective_layer_height| effective-height  |
    // | segment_annotations       | SlicedRegion.segment_annotations  | segment-annotations|
    // | top_shell_index           | SlicedRegion.top_shell_index      | top-shell-index   |
    // | bottom_shell_index        | SlicedRegion.bottom_shell_index   | bottom-shell-index|
    // | top_solid_fill            | SlicedRegion.top_solid_fill        | top-solid-fill    |
    // | bottom_solid_fill         | SlicedRegion.bottom_solid_fill     | bottom-solid-fill |
    // | is_bridge                 | SlicedRegion.is_bridge             | is-bridge         |
    // | bridge_areas              | SlicedRegion.bridge_areas           | bridge-areas      |
    // | bridge_orientation_deg    | SlicedRegion.bridge_orientation_deg | bridge-orientation|
    // | sparse_infill_area        | SlicedRegion.sparse_infill_area     | sparse-infill-area|
    // | held_claims               | CompiledModuleLive.claims           | held-claims       |
    // | z                         | SliceIR.z                           | z                 |
    let regions = input
        .slice
        .map(|slice| {
            slice
                .regions
                .iter()
                .map(|region| {
                    let mut view = SliceRegionView::default();
                    view.set_object_id(region.object_id.clone());
                    view.set_region_id(region.region_id);
                    view.set_polygons(region.polygons.clone());
                    view.set_infill_areas(region.infill_areas.clone());
                    view.set_effective_layer_height(region.effective_layer_height);
                    view.set_z(slice.z);
                    view.set_has_nonplanar(region.nonplanar_surface.is_some());
                    view.set_segment_annotations(region.segment_annotations.clone());
                    view.set_top_shell_index(region.top_shell_index);
                    view.set_bottom_shell_index(region.bottom_shell_index);
                    view.set_top_solid_fill(region.top_solid_fill.clone());
                    view.set_bottom_solid_fill(region.bottom_solid_fill.clone());
                    view.set_is_bridge(region.is_bridge);
                    view.set_bridge_areas(region.bridge_areas.clone());
                    view.set_bridge_orientation_deg(region.bridge_orientation_deg);
                    view.set_sparse_infill_area(region.sparse_infill_area.clone());
                    view.set_variant_chain(region.variant_chain.clone());
                    view.set_held_claims(module.claims.to_vec());
                    view.set_config((*module.config_view).clone());
                    view
                })
                .collect()
        })
        .unwrap_or_default();

    let perimeter_regions = input.perimeter.map(|perimeter| {
        perimeter
            .regions
            .iter()
            .map(|region| {
                let mut view = PerimeterRegionView::default();
                view.set_object_id(region.object_id.clone());
                view.set_region_id(region.region_id);
                view.set_wall_loops(region.walls.clone());
                view.set_infill_areas(region.infill_areas.clone());
                view.set_seam_candidates(region.seam_candidates.clone());
                view.set_resolved_seam(region.resolved_seam.clone());
                view.set_config((*module.config_view).clone());
                view
            })
            .collect()
    });

    let mut paint = input
        .paint_regions
        .as_ref()
        .map(|_| PaintRegionLayerView::with_paint_regions(layer_index, std::sync::Arc::new(())))
        .unwrap_or_else(|| PaintRegionLayerView::new(layer_index));
    if let Some(ir) = input.lightning_tree_ir.as_ref() {
        paint = paint.with_lightning_tree_ir(std::sync::Arc::clone(ir));
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
    // Completeness mirror (the wasm leg is `dispatch_prepass_call`):
    // | SDK field             | IR source                         |
    // | mesh_objects          | MeshIR.objects                    |
    // | object_ids            | MeshIR.objects[].id               |
    // | layer_plan            | LayerPlanIR                       |
    // | region_segmentation   | RegionMapIR                       |
    // | support_geometry      | SupportGeometryIR                |
    // | paint_objects         | MeshIR.objects + paint_data       |
    // | seam_regions          | SliceIR + LayerPlanIR + RegionMapIR|
    // | config                | CompiledModuleLive.config_view    |
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
                effective_layer_height: plan
                    .object_participation
                    .values()
                    .flat_map(|refs| refs.iter())
                    .find(|reference| reference.global_layer_index == layer.index)
                    .map(|reference| reference.effective_layer_height)
                    .unwrap_or(0.2),
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
    use std::sync::Arc;
    // The native envelope carries the same response fields as the WASM
    // resource. Its layer proposal has no original index/participation map,
    // and its seam candidate has no reason, so those IR fields cannot be
    // reconstructed here; supported metadata is otherwise retained below.
    match stage_export {
        "PrePass::LayerPlanning" => {
            let Some(output) = response.layer_plan.as_ref() else {
                return Ok(slicer_core::PrepassStageOutput::None);
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
                        Ok(slicer_ir::ActiveRegion {
                            object_id: region.object_id.clone(),
                            region_id: region
                                .region_id
                                .parse()
                                .map_err(|e| format!("invalid region id: {e}"))?,
                            effective_layer_height: region.effective_layer_height,
                            is_catchup_layer: region.is_catchup,
                            catchup_z_bottom: region.catchup_z_bottom,
                            ..Default::default()
                        })
                    })
                    .collect();
                let active_regions = active_regions?;
                for region in &proposal.active_regions {
                    let obj_refs = participation
                        .entry(region.object_id.clone())
                        .or_default();
                    let already_referenced =
                        obj_refs.iter().any(|r| r.global_layer_index == index as u32);
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
        "PrePass::SeamPlanning" => Ok(response
            .seam_planning
            .as_ref()
            .map(|output| {
                slicer_core::PrepassStageOutput::SeamPlan(Arc::new(slicer_ir::SeamPlanIR {
                    entries: output
                        .entries()
                        .iter()
                        .map(|entry| slicer_ir::SeamPlanEntry {
                            region_key: slicer_ir::RegionKey {
                                global_layer_index: entry.global_layer_index,
                                object_id: entry.object_id.clone(),
                                region_id: entry.region_id.parse().unwrap_or_default(),
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
                                    reason: slicer_ir::SeamReason::Aligned,
                                })
                                .collect(),
                        })
                        .collect(),
                    ..Default::default()
                }))
            })
            .unwrap_or(slicer_core::PrepassStageOutput::None)),
        "PrePass::SupportGeometry" => Ok(response
            .support_geometry
            .as_ref()
            .map(|output| {
                slicer_core::PrepassStageOutput::SupportPlan(Arc::new(slicer_ir::SupportPlanIR {
                    entries: output
                        .entries()
                        .iter()
                        .map(|entry| slicer_ir::SupportPlanEntry {
                            global_layer_index: entry.global_layer_index,
                            object_id: entry.object_id.clone(),
                            region_id: entry.region_id.parse().unwrap_or_default(),
                            branch_segments: entry
                                .branch_segments
                                .iter()
                                .map(|segment| slicer_ir::ExtrusionPath3D {
                                    points: segment.clone(),
                                    role: slicer_ir::ExtrusionRole::SupportMaterial,
                                    speed_factor: 1.0,
                                })
                                .collect(),
                        })
                        .collect(),
                    raft_plan: output.raft_plan().cloned().map(|plan| slicer_ir::RaftPlan {
                        raft_layers: plan.raft_layers,
                        raft_first_layer_density: plan.raft_first_layer_density,
                        base_raft_layers: plan.base_raft_layers,
                        interface_raft_layers: plan.interface_raft_layers,
                    }),
                    ..Default::default()
                }))
            })
            .unwrap_or(slicer_core::PrepassStageOutput::None)),
        "PrePass::MeshAnalysis" => Ok(response
            .mesh_analysis
            .as_ref()
            .map(|output| {
                slicer_core::PrepassStageOutput::MeshAnalysisAuxiliary(Arc::new(
                    slicer_core::MeshAnalysisAuxiliary {
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
                                             slicer_sdk::prepass_types::FacetClass::Normal => slicer_core::FacetClassRecord::Normal,
                                             slicer_sdk::prepass_types::FacetClass::NearHorizontal => slicer_core::FacetClassRecord::NearHorizontal,
                                             slicer_sdk::prepass_types::FacetClass::Overhang => slicer_core::FacetClassRecord::Overhang,
                                             slicer_sdk::prepass_types::FacetClass::Bridge => slicer_core::FacetClassRecord::Bridge,
                                             slicer_sdk::prepass_types::FacetClass::TopSurface => slicer_core::FacetClassRecord::TopSurface,
                                             slicer_sdk::prepass_types::FacetClass::BottomSurface => slicer_core::FacetClassRecord::BottomSurface,
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
                    },
                ))
            })
            .unwrap_or(slicer_core::PrepassStageOutput::None)),
        "PrePass::PaintSegmentation" => Err(
            "native path does not yet support stage PrePass::PaintSegmentation output commit"
                .to_string(),
        ),
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
        resolved_seam_origin: None,
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
        support_path_origins: Vec::new(),
        interface_path_origins: Vec::new(),
        raft_path_origins: Vec::new(),
    }
}

/// Commit a native layer response through the existing output converters.
pub fn commit_native_layer_response(
    response: &NativeLayerResponse,
    stage_export: &str,
    layer_index: u32,
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
            let ir = convert_infill_output(&collected, layer_index)?;
            Ok(Some(if stage_export.ends_with("PostProcess") {
                LayerStageCommit::InfillPostProcess(ir)
            } else {
                LayerStageCommit::Infill(ir)
            }))
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            let Some(builder) = response.support.as_ref() else {
                return Ok(None);
            };
            let collected = collect_support(builder);
            if collected.support_paths.is_empty()
                && collected.interface_paths.is_empty()
                && collected.raft_paths.is_empty()
            {
                return Ok(None);
            }
            let ir = convert_support_output(&collected, layer_index)?;
            Ok(Some(if stage_export.ends_with("PostProcess") {
                LayerStageCommit::SupportPostProcess(ir)
            } else {
                LayerStageCommit::Support(ir)
            }))
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
        "Layer::SlicePostProcess" => Err(format!(
            "native path does not yet support stage {stage_export} output commit"
        )),
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
