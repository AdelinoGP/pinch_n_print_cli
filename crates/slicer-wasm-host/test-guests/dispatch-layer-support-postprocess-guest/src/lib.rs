wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-support-postprocess/support-postprocess-module",
    generate_all,
});

use exports::slicer::layer_support_postprocess::support_postprocess::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{LayerIdx, SliceRegionView, SupportOutputBuilder};

struct Component;

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        regions: Vec<SliceRegionView>,
        output: SupportOutputBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        for region in &regions {
            let _object_id = region.object_id();
            let _region_id = region.region_id();
            let path = slicer::types::geometry::ExtrusionPath3d {
                points: vec![slicer::types::geometry::Point3WithWidth {
                    x: region.polygons().len() as f32,
                    y: 0.0,
                    z: region.z(),
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                }],
                role: slicer::types::geometry::ExtrusionRole::SupportMaterial,
                speed_factor: 1.0,
            };
            output
                .push_support_path(&path)
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
