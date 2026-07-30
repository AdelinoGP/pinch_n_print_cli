wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-perimeters/perimeters-module",
    generate_all,
});

use exports::slicer::layer_perimeters::perimeters::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{
    LayerIdx, PaintRegionLayerView, PerimeterOutputBuilder, SliceRegionView,
};

struct Component;

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        _regions: Vec<SliceRegionView>,
        _paint: PaintRegionLayerView,
        _output: PerimeterOutputBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
