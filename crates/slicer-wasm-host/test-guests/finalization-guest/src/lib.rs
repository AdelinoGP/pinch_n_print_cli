wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:finalization-layer-finalization/layer-finalization-module",
    generate_all,
});

use exports::slicer::finalization_layer_finalization::layer_finalization::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::finalization_layer_finalization::layer_finalization_types::{
    FinalizationOutputBuilder, LayerCollectionView,
};

struct Component;

impl Guest for Component {
    fn run(
        _layers: Vec<LayerCollectionView>,
        _output: FinalizationOutputBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        slicer::common::host_services::log(
            slicer::common::host_services::LogLevel::Info,
            "run: ok",
        );
        Ok(())
    }
}

export!(Component);
