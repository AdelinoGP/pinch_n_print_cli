wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:prepass-mesh-analysis/mesh-analysis-module",
    generate_all,
});

use exports::slicer::prepass_mesh_analysis::mesh_analysis::Guest;
use slicer::common::module_errors::ModuleError;
use slicer::config::config_types::ConfigView;
use slicer::prepass_mesh_analysis::mesh_analysis_types::{MeshAnalysisOutput, ObjectId};

struct Component;

impl Guest for Component {
    fn run(
        _objects: Vec<ObjectId>,
        _output: MeshAnalysisOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
