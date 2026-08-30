//! TDD guest for the anchored-events WIT round trip.

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    ConfigView, OrderedEventCollection, Point3,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

pub struct AnchoredEventsRoundtripGuest;

#[slicer_module]
impl LayerModule for AnchoredEventsRoundtripGuest {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_anchored_events(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        collection: &mut LayerCollectionBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let count = config.get_int("anchored_event_count").unwrap_or(2);
        let malformed = config.get_int("emit_malformed_geometry").unwrap_or(0);
        let duplicate = config.get_int("duplicate_proposal").unwrap_or(0);
        if count == 0 {
            return Ok(());
        }

        let bad_z = if malformed != 0 { 9000.0 } else { 0.3 };
        let first_geometry = if malformed == 2 {
            AnchoredGeometryContract::ZSpanning {
                min_z: 3000,
                max_z: 5000,
            }
        } else {
            AnchoredGeometryContract::Planar { z: 3000 }
        };
        let second_geometry = AnchoredGeometryContract::ZSpanning {
            min_z: 3000,
            max_z: 5000,
        };
        let event = |local_id, geometry, z| AnchoredEntity {
            local_id,
            anchor_global_layer_index: 7,
            geometry,
            input_capabilities: vec!["support.plan".to_string()],
            output_capabilities: vec!["extrusion.paths".to_string(), "cooling.account".to_string()],
            provenance: AnchoredEntityProvenance {
                requesting_feature: "same-z-support".to_string(),
                source_plan_entry: "plan-entry-4".to_string(),
            },
            path_points: vec![Point3 { x: 1.0, y: 1.0, z }, Point3 { x: 2.0, y: 2.0, z }],
        };
        let proposal = OrderedEventCollection {
            anchor_global_layer_index: 7,
            events: vec![
                event(0, first_geometry, bad_z),
                event(1, second_geometry, 0.3),
            ],
            runtime_hooks: AnchoredEventRuntimeHooks {
                optimize_paths: false,
                account_cooling: true,
                account_time: false,
            },
        };
        collection
            .set_anchored_event_collection(proposal.clone())
            .map_err(|e| ModuleError::fatal(1, e))?;
        if duplicate != 0 {
            collection
                .set_anchored_event_collection(proposal)
                .map_err(|e| ModuleError::fatal(1, e))?;
        }
        Ok(())
    }
}
