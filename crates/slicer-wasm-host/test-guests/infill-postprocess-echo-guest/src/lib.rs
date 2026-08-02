wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-infill-postprocess/infill-postprocess-module",
    generate_all,
});

use exports::slicer::layer_infill_postprocess::infill_postprocess::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{
    InfillOutputBuilder, LayerIdx, PerimeterRegionView, PriorInfillRegion,
};

struct Component;

fn builder_error(code: u32, message: String) -> ModuleError {
    ModuleError {
        code,
        message,
        fatal: true,
    }
}

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        regions: Vec<PerimeterRegionView>,
        prior_infill: Vec<PriorInfillRegion>,
        output: InfillOutputBuilder,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        for region in &prior_infill {
            output
                .set_current_origin(&region.object_id, &region.region_id)
                .map_err(|message| builder_error(1, message))?;
            for path in &region.sparse_infill {
                output
                    .push_sparse_path(path)
                    .map_err(|message| builder_error(2, message))?;
            }
            for path in &region.solid_infill {
                output
                    .push_solid_path(path)
                    .map_err(|message| builder_error(3, message))?;
            }
            for path in &region.ironing {
                output
                    .push_ironing_path(path)
                    .map_err(|message| builder_error(4, message))?;
            }
        }

        if config.get_int("emit_view_witness") == Some(1) {
            for region in &regions {
                let object_id = region.object_id();
                let region_id = region.region_id();
                output
                    .set_current_origin(&object_id, &region_id)
                    .map_err(|message| builder_error(5, message))?;

                let wall_source = region
                    .wall_source_region_id()
                    .and_then(|id| id.parse::<f32>().ok())
                    .unwrap_or(-1.0);
                let header = slicer::types::geometry::ExtrusionPath3d {
                    points: vec![slicer::types::geometry::Point3WithWidth {
                        overhang_distance_mm: None,
                        x: region.tool_index() as f32,
                        y: wall_source,
                        z: 0.0,
                        width: 777.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                    }],
                    role: slicer::types::geometry::ExtrusionRole::TopSolidInfill,
                    speed_factor: 1.0,
                };
                output
                    .push_solid_path(&header)
                    .map_err(|message| builder_error(6, message))?;

                let fields = [
                    region.sparse_infill_area(),
                    region.top_solid_fill(),
                    region.bottom_solid_fill(),
                    region.bridge_areas(),
                ];
                for (field_id, polygons) in fields.iter().enumerate() {
                    for (polygon_index, polygon) in polygons.iter().enumerate() {
                        let mut points = vec![slicer::types::geometry::Point3WithWidth {
                            overhang_distance_mm: None,
                            x: field_id as f32,
                            y: polygon_index as f32,
                            z: 0.0,
                            width: 888.0,
                            flow_factor: polygon.holes.len() as f32,
                            overhang_quartile: None,
                            dist_to_top_mm: 0.0,
                        }];
                        for point in &polygon.contour.points {
                            points.push(slicer::types::geometry::Point3WithWidth {
                                overhang_distance_mm: None,
                                x: point.x as f32,
                                y: point.y as f32,
                                z: 0.0,
                                width: 0.4,
                                flow_factor: 1.0,
                                overhang_quartile: None,
                                dist_to_top_mm: 0.0,
                            });
                        }
                        for hole in &polygon.holes {
                            for point in &hole.points {
                                points.push(slicer::types::geometry::Point3WithWidth {
                                    overhang_distance_mm: None,
                                    x: point.x as f32,
                                    y: point.y as f32,
                                    z: 0.0,
                                    width: 0.4,
                                    flow_factor: 1.0,
                                    overhang_quartile: None,
                                    dist_to_top_mm: 0.0,
                                });
                            }
                        }
                        output
                            .push_solid_path(&slicer::types::geometry::ExtrusionPath3d {
                                points,
                                role: slicer::types::geometry::ExtrusionRole::TopSolidInfill,
                                speed_factor: 1.0,
                            })
                            .map_err(|message| builder_error(7, message))?;
                    }
                }
            }
        }
        Ok(())
    }
}

export!(Component);
