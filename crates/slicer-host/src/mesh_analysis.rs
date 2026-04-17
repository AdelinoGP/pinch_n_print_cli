//! Host-built-in `PrePass::MeshAnalysis` stage (TASK-105).
//!
//! Produces a [`SurfaceClassificationIR`] from the blackboard-owned
//! [`MeshIR`] by classifying each triangle's normal and grouping the
//! results per object. The classifier is intentionally small and
//! deterministic — bridge detection, surface grouping by connectivity,
//! and printability heuristics are out of scope for this step; those
//! belong to later MeshAnalysis-tier modules that consume this IR.
//!
//! Reference: docs/01_system_architecture.md §"PrePass::MeshAnalysis",
//! docs/02_ir_schemas.md §"IR 2 — SurfaceClassificationIR",
//! docs/04_host_scheduler.md §"Full Lifecycle" (prepass).

use std::collections::HashMap;

use slicer_ir::{
    BridgeRegion, FacetClass, IndexedTriangleSet, MeshIR, ObjectId, ObjectSurfaceData,
    OverhangRegion, Point3, SemVer, SurfaceClassificationIR, SurfaceGroup, Transform3d,
};

/// Default overhang threshold: a facet whose downward tilt is at or below
/// this angle (i.e. facing nearly straight down) is reported as an
/// overhang requiring support. Matches the common 45° default seen in
/// existing slicers.
pub const DEFAULT_OVERHANG_THRESHOLD_DEG: f32 = 45.0;

/// Cosine-epsilon used to pick out top/bottom surfaces. A facet whose
/// normal z-component is within this distance of ±1.0 is considered
/// axis-aligned (i.e. a TopSurface or BottomSurface facet).
const TOP_BOTTOM_COSINE_EPSILON: f32 = 0.017_452_406; // cos(89°)→sin(1°) tolerance

/// Structured mesh-analysis failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshAnalysisError {
    /// An object's index buffer length is not a multiple of 3.
    IndicesNotMultipleOfThree {
        /// Object identifier.
        object_id: ObjectId,
        /// Reported index count.
        count: usize,
    },
    /// A triangle referenced a vertex index outside the vertex buffer.
    InvalidVertexIndex {
        /// Object identifier.
        object_id: ObjectId,
        /// Offending index value.
        index: u32,
        /// Vertex buffer length.
        vertex_count: usize,
    },
}

impl std::fmt::Display for MeshAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndicesNotMultipleOfThree { object_id, count } => write!(
                f,
                "object '{object_id}' index buffer length {count} is not a multiple of 3"
            ),
            Self::InvalidVertexIndex {
                object_id,
                index,
                vertex_count,
            } => write!(
                f,
                "object '{object_id}' triangle references vertex index {index} but only {vertex_count} vertices exist"
            ),
        }
    }
}

impl std::error::Error for MeshAnalysisError {}

/// Execute the built-in `PrePass::MeshAnalysis` stage.
///
/// Iteration order is stable (`mesh.objects` is a `Vec`, triangles are
/// visited in index order) and the classifier is pure, so repeated
/// invocations on the same mesh yield byte-identical output.
pub fn execute_mesh_analysis(mesh: &MeshIR) -> Result<SurfaceClassificationIR, MeshAnalysisError> {
    execute_mesh_analysis_with(mesh, DEFAULT_OVERHANG_THRESHOLD_DEG)
}

/// Same as [`execute_mesh_analysis`] but with a caller-supplied overhang
/// threshold (degrees of facet slope below which a down-facing facet is
/// classified as an overhang).
pub fn execute_mesh_analysis_with(
    mesh: &MeshIR,
    overhang_threshold_deg: f32,
) -> Result<SurfaceClassificationIR, MeshAnalysisError> {
    let mut per_object: HashMap<ObjectId, ObjectSurfaceData> =
        HashMap::with_capacity(mesh.objects.len());

    for object in &mesh.objects {
        let data = classify_object(
            &object.id,
            &object.mesh,
            &object.transform,
            overhang_threshold_deg,
        )?;
        per_object.insert(object.id.clone(), data);
    }

    Ok(SurfaceClassificationIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object,
    })
}

