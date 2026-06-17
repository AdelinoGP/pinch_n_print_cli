// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Core overhang classification module.
//!
//! Applies speed-factor mutations to wall entities on overhangs.

#![warn(missing_docs)]
#![warn(unused_imports)]

mod classify;
mod lines_distancer;

use classify::classify_layers;
use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView,
};

/// Core overhang classifier that applies speed-factor mutations to wall entities on overhangs.
pub struct OverhangClassifierDefault;

impl OverhangClassifierDefault {
    /// Returns the base wall speed for the given role from config.
    fn base_speed_for_role(role: &ExtrusionRole, config: &ConfigView) -> f64 {
        let key = match role {
            ExtrusionRole::OuterWall => "outer_wall_speed",
            ExtrusionRole::InnerWall => "inner_wall_speed",
            ExtrusionRole::ThinWall => "thin_wall_speed",
            _ => return 0.0,
        };
        config.get_float(key).unwrap_or(0.0)
    }

    /// Returns the overhang speed for the given quartile from config.
    fn overhang_speed_for_quartile(quartile: u8, config: &ConfigView) -> f64 {
        let key = match quartile {
            1 => "overhang_1_4_speed",
            2 => "overhang_2_4_speed",
            3 => "overhang_3_4_speed",
            4 => "overhang_4_4_speed",
            _ => return 0.0,
        };
        config.get_float(key).unwrap_or(0.0)
    }
}

#[slicer_module]
impl FinalizationModule for OverhangClassifierDefault {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(OverhangClassifierDefault)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let oh1 = config.get_float("overhang_1_4_speed").unwrap_or(0.0) as f32;
        let oh2 = config.get_float("overhang_2_4_speed").unwrap_or(0.0) as f32;
        let oh3 = config.get_float("overhang_3_4_speed").unwrap_or(0.0) as f32;
        let oh4 = config.get_float("overhang_4_4_speed").unwrap_or(0.0) as f32;

        if oh1 == 0.0 && oh2 == 0.0 && oh3 == 0.0 && oh4 == 0.0 {
            return Ok(());
        }

        let entity_quartiles = classify_layers(layers);

        for ((layer_idx, entity_id), quartile) in &entity_quartiles {
            if *quartile >= 4 {
                continue;
            }

            let overhang_speed = Self::overhang_speed_for_quartile(*quartile, config) as f32;

            let mut base_speed: f32 = 0.0;
            for layer in layers {
                if layer.layer_index() == *layer_idx {
                    for entity in layer.ordered_entities() {
                        if entity.entity_id == *entity_id {
                            base_speed = Self::base_speed_for_role(&entity.role, config) as f32;
                            break;
                        }
                    }
                    break;
                }
            }

            if base_speed <= 0.0 {
                continue;
            }

            let factor = overhang_speed / base_speed;
            output
                .modify_entity(
                    *layer_idx,
                    *entity_id,
                    EntityMutation::SetSpeedFactor(factor),
                )
                .map_err(ModuleError::from_str)?;
        }

        Ok(())
    }
}
