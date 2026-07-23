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
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let infill_speed = match config.get("infill_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

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
                        dist_to_top_mm: 0.0,
                    })
                    .collect();

                let _ = output.push_sparse_path(ExtrusionPath3D {
                    points,
                    role: ExtrusionRole::SparseInfill,
                    speed_factor,
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
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = LightningInfill::on_print_start(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
