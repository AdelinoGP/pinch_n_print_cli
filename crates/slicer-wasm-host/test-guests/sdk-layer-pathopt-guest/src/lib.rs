use slicer_ir::ConfigView;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

pub struct SdkLayerPathoptGuest;

#[slicer_module]
impl LayerModule for SdkLayerPathoptGuest {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let comment = format!(
            "regions={} walls={} infill={}",
            regions.len(),
            regions.iter().map(|region| region.wall_loops().len()).sum::<usize>(),
            regions.iter().map(|region| region.infill_areas().len()).sum::<usize>(),
        );
        output
            .push_comment(comment)
            .map_err(|message| ModuleError::fatal(1, message))?;

        for index in 0..regions.len() as u32 {
            output
                .push_tool_change(index, index, index + 1)
                .map_err(|message| ModuleError::fatal(1, message))?;
            output
                .push_z_hop(0, 0.5)
                .map_err(|message| ModuleError::fatal(1, message))?;
        }

        Ok(())
    }
}