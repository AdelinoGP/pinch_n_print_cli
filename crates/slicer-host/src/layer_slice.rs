//! Host-built-in `Layer::Slice` stage (TASK-107).
//!
//! Produces a `SliceIR` for a single global layer by calling
//! `slicer_core::slice_mesh_ex` on each object mesh at the layer's Z. The
//! `SliceIR` is staged in the per-layer arena before any user
//! `Layer::Slice` / `Layer::SlicePostProcess` module runs.

use std::collections::HashMap;
use std::fmt;

use slicer_core::slice_mesh_ex;
use slicer_ir::{FacetClass, Point2, Point3};
use slicer_ir::{
    GlobalLayer, MeshIR, ObjectId, ObjectMesh, ObjectSurfaceData, Polygon, SemVer, SliceIR,
    SlicedRegion, SurfaceClassificationIR, Transform3d,
};

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
) -> (bool, bool, bool) {
    let mesh = &object_mesh.mesh;
    let t = &object_mesh.transform;
    let tri_count = mesh.indices.len() / 3;

    let mut is_top = false;
    let mut is_bot = false;
    let mut is_bridge = false;

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
        if !is_top {
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
        if !is_bot {
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

        let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
        let polygons = sliced.pop().unwrap_or_default();

        // Extract region contours for classification (ExPolygon → Polygon).
        let (is_top_surface, is_bottom_surface, is_bridge) = if let Some(sc) = surface_class {
            if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                let contours: Vec<Polygon> = polygons.iter().map(|ep| ep.contour.clone()).collect();
                classify_region_surfaces(
                    object,
                    obj_data,
                    &contours,
                    layer.z,
                    next_layer_z,
                    prev_layer_z,
                )
            } else {
                (false, false, false)
            }
        } else {
            (false, false, false)
        };

        regions.push(SlicedRegion {
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
        });
    }

    Ok(SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 1,
            patch: 0,
        },
        global_layer_index: layer.index,
        z: layer.z,
        regions,
    })
}
