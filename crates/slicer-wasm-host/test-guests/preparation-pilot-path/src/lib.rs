//! Controlled text preparation fixture using the real path-optimization family.

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

pub struct PreparationPilotPath {
    offset: u64,
    max_bytes: u64,
}

fn decode_text(bytes: &[u8]) -> Result<Vec<&str>, ModuleError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ModuleError::non_fatal(1, "malformed pilot text"))?;
    let words: Vec<_> = text.split('|').collect();
    if words.iter().any(|word| word.is_empty()) {
        return Err(ModuleError::non_fatal(
            1,
            "malformed pilot text: empty word",
        ));
    }
    Ok(words)
}

impl LayerPreparation for PreparationPilotPath {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        let bytes = input.read_fixture(0, 10)?;
        let words = decode_text(&bytes)?;
        output.put("text", words.join("|").as_bytes())?;
        Ok(())
    }
}

#[slicer_module(preparation)]
impl LayerModule for PreparationPilotPath {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let offset = u64::try_from(config.get_int("pilot_range_offset").unwrap_or(0))
            .map_err(|_| ModuleError::non_fatal(1, "negative pilot_range_offset"))?;
        let max_bytes = u64::try_from(config.get_int("pilot_range_max_bytes").unwrap_or(5))
            .map_err(|_| ModuleError::non_fatal(1, "negative pilot_range_max_bytes"))?;
        Ok(Self { offset, max_bytes })
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
        let piece = plan
            .open("text")?
            .ok_or_else(|| ModuleError::non_fatal(1, "required piece text missing"))?;
        let bytes = piece.read(self.offset, self.max_bytes)?;
        for word in decode_text(&bytes)? {
            output
                .push_comment(word.to_owned())
                .map_err(|message| ModuleError::fatal(1, message))?;
        }
        Ok(())
    }
}
