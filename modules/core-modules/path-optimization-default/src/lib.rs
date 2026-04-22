//! Default path-optimization core module.
//!
//! Implements `LayerModule::run_path_optimization` for the canonical
//! `Layer::PathOptimization` stage (docs/04 §Fixed Stage Order).
//!
//! # Marker-comment-only output
//!
//! PerimeterIR is already correctly rotated by seam-placer during
//! `Layer::WallPostProcess`. PathOptimization reads seam-first geometry
//! but emits only a per-layer marker comment. No push-move calls are made.

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
    #[allow(dead_code)]
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

    /// TODO TASK-151: implement actual path-optimization logic here
    /// (e.g. nearest-neighbor travel ordering, cross-layer chaining, etc.)
    fn run_path_optimization(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // PerimeterIR is already correctly rotated by seam-placer during
        // Layer::WallPostProcess — path.optimization reads seam-first geometry
        // but emits only a marker comment (no push-move calls).
        // PathOptimization is comment-only; no wall loop replay needed.

        if self.emit_layer_markers {
            let region_count = regions.len();
            let entity_count: usize = regions.iter().map(|r| r.wall_loops().len()).sum();
            let marker = format!(
                "path-optimization layer {layer_index} regions={region_count} entities={entity_count}"
            );
            output
                .push_comment(marker)
                .map_err(|e| ModuleError::fatal(1, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use slicer_sdk::postpass_builders::GcodeOutputBuilder;
    use slicer_sdk::traits::LayerModule;

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

    #[test]
    fn disabled_markers_emit_no_comments() {
        let mut fields: HashMap<String, ConfigValue> = HashMap::new();
        fields.insert(
            "path_optimization_emit_layer_markers".into(),
            ConfigValue::Bool(false),
        );
        let config = ConfigView::from_map(fields);
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        let mut output = GcodeOutputBuilder::new();

        module
            .run_path_optimization(3, &[], &mut output, &config)
            .expect("path optimization should succeed with markers disabled");

        assert!(
            output.commands().is_empty(),
            "emit_layer_markers=false must suppress all marker comments"
        );
    }
}
