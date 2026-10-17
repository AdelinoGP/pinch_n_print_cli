//! Same-source native/component reader of a module-private prepared path.

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, ConfigView, ExtrusionRole,
    OrderedEventCollection, Point3WithWidth,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::preparation::{
    LayerPreparation, PreparedPlanOutput, PreparedPlanView, PrintPreparationInput,
};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

pub struct PreparationLifecycleAnchored;

fn endpoint(config: &ConfigView, key: &str, default: i64) -> Result<u16, ModuleError> {
    u16::try_from(config.get_int(key).unwrap_or(default))
        .map_err(|_| ModuleError::fatal(57403, format!("{key} must fit u16 millimeters")))
}

fn decode_probe(bytes: &[u8]) -> Result<[u16; 2], ModuleError> {
    let [a, b, c, d] = bytes else {
        return Err(ModuleError::fatal(57403, "malformed anchored probe"));
    };
    let endpoints = [u16::from_le_bytes([*a, *b]), u16::from_le_bytes([*c, *d])];
    if endpoints[0] >= endpoints[1] {
        return Err(ModuleError::fatal(
            57403,
            "anchored probe requires increasing X",
        ));
    }
    Ok(endpoints)
}

impl LayerPreparation for PreparationLifecycleAnchored {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        // Read committed layers and their selected targets, never an invocation
        // instance or a host-defined interpretation of this private payload.
        let mut selected_config = None;
        for layer in input
            .layers(0, u64::MAX)
            .map_err(|error| ModuleError::fatal(57402, format!("layers: {error:?}")))?
        {
            if let Some(target) = input
                .selected_targets(layer.index, 0, 1)
                .map_err(|error| ModuleError::fatal(57402, format!("selected_targets: {error:?}")))?
                .into_iter()
                .next()
            {
                selected_config = Some(target.config);
                break;
            }
        }
        let config = selected_config
            .ok_or_else(|| ModuleError::fatal(57402, "anchored fixture needs a selected target"))?;
        match config.get_string("probe_mode").unwrap_or("standard") {
            "module_error" => {
                return Err(ModuleError::fatal(57402, "anchored preparation failure"))
            }
            "trap" => panic!("anchored preparation trap"),
            "malformed_probe" => {
                output.put("probe", &[3])?;
                return Ok(());
            }
            "standard" => {}
            _ => return Err(ModuleError::fatal(57402, "unknown probe_mode")),
        }
        let start = endpoint(&config, "probe_start_mm", 3)?.to_le_bytes();
        let end = endpoint(&config, "probe_end_mm", 4)?.to_le_bytes();
        let bytes = [start[0], start[1], end[0], end[1]];
        decode_probe(&bytes)?;
        output.put("probe", &bytes)?;
        Ok(())
    }
}

#[slicer_module(preparation)]
impl LayerModule for PreparationLifecycleAnchored {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_anchored_events(
        &self,
        layer_index: u32,
        _regions: &[SliceRegionView],
        collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let piece = PreparedPlanView::current()?.require("probe")?;
        // Read one byte beyond the codec's length so oversized pieces fail too.
        let endpoints = decode_probe(&piece.read(0, 5)?)?;
        let path_points = endpoints
            .into_iter()
            .map(|x| Point3WithWidth {
                // Point3WithWidth stores millimeters, unlike planar contract Z.
                x: f32::from(x),
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect();
        collection
            .set_anchored_event_collection(OrderedEventCollection {
                anchor_global_layer_index: layer_index,
                events: vec![AnchoredEntity {
                    // exhaustive: fixture specifies complete anchored identity and geometry
                    // The module can emit this planar event at multiple anchor
                    // layers, whose Z=0 events are coalesced into one output row.
                    local_id: 57_403 + u64::from(layer_index),
                    anchor_global_layer_index: layer_index,
                    geometry: AnchoredGeometryContract::Planar { z: 0 },
                    input_capabilities: vec![],
                    output_capabilities: vec![],
                    provenance: AnchoredEntityProvenance {
                        requesting_feature: "preparation_lifecycle_fixture".into(),
                        source_plan_entry: "probe".into(),
                    },
                    path_points,
                    role: ExtrusionRole::SupportMaterial,
                }],
                runtime_hooks: Default::default(),
            })
            .map_err(|message| ModuleError::fatal(57404, message))
    }
}
