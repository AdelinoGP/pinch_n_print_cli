// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Fill/Lightning/Generator.cpp
// Original code owner: Ultimaker B.V. (Copyright (c) 2021 Ultimaker B.V.)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Lightning sparse infill sampler module.
//!
//! The canonical lightning algorithm commits tree-edge segments to the layer
//! paint view. This module samples those segments into raw sparse-infill paths.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Lightning sparse infill sampler.
pub struct LightningInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Infill print speed in mm/s.
    infill_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

impl LightningInfill {
    /// Returns the configured infill density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

#[slicer_module]
impl LayerModule for LightningInfill {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let infill_speed = match config.get("infill_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let width = |key: &str, fallback: f32| match config.get(key) {
            Some(ConfigValue::Float(w)) => *w as f32,
            Some(ConfigValue::Int(w)) => *w as f32,
            _ => fallback,
        };

        let line_width = slicer_core::flow::resolve_role_width(
            ExtrusionRole::SparseInfill,
            false,
            false,
            &slicer_core::flow::RoleWidthContext {
                // Packet 185 (AC-5): absent `line_width` is the canonical
                // auto-0 sentinel (1.125 × nozzle), not the legacy 0.4 mm.
                line_width: width("line_width", 0.0),
                nozzle_diameter: 0.4,
                bridge_line_width: width("bridge_line_width", 0.0),
                initial_layer_line_width: width("initial_layer_line_width", 0.0),
                sparse_infill_line_width: width("sparse_infill_line_width", 0.0),
                ..Default::default()
            },
        );

        Ok(Self {
            density,
            infill_speed,
            line_width,
        })
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let speed_factor = self.infill_speed / BASE_SPEED;

        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            if !region.should_emit(ExtrusionRole::SparseInfill) {
                continue;
            }

            let z = region.z();
            for segment in
                _paint.lightning_tree_segments_for(region.object_id(), *region.region_id())
            {
                let points = segment
                    .into_iter()
                    .map(|point| Point3WithWidth {
                        x: slicer_ir::units_to_mm(point.x),
                        y: slicer_ir::units_to_mm(point.y),
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        overhang_distance_mm: None,
                        dist_to_top_mm: 0.0,
                    })
                    .collect();

                let _ = output.push_sparse_path(ExtrusionPath3D {
                    points,
                    role: ExtrusionRole::SparseInfill,
                    speed_factor,
                    tool_index: None,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = LightningInfill::from_config(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        // Packet 185 (AC-5): absent line_width resolves to the canonical
        // auto width 1.125 × nozzle_diameter (0.45 at the module's fixed
        // 0.4 mm nozzle), not the legacy 0.4 mm default.
        assert!((module.line_width - 0.45).abs() < 0.001);
    }
}
