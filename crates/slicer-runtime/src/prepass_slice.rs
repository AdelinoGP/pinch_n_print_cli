//! Host built-in `PrePass::Slice` (formerly `Layer::Slice`).
//!
//! Produces a `Vec<SliceIR>` for the print by calling `slicer_core::slice_mesh_ex`
//! on each object mesh at every global layer's Z and, when configured,
//! applying the OrcaSlicer-style `slice_closing_radius` round-trip
//! (`+r` then `-r` Clipper2 offsets) per-region.
//!
//! The committed Vec is the canonical pre-Tier-2 slice geometry that
//! `PrePass::ShellClassification` then refines with `top_shell_index` /
//! `bottom_shell_index` / `top_solid_fill` / `bottom_solid_fill` annotations.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use slicer_core::polygon_ops::{intersection, offset, OffsetJoinType};
use slicer_core::slice_mesh_ex;
use slicer_core::triangle_mesh_slicer::apply_slice_closing_radius;
use slicer_ir::{
    ExPolygon, GlobalLayer, MeshIR, ObjectId, ObjectMesh, ObjectSurfaceData, Polygon, RegionKey,
    RegionMapIR, SemVer, SliceIR, SlicedRegion, SurfaceClassificationIR, Transform3d,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_ir::{Point2, Point3};

use crate::blackboard::{Blackboard, BlackboardError};
use crate::dag::BuiltinProducer;

/// `BuiltinProducer` for the host-side `PrePass::Slice` step.
pub static SLICE_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:slice",
    stage: "PrePass::Slice",
    ir_writes: &["SliceIR"],
    ir_reads: &[],
    claims_holds: &["host:slice"],
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

/// `BuiltinProducer` for the host-side `PrePass::ShellClassification` step.
pub static SHELL_CLASSIFICATION_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:shell_classification",
    stage: "PrePass::ShellClassification",
    ir_writes: &["SliceIR"],
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

/// Structured failures for the host built-in `PrePass::Slice` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSliceError {
    /// A layer referenced an `ObjectId` that is not present in `MeshIR`.
    UnknownObject {
        /// Layer that referenced the unknown object.
        layer_index: u32,
        /// The missing object identifier.
        object_id: ObjectId,
    },
    /// `commit_slice_builtin` could not commit the produced Vec — typically
    /// because `PrePass::Slice` was committed twice for the same print.
    Blackboard(BlackboardError),
    /// `PrePass::Slice` was invoked before `PrePass::LayerPlanning` committed
    /// `LayerPlanIR`. The all-layers wrapper has no global layers to iterate
    /// without a committed plan.
    MissingLayerPlan,
}

impl fmt::Display for LayerSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObject {
                layer_index,
                object_id,
            } => write!(
                f,
                "PrePass::Slice at layer {layer_index} references unknown object '{object_id}'"
            ),
            Self::Blackboard(inner) => write!(f, "PrePass::Slice blackboard error: {inner}"),
            Self::MissingLayerPlan => write!(
                f,
                "PrePass::Slice ran before PrePass::LayerPlanning committed LayerPlanIR"
            ),
        }
    }
}

impl From<BlackboardError> for LayerSliceError {
    fn from(value: BlackboardError) -> Self {
        Self::Blackboard(value)
    }
}

impl std::error::Error for LayerSliceError {}

// ============================================================================
// Internal geometry helpers
// ============================================================================

/// Apply a 4x4 column-major transform to a 3-D point.
/// A zero matrix is treated as identity for robustness.
fn transform_point(t: &Transform3d, p: &Point3) -> Point3 {
    slicer_core::transform_point3(&t.matrix, *p)
}

