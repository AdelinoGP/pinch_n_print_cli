//! Host-built-in `Layer::Slice` stage (TASK-107).
//!
//! Produces a `SliceIR` for a single global layer by calling
//! `slicer_core::slice_mesh_ex` on each object mesh at the layer's Z. The
//! `SliceIR` is staged in the per-layer arena before any user
//! `Layer::Slice` / `Layer::SlicePostProcess` module runs.

use std::collections::HashMap;
use std::fmt;

use slicer_core::polygon_ops::{intersection, offset, OffsetJoinType};
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ExPolygon, GlobalLayer, LayerPlanIR, MeshIR, ObjectId, ObjectMesh, ObjectSurfaceData, Polygon,
    RegionKey, RegionMapIR, SliceIR, SlicedRegion, SurfaceClassificationIR, Transform3d,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_ir::{FacetClass, Point2, Point3};

/// Structured failures for the host-built-in `Layer::Slice` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSliceError {
    /// A layer referenced an `ObjectId` that is not present in `MeshIR`.
    UnknownObject {
        /// Layer that referenced the unknown object.
        layer_index: u32,
        /// The missing object identifier.
        object_id: ObjectId,
    },
}

impl fmt::Display for LayerSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObject {
                layer_index,
                object_id,
            } => write!(
                f,
                "Layer::Slice at layer {layer_index} references unknown object '{object_id}'"
            ),
        }
    }
}

impl std::error::Error for LayerSliceError {}

// ============================================================================
// Internal geometry helpers
// ============================================================================

