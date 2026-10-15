//! Controlled module-owned numeric-text codec for preparation transport.

use slicer_ir::ConfigView;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::preparation::{
    LayerPreparation, PreparedPlanOutput, PreparedPlanView, PrintPreparationInput,
};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

pub struct TransportPathModule;

fn decode_pair(bytes: &[u8]) -> Result<String, ModuleError> {
    let [first, second] = bytes else {
        return Err(ModuleError::fatal(
            57403,
            "transport fixture malformed payload",
        ));
    };
    Ok(format!("{first},{second}"))
}

impl LayerPreparation for TransportPathModule {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        let input_reads = input.read_fixture(0, 4)?;
        let Some(&mode) = input_reads.first() else {
            return Ok(());
        };
        match mode {
            254 => panic!("transport fixture trap"),
            253 => return Err(ModuleError::non_fatal(57402, "transport fixture failure")),
            250 => {
                return Err(ModuleError::fatal(
                    slicer_sdk::preparation::POISONED_PREPARATION_CODE,
                    "transport fixture reserved code",
                ));
            }
            _ => {}
        }

        if input_reads == [3] {
            output.put("numbers", &[3])?;
        } else {
            output.put("numbers", &[3, 5, 11, 17])?;
        }
        output.put("π", &[])?;

        match mode {
            255 => output.put("numbers", &[3, 5, 11, 17])?,
            252 => {
                let _ = output.put("numbers", &[3, 5, 11, 17]);
            }
            251 => {
                let _ = output.put("", &[]);
            }
            _ => {}
        }
        Ok(())
    }
}

#[slicer_module(preparation)]
impl LayerModule for TransportPathModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let plan = PreparedPlanView::current()?;
        let piece = plan.require("numbers")?;
        if piece.len()? != 4 {
            return Err(ModuleError::fatal(
                57403,
                "transport fixture malformed payload",
            ));
        }
        for offset in [0, 2] {
            let comment = decode_pair(&piece.read(offset, 2)?)?;
            output
                .push_comment(comment)
                .map_err(|message| ModuleError::fatal(57403, message))?;
        }
        Ok(())
    }
}
