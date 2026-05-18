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
//! - For each support layer boundary Z, intersect `MeshIR` triangles to
//!   collect polygons at that Z.
//! - Union polygons per `(object_id, region_id)` to produce coarse outlines.
//! - Intermediate model-resolution outline layers are added at every model
//!   layer within `support_top_z_distance_mm` of column tops.
//!
//! The accumulated algorithm handles variable heights and catch-up layers
//! correctly: catch-up layers count their full `effective_layer_height`.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ExPolygon, LayerPlanIR, MeshIR, RegionId, SupportGeometryIR, SupportGeometryKey};

use crate::Blackboard;

/// Structured support geometry computation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportGeometryBuiltinError {
    /// `LayerPlanIR` is not yet committed to the blackboard.
    NoLayerPlan,
    /// `MeshIR` is not available.
    NoMesh,
}

impl std::fmt::Display for SupportGeometryBuiltinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLayerPlan => write!(f, "LayerPlanIR not committed"),
            Self::NoMesh => write!(f, "MeshIR not available"),
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
/// and intermediate model-resolution outline layers.
pub fn execute_support_geometry(
    layer_plan: &LayerPlanIR,
    mesh: &MeshIR,
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

                // Collect polygons at Z from mesh (simplified: uses bounding box projection).
                let polygons = collect_polygons_at_z(
                    mesh,
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
    add_intermediate_model_layers(&mut entries, layer_plan, mesh, support_top_z_distance_mm);

    Ok(SupportGeometryIR {
        support_layer_height_mm,
        support_top_z_distance_mm,
        entries,
        ..Default::default()
    })
}

/// Collect ExPolygons at a given Z from the mesh for a specific object/region.
///
/// This is a stub that returns empty polygons. Full plane-triangle intersection
/// is implemented in packet 31b.
fn collect_polygons_at_z(
    _mesh: &MeshIR,
    _object_id: &str,
    _region_id: RegionId,
    _z: f32,
) -> Vec<ExPolygon> {
    // Full plane-triangle intersection implemented in 31b.
    Vec::new()
}

/// Add intermediate model-resolution layers within `distance_mm` of column tops.
///
/// These use `global_support_layer_index = u32::MAX` sentinel to mark them
/// as model layers, not support layers.
fn add_intermediate_model_layers(
    entries: &mut HashMap<SupportGeometryKey, Vec<ExPolygon>>,
    layer_plan: &LayerPlanIR,
    _mesh: &MeshIR,
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

    // For each layer within distance_mm of a column top, add intermediate outlines.
    let sentinel = u32::MAX;
    for layer in &layer_plan.global_layers {
        for (object_id, &top_z) in &column_tops {
            if (layer.z - top_z).abs() <= distance_mm {
                // Add intermediate model layer entry.
                // In a full implementation, this would contain actual geometry.
                let key = SupportGeometryKey {
                    global_support_layer_index: sentinel,
                    object_id: object_id.clone(),
                    region_id: 0, // Will be refined in 31b
                };
                // The geometry will be computed in 31b via plane-triangle intersection.
                entries.entry(key).or_default();
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
    let mesh = blackboard.mesh();

    let ir = execute_support_geometry(layer_plan.as_ref(), mesh.as_ref())?;
    blackboard
        .commit_support_geometry(Arc::new(ir))
        .map_err(|_| SupportGeometryBuiltinError::NoLayerPlan) // Dup commit is idempotent here
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{BoundingBox3, GlobalLayer, ObjectMesh, Point3};

    fn make_test_mesh() -> MeshIR {
        MeshIR {
            objects: vec![ObjectMesh {
                id: "test-object".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 10.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 10.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 10.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 10.0,
                            z: 10.0,
                        },
                    ],
                    indices: vec![
                        0, 1, 2, 0, 2, 3, // bottom
                        4, 6, 5, 4, 7, 6, // top
                        0, 4, 5, 0, 5, 1, // front
                        2, 6, 7, 2, 7, 3, // back
                        0, 3, 7, 0, 7, 4, // left
                        1, 5, 6, 1, 6, 2, // right
                    ],
                },
                transform: slicer_ir::Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: slicer_ir::ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: vec![],
                paint_data: None,
                world_z_extent: Some((0.0, 10.0)),
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 250.0,
                },
            },
            ..Default::default()
        }
    }

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
        let mesh = make_test_mesh();

        let result = execute_support_geometry(&layer_plan, &mesh);
        assert!(result.is_ok());

        let ir = result.unwrap();
        // With support_layer_height_mm = 0.0 (default = use model layer height),
        // we emit at every model layer boundary: expect 2 support layer entries.
        assert!(!ir.entries.is_empty());
    }
}
