wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:prepass-layer-planning/layer-planning-module",
    generate_all,
});

use exports::slicer::prepass_layer_planning::layer_planning::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::prepass_layer_planning::layer_planning_types::{LayerPlanOutput, ObjectId};

struct Component;

impl Guest for Component {
    fn run(
        _objects: Vec<ObjectId>,
        _output: LayerPlanOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
