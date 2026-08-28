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

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        regions: Vec<PerimeterRegionView>,
        _prior_infill: Vec<PriorInfillRegion>,
        output: InfillOutputBuilder,
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
            let path = slicer::types::geometry::ExtrusionPath3d {
                points: vec![slicer::types::geometry::Point3WithWidth {
                    overhang_distance_mm: None,
                    x: walls.len() as f32,
                    y: region.infill_areas().len() as f32,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                }],
                role: slicer::types::geometry::ExtrusionRole::TopSolidInfill,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
            };
            output
                .push_solid_path(&path)
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