/// Apply a 4x4 column-major transform to a 3-D point.
/// A zero matrix is treated as identity for robustness.
fn transform_point(t: &Transform3d, p: &Point3) -> Point3 {
    let m = &t.matrix;
    if m.iter().all(|v| *v == 0.0) {
        return *p;
    }
    let x = p.x as f64;
    let y = p.y as f64;
    let z = p.z as f64;
    Point3 {
        x: (m[0] * x + m[4] * y + m[8] * z + m[12]) as f32,
        y: (m[1] * x + m[5] * y + m[9] * z + m[13]) as f32,
        z: (m[2] * x + m[6] * y + m[10] * z + m[14]) as f32,
    }
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

/// Classify a region's external-surface flags from prepass classification +
/// adjacent-layer Z.
///
/// Returns `(is_top_surface, is_bottom_surface, is_bridge)`.
///
/// * Top: facet `z_min < next_layer_z`, `FacetClass::TopSurface`,
///   any vertex XY ∈ region polygon.
/// * Bottom: facet `z_max > prev_layer_z`, `FacetClass::BottomSurface`,
///   any vertex XY ∈ region polygon.
/// * Bridge: facet listed in `bridge_regions[*].facet_indices`, world-Z range
///   straddles `layer_z`, any vertex XY ∈ region polygon.
///
/// Each window degrades to `false` when its corresponding `*_layer_z` is `None`.
pub fn classify_region_surfaces(
    object_mesh: &ObjectMesh,
    surface_data: &ObjectSurfaceData,
    region_polygons: &[Polygon],
    layer_z: f32,
    next_layer_z: Option<f32>,
    prev_layer_z: Option<f32>,
    top_shell_layers: u32,
    bottom_shell_layers: u32,
) -> (bool, bool, bool) {
    let mesh = &object_mesh.mesh;
    let t = &object_mesh.transform;
    let tri_count = mesh.indices.len() / 3;

    let mut is_top = false;
    let mut is_bot = false;
    let mut is_bridge = false;

    // N=0 short-circuit: disable the respective flag regardless of geometry.
    // The loop still runs for bridge detection.
    let top_enabled = top_shell_layers > 0;
    let bot_enabled = bottom_shell_layers > 0;

    // Build a fast lookup set for bridge facet indices.
    let bridge_set: std::collections::HashSet<u32> = surface_data
        .bridge_regions
        .iter()
        .flat_map(|br| br.facet_indices.iter().copied())
        .collect();

    for tri_idx in 0..tri_count {
        if is_top && is_bot && is_bridge {
            break;
        }

        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

        // Guard against malformed meshes.
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let wv0 = transform_point(t, &mesh.vertices[i0]);
        let wv1 = transform_point(t, &mesh.vertices[i1]);
        let wv2 = transform_point(t, &mesh.vertices[i2]);

        let fz_min = wv0.z.min(wv1.z).min(wv2.z);
        let fz_max = wv0.z.max(wv1.z).max(wv2.z);

        let facet_class = surface_data.facet_classes.get(tri_idx).copied();

        // Top surface check.
        // Window: facet z_min ∈ [layer_z, next_layer_z) — inclusive low, exclusive high.
        // When next_layer_z is None the window is [layer_z, ∞).
        // top_shell_layers=0 disables this check entirely.
        if !is_top && top_enabled {
            if let Some(FacetClass::TopSurface) = facet_class {
                let in_window = match next_layer_z {
                    Some(nz) => layer_z <= fz_min && fz_min < nz,
                    None => layer_z <= fz_min,
                };
                if in_window {
                    let p0 = Point2::from_mm(wv0.x, wv0.y);
                    let p1 = Point2::from_mm(wv1.x, wv1.y);
                    let p2 = Point2::from_mm(wv2.x, wv2.y);
                    if any_polygon_contains(region_polygons, p0)
                        || any_polygon_contains(region_polygons, p1)
                        || any_polygon_contains(region_polygons, p2)
                    {
                        is_top = true;
                    }
                }
            }
        }

        // Bottom surface check.
        // Window: facet z_max ∈ (prev_layer_z, layer_z] — exclusive low, inclusive high.
        // When prev_layer_z is None the window is (-∞, layer_z].
        // bottom_shell_layers=0 disables this check entirely.
        if !is_bot && bot_enabled {
            if let Some(FacetClass::BottomSurface) = facet_class {
                let in_window = match prev_layer_z {
                    Some(pz) => pz < fz_max && fz_max <= layer_z,
                    None => fz_max <= layer_z,
                };
                if in_window {
                    let p0 = Point2::from_mm(wv0.x, wv0.y);
                    let p1 = Point2::from_mm(wv1.x, wv1.y);
                    let p2 = Point2::from_mm(wv2.x, wv2.y);
                    if any_polygon_contains(region_polygons, p0)
                        || any_polygon_contains(region_polygons, p1)
                        || any_polygon_contains(region_polygons, p2)
                    {
                        is_bot = true;
                    }
                }
            }
        }

        // Bridge check
        if !is_bridge && bridge_set.contains(&(tri_idx as u32)) {
            // Z range straddles layer_z: z_min ≤ layer_z ≤ z_max
            if fz_min <= layer_z && layer_z <= fz_max {
                let p0 = Point2::from_mm(wv0.x, wv0.y);
                let p1 = Point2::from_mm(wv1.x, wv1.y);
                let p2 = Point2::from_mm(wv2.x, wv2.y);
                if any_polygon_contains(region_polygons, p0)
                    || any_polygon_contains(region_polygons, p1)
                    || any_polygon_contains(region_polygons, p2)
                {
                    is_bridge = true;
                }
            }
        }
    }

    (is_top, is_bot, is_bridge)
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
// execute_layer_slice
// ============================================================================

/// Produce the `SliceIR` for `layer` by slicing every referenced object mesh
/// at `layer.z`.
///
/// Deterministic: regions are emitted in `layer.active_regions` order.
/// If `layer.active_regions` is empty the returned `SliceIR` has an empty
/// `regions` vector (e.g. a layer with no participating objects).
///
/// When `surface_class` is `Some`, the helper [`classify_region_surfaces`] is
/// called for each region to populate `is_top_surface`, `is_bottom_surface`,
/// and `is_bridge`. When it is `None` the three flags remain `false`.
pub fn execute_layer_slice(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    next_layer_z: Option<f32>,
    prev_layer_z: Option<f32>,
    region_map: Option<&RegionMapIR>,
    layer_plan: Option<&LayerPlanIR>,
) -> Result<SliceIR, LayerSliceError> {
    let layer_idx = layer.index as usize;

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

        let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
        let polygons = sliced.pop().unwrap_or_default();

        // Resolve per-region shell counts from RegionMapIR, or use defaults.
        let (top_shell_layers, bottom_shell_layers) = {
            let resolved = region_map.and_then(|rm| {
                let key = RegionKey {
                    global_layer_index: layer.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                };
                rm.entries.get(&key)
            });
            match resolved {
                Some(plan) => (
                    plan.config.top_shell_layers,
                    plan.config.bottom_shell_layers,
                ),
                None => (3u32, 3u32),
            }
        };

        // Compute next/prev layer Z boundaries.
        // When layer_plan is Some, walk the window; otherwise fall back to caller-supplied values.
        let effective_next_z = if let Some(lp) = layer_plan {
            if top_shell_layers == 0 {
                // Window disabled; sentinel not used but provide a safe value.
                None
            } else {
                let ahead_idx = layer_idx + top_shell_layers as usize;
                match lp.global_layers.get(ahead_idx) {
                    Some(gl) => Some(gl.z),
                    None => Some(f32::INFINITY),
                }
            }
        } else {
            // No layer_plan: preserve single-layer semantics via caller-supplied value.
            next_layer_z
        };

        let effective_prev_z = if let Some(lp) = layer_plan {
            if bottom_shell_layers == 0 {
                None
            } else if layer_idx < bottom_shell_layers as usize {
                Some(f32::NEG_INFINITY)
            } else {
                lp.global_layers
                    .get(layer_idx - bottom_shell_layers as usize)
                    .map(|gl| gl.z)
                    .or(Some(f32::NEG_INFINITY))
            }
        } else {
            // No layer_plan: preserve single-layer semantics via caller-supplied value.
            prev_layer_z
        };

        // Extract region contours for classification (ExPolygon → Polygon).
        // When the slice produces no polygons (e.g. flat/horizontal triangles are
        // skipped by the slicer), fall back to a bounding-box polygon derived from
        // the object mesh's XY extents so that the XY containment check does not
        // incorrectly exclude surface-classified facets.
        let (is_top_surface, is_bottom_surface, is_bridge) = if let Some(sc) = surface_class {
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
                classify_region_surfaces(
                    object,
                    obj_data,
                    &contours,
                    layer.z,
                    effective_next_z,
                    effective_prev_z,
                    top_shell_layers,
                    bottom_shell_layers,
                )
            } else {
                (false, false, false)
            }
        } else {
            (false, false, false)
        };

        let mut sliced_region = SlicedRegion {
            object_id: active.object_id.clone(),
            region_id: active.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: active.effective_layer_height,
            boundary_paint: HashMap::new(),
            is_top_surface,
            is_bottom_surface,
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
