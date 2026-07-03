//! Pure consumer of the per-vertex `overhang_quartile` annotation written by
//! the upstream PrePass::OverhangAnnotation pipeline (ADR-0031, packet 106):
//! applies speed-factor mutations. No local geometric computation.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView,
};

/// Core overhang classifier that applies speed-factor mutations to wall entities on overhangs.
pub struct OverhangClassifierDefault;

/// Config float for `key`, defaulting to 0.0.
fn speed(config: &ConfigView, key: &str) -> f32 {
    config.get_float(key).unwrap_or(0.0) as f32
}

/// Base wall speed for `role` (0.0 for non-wall roles).
fn base_speed(role: &ExtrusionRole, config: &ConfigView) -> f32 {
    match role {
        ExtrusionRole::OuterWall => speed(config, "outer_wall_speed"),
        ExtrusionRole::InnerWall => speed(config, "inner_wall_speed"),
        ExtrusionRole::ThinWall => speed(config, "thin_wall_speed"),
        _ => 0.0,
    }
}

/// Overhang speed for `quartile` (1..=4), 0.0 otherwise.
fn overhang_speed(quartile: u8, config: &ConfigView) -> f32 {
    match quartile {
        1 => speed(config, "overhang_1_4_speed"),
        2 => speed(config, "overhang_2_4_speed"),
        3 => speed(config, "overhang_3_4_speed"),
        4 => speed(config, "overhang_4_4_speed"),
        _ => 0.0,
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
        if (1..=4).all(|q| overhang_speed(q, config) == 0.0) {
            return Ok(());
        }
        for layer in layers {
            for entity in layer.ordered_entities() {
                // MAX per-vertex quartile: the most severe overhang governs the whole segment.
                let pts = entity.path.points.iter();
                let Some(q) = pts.filter_map(|p| p.overhang_quartile).max() else {
                    continue;
                };
                let base = base_speed(&entity.role, config);
                if base <= 0.0 {
                    continue;
                }
                let mutation = EntityMutation::SetSpeedFactor(overhang_speed(q, config) / base);
                output
                    .modify_entity(layer.layer_index(), entity.entity_id, mutation)
                    .map_err(ModuleError::from_str)?;
            }
        }
        Ok(())
    }
}
