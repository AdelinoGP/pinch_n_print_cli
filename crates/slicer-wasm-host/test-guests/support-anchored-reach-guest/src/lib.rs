//! TDD guest proving that support-stage anchored output reaches the host drain.

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    ConfigView, ExtrusionRole, OrderedEventCollection, Point3WithWidth,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

pub struct SupportAnchoredReachGuest;

#[slicer_module]
impl LayerModule for SupportAnchoredReachGuest {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        collection
            .set_anchored_event_collection(OrderedEventCollection {
                anchor_global_layer_index: 7,
                // exhaustive: this fixture specifies every anchored entity contract field.
                events: vec![AnchoredEntity {
                    local_id: 0,
                    anchor_global_layer_index: 7,
                    geometry: AnchoredGeometryContract::Planar { z: 1_234_567 },
                    input_capabilities: vec!["support.plan".to_string()],
                    output_capabilities: vec!["extrusion.paths".to_string()],
                    provenance: AnchoredEntityProvenance {
                        requesting_feature: "support-stage".to_string(),
                        source_plan_entry: "support-plan-entry".to_string(),
                    },
                    path_points: vec![
                        Point3WithWidth {
                            x: 1.0,
                            y: 1.0,
                            z: 123.4567,
                            width: 0.45,
                            flow_factor: 1.0,
                            ..Default::default()
                        },
                        Point3WithWidth {
                            x: 2.0,
                            y: 2.0,
                            z: 123.4567,
                            width: 0.45,
                            flow_factor: 1.0,
                            ..Default::default()
                        },
                    ],
                    role: ExtrusionRole::SupportMaterial,
                }],
                runtime_hooks: AnchoredEventRuntimeHooks::default(),
            })
            .map_err(|e| ModuleError::fatal(1, e))
    }
}
