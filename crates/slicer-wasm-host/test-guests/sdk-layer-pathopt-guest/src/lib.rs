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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        collection: &mut LayerCollectionBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if config.get_int("emit_view_witness") == Some(1) {
            let mut witness = String::from("SDK-VIEW");
            witness.push_str(&format!(
                " stage_alias={:?} stage_canonical={:?}",
                config.get("support_overhang_angle"),
                config.get("support_threshold_angle"),
            ));
            for region in regions {
                witness.push_str(&format!(
                    " region={}:{} chain={:?} anchor={:?}",
                    region.object_id(),
                    region.region_id(),
                    region.variant_chain(),
                    region
                        .config()
                        .and_then(|config| config.get("infill_anchor_max")),
                ));
                witness.push_str(&format!(
                    " alias={:?} canonical={:?}",
                    region
                        .config()
                        .and_then(|config| config.get("support_overhang_angle")),
                    region
                        .config()
                        .and_then(|config| config.get("support_threshold_angle")),
                ));
                for wall in region.wall_loops() {
                    witness.push_str(&format!(
                        " path_role={:?} path={:?} profile={:?}",
                        wall.path.role, wall.path.points, wall.width_profile.widths
                    ));
                }
                witness.push_str(&format!(
                    " candidates={:?} resolved={:?}",
                    region.seam_candidates(),
                    region.resolved_seam()
                ));
            }
            for entity in collection.get_ordered_entities() {
                witness.push_str(&format!(" ordered={:?}", entity));
            }
            return Err(ModuleError::fatal(99, witness));
        }
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
