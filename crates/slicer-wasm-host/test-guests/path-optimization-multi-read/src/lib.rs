wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:layer-path-optimization/path-optimization-module",
    generate_all,
});

use exports::slicer::layer_path_optimization::path_optimization::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::ir_handles::ir_handles::{
    GcodeOutputBuilder, LayerCollectionBuilder, LayerIdx, PerimeterRegionView,
};

struct Component;

impl Guest for Component {
    fn run(
        _layer_index: LayerIdx,
        _regions: Vec<PerimeterRegionView>,
        _output: GcodeOutputBuilder,
        collection: LayerCollectionBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        let first_len = collection.get_ordered_entities().len();
        for _ in 0..4 {
            assert_eq!(
                collection.get_ordered_entities().len(),
                first_len,
                "path-optimization-multi-read: snapshot drifted across calls"
            );
        }
        Ok(())
    }
}

export!(Component);
