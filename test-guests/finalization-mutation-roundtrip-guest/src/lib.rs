//! TDD guest for AC-5 / NEG-3: EntityMutation round-trip.
//!
//! Strategy: (a) — reads `config.get_int("target_entity_id")` at runtime.
//! If set, targets that entity_id; otherwise defaults to entity_id=1.
//!
//! AC-5 fixture: no config override → targets entity_id=1, SetSpeedFactor(0.5).
//! NEG-3 fixture: config has `target_entity_id=99` → targets entity_id=99,
//!   which does not exist; host's apply_to() surfaces the structured error
//!   containing "entity_id" and "99".

use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use slicer_ir::ConfigView;

pub struct FinalizationMutationRoundtripGuest;

#[slicer_module]
impl FinalizationModule for FinalizationMutationRoundtripGuest {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Strategy (a): use config-driven entity_id so one guest binary
        // serves both AC-5 (entity_id=1) and NEG-3 (entity_id=99).
        let target_entity_id: u64 = config
            .get_int("target_entity_id")
            .map(|v| v as u64)
            .unwrap_or(1);

        for layer in layers {
            let layer_index = layer.layer_index();
            output
                .modify_entity(layer_index, target_entity_id, EntityMutation::SetSpeedFactor(0.5))
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        Ok(())
    }
}
