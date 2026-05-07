//! Wipe tower module for multi-material tool change purge/prime paths.
//!
//! Runs in the `PostPass::LayerFinalization` stage, operating on the full
//! set of `LayerCollectionIR` outputs after per-layer processing completes.
//! For each tool change, generates rectilinear purge scan lines within a
//! configurable rectangular region.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen,
    Point3WithWidth, PrintEntity, RegionKey,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Default layer height used when layer height cannot be inferred from
/// adjacent layers.
const DEFAULT_LAYER_HEIGHT: f32 = 0.2;

/// Wipe tower purge/prime path generator.
///
/// Generates rectangular rectilinear scan-line purge extrusions at each
/// tool change location across all layers.
pub struct WipeTower {
    tower_x: f32,
    tower_y: f32,
    tower_width: f32,
    purge_volume: f32,
    line_width: f32,
    enabled: bool,
}

impl WipeTower {
    /// Construct from a config view, reading wipe tower settings with defaults.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("wipe_tower_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let tower_x = match config.get("wipe_tower_x") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0,
        };

        let tower_y = match config.get("wipe_tower_y") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0,
        };

        let tower_width = match config.get("wipe_tower_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 60.0,
        };

        let purge_volume = match config.get("wipe_tower_purge_volume") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 70.0,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.4,
        };

        Ok(Self {
            tower_x,
            tower_y,
            tower_width,
            purge_volume,
            line_width,
            enabled,
        })
    }

    /// Process all layers, inserting wipe tower purge paths at tool changes.
    ///
    /// If the tower is disabled, returns immediately without modification.
    #[allow(clippy::ptr_arg)]
    pub fn process(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        for layer_idx in 0..layers.len() {
            if layers[layer_idx].tool_changes.is_empty() {
                continue;
            }

            let z = layers[layer_idx].z;

            // Estimate layer height from adjacent layers
            let layer_height = if layer_idx > 0 {
                let dz = z - layers[layer_idx - 1].z;
                if dz > 0.0 {
                    dz
                } else {
                    DEFAULT_LAYER_HEIGHT
                }
            } else {
                DEFAULT_LAYER_HEIGHT
            };

            // Clone tool_changes so we don't borrow layers while mutating
            let tool_changes = layers[layer_idx].tool_changes.clone();

            let global_layer_index = layers[layer_idx].global_layer_index;
            for tc in &tool_changes {
                let entities =
                    self.generate_purge_entities(z, layer_height, global_layer_index, tc);
                layers[layer_idx].ordered_entities.extend(entities);
            }
        }

        Ok(())
    }

    /// Generate purge entities for a single tool change.
    fn generate_purge_entities(
        &self,
        z: f32,
        layer_height: f32,
        global_layer_index: u32,
        _tc: &slicer_ir::ToolChange,
    ) -> Vec<PrintEntity> {
        // purge_depth = purge_volume / (line_width * layer_height * tower_width)
        let cross_section = self.line_width * layer_height * self.tower_width;
        if cross_section <= 0.0 {
            return Vec::new();
        }
        let purge_depth = self.purge_volume / cross_section;

        // Generate rectilinear scan lines within the rectangle
        // [tower_x, tower_y] to [tower_x + tower_width, tower_y + purge_depth]
        let x_min = self.tower_x;
        let x_max = self.tower_x + self.tower_width;
        let y_min = self.tower_y;
        let y_max = self.tower_y + purge_depth;

        let mut entities = Vec::new();
        let id_gen = LayerEntityIdGen::new();
        let mut y = y_min + self.line_width / 2.0;
        let mut forward = true;

        while y < y_max {
            let (start_x, end_x) = if forward {
                (x_min, x_max)
            } else {
                (x_max, x_min)
            };

            let path = ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: start_x,
                        y,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                    },
                    Point3WithWidth {
                        x: end_x,
                        y,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                    },
                ],
                role: ExtrusionRole::WipeTower,
                speed_factor: 1.0,
            };

            let region_key = RegionKey {
                global_layer_index,
                object_id: "__wipe_tower__".to_string(),
                region_id: 0,
            };

            entities.push(PrintEntity {
                entity_id: id_gen.next(),
                path,
                role: ExtrusionRole::WipeTower,
                region_key,
                topo_order: 0,
            });

            forward = !forward;
            y += self.line_width;
        }

        entities
    }

    /// Whether the wipe tower is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Tower X position in mm.
    pub fn tower_x(&self) -> f32 {
        self.tower_x
    }

    /// Tower Y position in mm.
    pub fn tower_y(&self) -> f32 {
        self.tower_y
    }

    /// Tower width in mm.
    pub fn tower_width(&self) -> f32 {
        self.tower_width
    }

    /// Purge volume in mm^3.
    pub fn purge_volume(&self) -> f32 {
        self.purge_volume
    }

    /// Line width in mm.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

// ── SDK authoring-path adoption (TASK-111 / packet-17) ─────────────────
//
// `on_print_start` delegates to the existing `from_config` constructor.
// `run_finalization` is now fully implemented via `LayerCollectionView`
// + `FinalizationOutputBuilder` (packet 17). The legacy `process()`
// helper remains for backwards compatibility.
#[slicer_module]
impl FinalizationModule for WipeTower {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        Self::from_config(config)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        for (idx, view) in layers.iter().enumerate() {
            if view.tool_changes().is_empty() {
                continue;
            }

            let z = view.z();
            let layer_index = view.layer_index();

            let layer_height = if idx > 0 {
                let dz = z - layers[idx - 1].z();
                if dz > 0.0 {
                    dz
                } else {
                    DEFAULT_LAYER_HEIGHT
                }
            } else {
                DEFAULT_LAYER_HEIGHT
            };

            let tool_changes = view.tool_changes().to_vec();
            for tc in &tool_changes {
                let entities = self.generate_purge_entities(z, layer_height, layer_index, tc);
                for entity in entities {
                    output
                        .push_entity_to_layer(layer_index, entity.path, entity.region_key)
                        .map_err(|e| ModuleError::fatal(1, e))?;
                }
            }
        }

        Ok(())
    }
}
