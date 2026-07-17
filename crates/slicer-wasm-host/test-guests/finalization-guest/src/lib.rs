wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:world-finalization/finalization-module",
    generate_all,
});

struct Component;

impl Guest for Component {
    fn run_finalization(
        _layers: Vec<LayerCollectionView>,
        _output: FinalizationOutputBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        slicer::common::host_services::log(
            slicer::common::host_services::LogLevel::Info,
            "run-finalization: ok",
        );
        Ok(())
    }
}

export!(Component);
