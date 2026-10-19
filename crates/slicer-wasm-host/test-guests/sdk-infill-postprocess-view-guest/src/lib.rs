use slicer_sdk::prelude::*;

pub struct SdkInfillPostprocessViewGuest;

#[slicer_module]
impl LayerModule for SdkInfillPostprocessViewGuest {
    fn from_config(_: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill_postprocess(
        &self,
        _: u32,
        regions: &[PerimeterRegionView],
        _: &[slicer_ir::InfillRegion],
        _: &mut InfillOutputBuilder,
        _: &ConfigView,
    ) -> Result<(), ModuleError> {
        let mut witness = String::from("SDK-VIEW");
        for region in regions {
            witness.push_str(&format!(
                " region={}:{} chain={:?} anchor={:?} walls={:?} candidates={:?} resolved={:?} infill={:?} sparse={:?} top={:?} bottom={:?} bridge={:?} raft={:?} tool={} donor={:?} height={:?} density={:?}",
                region.object_id(), region.region_id(), region.variant_chain(),
                region.config().and_then(|config| config.get("infill_anchor_max")),
                region.wall_loops(), region.seam_candidates(), region.resolved_seam(),
                region.infill_areas(), region.sparse_infill_area(), region.top_solid_fill(),
                region.bottom_solid_fill(), region.bridge_areas(), region.raft_fill(),
                region.tool_index(), region.wall_source_region_id(),
                region.config().and_then(|config| config.get("layer_height")),
                region.config().and_then(|config| config.get("infill_density")),
            ));
        }
        Err(ModuleError::fatal(99, witness))
    }
}
