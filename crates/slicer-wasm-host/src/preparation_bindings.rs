//! Composed Layer bindings sharing ordinary and preparation resource identities.

/// Preparation resource identity root and composed perimeters binding.
pub mod perimeters {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-perimeters-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0.print-preparation-input": super::super::preparation_host::PreparationInputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-output": super::super::preparation_host::PreparationOutputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-view": super::super::preparation_host::PreparedPlanViewData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-piece": super::super::preparation_host::PreparedPieceData,
        },
    });
}

/// Composed slice postprocessing binding.
pub mod slice_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-slice-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed perimeter postprocessing binding.
pub mod perimeters_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-perimeters-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed infill binding.
pub mod infill {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-infill-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:support-queries/support-queries@1.0.0": crate::host::layer_infill::slicer::support_queries::support_queries,
            "slicer:lightning-queries/lightning-queries@1.0.0": crate::host::layer_infill::slicer::lightning_queries::lightning_queries,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed infill postprocessing binding.
pub mod infill_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-infill-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed support binding.
pub mod support {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-support-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            // The projected ordinary linker registers this resource's concrete host identity.
            "slicer:support-queries/support-queries@1.0.0": crate::host::layer_support::slicer::support_queries::support_queries,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed support postprocessing binding.
pub mod support_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-support-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed anchored events binding.
pub mod anchored_events {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-anchored-events-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed path optimization binding.
pub mod path_optimization {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-path-optimization-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Lossless preparation-only projections into the canonical imported identities.
pub(super) mod input {
    use super::perimeters::slicer::layer_preparation::preparation_types as cap;
    use crate::host::layer_perimeters::slicer::{
        ir_handles::ir_handles as ir, types::geometry as geo,
    };
    use crate::host::{config_view_to_data, HostExecutionContext};
    use crate::marshal::{ir_to_wit_expolygons, ir_to_wit_paint_semantic};
    use slicer_schema::preparation::PreparationInputError;
    use slicer_sdk::preparation as dto;

    impl From<PreparationInputError> for cap::PreparationInputError {
        fn from(error: PreparationInputError) -> Self {
            match error {
                PreparationInputError::AccessDenied => Self::AccessDenied,
                PreparationInputError::RangeError => Self::RangeError,
                PreparationInputError::ArithmeticOverflow => Self::ArithmeticOverflow,
                PreparationInputError::MissingRequired { read } => Self::MissingRequired(read),
                PreparationInputError::UnsupportedRead { read } => Self::UnsupportedRead(read),
                PreparationInputError::InvalidInput { detail } => Self::InvalidInput(detail),
                PreparationInputError::UnknownObject { object_id } => {
                    Self::UnknownObject(object_id)
                }
                PreparationInputError::SelectionRejected { detail } => {
                    Self::SelectionRejected(detail)
                }
            }
        }
    }

    impl From<cap::PreparationInputError> for PreparationInputError {
        fn from(error: cap::PreparationInputError) -> Self {
            match error {
                cap::PreparationInputError::AccessDenied => Self::AccessDenied,
                cap::PreparationInputError::RangeError => Self::RangeError,
                cap::PreparationInputError::ArithmeticOverflow => Self::ArithmeticOverflow,
                cap::PreparationInputError::MissingRequired(read) => Self::MissingRequired { read },
                cap::PreparationInputError::UnsupportedRead(read) => Self::UnsupportedRead { read },
                cap::PreparationInputError::InvalidInput(detail) => Self::InvalidInput { detail },
                cap::PreparationInputError::UnknownObject(object_id) => {
                    Self::UnknownObject { object_id }
                }
                cap::PreparationInputError::SelectionRejected(detail) => {
                    Self::SelectionRejected { detail }
                }
            }
        }
    }

    fn paint_value(value: dto::PreparationPaintValue) -> cap::PreparationPaintValue {
        match value {
            dto::PreparationPaintValue::Flag(value) => cap::PreparationPaintValue::Flag(value),
            dto::PreparationPaintValue::Scalar(value) => cap::PreparationPaintValue::Scalar(value),
            dto::PreparationPaintValue::ToolIndex(value) => {
                cap::PreparationPaintValue::ToolIndex(value)
            }
            dto::PreparationPaintValue::Custom(value) => cap::PreparationPaintValue::Custom(value),
        }
    }

