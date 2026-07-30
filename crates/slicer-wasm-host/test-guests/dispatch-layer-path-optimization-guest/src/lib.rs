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
        regions: Vec<PerimeterRegionView>,
        output: GcodeOutputBuilder,
        _collection: LayerCollectionBuilder,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        let comment = format!(
            "regions={} walls={} infill={}",
            regions.len(),
            regions
                .iter()
                .map(|region| region.wall_loops().len())
                .sum::<usize>(),
            regions
                .iter()
                .map(|region| region.infill_areas().len())
                .sum::<usize>(),
        );
        output
            .push_comment(&comment)
            .map_err(|message| ModuleError {
                code: 1,
                message,
                fatal: true,
            })?;
        for index in 0..regions.len() as u32 {
            output
                .push_tool_change(index, index, index + 1)
                .map_err(|message| ModuleError {
                    code: 2,
                    message,
                    fatal: true,
                })?;
            output.push_z_hop(0, 0.5).map_err(|message| ModuleError {
                code: 3,
                message,
                fatal: true,
            })?;
        }
        Ok(())
    }
}

export!(Component);
