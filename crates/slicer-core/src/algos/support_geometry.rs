//! Support geometry computation algorithms.
//!
//! Computes coarse support layer boundaries from `LayerPlanIR` and produces
//! a `SupportGeometryIR`.

use std::collections::{BTreeSet, HashMap};

use slicer_ir::{
    ExPolygon, LayerPlanIR, ObjectId, RegionId, SliceIR, SupportGeometryIR, SupportGeometryKey,
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

/// Default distance in mm from column tops to add intermediate model layers.
const DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM: f32 = 5.0;

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
    let support_top_z_distance_mm = DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM;

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
        support_layer_height_mm: 0.0,
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

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ActiveRegion, GlobalLayer, ResolvedConfig};

    fn make_active_region(object_id: &str, layer_height: f32, support_lh: f32) -> ActiveRegion {
        ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig {
                support_layer_height_mm: support_lh,
                ..ResolvedConfig::default()
            },
            effective_layer_height: layer_height,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }
    }

    fn make_2_layer_plan() -> LayerPlanIR {
        LayerPlanIR {
            global_layers: vec![
                GlobalLayer {
                    index: 0,
                    z: 0.0,
                    active_regions: vec![make_active_region("test-object", 0.2, 0.0)],
                    has_nonplanar: false,
                    is_sync_layer: false,
                },
                GlobalLayer {
                    index: 1,
                    z: 0.2,
                    active_regions: vec![make_active_region("test-object", 0.2, 0.0)],
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
        assert!(!ir.entries.is_empty());
    }

    fn make_two_object_plan() -> LayerPlanIR {
        let mut global_layers = Vec::new();
        for i in 0u32..6 {
            global_layers.push(GlobalLayer {
                index: i,
                z: (i + 1) as f32 * 0.2,
                active_regions: vec![
                    make_active_region("obj-A", 0.2, 0.4),
                    make_active_region("obj-B", 0.2, 0.0),
                ],
                has_nonplanar: false,
                is_sync_layer: false,
            });
        }
        LayerPlanIR {
            global_layers,
            object_participation: HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn build_emit_schedule_two_objects_per_object_semantics() {
        let plan = make_two_object_plan();
        let schedule = build_emit_schedule(&plan);

        let a_sched = schedule.get("obj-A").cloned().unwrap_or_default();
        let b_sched = schedule.get("obj-B").cloned().unwrap_or_default();

        assert_eq!(
            a_sched,
            [1u32, 3, 5].iter().cloned().collect::<BTreeSet<u32>>(),
            "obj-A (support_layer_height_mm=0.4, model 0.2mm) must emit at layers {{1,3,5}}; \
             got {a_sched:?}"
        );

        assert_eq!(
            b_sched,
            (0u32..6).collect::<BTreeSet<u32>>(),
            "obj-B (support_layer_height_mm=0.0) must emit at every layer {{0..5}}; \
             got {b_sched:?}"
        );
    }
}