/// Ray-casting point-in-polygon test (integer coordinate space).
/// Returns `true` when `pt` lies strictly inside or on the boundary of `ring`.
fn point_in_ring(ring: &Polygon, pt: Point2) -> bool {
    let pts = &ring.points;
    if pts.len() < 3 {
        return false;
    }
    let px = pt.x;
    let py = pt.y;
    let mut inside = false;
    let n = pts.len();
    let mut j = n - 1;
    for i in 0..n {
        let xi = pts[i].x;
        let yi = pts[i].y;
        let xj = pts[j].x;
        let yj = pts[j].y;
        // Boundary check: point lies on the segment i→j
        if (yi == py && xi == px) || (yj == py && xj == px) {
            return true;
        }
        if ((yi > py) != (yj > py))
            && (px as i128)
                < ((xj - xi) as i128 * (py - yi) as i128 / (yj - yi) as i128 + xi as i128)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Return `true` if any of `polygons` contains `pt` in its contour ring.
fn any_polygon_contains(polygons: &[Polygon], pt: Point2) -> bool {
    polygons.iter().any(|p| point_in_ring(p, pt))
}

// ============================================================================
// Public helper: classify_region_surfaces
// ============================================================================

/// Detect whether the given region carries any bridge facets at this layer.
///
/// Returns `true` when at least one facet listed in
/// `surface_data.bridge_regions[*].facet_indices` has a world-Z range
/// straddling `layer_z` AND any of its three vertices' XY lies inside one
/// of `region_polygons`.
///
/// Top/bottom Z-window detection was removed in the slicing-promotion
/// refactor — `PrePass::ShellClassification` computes `top_shell_index` /
/// `bottom_shell_index` and the polygon-precise `top_solid_fill` /
/// `bottom_solid_fill` cross-layer instead. The `next_layer_z` / `prev_layer_z`
/// / `top_shell_layers` / `bottom_shell_layers` parameters that used to drive
/// the dead per-layer top/bottom path are gone.
pub fn classify_region_surfaces(
    object_mesh: &ObjectMesh,
    surface_data: &ObjectSurfaceData,
    region_polygons: &[Polygon],
    layer_z: f32,
) -> bool {
    let mesh = &object_mesh.mesh;
    let t = &object_mesh.transform;
    let tri_count = mesh.indices.len() / 3;

    let bridge_set: std::collections::HashSet<u32> = surface_data
        .bridge_regions
        .iter()
        .flat_map(|br| br.facet_indices.iter().copied())
        .collect();

    for tri_idx in 0..tri_count {
        if !bridge_set.contains(&(tri_idx as u32)) {
            continue;
        }

        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let wv0 = transform_point(t, &mesh.vertices[i0]);
        let wv1 = transform_point(t, &mesh.vertices[i1]);
        let wv2 = transform_point(t, &mesh.vertices[i2]);

        let fz_min = wv0.z.min(wv1.z).min(wv2.z);
        let fz_max = wv0.z.max(wv1.z).max(wv2.z);
        // Z range straddles layer_z: z_min ≤ layer_z ≤ z_max
        if fz_min <= layer_z && layer_z <= fz_max {
            let p0 = Point2::from_mm(wv0.x, wv0.y);
            let p1 = Point2::from_mm(wv1.x, wv1.y);
            let p2 = Point2::from_mm(wv2.x, wv2.y);
            if any_polygon_contains(region_polygons, p0)
                || any_polygon_contains(region_polygons, p1)
                || any_polygon_contains(region_polygons, p2)
            {
                return true;
            }
        }
    }

    false
}

// ============================================================================
// assemble_bridge_areas
// ============================================================================

/// Assemble expanded bridge polygons for a slice region.
///
/// For each valid `BridgeRegion` whose `xy_footprint` overlaps the region's
/// infill areas, computes:
///
/// `bridge_polygon = (xy_footprint ∩ infill_areas) ⊕ expansion_margin_mm`
/// `bridge_polygon = bridge_polygon ∩ infill_areas`
///
/// where `⊕` is Minkowski expansion via polygon offset. The result populates
/// `SlicedRegion.bridge_areas`. The `bridge_orientation_deg` is set to the
/// `bridge_direction_deg` of the intersecting bridge region with the largest
/// `bridge_length_mm`.
///
/// Requires `surface_class` to access `bridge_regions` with their `xy_footprint`
/// and `expansion_margin_mm`.
pub fn assemble_bridge_areas(
    region: &mut SlicedRegion,
    surface_class: Option<&SurfaceClassificationIR>,
) {
    let Some(sc) = surface_class else {
        return;
    };
    let Some(obj_data) = sc.per_object.get(&region.object_id) else {
        return;
    };

    let mut best_orientation_deg = 0.0_f32;
    let mut best_bridge_length = 0.0_f32;

    for br in &obj_data.bridge_regions {
        if !br.is_valid {
            continue;
        }
        if br.xy_footprint.is_empty() {
            continue;
        }
        if !br.expansion_margin_mm.is_finite() || br.expansion_margin_mm < 0.0 {
            continue;
        }

        // Check if xy_footprint overlaps region.infill_areas
        let footprint_as_expoly: Vec<ExPolygon> = br.xy_footprint.to_vec();
        let intersection_result = intersection(&footprint_as_expoly, &region.infill_areas);
        if intersection_result.is_empty() {
            continue;
        }

        // Offset by +expansion_margin_mm (Minkowski expansion)
        let expanded: Vec<ExPolygon> = offset(
            &intersection_result,
            br.expansion_margin_mm,
            OffsetJoinType::Miter,
            0.0,
        );

        // Intersect expanded result back with infill_areas to keep inside region
        let final_polys = intersection(&expanded, &region.infill_areas);

        if final_polys.is_empty() {
            continue;
        }

        // Accumulate the bridge polygons
        region.bridge_areas.extend(final_polys);

        // Track best orientation (largest bridge_length_mm wins)
        if br.bridge_length_mm > best_bridge_length {
            best_bridge_length = br.bridge_length_mm;
            best_orientation_deg = br.bridge_direction_deg;
        }
    }

    region.bridge_orientation_deg = best_orientation_deg;
}

// ============================================================================
// execute_prepass_slice_single_layer
// ============================================================================

/// Produce the `SliceIR` for `layer` by slicing every referenced object mesh
/// at `layer.z`.
///
/// Deterministic: regions are emitted in `layer.active_regions` order.
/// If `layer.active_regions` is empty the returned `SliceIR` has an empty
/// `regions` vector (e.g. a layer with no participating objects).
///
/// When `surface_class` is `Some`, [`classify_region_surfaces`] is called per
/// region to populate `is_bridge`. Top/bottom annotation is the job of
/// `PrePass::ShellClassification`, not this function — `top_shell_index` and
/// `bottom_shell_index` always emit as `None` here.
pub fn execute_prepass_slice_single_layer(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
) -> Result<SliceIR, LayerSliceError> {
    let mut regions = Vec::with_capacity(layer.active_regions.len());
    for active in &layer.active_regions {
        let object = mesh
            .objects
            .iter()
            .find(|o| o.id == active.object_id)
            .ok_or_else(|| LayerSliceError::UnknownObject {
                layer_index: layer.index,
                object_id: active.object_id.clone(),
            })?;

        // Resolve per-region config from RegionMapIR (single lookup; reused for
        // shell-layer counts and slice_closing_radius). Two fallback cases are
        // distinguished:
        //   - region_map = None         => documented test path; silent fallback.
        //   - region_map = Some, miss   => partial RegionMap; this is a scheduler
        //                                  contract violation. Warn loudly +
        //                                  debug_assert (panics in tests so we
        //                                  surface the bug; logs in release).
        let resolved_plan = if let Some(rm) = region_map {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: active.object_id.clone(),
                region_id: active.region_id,
            };
            let entry = rm.entries.get(&key);
            if entry.is_none() {
                log::warn!(
                    "PrePass::Slice: region_map present but missing entry for \
                     (layer={}, object={}, region={}) — falling back to legacy \
                     defaults; this indicates a partial RegionMap commit",
                    layer.index,
                    active.object_id,
                    active.region_id,
                );
                debug_assert!(
                    false,
                    "PrePass::Slice: region_map present but lookup miss for \
                     (layer={}, object={}, region={}); partial RegionMap \
                     violates the scheduler contract",
                    layer.index, active.object_id, active.region_id,
                );
            }
            entry
        } else {
            None
        };
        let slice_closing_radius_mm = match resolved_plan {
            Some(plan) => plan.config.slice_closing_radius,
            None => 0.0_f32,
        };

        // OrcaSlicer-style slice_closing_radius round-trip (`+r / -r` Clipper2),
        // applied per region's polygons immediately after `slice_mesh_ex`. Closes
        // a packet-60 wiring gap: the helper existed in slicer-core but was never
        // invoked by the production slice path.
        let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
        let raw_polygons = sliced.pop().unwrap_or_default();
        let polygons = if slice_closing_radius_mm > 0.0 {
            apply_slice_closing_radius(raw_polygons, slice_closing_radius_mm)
        } else {
            raw_polygons
        };

        // Extract region contours for bridge classification (ExPolygon → Polygon).
        // When the slice produces no polygons (e.g. flat/horizontal triangles are
        // skipped by the slicer), fall back to a bounding-box polygon derived from
        // the object mesh's XY extents so the XY containment check does not
        // incorrectly exclude bridge-classified facets.
        //
        // Top/bottom Z-window detection happens cross-layer in
        // PrePass::ShellClassification — the per-layer top_shell_index /
        // bottom_shell_index fields below stay None here and get populated
        // by the slice-postprocess pass.
        let is_bridge = if let Some(sc) = surface_class {
            if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                let contours: Vec<Polygon> = if polygons.is_empty() {
                    // Derive XY bounding box from the untransformed mesh vertices.
                    let verts = &object.mesh.vertices;
                    if verts.is_empty() {
                        Vec::new()
                    } else {
                        let mut min_x = verts[0].x;
                        let mut max_x = verts[0].x;
                        let mut min_y = verts[0].y;
                        let mut max_y = verts[0].y;
                        for v in verts.iter().skip(1) {
                            if v.x < min_x {
                                min_x = v.x;
                            }
                            if v.x > max_x {
                                max_x = v.x;
                            }
                            if v.y < min_y {
                                min_y = v.y;
                            }
                            if v.y > max_y {
                                max_y = v.y;
                            }
                        }
                        vec![Polygon {
                            points: vec![
                                Point2::from_mm(min_x, min_y),
                                Point2::from_mm(max_x, min_y),
                                Point2::from_mm(max_x, max_y),
                                Point2::from_mm(min_x, max_y),
                            ],
                        }]
                    }
                } else {
                    polygons.iter().map(|ep| ep.contour.clone()).collect()
                };
                classify_region_surfaces(object, obj_data, &contours, layer.z)
            } else {
                false
            }
        } else {
            false
        };

        let mut sliced_region = SlicedRegion {
            object_id: active.object_id.clone(),
            region_id: active.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: active.effective_layer_height,
            boundary_paint: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        };

        // Assemble expanded bridge polygons using bridge regions from surface classification.
        assemble_bridge_areas(&mut sliced_region, surface_class);

        regions.push(sliced_region);
    }

    Ok(SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: layer.index,
        z: layer.z,
        regions,
    })
}