    fn variant_chain(chain: dto::PreparationVariantChain) -> cap::PreparationVariantChain {
        chain
            .into_iter()
            .map(|(name, value)| (name, paint_value(value)))
            .collect()
    }

    fn region_key(key: dto::PreparationRegionKey) -> cap::PreparationRegionKey {
        cap::PreparationRegionKey {
            global_layer_index: key.global_layer_index,
            object_id: key.object_id,
            region_id: key.region_id.to_string(),
            variant_chain: variant_chain(key.variant_chain),
        }
    }

    fn point(point: slicer_ir::Point3) -> geo::Point3 {
        geo::Point3 {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn width_point(point: slicer_ir::Point3WithWidth) -> geo::Point3WithWidth {
        geo::Point3WithWidth {
            x: point.x,
            y: point.y,
            z: point.z,
            width: point.width,
            flow_factor: point.flow_factor,
            overhang_quartile: point.overhang_quartile,
            dist_to_top_mm: point.dist_to_top_mm,
            overhang_distance_mm: point.overhang_distance_mm,
        }
    }

    fn bounds(bounds: slicer_ir::BoundingBox3) -> geo::BoundingBox3 {
        geo::BoundingBox3 {
            min: point(bounds.min),
            max: point(bounds.max),
        }
    }

    pub(crate) fn layer(
        host: &mut HostExecutionContext,
        layer: dto::PreparationLayer,
    ) -> wasmtime::Result<cap::PreparationLayer> {
        Ok(cap::PreparationLayer {
            index: layer.index,
            z: layer.z,
            has_nonplanar: layer.has_nonplanar,
            is_sync_layer: layer.is_sync_layer,
            is_raft: layer.is_raft,
            active_regions: layer
                .active_regions
                .into_iter()
                .map(|active| {
                    Ok(cap::PreparationActiveRegion {
                        object_id: active.object_id,
                        region_id: active.region_id.to_string(),
                        config: host.push_config_view(config_view_to_data(&active.config))?,
                        effective_layer_height: active.effective_layer_height,
                        nonplanar_shell: active.nonplanar_shell.map(|shell| {
                            cap::PreparationNonplanarShell {
                                surface_group_id: shell.surface_group_id,
                                shell_index: shell.shell_index,
                            }
                        }),
                        is_catchup_layer: active.is_catchup_layer,
                        catchup_z_bottom: active.catchup_z_bottom,
                        tool_index: active.tool_index,
                    })
                })
                .collect::<wasmtime::Result<_>>()?,
        })
    }

    pub(crate) fn region(region: dto::PreparationRegion) -> cap::PreparationRegion {
        // HashMap iteration is not a wire-order contract. Sort semantic keys
        // without flattening the polygon/segment option nesting or Custom values.
        let mut annotations: Vec<_> = region.segment_annotations.into_iter().collect();
        annotations.sort_by(|(a, _), (b, _)| a.cmp(b));
        cap::PreparationRegion {
            global_layer_index: region.global_layer_index,
            z: region.z,
            object_id: region.object_id,
            region_id: region.region_id.to_string(),
            variant_chain: variant_chain(region.variant_chain),
            polygons: ir_to_wit_expolygons(&region.polygons),
            infill_areas: ir_to_wit_expolygons(&region.infill_areas),
            nonplanar_surface: region.nonplanar_surface,
            effective_layer_height: region.effective_layer_height,
            segment_annotations: annotations
                .into_iter()
                .map(
                    |(semantic, polygons)| cap::PreparationSegmentAnnotationsEntry {
                        semantic: ir_to_wit_paint_semantic(&semantic),
                        polygons: polygons
                            .into_iter()
                            .map(|values| cap::PreparationSegmentAnnotationsPolygon {
                                values: values
                                    .into_iter()
                                    .map(|value| value.map(paint_value))
                                    .collect(),
                            })
                            .collect(),
                    },
                )
                .collect(),
            top_shell_index: region.top_shell_index,
            bottom_shell_index: region.bottom_shell_index,
            top_solid_fill: ir_to_wit_expolygons(&region.top_solid_fill),
            bottom_solid_fill: ir_to_wit_expolygons(&region.bottom_solid_fill),
            is_bridge: region.is_bridge,
            bridge_areas: ir_to_wit_expolygons(&region.bridge_areas),
            bridge_orientation_deg: region.bridge_orientation_deg,
            sparse_infill_area: ir_to_wit_expolygons(&region.sparse_infill_area),
            internal_solid_fill: ir_to_wit_expolygons(&region.internal_solid_fill),
            internal_bridge_areas: ir_to_wit_expolygons(&region.internal_bridge_areas),
            internal_bridge_angles_deg: region.internal_bridge_angles_deg,
            raft_fill: ir_to_wit_expolygons(&region.raft_fill),
            phase: match region.phase {
                dto::PreparationInputPhase::CommittedPrepass => {
                    cap::PreparationInputPhase::CommittedPrepass
                }
            },
        }
    }

    pub(crate) fn region_config(
        host: &mut HostExecutionContext,
        config: dto::PreparationRegionConfig,
    ) -> wasmtime::Result<cap::PreparationRegionConfig> {
        Ok(cap::PreparationRegionConfig {
            key: region_key(config.key),
            config: host.push_config_view(config_view_to_data(&config.config))?,
        })
    }

    fn facet_class(class: dto::PreparationFacetClass) -> cap::PreparationFacetClass {
        match class {
            dto::PreparationFacetClass::Normal => cap::PreparationFacetClass::Normal,
            dto::PreparationFacetClass::NearHorizontal { slope_angle_deg } => {
                cap::PreparationFacetClass::NearHorizontal(cap::PreparationNearHorizontalFacet {
                    slope_angle_deg,
                })
            }
            dto::PreparationFacetClass::Overhang { angle_deg } => {
                cap::PreparationFacetClass::Overhang(cap::PreparationOverhangFacet { angle_deg })
            }
            dto::PreparationFacetClass::Bridge => cap::PreparationFacetClass::Bridge,
            dto::PreparationFacetClass::TopSurface => cap::PreparationFacetClass::TopSurface,
            dto::PreparationFacetClass::BottomSurface => cap::PreparationFacetClass::BottomSurface,
        }
    }

    pub(crate) fn surface_object(
        object: dto::PreparationSurfaceObject,
    ) -> cap::PreparationSurfaceObject {
        cap::PreparationSurfaceObject {
            object_id: object.object_id,
            facet_classes: object.facet_classes.into_iter().map(facet_class).collect(),
            surface_groups: object
                .surface_groups
                .into_iter()
                .map(|group| ir::SurfaceGroup {
                    id: group.id,
                    facet_indices: group.facet_indices,
                    z_min: group.z_min,
                    z_max: group.z_max,
                    area_mm2: group.area_mm2,
                    printable: group.printable,
                    shell_count: group.shell_count,
                })
                .collect(),
            bridge_regions: object
                .bridge_regions
                .into_iter()
                .map(|bridge| cap::PreparationBridgeRegion {
                    id: bridge.id,
                    facet_indices: bridge.facet_indices,
                    bridge_direction_deg: bridge.bridge_direction_deg,
                    anchor_width_mm: bridge.anchor_width_mm,
                    bridge_length_mm: bridge.bridge_length_mm,
                    expansion_margin_mm: bridge.expansion_margin_mm,
                    is_valid: bridge.is_valid,
                    xy_footprint: ir_to_wit_expolygons(&bridge.xy_footprint),
                })
                .collect(),
            overhang_regions: object
                .overhang_regions
                .into_iter()
                .map(|overhang| cap::PreparationOverhangRegion {
                    id: overhang.id,
                    facet_indices: overhang.facet_indices,
                    max_angle_deg: overhang.max_angle_deg,
                    needs_support: overhang.needs_support,
                    xy_footprint: ir_to_wit_expolygons(&overhang.xy_footprint),
                })
                .collect(),
        }
    }

    pub(crate) fn surface_layer(
        layer: dto::PreparationSurfaceLayer,
    ) -> cap::PreparationSurfaceLayer {
        cap::PreparationSurfaceLayer {
            object_id: layer.object_id,
            global_layer_index: layer.global_layer_index,
            overhang_quartile_polygons: layer
                .overhang_quartile_polygons
                .into_iter()
                .map(|band| ir::QuartileBand {
                    quartile: band.quartile,
                    polygons: ir_to_wit_expolygons(&band.polygons),
                })
                .collect(),
            prev_layer_boundaries: ir_to_wit_expolygons(&layer.prev_layer_boundaries),
        }
    }

    fn paint_layer(layer: dto::PreparationPaintLayer) -> cap::PreparationPaintLayer {
        cap::PreparationPaintLayer {
            semantic: layer.semantic,
            facet_values: layer
                .facet_values
                .into_iter()
                .map(|value| value.map(paint_value))
                .collect(),
            strokes: layer
                .strokes
                .into_iter()
                .map(|stroke| cap::PreparationPaintStroke {
                    triangles: stroke.triangles.into_iter().map(point).collect(),
                    semantic: stroke.semantic,
                    value: paint_value(stroke.value),
                })
                .collect(),
        }
    }

    pub(crate) fn mesh_object(object: dto::PreparationMeshObject) -> cap::PreparationMeshObject {
        cap::PreparationMeshObject {
            object_id: object.object_id,
            bounds: object.bounds.map(bounds),
            vertex_count: object.vertex_count,
            triangle_count: object.triangle_count,
            paint_layers: object.paint_layers.into_iter().map(paint_layer).collect(),
        }
    }

    pub(crate) fn mesh_query(query: cap::PreparationMeshQuery) -> dto::PreparationMeshQuery {
        match query {
            cap::PreparationMeshQuery::RaycastZDown(query) => {
                dto::PreparationMeshQuery::RaycastZDown {
                    object_id: query.object_id,
                    x: query.x,
                    y: query.y,
                    start_z: query.start_z,
                }
            }
            cap::PreparationMeshQuery::SurfaceNormalAt(query) => {
                dto::PreparationMeshQuery::SurfaceNormalAt {
                    object_id: query.object_id,
                    x: query.x,
                    y: query.y,
                    z: query.z,
                }
            }
            cap::PreparationMeshQuery::ObjectBounds(query) => {
                dto::PreparationMeshQuery::ObjectBounds {
                    object_id: query.object_id,
                }
            }
        }
    }

    pub(crate) fn mesh_query_result(
        result: dto::PreparationMeshQueryResult,
    ) -> cap::PreparationMeshQueryResult {
        match result {
            dto::PreparationMeshQueryResult::RaycastZDown(hit) => {
                cap::PreparationMeshQueryResult::RaycastZDown(hit)
            }
            dto::PreparationMeshQueryResult::SurfaceNormalAt(normal) => {
                cap::PreparationMeshQueryResult::SurfaceNormalAt(normal.map(point))
            }
            dto::PreparationMeshQueryResult::ObjectBounds(value) => {
                cap::PreparationMeshQueryResult::ObjectBounds(bounds(value))
            }
        }
    }

    pub(crate) fn seam(seam: dto::PreparationSeam) -> cap::PreparationSeam {
        cap::PreparationSeam {
            region_key: region_key(seam.region_key),
            chosen_candidate: ir::SeamPosition {
                point: width_point(seam.chosen_candidate.point),
                wall_index: seam.chosen_candidate.wall_index,
            },
            scored_candidates: seam
                .scored_candidates
                .into_iter()
                .map(|candidate| cap::PreparationScoredSeamCandidate {
                    position: width_point(candidate.position),
                    score: candidate.score,
                    reason: match candidate.reason {
                        dto::PreparationSeamReason::Concave => cap::PreparationSeamReason::Concave,
                        dto::PreparationSeamReason::Aligned => cap::PreparationSeamReason::Aligned,
                        dto::PreparationSeamReason::UserForced => {
                            cap::PreparationSeamReason::UserForced
                        }
                        dto::PreparationSeamReason::Sharp => cap::PreparationSeamReason::Sharp,
                    },
                })
                .collect(),
        }
    }

    fn support_geometry_entry(
        entry: dto::PreparationSupportGeometryEntry,
    ) -> cap::PreparationSupportGeometryEntry {
        cap::PreparationSupportGeometryEntry {
            key: cap::PreparationSupportGeometryKey {
                global_support_layer_index: entry.key.global_support_layer_index,
                object_id: entry.key.object_id,
                region_id: entry.key.region_id.to_string(),
                variant_chain: variant_chain(entry.key.variant_chain),
            },
            polygons: ir_to_wit_expolygons(&entry.polygons),
        }
    }

    pub(crate) fn support_analysis(
        batch: dto::PreparationSupportAnalysisBatch,
    ) -> cap::PreparationSupportAnalysisBatch {
        cap::PreparationSupportAnalysisBatch {
            candidates: batch
                .candidates
                .into_iter()
                .map(|candidate| cap::PreparationSupportCandidate {
                    id: candidate.id,
                    geometry: ir_to_wit_expolygons(&candidate.geometry),
                    source: cap::PreparationSupportCandidateSource {
                        object_id: candidate.source.object_id,
                        region_id: candidate.source.region_id.to_string(),
                        global_layer_index: candidate.source.global_layer_index,
                        z_units: candidate.source.z_units,
                        variant_chain: variant_chain(candidate.source.variant_chain),
                    },
                    enforced: candidate.enforced,
                    blocked: candidate.blocked,
                })
                .collect(),
            model_occupancy: batch
                .model_occupancy
                .into_iter()
                .map(support_geometry_entry)
                .collect(),
            cantilever_surfaces: batch
                .cantilever_surfaces
                .into_iter()
                .map(support_geometry_entry)
                .collect(),
            termination_surfaces: batch
                .termination_surfaces
                .into_iter()
                .map(support_geometry_entry)
                .collect(),
            support_territory: batch
                .support_territory
                .into_iter()
                .map(support_geometry_entry)
                .collect(),
            shared_settings: batch.shared_settings,
            baseline_feasible_envelope: ir_to_wit_expolygons(&batch.baseline_feasible_envelope),
            family_assignments: batch
                .family_assignments
                .into_iter()
                .map(|assignment| cap::PreparationSupportFamilyAssignment {
                    key: cap::PreparationSupportSourceKey {
                        object_id: assignment.key.object_id,
                        region_id: assignment.key.region_id.to_string(),
                        variant_chain: variant_chain(assignment.key.variant_chain),
                    },
                    family_id: assignment.family_id,
                })
                .collect(),
        }
    }

    pub(crate) fn support_geometry(
        batch: dto::PreparationSupportGeometryBatch,
    ) -> cap::PreparationSupportGeometryBatch {
        cap::PreparationSupportGeometryBatch {
            support_layer_height_mm: batch.support_layer_height_mm,
            support_top_z_distance_mm: batch.support_top_z_distance_mm,
            entries: batch
                .entries
                .into_iter()
                .map(support_geometry_entry)
                .collect(),
        }
    }

    fn support_plan_entry(
        entry: dto::PreparationSupportPlanEntry,
    ) -> cap::PreparationSupportPlanEntry {
        cap::PreparationSupportPlanEntry {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id,
            region_id: entry.region_id.to_string(),
            family_id: entry.family_id,
            demand_ids: entry.demand_ids,
            body_ids: entry.body_ids,
            anchor_layer_index: entry.anchor_layer_index,
            anchor_z: entry.anchor_z,
            roles: entry
                .roles
                .into_iter()
                .map(|role| cap::PreparationSupportPlanRoleRegion {
                    role: match role.role {
                        dto::PreparationSupportPlanRole::SupportBody => {
                            cap::PreparationSupportPlanRole::SupportBody
                        }
                        dto::PreparationSupportPlanRole::TopInterface => {
                            cap::PreparationSupportPlanRole::TopInterface
                        }
                        dto::PreparationSupportPlanRole::BaseInterface => {
                            cap::PreparationSupportPlanRole::BaseInterface
                        }
                        dto::PreparationSupportPlanRole::BottomInterface => {
                            cap::PreparationSupportPlanRole::BottomInterface
                        }
                        dto::PreparationSupportPlanRole::RaftRelated => {
                            cap::PreparationSupportPlanRole::RaftRelated
                        }
                    },
                    regions: ir_to_wit_expolygons(&role.regions),
                })
                .collect(),
            skeleton: entry
                .skeleton
                .map(|skeleton| cap::PreparationSupportPlanSkeleton {
                    points: skeleton.points.into_iter().map(point).collect(),
                    wall_counts: skeleton.wall_counts,
                }),
            capabilities: entry.capabilities,
            provenance: entry.provenance,
            decline_reason: entry.decline_reason.map(|reason| match reason {
                dto::PreparationSupportPlanDeclineReason::DeclinedPolicy => {
                    cap::PreparationSupportPlanDeclineReason::DeclinedPolicy
                }
                dto::PreparationSupportPlanDeclineReason::NoRoute => {
                    cap::PreparationSupportPlanDeclineReason::NoRoute
                }
                dto::PreparationSupportPlanDeclineReason::Blocked => {
                    cap::PreparationSupportPlanDeclineReason::Blocked
                }
                dto::PreparationSupportPlanDeclineReason::UnsupportedMode => {
                    cap::PreparationSupportPlanDeclineReason::UnsupportedMode
                }
            }),
            variant_chain: variant_chain(entry.variant_chain),
        }
    }

    pub(crate) fn support_plan(
        batch: dto::PreparationSupportPlanBatch,
    ) -> cap::PreparationSupportPlanBatch {
        cap::PreparationSupportPlanBatch {
            entries: batch.entries.into_iter().map(support_plan_entry).collect(),
            raft_plan: batch.raft_plan.map(|raft| cap::PreparationRaftPlan {
                raft_layers: raft.raft_layers,
                raft_first_layer_density: raft.raft_first_layer_density,
                base_raft_layers: raft.base_raft_layers,
                interface_raft_layers: raft.interface_raft_layers,
            }),
        }
    }

    fn target_source(source: dto::PreparationTargetSource) -> cap::PreparationTargetSource {
        match source {
            dto::PreparationTargetSource::Model { key, members } => {
                cap::PreparationTargetSource::Model(cap::PreparationModelTargetSource {
                    key: region_key(key),
                    members: members.into_iter().map(region_key).collect(),
                })
            }
            dto::PreparationTargetSource::SupportCarrier {
                entry_index,
                object_id,
                region_id,
                variant_chain: chain,
                family,
                demand_ids,
                body_ids,
                provenance,
                anchor_layer_index,
            } => cap::PreparationTargetSource::SupportCarrier(
                cap::PreparationSupportCarrierTargetSource {
                    entry_index,
                    object_id,
                    region_id: region_id.to_string(),
                    variant_chain: variant_chain(chain),
                    family,
                    demand_ids,
                    body_ids,
                    provenance,
                    anchor_layer_index,
                },
            ),
            dto::PreparationTargetSource::Raft { global_layer_index } => {
                cap::PreparationTargetSource::Raft(cap::PreparationRaftTargetSource {
                    global_layer_index,
                })
            }
            dto::PreparationTargetSource::AnchoredStage { anchor_layer_index } => {
                cap::PreparationTargetSource::AnchoredStage(
                    cap::PreparationAnchoredStageTargetSource { anchor_layer_index },
                )
            }
        }
    }

    pub(crate) fn selected_target(
        host: &mut HostExecutionContext,
        target: dto::PreparationSelectedTarget,
    ) -> wasmtime::Result<cap::PreparationSelectedTarget> {
        Ok(cap::PreparationSelectedTarget {
            global_layer_index: target.global_layer_index,
            stage_id: target.stage_id,
            invoke: target.invoke,
            source: target_source(target.source),
            config: host.push_config_view(config_view_to_data(&target.config))?,
            held_claims: target.held_claims,
            authored_coloring: target.authored_coloring,
            resolved_role_tool: target.resolved_role_tool,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // test-quality: typed-link witness for prepared Support's ordinary support-query-view resource identity.
    #[cfg(feature = "preparation-test-fixtures")]
    #[test]
    fn prepared_support_loads_with_ordinary_query_resource_identity() {
        use crate::host::HostExecutionContextBuilder;
        use crate::preparation::{freeze_wasm, PreparedHostState, PreparedOwnerStore};
        use slicer_schema::preparation::PreparationDeclaration;
        use wasmtime::component::Component;

        let engine = wasmtime::Engine::default();
        let component = Component::from_file(
            &engine,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test-guests/whole-print-input-support.component.wasm"
            ),
        )
        .expect("required support fixture; run cargo xtask build-guests");
        let stage_id = slicer_schema::STAGES
            .iter()
            .find(|stage| stage.wit_world == "support-module")
            .expect("canonical support stage")
            .stage_id;
        let binding = freeze_wasm(
            "support-query-identity".into(),
            Some(PreparationDeclaration::new(
                "slicer:layer-preparation/prepare@1.0.0",
                vec![],
                vec!["observations".into()],
            )),
            engine.clone(),
            component.clone(),
            stage_id.into(),
        )
        .expect("freeze canonical support surface");
        let authority = PreparedOwnerStore::new();
        let owner = authority.create_owner(&binding).expect("support owner");
        let ordinary = HostExecutionContextBuilder::new(binding.module_id(), 0.0, 1.0).build();
        let mut store = wasmtime::Store::new(&engine, PreparedHostState::new(ordinary, owner));
        let linker = PreparedHostState::linker(&engine, stage_id).expect("ordinary support linker");
        support::PreparedSupportModule::instantiate(&mut store, &component, &linker)
            .unwrap_or_else(|error| panic!("prepared support typed loading: {error:#}"));
    }

    #[test]
    fn every_layer_stage_with_a_package_has_a_prepared_binding() {
        let expected: Vec<_> = slicer_schema::STAGES
            .iter()
            .filter(|stage| {
                stage.tier_id == slicer_schema::TIER_LAYER && !stage.wit_package.is_empty()
            })
            .collect();
        assert!(
            !expected.is_empty(),
            "schema contains no packaged Layer stages"
        );
        for stage in &expected {
            // Each arm names a generated concrete type; removing its binding breaks compilation.
            let compiled = match stage.wit_world {
                "slice-postprocess-module" => {
                    std::any::type_name::<slice_postprocess::PreparedSlicePostprocessModule>()
                }
                "perimeters-module" => {
                    std::any::type_name::<perimeters::PreparedPerimetersModule>()
                }
                "perimeters-postprocess-module" => std::any::type_name::<
                    perimeters_postprocess::PreparedPerimetersPostprocessModule,
                >(),
                "infill-module" => std::any::type_name::<infill::PreparedInfillModule>(),
                "infill-postprocess-module" => {
                    std::any::type_name::<infill_postprocess::PreparedInfillPostprocessModule>()
                }
                "support-module" => std::any::type_name::<support::PreparedSupportModule>(),
                "support-postprocess-module" => {
                    std::any::type_name::<support_postprocess::PreparedSupportPostprocessModule>()
                }
                "anchored-events-module" => {
                    std::any::type_name::<anchored_events::PreparedAnchoredEventsModule>()
                }
                "path-optimization-module" => {
                    std::any::type_name::<path_optimization::PreparedPathOptimizationModule>()
                }
                _ => panic!(
                    "no compiled prepared binding for {} ({})",
                    stage.stage_id, stage.wit_world
                ),
            };
            let expected_type = format!(
                "Prepared{}",
                stage
                    .wit_world
                    .split('-')
                    .map(|part| {
                        let mut chars = part.chars();
                        chars
                            .next()
                            .expect("nonempty world segment")
                            .to_uppercase()
                            .collect::<String>()
                            + chars.as_str()
                    })
                    .collect::<String>()
            );
            assert_eq!(
                compiled.rsplit("::").next(),
                Some(expected_type.as_str()),
                "compiled prepared binding does not match {}",
                stage.stage_id
            );
        }
        println!("DERIVED_LAYER_STAGE_COUNT={}", expected.len());
    }
}
