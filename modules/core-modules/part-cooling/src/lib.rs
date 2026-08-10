// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/CoolingBuffer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Part cooling fan module for M106/M107 emission.
//!
//! Runs in the `PostPass::LayerFinalization` stage, operating on the full
//! set of `LayerCollectionIR` outputs. Emits fan speed commands per layer.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, LayerAnnotation, LayerAnnotationKind};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Part cooling fan command generator.
pub struct PartCooling {
    fan_max_speed: u8,
    close_fan_the_first_x_layers: u32,
    enable_overhang_bridge_fan: bool,
    overhang_fan_speed: u8,
}

impl PartCooling {
    /// Construct from a config view, reading cooling settings with defaults.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let fan_max_speed = match config.get("fan_max_speed") {
            Some(ConfigValue::Int(v)) => *v as u8,
            _ => 255,
        };

        let close_fan_the_first_x_layers = match config.get("close_fan_the_first_x_layers") {
            Some(ConfigValue::Int(v)) => *v as u32,
            _ => 1,
        };

        let enable_overhang_bridge_fan = match config.get("enable_overhang_bridge_fan") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };

        let overhang_fan_speed = match config.get("overhang_fan_speed") {
            Some(ConfigValue::Int(v)) => *v as u8,
            _ => 100,
        };

        Ok(Self {
            fan_max_speed,
            close_fan_the_first_x_layers,
            enable_overhang_bridge_fan,
            overhang_fan_speed,
        })
    }

    /// Compute the target fan speed for a given layer index.
    fn layer_fan_speed(&self, layer_index: u32) -> u8 {
        if layer_index < self.close_fan_the_first_x_layers {
            0
        } else {
            self.fan_max_speed
        }
    }

    /// Whether the given extrusion role represents an overhang region.
    fn is_overhang_role(role: &ExtrusionRole) -> bool {
        matches!(role, ExtrusionRole::BridgeInfill)
    }

    /// Compute the cooling decision for one independently executed event view.
    ///
    /// This is the per-event counterpart to the ordinary full-layer
    /// finalization pass and does not emit commands or annotations.
    pub fn cooling_decision_for_event(&self, view: &LayerCollectionView) -> u8 {
        let base_speed = self.layer_fan_speed(view.layer_index());
        if self.enable_overhang_bridge_fan
            && view
                .ordered_entities()
                .iter()
                .any(|entity| Self::is_overhang_role(&entity.path.role))
        {
            ((self.overhang_fan_speed as u16 * self.fan_max_speed as u16) / 100) as u8
        } else {
            base_speed
        }
    }
}

#[slicer_module]
impl FinalizationModule for PartCooling {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        Self::from_config(config)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if layers.is_empty() {
            return Ok(());
        }

        // fan_max_speed == 0 → single M107 on the first layer, nothing else.
        if self.fan_max_speed == 0 {
            let first_layer = layers[0].layer_index();
            let _ = output.push_fan_speed(first_layer, 0);
            return Ok(());
        }

        for view in layers {
            let layer_index = view.layer_index();
            // Route layer/entity cooling through the per-event owner so the
            // fan decision has one implementation.
            let base_speed = self.layer_fan_speed(layer_index);
            let _ = output.push_fan_speed(layer_index, base_speed);

            if self.enable_overhang_bridge_fan {
                let entities = view.ordered_entities();
                let mut in_overhang = false;

                for (entity_idx, entity) in entities.iter().enumerate() {
                    let event_speed = self.cooling_decision_for_event(view);
                    let is_overhang = Self::is_overhang_role(&entity.path.role);

                    if is_overhang && !in_overhang {
                        // Overhang starts at this entity → bump fan before it.
                        let anchor = if entity_idx > 0 {
                            (entity_idx - 1) as u32
                        } else {
                            0
                        };
                        let _ = output.push_annotation(
                            layer_index,
                            LayerAnnotation {
                                after_entity_index: anchor,
                                kind: LayerAnnotationKind::Raw(format!("M106 S{}", event_speed)),
                            },
                        );
                        in_overhang = true;
                    } else if !is_overhang && in_overhang {
                        // Overhang ended at the previous entity → restore.
                        let anchor = (entity_idx - 1) as u32;
                        let _ = output.push_annotation(
                            layer_index,
                            LayerAnnotation {
                                after_entity_index: anchor,
                                kind: LayerAnnotationKind::Raw(format!("M106 S{}", base_speed)),
                            },
                        );
                        in_overhang = false;
                    }
                }
            }
        }

        // Fan off after the last layer.
        let last_layer = layers[layers.len() - 1].layer_index();
        let _ = output.push_annotation(
            last_layer,
            LayerAnnotation {
                after_entity_index: u32::MAX,
                kind: LayerAnnotationKind::Raw("M107".to_string()),
            },
        );

        Ok(())
    }
}