// ============================================================================
// PrePass::Slice host built-in (whole-print wrappers)
// ============================================================================

/// Whole-print wrapper: produces one `SliceIR` per `global_layer` in the
/// committed layer plan, in plan order. Reads `MeshIR`, `LayerPlanIR`,
/// `SurfaceClassificationIR`, and `RegionMapIR` (all immutable) from the
/// blackboard. Used by [`commit_slice_builtin`].
pub fn execute_prepass_slice_all_layers(
    blackboard: &Blackboard,
) -> Result<Vec<SliceIR>, LayerSliceError> {
    let mesh = blackboard.mesh();
    let layer_plan = blackboard
        .layer_plan()
        .ok_or(LayerSliceError::MissingLayerPlan)?;
    let surface_class = blackboard.surface_classification().map(|a| a.as_ref());
    let region_map = blackboard.region_map().map(|a| a.as_ref());

    layer_plan
        .global_layers
        .iter()
        .map(|gl| execute_prepass_slice_single_layer(mesh.as_ref(), gl, surface_class, region_map))
        .collect()
}

/// `PrePass::Slice` host built-in entry point. Computes the per-global-layer
/// `Vec<SliceIR>` from blackboard reads and commits it via
/// [`Blackboard::commit_slice_ir`]. A duplicate commit surfaces as
/// `LayerSliceError::Blackboard(BlackboardError::DuplicatePrepassCommit)` —
/// never silently swallowed.
pub fn commit_slice_builtin(blackboard: &mut Blackboard) -> Result<(), LayerSliceError> {
    let slices = execute_prepass_slice_all_layers(blackboard)?;
    blackboard.commit_slice_ir(Arc::new(slices))?;
    Ok(())
}
