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

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use slicer_ir::{
    ExPolygon, LayerPlanIR, ObjectId, RegionId, SemVer, SliceIR, SupportGeometryIR,
    SupportGeometryKey,
};

use crate::dag::BuiltinProducer;
use crate::Blackboard;

/// `BuiltinProducer` for the host-side `PrePass::SupportGeometry` step.
pub static SUPPORT_GEOMETRY_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:support_geometry",
    stage: "PrePass::SupportGeometry",
    ir_writes: &["SupportGeometryIR"],
    ir_reads: &[],
    claims_holds: &[],
    claims_requires: &[],
    requires_modules: &[],
    min_ir_schema: SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    },
    max_ir_schema: SemVer {
        major: 4,
        minor: 0,
        patch: 0,
    },
    _cache_ir_writes: OnceLock::new(),
    _cache_ir_reads: OnceLock::new(),
    _cache_claims_holds: OnceLock::new(),
    _cache_claims_requires: OnceLock::new(),
    _cache_requires_modules: OnceLock::new(),
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
///
/// The schedule is derived from `region.resolved_config.support_layer_height_mm`:
/// - `0.0` means "use the object's effective layer height" → emit at every
///   model layer (every global layer index where the object is active).
/// - Any positive value accumulates `effective_layer_height` per region visit
///   and emits whenever the running total reaches or exceeds the target.
///
/// Each object's accumulator resets after each emission, so coarser support
/// layers are spaced correctly even when the model uses variable layer heights.
///
/// **Multi-region collapse**: when an object has ≥2 active regions on the same
/// global layer, the accumulator advances exactly once per `(object, layer)`
/// using the first-encountered region's `support_layer_height_mm` /
/// `effective_layer_height`. `support_layer_height_mm` is a **per-object**
/// config key (`crates/slicer-ir/src/resolved_config.rs §support_layer_height_mm`)
/// and a `debug_assert!` here enforces inter-region agreement on the same layer.
/// Per-region cadence was evaluated and explicitly scoped out (see DEV-064);
/// the per-object invariant is the intended final contract, not a transitional
/// placeholder.
pub(crate) fn build_emit_schedule(layer_plan: &LayerPlanIR) -> HashMap<String, BTreeSet<u32>> {
    let mut acc: HashMap<String, f32> = HashMap::new();
    let mut schedule: HashMap<String, BTreeSet<u32>> = HashMap::new();
    for gl in &layer_plan.global_layers {
        // Collapse per-(object, layer): first-encountered region's target/height
        // wins. `support_layer_height_mm` is a per-object config (DEV-064 scope-out
        // of per-region cadence); the debug_assert below enforces inter-region
        // agreement on the same object/layer to surface mis-stamped overlays.
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
///
/// Produces a `SupportGeometryIR` with coarse support layer boundaries
/// and intermediate model-resolution outline layers, both populated from
/// the prepass-committed `Vec<SliceIR>`.
///
/// Support layer emission is governed per-object by
/// `region.resolved_config.support_layer_height_mm`:
/// - `0.0` → emit at every model layer (use the object's effective layer height).
/// - positive → accumulate `effective_layer_height` across regions; emit when
///   the running total reaches or exceeds the target.
pub fn execute_support_geometry(
    layer_plan: &LayerPlanIR,
    slice_vec: &[SliceIR],
) -> Result<SupportGeometryIR, SupportGeometryBuiltinError> {
    let support_top_z_distance_mm = DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM;

    // Precompute per-object emit schedule from resolved_config.
    let emit_schedule = build_emit_schedule(layer_plan);

    let mut entries: HashMap<SupportGeometryKey, Vec<ExPolygon>> = HashMap::new();

    for global_layer in &layer_plan.global_layers {
        for region in &global_layer.active_regions {
            let oid = &region.object_id;

            let should_emit = emit_schedule
                .get(oid)
                .map_or(false, |s| s.contains(&global_layer.index));

            if should_emit {
                // global_support_layer_index is the MODEL layer index. The
                // support-planner guest at modules/core-modules/support-planner/
                // src/lib.rs:211 reads this field as an index into
                // collision_cache: Vec<LayerCollisionCache> (sized by
                // layer_plan.layers.len()). Anything other than the model layer
                // index here misaligns the collision cache (DEV-NNN, Q5c).
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

    // Add intermediate model-resolution layers within support_top_z_distance_mm of column tops.
    add_intermediate_model_layers(
        &mut entries,
        layer_plan,
        slice_vec,
        support_top_z_distance_mm,
    );

    // Use 0.0 as the stored support_layer_height_mm — per-object values are
    // consumed at schedule-build time; no single sentinel value applies.
    Ok(SupportGeometryIR {
        support_layer_height_mm: 0.0,
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
        // Two-layer plan with default support_layer_height_mm = 0.0.
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
        // With support_layer_height_mm = 0.0 (default = use model layer height),
        // we emit at every model layer boundary: expect 2 support layer entries.
        assert!(!ir.entries.is_empty());
    }

    /// Build a 6-layer plan (layers 0-5, each 0.2 mm) with two objects:
    /// - obj-A: `support_layer_height_mm = 0.4` → emit every 2 model layers
    /// - obj-B: `support_layer_height_mm = 0.0` → emit at every model layer
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

    /// Unit test for `build_emit_schedule` with two objects:
    /// - obj-A (`support_layer_height_mm=0.4`, 0.2mm model layers): emits
    ///   every second layer → global indices {1, 3, 5}.
    /// - obj-B (`support_layer_height_mm=0.0`): emits at every model layer
    ///   → global indices {0, 1, 2, 3, 4, 5}.
    #[test]
    fn build_emit_schedule_two_objects_per_object_semantics() {
        let plan = make_two_object_plan();
        let schedule = build_emit_schedule(&plan);

        let a_sched = schedule.get("obj-A").cloned().unwrap_or_default();
        let b_sched = schedule.get("obj-B").cloned().unwrap_or_default();

        // obj-A accumulates 0.2 per layer; emits when >= 0.4 -> layers 1, 3, 5.
        assert_eq!(
            a_sched,
            [1u32, 3, 5].iter().cloned().collect::<BTreeSet<u32>>(),
            "obj-A (support_layer_height_mm=0.4, model 0.2mm) must emit at layers {{1,3,5}}; \
             got {a_sched:?}"
        );

        // obj-B always emits (target=0.0) -> all six layers.
        assert_eq!(
            b_sched,
            (0u32..6).collect::<BTreeSet<u32>>(),
            "obj-B (support_layer_height_mm=0.0) must emit at every layer {{0..5}}; \
             got {b_sched:?}"
        );
    }
}
