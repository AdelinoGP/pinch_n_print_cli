//! Same-source native/component lifecycle witness with a private probe codec.

use slicer_ir::ConfigView;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::preparation::{
    LayerPreparation, PreparationInputError, PreparedPlanOutput, PreparedPlanView,
    PrintPreparationInput,
};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

pub struct PreparationLifecycleGuest {
    ready_empty: bool,
}

#[cfg(target_arch = "wasm32")]
const PROBE: [u8; 4] = *b"WPRB";
#[cfg(not(target_arch = "wasm32"))]
const PROBE: [u8; 4] = *b"NPRB";

fn input_error(error: PreparationInputError) -> ModuleError {
    ModuleError::fatal(57402, format!("lifecycle fixture input: {error:?}"))
}

// Independent module-owned format: a leg tag followed by the literal PRB.
// Reading a fifth byte below rejects oversized pieces as well as short ones.
fn decode_probe(bytes: &[u8]) -> Result<&'static str, ModuleError> {
    match bytes {
        b"NPRB" => Ok("PREPARATION-LIFECYCLE-NATIVE"),
        b"WPRB" => Ok("PREPARATION-LIFECYCLE-WASM"),
        _ => Err(ModuleError::fatal(
            57403,
            "lifecycle fixture malformed payload",
        )),
    }
}

impl LayerPreparation for PreparationLifecycleGuest {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        // Exercise the committed typed facade, not the pilot read-fixture seam.
        let layers = input.layers(0, u64::MAX).map_err(input_error)?;
        input.mesh_objects(0, u64::MAX).map_err(input_error)?;
        input.surface_objects(0, u64::MAX).map_err(input_error)?;
        input.support_analysis(0, u64::MAX).map_err(input_error)?;
        input.support_geometry(0, u64::MAX).map_err(input_error)?;
        input.support_plan(0, u64::MAX).map_err(input_error)?;
        let mut selected_config = None;
        for layer in layers {
            input
                .regions(layer.index, 0, u64::MAX)
                .map_err(input_error)?;
            input
                .region_configs(layer.index, 0, u64::MAX)
                .map_err(input_error)?;
            input
                .surface_layers(layer.index, 0, u64::MAX)
                .map_err(input_error)?;
            input.seams(layer.index, 0, u64::MAX).map_err(input_error)?;
            for target in input
                .selected_targets(layer.index, 0, u64::MAX)
                .map_err(input_error)?
            {
                if selected_config.is_none() {
                    selected_config = Some(target.config);
                }
            }
        }
        let config = selected_config.ok_or_else(|| {
            ModuleError::fatal(57402, "lifecycle fixture needs a selected target")
        })?;
        let mode = config
            .get_string("preparation_probe_mode")
            .unwrap_or("ready");
        match mode {
            "ready_empty" => return Ok(()),
            "module_error" => {
                return Err(ModuleError {
                    code: 57451,
                    message: "prepare rejected".into(),
                    fatal: false,
                });
            }
            "panic" | "trap" => panic!("lifecycle fixture preparation trap"),
            "ready" | "required_decoder" | "duplicate_put" => {}
            _ => return Err(ModuleError::fatal(57402, "unknown preparation_probe_mode")),
        }
        let token = config
            .get_string("preparation_probe_token")
            .unwrap_or("probe");
        let bytes = match config.get_int("preparation_probe_length").unwrap_or(4) {
            4 => &PROBE[..],
            2 => &PROBE[..2],
            _ => {
                return Err(ModuleError::fatal(
                    57402,
                    "preparation_probe_length must be 2 or 4",
                ))
            }
        };
        output.put(token, bytes)?;
        if mode == "duplicate_put" {
            // Deliberately catch the write error: successful module completion
            // must still fail publication because the transaction is poisoned.
            if output.put(token, bytes).is_ok() {
                return Err(ModuleError::fatal(
                    57402,
                    "duplicate put unexpectedly accepted",
                ));
            }
        }
        Ok(())
    }
}

#[slicer_module(preparation)]
impl LayerModule for PreparationLifecycleGuest {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        // Ordinary calls reconstruct this value; no prepared invocation instance
        // or thread-local config is retained between calls or between prints.
        Ok(Self {
            ready_empty: config.get_string("preparation_probe_mode") == Some("ready_empty"),
        })
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let view = PreparedPlanView::current()?;
        if view.open("optional")?.is_some() {
            return Err(ModuleError::fatal(
                57403,
                "lifecycle fixture optional unexpectedly present",
            ));
        }
        let marker = if self.ready_empty {
            "PREPARATION-LIFECYCLE-READY-EMPTY"
        } else {
            // Each ordinary call obtains its own fresh view and required piece.
            let piece = view.require("probe")?;
            let marker = decode_probe(&piece.read(0, 5)?)?;
            // Independent offsets and a newly opened view must not share a
            // cursor with the first reader or another concurrent invocation.
            let fresh = PreparedPlanView::current()?.require("probe")?;
            if fresh.read(1, 3)? != b"PRB" || fresh.read(0, 1)? != PROBE[..1] {
                return Err(ModuleError::fatal(
                    57403,
                    "lifecycle fixture fresh read mismatch",
                ));
            }
            marker
        };
        output
            .push_comment("PREPARATION-LIFECYCLE-OPTIONAL-NONE".to_owned())
            .map_err(|message| ModuleError::fatal(57404, message))?;
        output
            .push_comment(marker.to_owned())
            .map_err(|message| ModuleError::fatal(57404, message))
    }
}
