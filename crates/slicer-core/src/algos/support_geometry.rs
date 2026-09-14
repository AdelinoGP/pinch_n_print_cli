// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Support geometry computation algorithms.
//!
//! Computes coarse support layer boundaries from `LayerPlanIR` and produces
//! a `SupportGeometryIR`.

use std::collections::{BTreeSet, HashMap};

use slicer_ir::{
    ExPolygon, LayerPlanIR, ObjectId, RegionId, ResolvedConfig, SliceIR, SupportGeometryIR,
    SupportGeometryKey,
};

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

/// Precompute, for each object, the set of global layer indices at which a
/// support layer boundary should be emitted.
pub fn build_emit_schedule(layer_plan: &LayerPlanIR) -> HashMap<String, BTreeSet<u32>> {
    let mut acc: HashMap<String, f32> = HashMap::new();
    let mut schedule: HashMap<String, BTreeSet<u32>> = HashMap::new();
    for gl in &layer_plan.global_layers {
        let mut seen: HashMap<&str, (f32, f32)> = HashMap::new();
        for region in &gl.active_regions {
            let oid = region.object_id.as_str();
            let target = region.resolved_config.support_layer_height_mm;
            let h = region.effective_layer_height;
            match seen.get(oid) {
                None => {
                    seen.insert(oid, (target, h));
                }
                Some(&(existing_target, _existing_h)) => {
                    debug_assert!(
                        (existing_target - target).abs() < f32::EPSILON,
                        "support_layer_height_mm disagreement among regions of \
                         object '{}' on layer {}; per-object invariant violated — \
                         see resolved_config.rs support_layer_height_mm doc",
                        oid,
                        gl.index
                    );
                }
            }
        }
        for (oid, (target, h)) in seen {
            let a = acc.entry(oid.to_string()).or_insert(0.0);
            *a += h;
            if target == 0.0 || *a >= target {
                schedule
                    .entry(oid.to_string())
                    .or_default()
                    .insert(gl.index);
                *a = 0.0;
            }
        }
    }
    schedule
}

/// Execute the built-in `PrePass::SupportGeometry` stage.
pub fn execute_support_geometry(
    layer_plan: &LayerPlanIR,
    slice_vec: &[SliceIR],
) -> Result<SupportGeometryIR, SupportGeometryBuiltinError> {
    let resolved_config = layer_plan
        .global_layers
        .iter()
        .flat_map(|layer| &layer.active_regions)
        .next()
        .map(|region| &region.resolved_config);
    let (support_layer_height_mm, support_top_z_distance_mm) = resolved_config
        .map(|config| {
            (
                config.support_layer_height_mm,
                config.support_top_z_distance,
            )
        })
        .unwrap_or_else(|| {
            let config = ResolvedConfig::default();
            (
                config.support_layer_height_mm,
                config.support_top_z_distance,
            )
        });

    let emit_schedule = build_emit_schedule(layer_plan);

    let mut entries: HashMap<SupportGeometryKey, Vec<ExPolygon>> = HashMap::new();

    for global_layer in &layer_plan.global_layers {
        for region in &global_layer.active_regions {
            let oid = &region.object_id;

            let should_emit = emit_schedule
                .get(oid)
                .map_or(false, |s| s.contains(&global_layer.index));

            if should_emit {
                let key = SupportGeometryKey {
                    global_support_layer_index: global_layer.index,
                    object_id: oid.clone(),
                    region_id: region.region_id,
                };

                let polygons = collect_polygons_at_z(
                    slice_vec,
                    layer_plan,
                    oid,
                    region.region_id,
                    global_layer.z,
                );

                entries.entry(key).or_default().extend(polygons);
            }
        }
    }

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
            if i >= slice_vec.len() {
                return Vec::new();
            }
            i
        }
    };
    extract_region_polys(&slice_vec[idx], object_id, region_id)
}

/// Pull the polygons for a specific `(object_id, region_id)` out of a single
/// committed `SliceIR`.
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
fn add_intermediate_model_layers(
    entries: &mut HashMap<SupportGeometryKey, Vec<ExPolygon>>,
    layer_plan: &LayerPlanIR,
    slice_vec: &[SliceIR],
    distance_mm: f32,
) {
    let mut column_tops: HashMap<String, f32> = HashMap::new();
    for layer in layer_plan.global_layers.iter().rev() {
        for region in &layer.active_regions {
            let current_top = column_tops.get(&region.object_id).copied().unwrap_or(0.0);
            if layer.z > current_top {
                column_tops.insert(region.object_id.clone(), layer.z);
            }
        }
    }

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
