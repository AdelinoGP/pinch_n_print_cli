//! Arachne perimeter generator module (M2 foundations skeleton).
//!
//! Implements `LayerModule::run_perimeters` for the `Layer::Perimeters` stage
//! as an empty, non-functional placeholder. This module exists so the DAG
//! scheduler can validate `incompatible-with` semantics against
//! `classic-perimeters`, and so later packets (P111/P112) have a module to
//! build real Arachne (variable-width, Voronoi-based) wiring into.
//!
//! No walls are produced yet — every invocation emits a warning via
//! `slicer_sdk::host::log_warn` and returns `Ok(())`.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::ConfigView;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Arachne perimeter generator (skeleton).
///
/// Holds no state. Real Arachne variable-width wall generation ships in
/// P112; this stub only proves the module loads, participates in DAG
/// validation, and satisfies the `LayerModule` trait contract.
pub struct ArachnePerimeters;

#[slicer_module]
impl LayerModule for ArachnePerimeters {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_perimeters(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint_regions: &PaintRegionLayerView,
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        slicer_sdk::host::log_warn(
            "arachne-perimeters skeleton loaded — no walls produced; real impl ships in P112",
        );
        Ok(())
    }
}