fn classify_object(
    object_id: &ObjectId,
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    overhang_threshold_deg: f32,
) -> Result<ObjectSurfaceData, MeshAnalysisError> {
    if mesh.indices.len() % 3 != 0 {
        return Err(MeshAnalysisError::IndicesNotMultipleOfThree {
            object_id: object_id.clone(),
            count: mesh.indices.len(),
        });
    }

    let tri_count = mesh.indices.len() / 3;
    let mut facet_classes: Vec<FacetClass> = Vec::with_capacity(tri_count);
    let mut overhang_facets: Vec<u32> = Vec::new();
    let mut overhang_max_angle: f32 = 0.0;

    // Surface-group bookkeeping: for this built-in we emit one group per
    // object that spans every facet, with aggregate z/area statistics.
    // Later MeshAnalysis modules may re-segment by connectivity — that is
    // their job; ours is only to produce a valid baseline IR.
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    let mut total_area: f32 = 0.0;
    let mut all_facet_indices: Vec<u32> = Vec::with_capacity(tri_count);

    let overhang_cos_threshold = (overhang_threshold_deg.to_radians()).sin();
    // Down-facing normal -> z < -cos(threshold_from_down)
    // where threshold_from_down = 90° - overhang_threshold_deg.
    // Equivalently: overhang iff -normal.z >= sin(overhang_threshold_deg).
    let _ = overhang_cos_threshold; // documented inline below

    for tri_idx in 0..tri_count {
        let i0 = mesh.indices[tri_idx * 3];
        let i1 = mesh.indices[tri_idx * 3 + 1];
        let i2 = mesh.indices[tri_idx * 3 + 2];

        let v0 = get_vertex(mesh, object_id, i0)?;
        let v1 = get_vertex(mesh, object_id, i1)?;
        let v2 = get_vertex(mesh, object_id, i2)?;

        let wv0 = apply_transform(transform, v0);
        let wv1 = apply_transform(transform, v1);
        let wv2 = apply_transform(transform, v2);

        let (normal, area) = triangle_normal_area(wv0, wv1, wv2);
        total_area += area;

        let z0 = wv0.z;
        let z1 = wv1.z;
        let z2 = wv2.z;
        z_min = z_min.min(z0).min(z1).min(z2);
        z_max = z_max.max(z0).max(z1).max(z2);

        all_facet_indices.push(tri_idx as u32);

        let class = classify_facet(normal, overhang_threshold_deg);
        if let FacetClass::Overhang { angle_deg } = class {
            overhang_facets.push(tri_idx as u32);
            if angle_deg > overhang_max_angle {
                overhang_max_angle = angle_deg;
            }
        }
        facet_classes.push(class);
    }

    if tri_count == 0 {
        z_min = 0.0;
        z_max = 0.0;
    }

    let surface_groups = if tri_count == 0 {
        Vec::new()
    } else {
        vec![SurfaceGroup {
            id: 0,
            facet_indices: all_facet_indices,
            z_min,
            z_max,
            area_mm2: total_area,
            printable: true,
            shell_count: 1,
        }]
    };

    let overhang_regions: Vec<OverhangRegion> = if overhang_facets.is_empty() {
        Vec::new()
    } else {
        vec![OverhangRegion {
            id: 0,
            facet_indices: overhang_facets,
            max_angle_deg: overhang_max_angle,
            needs_support: true,
        }]
    };

    Ok(ObjectSurfaceData {
        facet_classes,
        surface_groups,
        bridge_regions: Vec::<BridgeRegion>::new(),
        overhang_regions,
    })
}

