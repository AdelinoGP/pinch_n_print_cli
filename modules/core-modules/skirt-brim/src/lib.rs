//! Skirt and brim module for priming and adhesion paths.
//!
//! Runs in the `PostPass::LayerFinalization` stage, operating on the full
//! set of `LayerCollectionIR` outputs. Generates skirt loops around the
//! print for priming and/or brim adhesion paths at the base.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen,
    Point3WithWidth, PrintEntity, RegionKey,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Skirt and brim path generator.
///
/// Generates rectangular skirt loops around the print bounding box for
/// extruder priming, and optionally generates brim paths for bed adhesion.
pub struct SkirtBrim {
    skirt_loops: u32,
    skirt_distance: f32,
    skirt_height: u32,
    brim_width: f32,
    line_width: f32,
    enabled: bool,
}

impl SkirtBrim {
    /// Construct from a config view, reading skirt/brim settings with defaults.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("skirt_brim_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };

        let skirt_loops = match config.get("skirt_loops") {
            Some(ConfigValue::Int(v)) => *v as u32,
            _ => 1,
        };

        let skirt_distance = match config.get("skirt_distance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 6.0,
        };

        let skirt_height = match config.get("skirt_height") {
            Some(ConfigValue::Int(v)) => *v as u32,
            _ => 1,
        };

        let brim_width = match config.get("brim_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.4,
        };

