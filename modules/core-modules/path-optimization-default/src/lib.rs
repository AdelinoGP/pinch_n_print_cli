//! Default path-optimization core module.
//!
//! Implements `LayerModule::run_path_optimization` for the canonical
//! `Layer::PathOptimization` stage (docs/04 §Fixed Stage Order).
//!
//! # MVP scope
//!
//! Full travel optimisation (nearest-neighbor / TSP / seam scheduling
//! à la OrcaSlicer's `GCode::_do_export` + `LayerTools` +
//! `SeamPlacer`) is deferred — see the Step D handoff deviations list
//! for the concrete set of OrcaSlicer `PathOptimization`-family
//! passes that are NOT yet implemented here. This first canonical
//! module:
//!
//! 1. Runs in the real `Layer::PathOptimization` slot against real
//!    `PerimeterRegionView` content,
//! 2. Emits one deterministic `; path-optimization layer <n>
//!    regions=<r> entities=<e>` Comment per layer via
//!    `GcodeOutputBuilder::push_comment`, which the commit path
//!    routes into `LayerCollectionIR.annotations` (anchor = last
//!    ordered entity) and the default G-code emitter turns into a
//!    real `; ...` line in the emitted file.
//!
//! Determinism: the emitted string is a pure function of the layer
//! index and the guest-visible region/entity counts, so two
//! identical runs produce byte-identical G-code output.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_sdk::error::ModuleError;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;
use slicer_ir::{ConfigValue, ConfigView};

/// Default path-optimization module.
pub struct PathOptimizationDefault {
    /// Whether to emit the per-layer marker comment. Defaults to
    /// `true` so the stage is observable in the generated G-code;
    /// byte-size-sensitive presets can disable it via the manifest
    /// config key `path_optimization_emit_layer_markers`.
    emit_layer_markers: bool,
}

#[slicer_module]
impl LayerModule for PathOptimizationDefault {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let emit_layer_markers = match config.get("path_optimization_emit_layer_markers") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };
        Ok(Self { emit_layer_markers })
    }

    fn run_path_optimization(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.emit_layer_markers {
            return Ok(());
        }

        // Count the total number of wall-loop "entities" observable
        // through the region view. PerimeterRegionView is read-only
        // so the guest just tallies; the host has already frozen
        // the real `ordered_entities` into the layer collection at
        // this point (see layer_executor.rs pre-stage for
        // Layer::PathOptimization).
        let region_count = regions.len();
        let entity_count: usize = regions.iter().map(|r| r.wall_loops().len()).sum();

        let marker = format!(
            "path-optimization layer {layer_index} regions={region_count} entities={entity_count}"
        );
        output
            .push_comment(marker)
            .map_err(|e| ModuleError::fatal(1, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn defaults_emit_layer_markers_true() {
        let config = ConfigView::from_map(HashMap::new());
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        assert!(module.emit_layer_markers);
    }

    #[test]
    fn explicit_false_config_disables_markers() {
        let mut fields: HashMap<String, ConfigValue> = HashMap::new();
        fields.insert(
            "path_optimization_emit_layer_markers".into(),
            ConfigValue::Bool(false),
        );
        let config = ConfigView::from_map(fields);
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        assert!(!module.emit_layer_markers);
    }
}
