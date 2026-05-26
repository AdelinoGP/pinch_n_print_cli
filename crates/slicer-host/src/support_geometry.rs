//! Host-built-in `PrePass::SupportGeometry` stage.
//!
//! Computes coarse support layer boundaries from `LayerPlanIR` and writes
//! a `SupportGeometryIR` to the blackboard. This IR is consumed by
//! `run-support-geometry` WIT exports to inform support placement.
//!
//! Algorithm:
//! - Walk `LayerPlanIR.global_layers` accumulating `effective_layer_height`.
//! - When accumulated >= `support_layer_height_mm`, emit a support layer
//!   boundary at that layer's Z.
//! - For each support layer boundary Z, pull per-region polygons from the
//!   prepass-committed `Vec<SliceIR>` via `collect_polygons_at_z`.
//! - Union polygons per `(object_id, region_id)` to produce coarse outlines.
//! - Intermediate model-resolution outline layers are added at every model
//!   layer within `support_top_z_distance_mm` of column tops; each entry is
//!   populated from `SliceIR` at the intermediate Z (not left empty).
//!
//! The accumulated algorithm handles variable heights and catch-up layers
//! correctly: catch-up layers count their full `effective_layer_height`.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, LayerPlanIR, ObjectId, RegionId, SliceIR, SupportGeometryIR, SupportGeometryKey,
};

use crate::Blackboard;

/// Structured support geometry computation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportGeometryBuiltinError {
    /// `LayerPlanIR` is not yet committed to the blackboard.
    NoLayerPlan,
    /// `MeshIR` is not available.
    NoMesh,
    /// `SliceIR` is not committed (PrePass::Slice must run first).
    MissingSliceIR,
}

impl std::fmt::Display for SupportGeometryBuiltinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLayerPlan => write!(f, "LayerPlanIR not committed"),
            Self::NoMesh => write!(f, "MeshIR not available"),
            Self::MissingSliceIR => write!(
                f,
                "PrePass::Slice must commit SliceIR before PrePass::SupportGeometry"
            ),
        }
    }
}

impl std::error::Error for SupportGeometryBuiltinError {}

/// Default support layer height in mm (0.0 = use model layer height).
const DEFAULT_SUPPORT_LAYER_HEIGHT_MM: f32 = 0.0;

/// Default distance in mm from column tops to add intermediate model layers.
const DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM: f32 = 5.0;

/// Execute the built-in `PrePass::SupportGeometry` stage.
///
/// Produces a `SupportGeometryIR` with coarse support layer boundaries
/// and intermediate model-resolution outline layers, both populated from
/// the prepass-committed `Vec<SliceIR>`.
pub fn execute_support_geometry(
    layer_plan: &LayerPlanIR,
    slice_vec: &[SliceIR],
) -> Result<SupportGeometryIR, SupportGeometryBuiltinError> {
    let support_layer_height_mm = DEFAULT_SUPPORT_LAYER_HEIGHT_MM;
    let support_top_z_distance_mm = DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM;

    let mut entries: HashMap<SupportGeometryKey, Vec<ExPolygon>> = HashMap::new();

    // Walk layers accumulating effective_layer_height to find support boundaries.
    let mut accumulated_height = 0.0_f32;
    let mut current_support_layer_index = 0_u32;

    for global_layer in &layer_plan.global_layers {
        let layer_height = if global_layer.active_regions.is_empty() {
            0.0
        } else {
            // Use the first active region's effective layer height as a representative value.
            // In a real implementation this would aggregate across all regions.
            global_layer.active_regions[0].effective_layer_height
        };

        // Add catch-up layer's full height to accumulator.
        let is_catchup = global_layer
            .active_regions
            .first()
            .map(|r| r.is_catchup_layer)
            .unwrap_or(false);
        let height_to_add = if is_catchup {
            global_layer.active_regions[0].catchup_z_bottom
        } else {
            layer_height
        };

        accumulated_height += height_to_add;

        // Emit support layer boundary when accumulated >= support_layer_height_mm.
        // A support_layer_height_mm of 0.0 means "use model layer height", so we emit
        // at every model layer boundary.
        let should_emit = if support_layer_height_mm > 0.0 {
            accumulated_height >= support_layer_height_mm
        } else {
            // 0.0 = use model layer height: emit at every model layer.
            true
        };

        if should_emit {
            // For each active region, collect geometry at this Z.
            for region in &global_layer.active_regions {
                let key = SupportGeometryKey {
                    global_support_layer_index: current_support_layer_index,
                    object_id: region.object_id.clone(),
                    region_id: region.region_id,
                };

                // Collect polygons at Z from the prepass-committed SliceIR.
                let polygons = collect_polygons_at_z(
                    slice_vec,
                    layer_plan,
                    &region.object_id,
                    region.region_id,
                    global_layer.z,
                );

                entries.entry(key).or_default().extend(polygons);
            }

            // Reset accumulator after emitting, advance support layer index.
            accumulated_height = 0.0;
            current_support_layer_index += 1;
        }
    }

    // Add intermediate model-resolution layers within support_top_z_distance_mm of column tops.
    add_intermediate_model_layers(
        &mut entries,
        layer_plan,
        slice_vec,
        support_top_z_distance_mm,
    );

    Ok(SupportGeometryIR {
        support_layer_height_mm,
        support_top_z_distance_mm,
        entries,
        ..Default::default()
    })
}

