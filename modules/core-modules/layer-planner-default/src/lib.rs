// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Layer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Default uniform layer planner for ModularSlicer.
//!
//! Implements the `PrepassModule` trait for the `PrePass::LayerPlanning` stage.
//! Computes global Z-plane sequences from object heights and layer-height config.
//!
//! # Algorithm (MVP — uniform layers)
//!
//! 1. Read `layer_height` and `first_layer_height` from config
//! 2. For each object: read height from config key `"object_height:<object_id>"`
//!    when supplied, otherwise query `host::object_bounds`
//! 3. Generate layer sequence: first_layer_height, then layer_height increments
//! 4. For multi-object with different layer heights: compute LCM sync interval
//! 5. Generate catch-up layers for objects that skip intermediate global layers
//! 6. Push each layer proposal to output

use slicer_sdk::prelude::*;

/// Default layer planner that produces uniform layer heights.
///
/// Reads `layer_height`, `first_layer_height`, and per-object height keys
/// from the config view. For multi-object prints with different layer heights,
/// it synchronizes via LCM intervals and inserts catch-up layers.
pub struct DefaultLayerPlanner {
    /// Base layer height in mm.
    layer_height: f32,
    /// First layer height in mm.
    first_layer_height: f32,
}

#[slicer_module]
impl PrepassModule for DefaultLayerPlanner {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let layer_height = config
            .get("layer_height")
            .and_then(|v| match v {
                ConfigValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(0.2);

        let first_layer_height = config
            .get("first_layer_height")
            .and_then(|v| match v {
                ConfigValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(layer_height);

        Ok(Self {
            layer_height,
            first_layer_height,
        })
    }

    fn run_layer_planning(
        &self,
        objects: &[ObjectId],
        output: &mut LayerPlanOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if objects.is_empty() {
            return Err(ModuleError::fatal(
                1,
                "no objects provided for layer planning",
            ));
        }

        if self.layer_height <= 0.0 {
            return Err(ModuleError::fatal(2, "layer_height must be positive"));
        }

        if self.first_layer_height <= 0.0 {
            return Err(ModuleError::fatal(3, "first_layer_height must be positive"));
        }

        // Build per-object plans
        let mut plans = Vec::new();
        for obj_id in objects {
            let height = object_height(config, obj_id)
                .or_else(|| {
                    host::object_bounds(obj_id)
                        .ok()
                        .map(|bounds| bounds.max.z - bounds.min.z)
                })
                .unwrap_or(0.0);
            if height <= 0.0 {
                continue;
            }
            let lh = object_layer_height(config, obj_id, self.layer_height);
            plans.push(ObjectPlan {
                object_id: obj_id.clone(),
                height,
                layer_height: lh,
                first_layer_height: self.first_layer_height,
            });
        }

        if plans.is_empty() {
            return Err(ModuleError::fatal(4, "no objects with positive height"));
        }

        // Merge layer sequences
        let merged = merge_layer_sequences(&plans, self.first_layer_height);

        // Push proposals to output
        for layer in merged {
            output
                .push_layer(LayerProposal {
                    z: layer.z,
                    active_regions: layer.regions,
                })
                .map_err(|e| ModuleError::fatal(5, e))?;
        }

        Ok(())
    }
}

/// Get the per-object layer height override from config, or fall back to default.
///
/// Looks for config key `"layer_height:<object_id>"`. If not found, returns `default`.
pub fn object_layer_height(config: &ConfigView, object_id: &str, default: f32) -> f32 {
    let key = format!("layer_height:{}", object_id);
    config
        .get(&key)
        .and_then(|v| match v {
            ConfigValue::Float(f) => Some(*f as f32),
            _ => None,
        })
        .unwrap_or(default)
}

/// Get the object height from config.
///
/// Looks for config key `"object_height:<object_id>"`. Returns `None` if not found.
pub fn object_height(config: &ConfigView, object_id: &str) -> Option<f32> {
    let key = format!("object_height:{}", object_id);
    config.get(&key).and_then(|v| match v {
        ConfigValue::Float(f) => Some(*f as f32),
        _ => None,
    })
}

/// Information about an object's layer planning parameters.
#[derive(Debug, Clone)]
struct ObjectPlan {
    /// Object ID.
    object_id: ObjectId,
    /// Object height in mm.
    height: f32,
    /// Layer height for this object in mm.
    layer_height: f32,
    /// First layer height in mm.
    first_layer_height: f32,
}

/// A merged global layer with per-object participation info.
#[derive(Debug, Clone)]
struct MergedLayer {
    /// Z coordinate of this layer.
    z: f32,
    /// Regions active at this layer.
    regions: Vec<RegionLayerProposal>,
}

/// Generate uniform Z-plane sequence for a single object.
fn generate_object_layers(plan: &ObjectPlan) -> Vec<f32> {
    let mut layers = Vec::new();
    let mut z = plan.first_layer_height;
    while z <= plan.height + 1e-6 {
        layers.push(z);
        z += plan.layer_height;
    }
    layers
}

/// Merge layer sequences from multiple objects into a global Z-plane sequence.
///
/// For objects with different layer heights, this inserts sync layers at LCM intervals
/// and catch-up layers where needed.
fn merge_layer_sequences(plans: &[ObjectPlan], _first_layer_height: f32) -> Vec<MergedLayer> {
    if plans.is_empty() {
        return Vec::new();
    }

    // If all objects have the same layer height, simple merge
    let all_same_height = plans
        .iter()
        .all(|p| (p.layer_height - plans[0].layer_height).abs() < 1e-6);

    if all_same_height {
        return merge_same_height(plans);
    }

    merge_different_heights(plans)
}

/// Merge layers for objects that all share the same layer height.
fn merge_same_height(plans: &[ObjectPlan]) -> Vec<MergedLayer> {
    // Find max height across all objects
    let max_height = plans.iter().map(|p| p.height).fold(0.0f32, f32::max);

    let first = &plans[0];
    let mut layers = Vec::new();
    let mut z = first.first_layer_height;
    let lh = first.layer_height;

    while z <= max_height + 1e-6 {
        let regions: Vec<RegionLayerProposal> = plans
            .iter()
            .filter(|p| z <= p.height + 1e-6)
            .map(|p| {
                let effective_lh = if layers.is_empty() {
                    p.first_layer_height
                } else {
                    p.layer_height
                };
                RegionLayerProposal {
                    object_id: p.object_id.clone(),
                    region_id: "0".to_string(),
                    effective_layer_height: effective_lh,
                    is_catchup: false,
                    catchup_z_bottom: 0.0,
                }
            })
            .collect();

        if !regions.is_empty() {
            layers.push(MergedLayer { z, regions });
        }
        z += lh;
    }
    layers
}

/// Merge layers for objects with different layer heights using LCM sync.
///
/// At every global Z plane (union of all objects' native layers), every active
/// object participates. Objects without a native layer at that Z get a catch-up
/// layer bridging from their last participated Z to the current one.
fn merge_different_heights(plans: &[ObjectPlan]) -> Vec<MergedLayer> {
    // Generate per-object Z sequences
    let object_zs: Vec<Vec<f32>> = plans.iter().map(generate_object_layers).collect();

    // Collect all unique Z values, sorted
    let mut all_zs: Vec<f32> = object_zs.iter().flatten().copied().collect();
    all_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut layers = Vec::new();
    // Track the last Z at which each object participated
    let mut last_z: Vec<f32> = vec![0.0; plans.len()];

    for &z in &all_zs {
        let mut regions = Vec::new();

        for (i, plan) in plans.iter().enumerate() {
            if z > plan.height + 1e-6 {
                continue;
            }

            // Check if this object has a native layer at this Z
            let is_native = object_zs[i].iter().any(|oz| (*oz - z).abs() < 1e-6);

            if is_native {
                // Regular layer for this object
                let effective_lh = if (last_z[i] - 0.0).abs() < 1e-6 {
                    plan.first_layer_height
                } else {
                    plan.layer_height
                };
                regions.push(RegionLayerProposal {
                    object_id: plan.object_id.clone(),
                    region_id: "0".to_string(),
                    effective_layer_height: effective_lh,
                    is_catchup: false,
                    catchup_z_bottom: 0.0,
                });
                last_z[i] = z;
            } else {
                // Catch-up layer: this object doesn't have a native layer here
                let bottom_z = last_z[i];
                let catchup_height = z - bottom_z;
                if catchup_height > 1e-6 {
                    regions.push(RegionLayerProposal {
                        object_id: plan.object_id.clone(),
                        region_id: "0".to_string(),
                        effective_layer_height: catchup_height,
                        is_catchup: true,
                        catchup_z_bottom: bottom_z,
                    });
                    last_z[i] = z;
                }
            }
        }

        if !regions.is_empty() {
            layers.push(MergedLayer { z, regions });
        }
    }
    layers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let planner = DefaultLayerPlanner::on_print_start(&config).unwrap();
        assert!((planner.layer_height - 0.2).abs() < 1e-6);
        assert!((planner.first_layer_height - 0.2).abs() < 1e-6);
    }
}
