// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Layer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Default uniform layer planner for Pinch 'n Print.
//!
//! Implements the `PrepassModule` trait for the `PrePass::LayerPlanning` stage.
//! Computes global Z-plane sequences from typed per-object planning inputs.
//!
//! # Algorithm (MVP — uniform layers)
//!
//! 1. Consume each object's resolved planning record
//! 2. Generate any object-specific raft prefix
//! 3. Generate layer sequence: first_layer_height, then layer_height increments
//! 4. For multi-object with different layer heights: compute LCM sync interval
//! 5. Generate catch-up layers for objects that skip intermediate global layers
//! 6. Push each layer proposal to output

use slicer_sdk::prelude::*;
use slicer_sdk::traits::LayerPlanningObject;

/// Default layer planner that produces uniform layer heights.
///
/// Consumes resolved per-object values from `LayerPlanningObject`. For
/// multi-object prints with different layer heights, it synchronizes via LCM
/// intervals and inserts catch-up layers.
pub struct DefaultLayerPlanner;

#[slicer_module]
impl PrepassModule for DefaultLayerPlanner {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_layer_planning(
        &self,
        objects: &[LayerPlanningObject],
        output: &mut LayerPlanOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if objects.is_empty() {
            return Err(ModuleError::fatal(
                1,
                "no objects provided for layer planning",
            ));
        }

        // Build per-object plans
        let mut plans = Vec::new();
        for object in objects {
            if object.object_height <= 0.0 {
                continue;
            }
            if object.layer_height <= 0.0 {
                return Err(ModuleError::fatal(2, "layer_height must be positive"));
            }
            if object.first_layer_height <= 0.0 {
                return Err(ModuleError::fatal(3, "first_layer_height must be positive"));
            }
            plans.push(ObjectPlan {
                object_id: object.object_id.clone(),
                height: object.object_height,
                layer_height: object.layer_height,
                first_layer_height: object.first_layer_height,
                raft_layers: object.support_raft_layers,
            });
        }

        if plans.is_empty() {
            return Err(ModuleError::fatal(4, "no objects with positive height"));
        }

        let raft_top = raft_top(&plans);
        for layer in merge_raft_sequences(&plans) {
            output
                .push_layer(LayerProposal {
                    z: layer.z,
                    active_regions: layer.regions,
                    is_raft: true,
                })
                .map_err(|e| ModuleError::fatal(5, e))?;
        }

        // Merge layer sequences
        let merged = merge_layer_sequences(&plans, raft_top);

        // Push proposals to output
        for layer in merged {
            output
                .push_layer(LayerProposal {
                    z: layer.z,
                    active_regions: layer.regions,
                    is_raft: false,
                })
                .map_err(|e| ModuleError::fatal(5, e))?;
        }

        Ok(())
    }
}

/// Information about an object's layer planning parameters.
#[derive(Debug, Clone)]
struct ObjectPlan {
    /// Object ID.
    object_id: ObjectId,
    /// Object height in mm.
    ///
    /// `f64` so the layer-Z formula's termination check (`z > height + 1e-6`)
    /// compares in the same precision as the formula itself. `height` only
    /// bounds the loop; it does not enter the Z value, so an `f32` source
    /// (e.g. `host::object_bounds`'s `Point3.z: f32`) is widened here once.
    height: f64,
    /// Layer height for this object in mm. `f64` — feeds the Z formula.
    layer_height: f64,
    /// First layer height in mm. `f64` — feeds the Z formula.
    first_layer_height: f64,
    /// Number of raft layers contributed by this object.
    raft_layers: u32,
}

/// A merged global layer with per-object participation info.
#[derive(Debug, Clone)]
struct MergedLayer {
    /// Z coordinate of this layer.
    z: f32,
    /// Regions active at this layer.
    regions: Vec<RegionLayerProposal>,
}

fn raft_top(plans: &[ObjectPlan]) -> f64 {
    plans
        .iter()
        .filter(|plan| plan.raft_layers > 0)
        .map(|plan| {
            plan.first_layer_height + (f64::from(plan.raft_layers) - 1.0) * plan.layer_height
        })
        .fold(0.0, f64::max)
}

fn merge_raft_sequences(plans: &[ObjectPlan]) -> Vec<MergedLayer> {
    let mut all_zs: Vec<f32> = plans
        .iter()
        .flat_map(|plan| {
            (0..plan.raft_layers).map(|index| {
                (plan.first_layer_height + f64::from(index) * plan.layer_height) as f32
            })
        })
        .collect();
    all_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    all_zs
        .into_iter()
        .map(|z| {
            let regions = plans
                .iter()
                .filter_map(|plan| {
                    (0..plan.raft_layers)
                        .find(|index| {
                            let object_z = (plan.first_layer_height
                                + f64::from(*index) * plan.layer_height)
                                as f32;
                            (object_z - z).abs() < 1e-6
                        })
                        .map(|index| RegionLayerProposal {
                            object_id: plan.object_id.clone(),
                            region_id: "0".to_string(),
                            effective_layer_height: if index == 0 {
                                plan.first_layer_height as f32
                            } else {
                                plan.layer_height as f32
                            },
                            is_catchup: false,
                            catchup_z_bottom: 0.0,
                        })
                })
                .collect();
            MergedLayer { z, regions }
        })
        .collect()
}

