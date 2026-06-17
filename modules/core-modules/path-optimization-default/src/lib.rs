// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
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

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, RetractMode};
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

/// Controls the order in which wall perimeters are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallSequence {
    /// Walls printed from innermost to outermost (default).
    InnerOuter,
    /// Walls printed from outermost to innermost.
    OuterInner,
}

/// Deterministically permutes `entities` using a greedy nearest-neighbor
/// heuristic starting from position (0.0, 0.0).
///
/// At each step the unvisited entity whose `start_point` (x, y) is nearest
/// (Euclidean distance) to the current cursor position is selected. When two
/// candidates are equidistant within 0.001 mm, ties go to the lower `original_index`. After selection the cursor
/// advances to the picked entity's `end_point`. The reversal flag is always
/// `false`. Output is keyed on `view.original_index`.
fn nearest_neighbor_permutation(entities: &[&OrderedEntityView]) -> Vec<(u32, bool)> {
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
                i < best_idx
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

/// Extracts tool_index from an OrderedEntityView.
///
/// Tool index is propagated through `region_key.region_id` at assembly time
/// via the host's per-region ActiveRegion.tool_index.
fn tool_index_of(entity: &OrderedEntityView) -> u32 {
    entity.region_key.region_id as u32
}

/// Tool change record emitted at a tool-index boundary during path optimization.
#[derive(Debug, Clone, Copy)]
struct ToolChangeRecord {
    /// Entity index in the final permutation after which this change occurs.
    after_entity_index: u32,
    from_tool: u32,
    to_tool: u32,
}

/// Default path-optimization module.
///
/// Cross-layer tool ordering (avoiding redundant layer-boundary T emissions)
/// is handled by the host's gcode emitter, which post-processes entity order
/// before emission. This module always emits clusters in ascending tool order.
pub struct PathOptimizationDefault {
    emit_layer_markers: bool,
    retract_length: f32,
    retract_speed: f32,
    travel_z_hop: f32,
    retract_mode: RetractMode,
    wall_sequence: WallSequence,
}

impl Default for PathOptimizationDefault {
    fn default() -> Self {
        Self {
            emit_layer_markers: true,
            retract_length: DEFAULT_RETRACT_LENGTH,
            retract_speed: DEFAULT_RETRACT_SPEED,
            travel_z_hop: DEFAULT_TRAVEL_Z_HOP,
            retract_mode: RetractMode::default(),
            wall_sequence: WallSequence::InnerOuter,
        }
    }
}

impl PathOptimizationDefault {
    fn role_group(&self, role: &ExtrusionRole) -> u32 {
        let (inner_group, outer_group) = match self.wall_sequence {
            WallSequence::InnerOuter => (1, 2),
            WallSequence::OuterInner => (2, 1),
        };
        match role {
            ExtrusionRole::Skirt => 0,
            ExtrusionRole::InnerWall => inner_group,
            ExtrusionRole::OuterWall => outer_group,
            ExtrusionRole::ThinWall => 3,
            ExtrusionRole::BottomSolidInfill
            | ExtrusionRole::TopSolidInfill
            | ExtrusionRole::SparseInfill
            | ExtrusionRole::BridgeInfill => 4,
            ExtrusionRole::Ironing => 5,
            ExtrusionRole::SupportMaterial => 6,
            ExtrusionRole::SupportInterface => 7,
            ExtrusionRole::WipeTower | ExtrusionRole::PrimeTower => 8,
            ExtrusionRole::Custom(_) => 9,
        }
    }

    fn group_then_nearest_neighbor(
        &self,
        entities: &[OrderedEntityView],
    ) -> (Vec<(u32, bool)>, Vec<ToolChangeRecord>) {
        if entities.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let mut tool_clusters: std::collections::BTreeMap<u32, Vec<&OrderedEntityView>> =
            std::collections::BTreeMap::new();
        for entity in entities {
            tool_clusters
                .entry(tool_index_of(entity))
                .or_default()
                .push(entity);
        }

        let ordered_tool_keys: Vec<u32> = tool_clusters.keys().copied().collect();

        let mut final_permutation: Vec<(u32, bool)> = Vec::with_capacity(entities.len());
        for &tool_idx in &ordered_tool_keys {
            let cluster_entities = &tool_clusters[&tool_idx];

            let mut role_groups: std::collections::BTreeMap<u32, Vec<&OrderedEntityView>> =
                std::collections::BTreeMap::new();
            for entity in cluster_entities {
                role_groups
                    .entry(self.role_group(&entity.role))
                    .or_default()
                    .push(entity);
            }

            for group_entities in role_groups.values() {
                for (orig_idx, reversal) in nearest_neighbor_permutation(group_entities) {
                    final_permutation.push((orig_idx, reversal));
                }
            }
        }

        let mut tool_change_records: Vec<ToolChangeRecord> = Vec::new();
        let mut prev_tool = None;
        let mut prev_global_pos = 0usize;

        for (global_pos, &(orig_idx, _)) in final_permutation.iter().enumerate() {
            let current_tool = tool_index_of(&entities[orig_idx as usize]);
            if let Some(prev) = prev_tool {
                if current_tool != prev {
                    tool_change_records.push(ToolChangeRecord {
                        after_entity_index: prev_global_pos as u32,
                        from_tool: prev,
                        to_tool: current_tool,
                    });
                }
            }
            prev_tool = Some(current_tool);
            prev_global_pos = global_pos;
        }

        (final_permutation, tool_change_records)
    }
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
        let retract_mode = match config.get("retract_mode") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "gcode" => RetractMode::Gcode,
                "firmware" => RetractMode::Firmware,
                other => {
                    return Err(ModuleError::fatal(
                        8,
                        format!("invalid retract_mode '{other}'; expected 'gcode' or 'firmware'"),
                    ));
                }
            },
            _ => RetractMode::default(),
        };
        let wall_sequence = match config.get("wall_sequence") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "inner_outer" => WallSequence::InnerOuter,
                "outer_inner" => WallSequence::OuterInner,
                other => {
                    return Err(ModuleError::fatal(
                        9,
                        format!("invalid wall_sequence '{other}'"),
                    ));
                }
            },
            _ => WallSequence::InnerOuter,
        };
        Ok(Self {
            emit_layer_markers,
            retract_length,
            retract_speed,
            travel_z_hop,
            retract_mode,
            wall_sequence,
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
            let (items, tool_changes) = self.group_then_nearest_neighbor(snapshot);
            collection
                .set_entity_order(items)
                .map_err(|e| ModuleError::fatal(6, e))?;

            for record in tool_changes {
                output
                    .push_tool_change(record.after_entity_index, record.from_tool, record.to_tool)
                    .map_err(|e| ModuleError::fatal(7, e))?;
            }
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

        // Emit inter-layer travel retract for any non-first layer (packet 34, Issue #1).
        // Without this, single-region per-layer prints (e.g. Benchy perimeters) emit zero
        // retracts in the live G-code path. Safer-variant first cut: retract → unretract
        // only (no z-hop, no travel move) since the layer planner already handles the
        // Z transition and the move to the first entity start.
        if layer_index > 0
            && regions
                .first()
                .and_then(|r| r.wall_loops().first())
                .and_then(|w| w.path.points.first())
                .is_some()
        {
            output
                .push_retract(self.retract_length, self.retract_speed, self.retract_mode)
                .map_err(|e| ModuleError::fatal(8, e))?;
            output
                .push_unretract(self.retract_length, self.retract_speed, self.retract_mode)
                .map_err(|e| ModuleError::fatal(9, e))?;
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
                    .push_retract(self.retract_length, self.retract_speed, self.retract_mode)
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
                    .push_unretract(self.retract_length, self.retract_speed, self.retract_mode)
                    .map_err(|e| ModuleError::fatal(5, e))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{Point3WithWidth, RegionKey};
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

    fn make_entity(original_index: u32, role: ExtrusionRole, x: f32, y: f32) -> OrderedEntityView {
        OrderedEntityView {
            original_index,
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "test".to_string(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            role,
            start_point: Point3WithWidth {
                x,
                y,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            end_point: Point3WithWidth {
                x,
                y,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            point_count: 0,
        }
    }

    // ------------------------------------------------------------------
    // Role-ordering tests (TDD red step for packet 61)
    // ------------------------------------------------------------------

    #[test]
    fn role_orders_inner_before_outer() {
        let entities = vec![
            make_entity(0, ExtrusionRole::InnerWall, 0.0, 100.0),
            make_entity(1, ExtrusionRole::OuterWall, 0.0, 10.0),
        ];
        let module = PathOptimizationDefault::default();
        let (perm, _changes) = module.group_then_nearest_neighbor(&entities);
        assert!(
            entities[perm[0].0 as usize].role == ExtrusionRole::InnerWall,
            "InnerWall must precede OuterWall (role-priority ordering)"
        );
    }

    #[test]
    fn role_orders_outer_before_inner() {
        let mut fields: HashMap<String, ConfigValue> = HashMap::new();
        fields.insert(
            "wall_sequence".into(),
            ConfigValue::String("outer_inner".into()),
        );
        let config = ConfigView::from_map(fields);
        let module = PathOptimizationDefault::on_print_start(&config).unwrap();
        let entities = vec![
            make_entity(0, ExtrusionRole::OuterWall, 0.0, 100.0),
            make_entity(1, ExtrusionRole::InnerWall, 0.0, 10.0),
        ];
        let (perm, _changes) = module.group_then_nearest_neighbor(&entities);
        assert!(
            entities[perm[0].0 as usize].role == ExtrusionRole::OuterWall,
            "OuterWall must precede InnerWall (outer-inner mode)"
        );
    }

    #[test]
    fn role_orders_walls_before_infill() {
        let entities = vec![
            make_entity(0, ExtrusionRole::InnerWall, 0.0, 100.0),
            make_entity(1, ExtrusionRole::SparseInfill, 0.0, 5.0),
            make_entity(2, ExtrusionRole::TopSolidInfill, 0.0, 20.0),
        ];
        let module = PathOptimizationDefault::default();
        let (perm, _changes) = module.group_then_nearest_neighbor(&entities);
        let first_role = &entities[perm[0].0 as usize].role;
        assert!(
            matches!(first_role, ExtrusionRole::InnerWall),
            "First entity must be a wall role, got {:?}",
            first_role
        );
    }

    #[test]
    fn role_chains_infill_together() {
        let entities = vec![
            make_entity(0, ExtrusionRole::SparseInfill, 0.0, 5.0),
            make_entity(1, ExtrusionRole::BridgeInfill, 0.0, 100.0),
            make_entity(2, ExtrusionRole::TopSolidInfill, 0.0, 20.0),
            make_entity(3, ExtrusionRole::InnerWall, 0.0, 10.0),
        ];
        let module = PathOptimizationDefault::default();
        let (perm, _changes) = module.group_then_nearest_neighbor(&entities);
        let infill_positions: Vec<usize> = perm
            .iter()
            .enumerate()
            .filter(|(_, &(idx, _))| {
                matches!(
                    entities[idx as usize].role,
                    ExtrusionRole::SparseInfill
                        | ExtrusionRole::BridgeInfill
                        | ExtrusionRole::TopSolidInfill
                )
            })
            .map(|(i, _)| i)
            .collect();
        for i in 1..infill_positions.len() {
            assert_eq!(
                infill_positions[i],
                infill_positions[i - 1] + 1,
                "Infill entities must be consecutive, not interleaved with other roles"
            );
        }
    }

    #[test]
    fn role_handles_all_extrusion_roles() {
        let roles = [
            ExtrusionRole::Skirt,
            ExtrusionRole::InnerWall,
            ExtrusionRole::OuterWall,
            ExtrusionRole::ThinWall,
            ExtrusionRole::SparseInfill,
            ExtrusionRole::BridgeInfill,
            ExtrusionRole::BottomSolidInfill,
            ExtrusionRole::TopSolidInfill,
            ExtrusionRole::SupportMaterial,
            ExtrusionRole::SupportInterface,
            ExtrusionRole::Ironing,
            ExtrusionRole::WipeTower,
            ExtrusionRole::PrimeTower,
            ExtrusionRole::Custom("test".to_string()),
        ];
        let module = PathOptimizationDefault::default();
        for role in &roles {
            let _group = module.role_group(role);
        }
    }

    #[test]
    fn role_preserves_global_sequence() {
        // Positions are decreasing from origin so nearest-neighbor picks
        // highest-group entities first — the assertion will fail until
        // role-aware grouping is implemented.
        let entities = [
            make_entity(0, ExtrusionRole::Skirt, 0.0, 500.0),
            make_entity(1, ExtrusionRole::InnerWall, 0.0, 400.0),
            make_entity(2, ExtrusionRole::OuterWall, 0.0, 300.0),
            make_entity(3, ExtrusionRole::ThinWall, 0.0, 200.0),
            make_entity(4, ExtrusionRole::SparseInfill, 0.0, 100.0),
            make_entity(5, ExtrusionRole::Ironing, 0.0, 50.0),
            make_entity(6, ExtrusionRole::SupportMaterial, 0.0, 10.0),
            make_entity(7, ExtrusionRole::SupportInterface, 0.0, 5.0),
            make_entity(8, ExtrusionRole::WipeTower, 0.0, 1.0),
            make_entity(9, ExtrusionRole::Custom("seq".to_string()), 0.0, 0.0),
        ];
        let module = PathOptimizationDefault::default();
        let (perm, _changes) = module.group_then_nearest_neighbor(&entities);
        let groups: Vec<u32> = perm
            .iter()
            .map(|&(idx, _)| module.role_group(&entities[idx as usize].role))
            .collect();
        for i in 1..groups.len() {
            assert!(
                groups[i] >= groups[i - 1],
                "Role groups must be non-decreasing: group {} < group {} at position {}",
                groups[i],
                groups[i - 1],
                i
            );
        }
    }

    #[test]
    fn role_rejects_invalid_wall_sequence() {
        let mut fields: HashMap<String, ConfigValue> = HashMap::new();
        fields.insert(
            "wall_sequence".into(),
            ConfigValue::String("invalid_value".into()),
        );
        let config = ConfigView::from_map(fields);
        let result = PathOptimizationDefault::on_print_start(&config);
        let err = match result {
            Err(e) => format!("{}", e),
            Ok(_) => panic!("invalid wall_sequence must be rejected"),
        };
        assert!(
            err.contains("invalid_value"),
            "error must contain the rejected value, got: {err}"
        );
    }

    #[test]
    fn role_ordering_is_deterministic() {
        let entities = vec![
            make_entity(0, ExtrusionRole::InnerWall, 0.0, 100.0),
            make_entity(1, ExtrusionRole::OuterWall, 0.0, 10.0),
        ];
        let module = PathOptimizationDefault::default();
        let (perm1, _changes1) = module.group_then_nearest_neighbor(&entities);
        let (perm2, _changes2) = module.group_then_nearest_neighbor(&entities);
        assert_eq!(
            perm1, perm2,
            "permutation must be byte-identical across runs"
        );
    }
}