        Ok(Self {
            skirt_loops,
            skirt_distance,
            skirt_height,
            brim_width,
            line_width,
            enabled,
        })
    }

    /// Process all layers, inserting skirt and/or brim paths.
    ///
    /// If the module is disabled, returns immediately without modification.
    #[allow(clippy::ptr_arg)]
    pub fn process(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<(), ModuleError> {
        if !self.enabled || layers.is_empty() {
            return Ok(());
        }

        // Compute bounding box across layers affected by skirt_height
        let max_layer = (self.skirt_height as usize).min(layers.len());
        let bbox = match self.compute_bbox(layers, max_layer) {
            Some(b) => b,
            None => return Ok(()), // no entities, nothing to do
        };

        // Generate skirt on first N layers
        for layer in layers.iter_mut().take(max_layer) {
            let skirt_entities =
                self.generate_skirt_entities(&bbox, layer.z, layer.global_layer_index);
            // Prepend skirt entities
            let mut new_entities = skirt_entities;
            new_entities.append(&mut layer.ordered_entities);
            layer.ordered_entities = new_entities;
        }

        // Generate brim on layer 0 only
        if self.brim_width > 0.0 {
            let z = layers[0].z;
            let global_layer_index = layers[0].global_layer_index;
            let brim_entities = self.generate_brim_entities(&bbox, z, global_layer_index);
            // Prepend brim entities (before skirt which is already prepended)
            let mut new_entities = brim_entities;
            new_entities.append(&mut layers[0].ordered_entities);
            layers[0].ordered_entities = new_entities;
        }

        Ok(())
    }

    /// Compute bounding box of all entities across the first `max_layer` layers.
    fn compute_bbox(&self, layers: &[LayerCollectionIR], max_layer: usize) -> Option<BBox2D> {
        let mut bbox: Option<BBox2D> = None;

        for layer in layers.iter().take(max_layer) {
            for entity in &layer.ordered_entities {
                for pt in &entity.path.points {
                    match &mut bbox {
                        Some(b) => {
                            b.x_min = b.x_min.min(pt.x);
                            b.y_min = b.y_min.min(pt.y);
                            b.x_max = b.x_max.max(pt.x);
                            b.y_max = b.y_max.max(pt.y);
                        }
                        None => {
                            bbox = Some(BBox2D {
                                x_min: pt.x,
                                y_min: pt.y,
                                x_max: pt.x,
                                y_max: pt.y,
                            });
                        }
                    }
                }
            }
        }

        bbox
    }

    /// Generate skirt loop entities around the bounding box.
    fn generate_skirt_entities(
        &self,
        bbox: &BBox2D,
        z: f32,
        global_layer_index: u32,
    ) -> Vec<PrintEntity> {
        let mut entities = Vec::new();
        let id_gen = LayerEntityIdGen::new();

        for i in 0..self.skirt_loops {
            let offset = self.skirt_distance + (i as f32) * self.line_width;
            let x_min = bbox.x_min - offset;
            let y_min = bbox.y_min - offset;
            let x_max = bbox.x_max + offset;
            let y_max = bbox.y_max + offset;

            let path = self.make_rect_loop(x_min, y_min, x_max, y_max, z);

            let region_key = RegionKey {
                global_layer_index,
                object_id: "__skirt__".to_string(),
                region_id: 0,
            };

            entities.push(PrintEntity {
                entity_id: id_gen.next(),
                path,
                role: ExtrusionRole::Skirt,
                region_key,
                topo_order: 0,
            });
        }

        entities
    }

    /// Generate brim entities around the bounding box (layer 0 only).
    fn generate_brim_entities(
        &self,
        bbox: &BBox2D,
        z: f32,
        global_layer_index: u32,
    ) -> Vec<PrintEntity> {
        let num_loops = (self.brim_width / self.line_width).ceil() as u32;
        let mut entities = Vec::new();
        let id_gen = LayerEntityIdGen::new();

        for i in 0..num_loops {
            // Brim loops go from outermost inward toward the object
            let offset = self.brim_width - (i as f32) * self.line_width;
            // Ensure offset doesn't go negative
            if offset < 0.0 {
                break;
            }
            let x_min = bbox.x_min - offset;
            let y_min = bbox.y_min - offset;
            let x_max = bbox.x_max + offset;
            let y_max = bbox.y_max + offset;

            let path = self.make_rect_loop(x_min, y_min, x_max, y_max, z);

            let region_key = RegionKey {
                global_layer_index,
                object_id: "__brim__".to_string(),
                region_id: 0,
            };

            entities.push(PrintEntity {
                entity_id: id_gen.next(),
                path,
                role: ExtrusionRole::Skirt,
                region_key,
                topo_order: 0,
            });
        }

        entities
    }

    /// Create a closed rectangular extrusion loop.
    fn make_rect_loop(
        &self,
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
        z: f32,
    ) -> ExtrusionPath3D {
        let mk = |x: f32, y: f32| Point3WithWidth {
            x,
            y,
            z,
            width: self.line_width,
            flow_factor: 1.0,
        };

        ExtrusionPath3D {
            points: vec![
                mk(x_min, y_min),
                mk(x_max, y_min),
                mk(x_max, y_max),
                mk(x_min, y_max),
                mk(x_min, y_min), // close the loop
            ],
            role: ExtrusionRole::Skirt,
            speed_factor: 1.0,
        }
    }

    /// Whether the module is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Number of skirt loops.
    pub fn skirt_loops(&self) -> u32 {
        self.skirt_loops
    }

    /// Skirt distance from print in mm.
    pub fn skirt_distance(&self) -> f32 {
        self.skirt_distance
    }

    /// Number of layers to apply skirt to.
    pub fn skirt_height(&self) -> u32 {
        self.skirt_height
    }

    /// Brim width in mm (0 = disabled).
    pub fn brim_width(&self) -> f32 {
        self.brim_width
    }

    /// Line width in mm.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

/// 2D axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
struct BBox2D {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

// `on_print_start` delegates to `from_config`; `run_finalization` is
// fully implemented via `LayerCollectionView` + `FinalizationOutputBuilder`
// (packet 16, TASK-142). The legacy `process()` helper remains for tests
// but is no longer called on the live host path.
#[slicer_module]
impl FinalizationModule for SkirtBrim {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        Self::from_config(config)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled || layers.is_empty() {
            return Ok(());
        }

        let max_layer = (self.skirt_height as usize).min(layers.len());

        // Compute bbox from the first max_layer layers using the read-only view.
        let mut bbox: Option<BBox2D> = None;
        for view in layers.iter().take(max_layer) {
            for entity in view.ordered_entities() {
                for pt in &entity.path.points {
                    match &mut bbox {
                        Some(b) => {
                            b.x_min = b.x_min.min(pt.x);
                            b.y_min = b.y_min.min(pt.y);
                            b.x_max = b.x_max.max(pt.x);
                            b.y_max = b.y_max.max(pt.y);
                        }
                        None => {
                            bbox = Some(BBox2D {
                                x_min: pt.x,
                                y_min: pt.y,
                                x_max: pt.x,
                                y_max: pt.y,
                            });
                        }
                    }
                }
            }
        }

        let bbox = match bbox {
            Some(b) => b,
            None => return Ok(()), // no entities, nothing to do
        };

        // Emit skirt entity pushes for each targeted layer.
        for view in layers.iter().take(max_layer) {
            let layer_index = view.layer_index();
            let z = view.z();
            for entity in self.generate_skirt_entities(&bbox, z, layer_index) {
                output
                    .push_entity_to_layer(layer_index, entity.path, entity.region_key)
                    .map_err(|e| ModuleError::fatal(1, e))?;
            }
        }

        // Emit brim entity pushes on layer 0 only.
        if self.brim_width > 0.0 {
            if let Some(view) = layers.first() {
                let layer_index = view.layer_index();
                let z = view.z();
                for entity in self.generate_brim_entities(&bbox, z, layer_index) {
                    output
                        .push_entity_to_layer(layer_index, entity.path, entity.region_key)
                        .map_err(|e| ModuleError::fatal(1, e))?;
                }
            }
        }

        Ok(())
    }
}
