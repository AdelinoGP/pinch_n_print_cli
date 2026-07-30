wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-perimeters-postprocess/perimeters-postprocess-module",
    generate_all,
});

use exports::slicer::layer_perimeters_postprocess::perimeters_postprocess::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{LayerIdx, PerimeterOutputBuilder, PerimeterRegionView};

struct Component;

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        regions: Vec<PerimeterRegionView>,
        output: PerimeterOutputBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        for region in &regions {
            let walls = region.wall_loops();
            let Some(z) = walls
                .first()
                .and_then(|wall| wall.path.points.first())
                .map(|point| point.z)
            else {
                continue;
            };
            let wall_loop = slicer::ir_handles::ir_handles::WallLoopView {
                perimeter_index: walls.len() as u32,
                loop_type: slicer::ir_handles::ir_handles::WallLoopType::Outer,
                path: slicer::types::geometry::ExtrusionPath3d {
                    points: vec![slicer::types::geometry::Point3WithWidth {
                        x: walls.len() as f32,
                        y: region.infill_areas().len() as f32,
                        z,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                    }],
                    role: slicer::types::geometry::ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                feature_flags: vec![slicer::ir_handles::ir_handles::WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: Vec::new(),
                }],
                boundary_type: slicer::ir_handles::ir_handles::WallBoundaryType::ExteriorSurface,
            };
            output
                .push_wall_loop(&wall_loop)
                .map_err(|message| ModuleError {
                    code: 1,
                    message,
                    fatal: true,
                })?;
        }
        Ok(())
    }
}

export!(Component);
