//! Whole-print preparation witness paired with real Support only.

use std::cell::RefCell;

use slicer_ir::ConfigView;
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::preparation::{LayerPreparation, PreparedPlanOutput, PrintPreparationInput};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

#[path = "../../whole-print-input-path/src/observations.rs"]
pub mod observations;

pub struct WholePrintInputSupport;

thread_local! {
    // Preparation is an associated hook, so park the wrapper's configuration
    // here rather than trying to recover an instance from the ordinary stage.
    static INVOCATION_CONFIG: RefCell<Option<ConfigView>> = const { RefCell::new(None) };
}

fn fallback_controls(input: &PrintPreparationInput) -> Option<ConfigView> {
    // The observer repeats these standard reads and records their ordinary
    // error lines. Failure to discover controls must not prevent that report.
    slicer_sdk::host::log_info("wpi read layers");
    let layers = input.layers(0, u64::MAX).ok()?;
    for layer in layers {
        slicer_sdk::host::log_info("wpi read region_configs");
        let config = input
            .region_configs(layer.index, 0, u64::MAX)
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.config)
            .or_else(|| {
                slicer_sdk::host::log_info("wpi read selected_targets");
                input
                    .selected_targets(layer.index, 0, u64::MAX)
                    .ok()
                    .and_then(|rows| rows.into_iter().next())
                    .map(|row| row.config)
            });
        if let Some(config) = config {
            // Preserve every key already visible in this filtered config view.
            let fields = config
                .iter_entries()
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect();
            return Some(ConfigView::from_map(fields));
        }
    }
    None
}

impl LayerPreparation for WholePrintInputSupport {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        let invocation_config = INVOCATION_CONFIG
            .with(|config| config.borrow().clone())
            .or_else(|| fallback_controls(input));
        let mode = invocation_config
            .as_ref()
            .map(observations::FixtureMode::from_config)
            .transpose()?
            .unwrap_or_default();
        observations::observe_all(input, output, mode, invocation_config.as_ref())
    }
}

#[slicer_module(preparation)]
impl LayerModule for WholePrintInputSupport {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        INVOCATION_CONFIG.with(|stored| *stored.borrow_mut() = Some(config.clone()));
        Ok(Self)
    }

    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Preparation observations are the witness; ordinary builders stay empty.
        Ok(())
    }
}