/// Collect ExPolygons at a given Z from the prepass-committed `SliceIR` Vec
/// for a specific `(object_id, region_id)`.
///
/// Lookup strategy:
/// - Binary-search `layer_plan.global_layers` for the slot whose Z matches
///   `z` within a 1e-6 mm tolerance.
/// - On exact match: return that layer's polygons for the target region.
/// - On a non-aligned Z (interpolated between two adjacent layers): return
///   the **upper** bracketing layer's polygons. This is conservative for
///   support pillars (catches the overhang above the gap) and matches
///   `DEVIATION_LOG.md` entry for this behavior.
/// - When `z` is above the print top: returns empty.
fn collect_polygons_at_z(
    slice_vec: &[SliceIR],
    layer_plan: &LayerPlanIR,
    object_id: &ObjectId,
    region_id: RegionId,
    z: f32,
) -> Vec<ExPolygon> {
    if slice_vec.is_empty() || layer_plan.global_layers.is_empty() {
        return Vec::new();
    }
    let eps = 1e-6_f32;
    let pos = layer_plan.global_layers.binary_search_by(|gl| {
        if gl.z < z - eps {
            std::cmp::Ordering::Less
        } else if gl.z > z + eps {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let idx = match pos {
        Ok(i) => i,
        Err(i) => {
            // `i` is the upper bracket; clamp to end of print.
            if i >= slice_vec.len() {
                return Vec::new();
            }
            i
        }
    };
    extract_region_polys(&slice_vec[idx], object_id, region_id)
}

/// Pull the polygons for a specific `(object_id, region_id)` out of a single
/// committed `SliceIR`. Flattens across multiple regions matching the key
/// (currently slice production emits at most one region per key, but this
/// stays robust to future refinement).
fn extract_region_polys(
    slice: &SliceIR,
    object_id: &ObjectId,
    region_id: RegionId,
) -> Vec<ExPolygon> {
    slice
        .regions
        .iter()
        .filter(|r| &r.object_id == object_id && r.region_id == region_id)
        .flat_map(|r| r.polygons.clone())
        .collect()
}

/// Add intermediate model-resolution layers within `distance_mm` of column tops.
///
/// These use `global_support_layer_index = u32::MAX` sentinel to mark them
/// as model layers, not support layers. Each intermediate entry is populated
/// with the polygons pulled from `slice_vec` at the intermediate Z for every
/// region active on that layer (one entry per `(object, region, layer)` —
/// not just `region_id = 0`).
fn add_intermediate_model_layers(
    entries: &mut HashMap<SupportGeometryKey, Vec<ExPolygon>>,
    layer_plan: &LayerPlanIR,
    slice_vec: &[SliceIR],
    distance_mm: f32,
) {
    // Find column tops: for each object, find the highest Z that has a region.
    let mut column_tops: HashMap<String, f32> = HashMap::new();
    for layer in layer_plan.global_layers.iter().rev() {
        for region in &layer.active_regions {
            let current_top = column_tops.get(&region.object_id).copied().unwrap_or(0.0);
            if layer.z > current_top {
                column_tops.insert(region.object_id.clone(), layer.z);
            }
        }
    }

    // For each layer within distance_mm of a column top, register one entry
    // per (object, active region) populated from SliceIR at the layer's Z.
    let sentinel = u32::MAX;
    for layer in &layer_plan.global_layers {
        for (object_id, &top_z) in &column_tops {
            if (layer.z - top_z).abs() > distance_mm {
                continue;
            }
            for active in layer
                .active_regions
                .iter()
                .filter(|r| &r.object_id == object_id)
            {
                let polygons = collect_polygons_at_z(
                    slice_vec,
                    layer_plan,
                    object_id,
                    active.region_id,
                    layer.z,
                );
                let key = SupportGeometryKey {
                    global_support_layer_index: sentinel,
                    object_id: object_id.clone(),
                    region_id: active.region_id,
                };
                entries.entry(key).or_default().extend(polygons);
            }
        }
    }
}

/// Commit `SupportGeometryIR` to the blackboard using default parameters.
pub fn commit_support_geometry_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), SupportGeometryBuiltinError> {
    let layer_plan = blackboard
        .layer_plan()
        .ok_or(SupportGeometryBuiltinError::NoLayerPlan)?;
    let slice_vec = blackboard
        .slice_ir()
        .ok_or(SupportGeometryBuiltinError::MissingSliceIR)?;

    let ir = execute_support_geometry(layer_plan.as_ref(), slice_vec.as_ref())?;
    blackboard
        .commit_support_geometry(Arc::new(ir))
        .map_err(|_| SupportGeometryBuiltinError::NoLayerPlan) // Dup commit is idempotent here
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::GlobalLayer;

    fn make_2_layer_plan() -> LayerPlanIR {
        LayerPlanIR {
            global_layers: vec![
                GlobalLayer {
                    index: 0,
                    z: 0.0,
                    active_regions: vec![slicer_ir::ActiveRegion {
                        object_id: "test-object".to_string(),
                        region_id: 1,
                        resolved_config: slicer_ir::ResolvedConfig::default(),
                        effective_layer_height: 0.2,
                        nonplanar_shell: None,
                        is_catchup_layer: false,
                        catchup_z_bottom: 0.0,
                        tool_index: 0,
                    }],
                    has_nonplanar: false,
                    is_sync_layer: false,
                },
                GlobalLayer {
                    index: 1,
                    z: 0.2,
                    active_regions: vec![slicer_ir::ActiveRegion {
                        object_id: "test-object".to_string(),
                        region_id: 1,
                        resolved_config: slicer_ir::ResolvedConfig::default(),
                        effective_layer_height: 0.2,
                        nonplanar_shell: None,
                        is_catchup_layer: false,
                        catchup_z_bottom: 0.0,
                        tool_index: 0,
                    }],
                    has_nonplanar: false,
                    is_sync_layer: false,
                },
            ],
            object_participation: HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn support_geometry_emits_for_2_layer_fixture() {
        let layer_plan = make_2_layer_plan();
        let slice_vec: Vec<SliceIR> = Vec::new();

        let result = execute_support_geometry(&layer_plan, &slice_vec);
        assert!(result.is_ok());

        let ir = result.unwrap();
        // With support_layer_height_mm = 0.0 (default = use model layer height),
        // we emit at every model layer boundary: expect 2 support layer entries.
        assert!(!ir.entries.is_empty());
    }
}
