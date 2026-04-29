//! Default path-optimization core module.
//!
//! Implements `LayerModule::run_path_optimization` for the canonical
//! `Layer::PathOptimization` stage (docs/04 §Fixed Stage Order).
//!
//! # Travel retract/no-retract policy (packet-15)
//!
//! This module is the canonical decision surface for live travel retraction:
//! - Inter-region travel (moving from one `PerimeterRegionView` to the next)
//!   is classified as **external** → emit Retract + ZHop (if `travel_z_hop > 0`)
//!   + travel Move + Unretract (OrcaSlicer canonical order: lift before travel).
//! - Intra-region travel (within the same `PerimeterRegionView`) is classified
//!   as **internal** → suppress retraction.
//!
//! `DefaultGCodeEmitter` (packet-11) serialises whatever commands this module
//! emits; it does not own the retract/no-retract decision.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole};
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::{GcodeMoveCmd, GcodeOutputBuilder};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::OrderedEntityView;
use slicer_sdk::views::PerimeterRegionView;

const DEFAULT_RETRACT_LENGTH: f32 = 0.8;
const DEFAULT_RETRACT_SPEED: f32 = 25.0;
const DEFAULT_TRAVEL_Z_HOP: f32 = 0.0;

/// Deterministically permutes `entities` using a greedy nearest-neighbor
/// heuristic starting from position (0.0, 0.0).
///
/// At each step the unvisited entity whose `start_point` (x, y) is nearest
/// (Euclidean distance) to the current cursor position is selected. When two
/// candidates are equidistant within 0.001 mm, `BridgeInfill` is preferred.
/// Further ties go to the lower `original_index`. After selection the cursor
/// advances to the picked entity's `end_point`. The reversal flag is always
/// `false`. Output is keyed on `view.original_index`.
fn nearest_neighbor_permutation(entities: &[OrderedEntityView]) -> Vec<(u32, bool)> {
    let n = entities.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![(entities[0].original_index, false)];
    }

    let mut result = Vec::with_capacity(n);
    let mut used = vec![false; n];
    let mut cur_x: f32 = 0.0;
    let mut cur_y: f32 = 0.0;

    for _ in 0..n {
        let mut best_idx = usize::MAX;
        let mut best_dist = f32::INFINITY;

        for (i, view) in entities.iter().enumerate() {
            if used[i] {
                continue;
            }
            let (sx, sy) = (view.start_point.x, view.start_point.y);
            let dx = sx - cur_x;
            let dy = sy - cur_y;
            let dist = (dx * dx + dy * dy).sqrt();

            let better = if best_idx == usize::MAX {
                true
            } else if (dist - best_dist).abs() < 0.001 {
                let curr_bridge = view.role == ExtrusionRole::BridgeInfill;
                let best_bridge = entities[best_idx].role == ExtrusionRole::BridgeInfill;
                if curr_bridge && !best_bridge {
                    true
                } else if !curr_bridge && best_bridge {
                    false
                } else {
                    i < best_idx
                }
            } else {
                dist < best_dist
            };

            if better {
                best_idx = i;
                best_dist = dist;
            }
        }

        used[best_idx] = true;
        let (nx, ny) = (
            entities[best_idx].end_point.x,
            entities[best_idx].end_point.y,
        );
        cur_x = nx;
        cur_y = ny;
        result.push((entities[best_idx].original_index, false));
    }

    result
}

/// Default path-optimization module.
pub struct PathOptimizationDefault {
    emit_layer_markers: bool,
    retract_length: f32,
    retract_speed: f32,
    travel_z_hop: f32,
}

