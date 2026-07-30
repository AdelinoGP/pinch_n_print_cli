wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-slice-postprocess/slice-postprocess-module",
    generate_all,
});

use exports::slicer::layer_slice_postprocess::slice_postprocess::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{
    LayerIdx, PaintRegionLayerView, SlicePostprocessBuilder, SliceRegionView,
};

struct Component;

impl Guest for Component {
    fn run(
        layer_index: LayerIdx,
        regions: Vec<SliceRegionView>,
        _paint: PaintRegionLayerView,
        output: SlicePostprocessBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        for region in &regions {
            let key = slicer::ir_handles::ir_handles::RegionKey {
                variant_chain: Vec::new(),
                layer_index,
                object_id: region.object_id(),
                region_id: region.region_id(),
            };
            let polygon = slicer::types::geometry::ExPolygon {
                contour: slicer::types::geometry::Polygon {
                    points: vec![
                        slicer::types::geometry::Point2 { x: 0, y: 0 },
                        slicer::types::geometry::Point2 { x: 1000, y: 0 },
                        slicer::types::geometry::Point2 { x: 1000, y: 1000 },
                    ],
                },
                holes: Vec::new(),
            };
            output
                .set_polygons(&key, &[polygon])
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