fn get_vertex<'a>(
    mesh: &'a IndexedTriangleSet,
    object_id: &ObjectId,
    idx: u32,
) -> Result<&'a Point3, MeshAnalysisError> {
    mesh.vertices
        .get(idx as usize)
        .ok_or_else(|| MeshAnalysisError::InvalidVertexIndex {
            object_id: object_id.clone(),
            index: idx,
            vertex_count: mesh.vertices.len(),
        })
}

/// Apply a 4x4 column-major transform to a point. A zero matrix would
/// collapse the mesh; we treat it as identity for robustness against
/// fixtures that leave `Transform3d::matrix` unset.
fn apply_transform(t: &Transform3d, p: &Point3) -> Point3 {
    // Column-major: column c, row r → matrix[c * 4 + r]
    let m = &t.matrix;
    if m.iter().all(|v| *v == 0.0) {
        return *p;
    }
    let x = p.x as f64;
    let y = p.y as f64;
    let z = p.z as f64;
    let tx = m[0] * x + m[4] * y + m[8] * z + m[12];
    let ty = m[1] * x + m[5] * y + m[9] * z + m[13];
    let tz = m[2] * x + m[6] * y + m[10] * z + m[14];
    Point3 {
        x: tx as f32,
        y: ty as f32,
        z: tz as f32,
    }
}

fn triangle_normal_area(a: Point3, b: Point3, c: Point3) -> ([f32; 3], f32) {
    let ux = b.x - a.x;
    let uy = b.y - a.y;
    let uz = b.z - a.z;
    let vx = c.x - a.x;
    let vy = c.y - a.y;
    let vz = c.z - a.z;
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let mag = (nx * nx + ny * ny + nz * nz).sqrt();
    if mag == 0.0 {
        ([0.0, 0.0, 0.0], 0.0)
    } else {
        ([nx / mag, ny / mag, nz / mag], 0.5 * mag)
    }
}

fn classify_facet(normal: [f32; 3], overhang_threshold_deg: f32) -> FacetClass {
    let nz = normal[2];

    // Degenerate normal — classify as Normal for safety.
    if !nz.is_finite() || (normal[0] == 0.0 && normal[1] == 0.0 && normal[2] == 0.0) {
        return FacetClass::Normal;
    }

    if nz >= 1.0 - TOP_BOTTOM_COSINE_EPSILON {
        return FacetClass::TopSurface;
    }
    if nz <= -(1.0 - TOP_BOTTOM_COSINE_EPSILON) {
        return FacetClass::BottomSurface;
    }

    // Overhang: facet faces downward beyond the threshold. We measure the
    // tilt *from horizontal* of the downward-facing side so that a facet
    // pointing straight down is reported as angle_deg = 0 and a facet
    // pointing at the horizon is angle_deg = 90°. The facet is an overhang
    // when its downward tilt is within `overhang_threshold_deg` of the
    // horizontal plane (i.e. nearly horizontal but facing down).
    if nz < 0.0 {
        // Angle of the normal from straight down (-Z axis), in degrees:
        // 0° = normal points straight down, 90° = normal is horizontal.
        let angle_from_down_deg = (-nz).clamp(0.0, 1.0).acos().to_degrees();
        if angle_from_down_deg <= overhang_threshold_deg {
            return FacetClass::Overhang {
                angle_deg: angle_from_down_deg,
            };
        }
    }

    FacetClass::Normal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_straight_down_normal_as_bottom() {
        assert!(matches!(
            classify_facet([0.0, 0.0, -1.0], DEFAULT_OVERHANG_THRESHOLD_DEG),
            FacetClass::BottomSurface
        ));
    }

    #[test]
    fn classifies_straight_up_normal_as_top() {
        assert!(matches!(
            classify_facet([0.0, 0.0, 1.0], DEFAULT_OVERHANG_THRESHOLD_DEG),
            FacetClass::TopSurface
        ));
    }
}
