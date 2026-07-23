#![allow(missing_docs)]

pub mod connect;
pub mod graph;
pub mod offset;
pub mod orchestrate;

use slicer_ir::{ConfigValue, ConfigView, InfillRegion};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

#[derive(Debug)]
pub struct InfillLinker {
    infill_overlap: f64,
    line_width: f32,
}

impl InfillLinker {
    #[must_use]
    pub fn infill_overlap(&self) -> f64 {
        self.infill_overlap
    }

    fn copy_ironing(
        region: &InfillRegion,
        output: &mut InfillOutputBuilder,
    ) -> Result<(), ModuleError> {
        output.begin_region(&region.object_id, region.region_id);
        for path in &region.ironing {
            output
                .push_ironing_path(path.clone())
                .map_err(|error| ModuleError::fatal(1, error))?;
        }
        Ok(())
    }
}

#[slicer_module]
impl LayerModule for InfillLinker {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let infill_overlap = match config.get("infill_overlap") {
            Some(ConfigValue::Float(value))
                if value.is_finite() && *value >= 0.0 && *value <= 1.0 =>
            {
                *value
            }
            Some(ConfigValue::Int(value)) if (0..=1).contains(value) => *value as f64,
            _ => 0.45,
        };
        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(value)) if value.is_finite() && *value > 0.0 => *value as f32,
            Some(ConfigValue::Int(value)) if *value > 0 => *value as f32,
            _ => 0.4,
        };
        Ok(Self {
            infill_overlap,
            line_width,
        })
    }

    fn run_infill_postprocess(
        &self,
        _layer_index: u32,
        regions: &[PerimeterRegionView],
        prior_infill: &[InfillRegion],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let spacing_mm = match config.get("line_width") {
            Some(ConfigValue::Float(value)) if value.is_finite() && *value > 0.0 => *value as f32,
            Some(ConfigValue::Int(value)) if *value > 0 => *value as f32,
            _ => self.line_width,
        };
        orchestrate::orchestrate_infill(
            prior_infill,
            regions,
            self.infill_overlap as f32,
            spacing_mm,
            output,
        )
        .map_err(|error| ModuleError::fatal(1, error))?;
        for region in prior_infill {
            Self::copy_ironing(region, output)?;
        }
        Ok(())
    }
}