#[slicer_module]
impl LayerModule for PathOptimizationDefault {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let emit_layer_markers = match config.get("path_optimization_emit_layer_markers") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };
        let retract_length = match config.get("retract_length") {
            Some(ConfigValue::Float(f)) => *f as f32,
            _ => DEFAULT_RETRACT_LENGTH,
        };
        let retract_speed = match config.get("retract_speed") {
            Some(ConfigValue::Float(f)) => *f as f32,
            _ => DEFAULT_RETRACT_SPEED,
        };
        let travel_z_hop = match config.get("travel_z_hop") {
            Some(ConfigValue::Float(f)) => *f as f32,
            _ => DEFAULT_TRAVEL_Z_HOP,
        };
        Ok(Self {
            emit_layer_markers,
            retract_length,
            retract_speed,
            travel_z_hop,
        })
    }

    fn run_path_optimization(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let snapshot = collection.get_ordered_entities();
        if !snapshot.is_empty() {
            let items = nearest_neighbor_permutation(snapshot);
            collection
                .set_entity_order(items)
                .map_err(|e| ModuleError::fatal(6, e))?;
        }

        if self.emit_layer_markers {
            let region_count = regions.len();
            let entity_count: usize = regions.iter().map(|r| r.wall_loops().len()).sum();
            output
                .push_comment(format!(
                    "path-optimization layer {layer_index} regions={region_count} entities={entity_count}"
                ))
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        // Emit inter-region travel retract decisions (external travel).
        // Each gap between consecutive PerimeterRegionView instances is an external
        // travel: the path crosses outside the current region boundary.
        // Intra-region gaps between wall loops are internal and are suppressed.
        for i in 0..regions.len().saturating_sub(1) {
            let from_region = &regions[i];
            let to_region = &regions[i + 1];

            let from_pt = from_region
                .wall_loops()
                .last()
                .and_then(|w| w.path.points.last())
                .cloned();
            let to_pt = to_region
                .wall_loops()
                .first()
                .and_then(|w| w.path.points.first())
                .cloned();

            if let (Some(_from), Some(to)) = (from_pt, to_pt) {
                output
                    .push_retract(self.retract_length, self.retract_speed)
                    .map_err(|e| ModuleError::fatal(2, e))?;
                // ZHop before the travel move (OrcaSlicer canonical: lift, then move).
                // The entity index is normalized to the global anchor by the host dispatch
                // so that ZHop, Retract, and TravelMove all land at the same entity position.
                if self.travel_z_hop > 0.0 {
                    output
                        .push_z_hop(0, self.travel_z_hop)
                        .map_err(|e| ModuleError::fatal(3, e))?;
                }
                output
                    .push_move(GcodeMoveCmd::new(
                        Some(to.x),
                        Some(to.y),
                        None,
                        None,
                        None,
                        ExtrusionRole::Custom("travel".to_string()),
                    ))
                    .map_err(|e| ModuleError::fatal(4, e))?;
                output
                    .push_unretract(self.retract_length, self.retract_speed)
                    .map_err(|e| ModuleError::fatal(5, e))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
    use slicer_sdk::postpass_builders::GcodeOutputBuilder;
    use slicer_sdk::traits::LayerModule;
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
        let mut collection = LayerCollectionBuilder::new();

        module
            .run_path_optimization(3, &[], &mut output, &mut collection, &config)
            .expect("path optimization should succeed with markers disabled");

        assert!(
            output.commands().is_empty(),
            "emit_layer_markers=false must suppress all marker comments"
        );
    }

    #[test]
    fn path_optimization_processes_pre_ordered_entity_sequence() {
        // Verifies the module correctly processes a pre-ordered PerimeterRegionView
        // list (the host ordering helper guarantees this order before dispatch).
        // With emit_layer_markers=true and an empty regions list the module emits
        // one marker comment referencing layer 0 with 0 regions and 0 entities.
        let config = ConfigView::from_map(HashMap::new());
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        let mut output = GcodeOutputBuilder::new();
        let mut collection = LayerCollectionBuilder::new();

        module
            .run_path_optimization(0, &[], &mut output, &mut collection, &config)
            .expect("path optimization must succeed on a pre-ordered (empty) entity list");

        assert_eq!(
            output.commands().len(),
            1,
            "one marker comment expected for a pre-ordered layer with no entities"
        );
    }

    #[test]
    fn retract_length_read_from_config() {
        let mut fields: HashMap<String, ConfigValue> = HashMap::new();
        fields.insert("retract_length".into(), ConfigValue::Float(1.5));
        let config = ConfigView::from_map(fields);
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        assert!(
            (module.retract_length - 1.5_f32).abs() < 1e-4,
            "retract_length must be read from config"
        );
    }
}