/// Generate uniform Z-plane sequence for a single object.
///
/// Z values are computed in `f64` using the direct formula
/// `first_layer_height + n * layer_height` and then converted to `f32` at
/// the return boundary. `ObjectPlan` fields are `f64` (sourced from
/// `ResolvedConfig`'s `f64` `layer_height`/`first_layer_height`), so the
/// formula runs in untainted `f64` — the `f32` bit pattern of `0.2` is
/// `0.20000000298...`, which (if narrowed to `f32` and re-widened) would
/// drift `93 * 0.20000000298... = 18.80000028...` onto the adjacent `f32`
/// `18.80000114...` instead of the STL's `f32(18.8) = 18.79999924`,
/// missing the vertex in `classify_vertex` and breaking the topology
/// walk (the benchy z=18.8 regression). OrcaSlicer computes layer Z in
/// `coordf_t` (= `double`) — see
/// `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp:807-867`
/// (`generate_object_layers`); this function mirrors that, casting to
/// `f32` only here (equivalent to OrcaSlicer's `float(print_z)` at
/// `slice_facet`'s `slice_z` parameter, `TriangleMeshSlicer.cpp:158`).
fn generate_object_layers(plan: &ObjectPlan, raft_top: f64) -> Vec<f32> {
    let mut layers = Vec::new();
    let first = plan.first_layer_height;
    let step = plan.layer_height;
    let height = plan.height;
    let mut n: u32 = 0;
    loop {
        let z_f64 = first + (n as f64) * step;
        if z_f64 > height + 1e-6 {
            break;
        }
        layers.push((raft_top + z_f64) as f32);
        n += 1;
    }
    layers
}

/// Merge layer sequences from multiple objects into a global Z-plane sequence.
///
/// For objects with different layer heights, this inserts sync layers at LCM intervals
/// and catch-up layers where needed.
fn merge_layer_sequences(plans: &[ObjectPlan], raft_top: f64) -> Vec<MergedLayer> {
    if plans.is_empty() {
        return Vec::new();
    }

    // If all objects have the same layer height, simple merge
    let all_same_height = plans.iter().all(|p| {
        (p.layer_height - plans[0].layer_height).abs() < 1e-6
            && (p.first_layer_height - plans[0].first_layer_height).abs() < 1e-6
    });

    if all_same_height {
        return merge_same_height(plans, raft_top);
    }

    merge_different_heights(plans, raft_top)
}

/// Merge layers for objects that all share the same layer height.
fn merge_same_height(plans: &[ObjectPlan], raft_top: f64) -> Vec<MergedLayer> {
    // Find max height across all objects
    let max_height = plans.iter().map(|p| p.height).fold(0.0f64, f64::max);

    let first = &plans[0];
    let mut layers = Vec::new();
    let first_z_f64 = first.first_layer_height;
    let lh_f64 = first.layer_height;

    let mut n: u32 = 0;
    loop {
        let base_z_f64 = first_z_f64 + (n as f64) * lh_f64;
        if base_z_f64 > max_height + 1e-6 {
            break;
        }
        let z_f64 = raft_top + base_z_f64;
        // Single terminal `as f32` cast: the Z is computed entirely in `f64`.
        let z = z_f64 as f32;
        let regions: Vec<RegionLayerProposal> = plans
            .iter()
            .filter(|p| base_z_f64 <= p.height + 1e-6)
            .map(|p| {
                let effective_lh = if layers.is_empty() {
                    p.first_layer_height
                } else {
                    p.layer_height
                };
                RegionLayerProposal {
                    object_id: p.object_id.clone(),
                    region_id: "0".to_string(),
                    effective_layer_height: effective_lh as f32,
                    is_catchup: false,
                    catchup_z_bottom: 0.0,
                }
            })
            .collect();

        if !regions.is_empty() {
            layers.push(MergedLayer { z, regions });
        }
        n += 1;
    }
    layers
}

/// Merge layers for objects with different layer heights using LCM sync.
///
/// At every global Z plane (union of all objects' native layers), every active
/// object participates. Objects without a native layer at that Z get a catch-up
/// layer bridging from their last participated Z to the current one.
fn merge_different_heights(plans: &[ObjectPlan], raft_top: f64) -> Vec<MergedLayer> {
    // Generate per-object Z sequences
    let object_zs: Vec<Vec<f32>> = plans
        .iter()
        .map(|plan| generate_object_layers(plan, raft_top))
        .collect();

    // Collect all unique Z values, sorted
    let mut all_zs: Vec<f32> = object_zs.iter().flatten().copied().collect();
    all_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut layers = Vec::new();
    // Track the last Z at which each object participated
    let mut last_z: Vec<f32> = vec![0.0; plans.len()];

    for &z in &all_zs {
        let mut regions = Vec::new();
        let z_f64 = z as f64;
        let base_z_f64 = z_f64 - raft_top;

        for (i, plan) in plans.iter().enumerate() {
            if base_z_f64 > plan.height + 1e-6 {
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
                    effective_layer_height: effective_lh as f32,
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
    fn from_config_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        DefaultLayerPlanner::from_config(&config).unwrap();
    }
}
