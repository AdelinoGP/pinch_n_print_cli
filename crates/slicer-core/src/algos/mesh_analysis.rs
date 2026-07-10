// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/TriangleMesh.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Mesh analysis algorithms.
//!
//! Produces a [`SurfaceClassificationIR`] from a [`MeshIR`] by classifying
//! each triangle's normal and grouping the results per object.

use std::collections::HashMap;
use std::collections::VecDeque;

use slicer_ir::{
    BridgeRegion, ExPolygon, FacetClass, IndexedTriangleSet, MeshIR, ObjectId, ObjectSurfaceData,
    OverhangRegion, Point2, Point3, Polygon, SurfaceClassificationIR, SurfaceGroup, Transform3d,
    CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
};

/// Configuration for mesh bridge analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshAnalysisConfig {
    /// Project policy: bridges shorter than this are too short to need explicit bridging treatment.
    pub min_bridge_length_mm: f32,
    /// Project policy: anchor runs narrower than this cannot reliably support a bridge.
    pub anchor_width_mm: f32,
    /// Aligns with Orca's BRIDGE_INFILL_MARGIN.
    pub expansion_margin_mm: f32,
    /// Facet-normal angle (from horizontal) above which a downward-facing facet is classified as Overhang.
    pub overhang_threshold_deg: f32,
}

impl Default for MeshAnalysisConfig {
    fn default() -> Self {
        Self {
            min_bridge_length_mm: 10.0,
            anchor_width_mm: 0.5,
            expansion_margin_mm: 1.0,
            overhang_threshold_deg: 45.0,
        }
    }
}

/// Default overhang threshold: a facet whose downward tilt is at or below
/// this angle (i.e. facing nearly straight down) is reported as an
/// overhang requiring support.
pub const DEFAULT_OVERHANG_THRESHOLD_DEG: f32 = 45.0;

/// Cosine-epsilon used to pick out top/bottom surfaces.
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
    execute_mesh_analysis_with(mesh, MeshAnalysisConfig::default())
}

/// Same as [`execute_mesh_analysis`] but with a caller-supplied config.
pub fn execute_mesh_analysis_with(
    mesh: &MeshIR,
    config: MeshAnalysisConfig,
) -> Result<SurfaceClassificationIR, MeshAnalysisError> {
    let mut per_object: HashMap<ObjectId, ObjectSurfaceData> =
        HashMap::with_capacity(mesh.objects.len());

    for object in &mesh.objects {
        let data = classify_object(
            &object.id,
            &object.mesh,
            &object.transform,
            config.overhang_threshold_deg,
            &config,
        )?;
        per_object.insert(object.id.clone(), data);
    }

    Ok(SurfaceClassificationIR {
        schema_version: CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
        per_object,
        overhang_quartile_polygons: HashMap::new(),
    })
}

fn classify_object(
    object_id: &ObjectId,
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    overhang_threshold_deg: f32,
    config: &MeshAnalysisConfig,
) -> Result<ObjectSurfaceData, MeshAnalysisError> {
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(MeshAnalysisError::IndicesNotMultipleOfThree {
            object_id: object_id.clone(),
            count: mesh.indices.len(),
        });
    }

    let tri_count = mesh.indices.len() / 3;
    let mut facet_classes: Vec<FacetClass> = Vec::with_capacity(tri_count);
    let mut overhang_facets: Vec<u32> = Vec::new();
    let mut overhang_max_angle: f32 = 0.0;

    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    let mut total_area: f32 = 0.0;
    let mut all_facet_indices: Vec<u32> = Vec::with_capacity(tri_count);

    let mut facet_normals: Vec<[f32; 3]> = Vec::with_capacity(tri_count);

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
        facet_normals.push(normal);
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
        let xy_footprint = compute_xy_footprint(mesh, transform, &overhang_facets);
        vec![OverhangRegion {
            id: 0,
            facet_indices: overhang_facets,
            max_angle_deg: overhang_max_angle,
            needs_support: true,
            xy_footprint,
        }]
    };

    let bridge_regions =
        compute_bridge_metrics(mesh, transform, &facet_classes, &facet_normals, config);

    Ok(ObjectSurfaceData {
        facet_classes,
        surface_groups,
        bridge_regions,
        overhang_regions,
    })
}

// ---------------------------------------------------------------------------
// Bridge detection via half-edge adjacency analysis
// ---------------------------------------------------------------------------

/// Canonical (sorted) edge key for half-edge adjacency lookup.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Directed edge representation for half-edge structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DirectedEdge {
    src: u32,
    dst: u32,
}

impl DirectedEdge {
    fn new(src: u32, dst: u32) -> Self {
        Self { src, dst }
    }
    fn key(&self) -> (u32, u32) {
        edge_key(self.src, self.dst)
    }
}

/// Half-edge entry for adjacency map.
#[derive(Debug, Clone, Copy)]
struct HalfEdgeEntry {
    tri: usize,
}

/// Build the half-edge adjacency map for a mesh.
fn build_half_edge_map(mesh: &IndexedTriangleSet) -> HashMap<(u32, u32), Vec<HalfEdgeEntry>> {
    let tri_count = mesh.indices.len() / 3;
    let mut edge_map: HashMap<(u32, u32), Vec<HalfEdgeEntry>> = HashMap::new();
    for tri in 0..tri_count {
        let i0 = mesh.indices[tri * 3];
        let i1 = mesh.indices[tri * 3 + 1];
        let i2 = mesh.indices[tri * 3 + 2];
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            edge_map
                .entry(edge_key(a, b))
                .or_default()
                .push(HalfEdgeEntry { tri });
        }
    }
    edge_map
}

/// A cluster of bridge-eligible facets.
#[derive(Debug, Clone)]
struct BridgeCluster {
    anchor_edges: Vec<DirectedEdge>,
    facet_indices: Vec<u32>,
}

fn is_bridge_candidate(class: &FacetClass, normal_z: f32) -> bool {
    normal_z <= 0.0 && matches!(class, FacetClass::Bridge | FacetClass::Overhang { .. })
}

fn find_bridge_clusters(
    mesh: &IndexedTriangleSet,
    facet_classes: &[FacetClass],
    facet_normals: &[[f32; 3]],
) -> Vec<BridgeCluster> {
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 {
        return Vec::new();
    }

    let edge_map = build_half_edge_map(mesh);
    let mut visited = vec![false; tri_count];
    let mut clusters = Vec::new();

    for tri in 0..tri_count {
        if visited[tri] {
            continue;
        }
        let class = match facet_classes.get(tri) {
            Some(c) => c,
            None => continue,
        };
        if !is_bridge_candidate(class, facet_normals[tri][2]) {
            continue;
        }

        let mut cluster_tris = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(tri);
        visited[tri] = true;

        while let Some(current) = queue.pop_front() {
            cluster_tris.push(current as u32);
            let i0 = mesh.indices[current * 3];
            let i1 = mesh.indices[current * 3 + 1];
            let i2 = mesh.indices[current * 3 + 2];

            for edge_dir in &[
                DirectedEdge::new(i0, i1),
                DirectedEdge::new(i1, i2),
                DirectedEdge::new(i2, i0),
            ] {
                let key = edge_dir.key();
                if let Some(half_edges) = edge_map.get(&key) {
                    for &he in half_edges {
                        if he.tri == current || visited[he.tri] {
                            continue;
                        }
                        if let Some(nc) = facet_classes.get(he.tri) {
                            if is_bridge_candidate(nc, facet_normals[he.tri][2]) {
                                visited[he.tri] = true;
                                queue.push_back(he.tri);
                            }
                        }
                    }
                }
            }
        }

        // Anchor edges: edges shared with a non-bridge-candidate neighbor.
        let mut anchor_edges = Vec::new();
        for &t in &cluster_tris {
            let t = t as usize;
            let i0 = mesh.indices[t * 3];
            let i1 = mesh.indices[t * 3 + 1];
            let i2 = mesh.indices[t * 3 + 2];

            for edge_dir in &[
                DirectedEdge::new(i0, i1),
                DirectedEdge::new(i1, i2),
                DirectedEdge::new(i2, i0),
            ] {
                let key = edge_dir.key();
                if let Some(half_edges) = edge_map.get(&key) {
                    for &he in half_edges {
                        if he.tri == t {
                            continue;
                        }
                        let nc = match facet_classes.get(he.tri) {
                            Some(c) => c,
                            None => continue,
                        };
                        if !is_bridge_candidate(nc, facet_normals[he.tri][2]) {
                            anchor_edges.push(*edge_dir);
                        }
                    }
                }
            }
        }

        clusters.push(BridgeCluster {
            facet_indices: cluster_tris,
            anchor_edges,
        });
    }

    clusters
}

fn get_vertex_unchecked<'a>(mesh: &'a IndexedTriangleSet, idx: u32) -> &'a Point3 {
    &mesh.vertices[idx as usize]
}

/// An anchor-edge run: a chain of contiguous boundary edges, with world-space endpoints.
struct AnchorRun {
    /// Ordered world-space XY positions of the run's vertices.
    points_mm: Vec<(f32, f32)>,
    /// Total Euclidean length of the run (sum of segment lengths).
    length_mm: f32,
}

/// Group the anchor edges of a cluster into contiguous runs.
fn build_anchor_runs(
    anchor_edges: &[DirectedEdge],
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
) -> Vec<AnchorRun> {
    if anchor_edges.is_empty() {
        return Vec::new();
    }

    let mut next_map: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, e) in anchor_edges.iter().enumerate() {
        next_map.entry(e.src).or_default().push(i);
    }

    let mut used = vec![false; anchor_edges.len()];
    let mut runs = Vec::new();

    for start in 0..anchor_edges.len() {
        if used[start] {
            continue;
        }
        used[start] = true;

        let mut chain: Vec<usize> = vec![start];
        let mut current_dst = anchor_edges[start].dst;
        loop {
            let mut found = false;
            if let Some(candidates) = next_map.get(&current_dst) {
                for &ci in candidates {
                    if !used[ci] {
                        used[ci] = true;
                        chain.push(ci);
                        current_dst = anchor_edges[ci].dst;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                break;
            }
        }

        let mut pts: Vec<(f32, f32)> = Vec::with_capacity(chain.len() + 1);
        {
            let first_edge = &anchor_edges[chain[0]];
            let wv = apply_transform(transform, get_vertex_unchecked(mesh, first_edge.src));
            pts.push((wv.x, wv.y));
        }
        let mut total_len = 0.0_f32;
        for &ei in &chain {
            let e = &anchor_edges[ei];
            let wv = apply_transform(transform, get_vertex_unchecked(mesh, e.dst));
            let prev = *pts.last().unwrap();
            let dx = wv.x - prev.0;
            let dy = wv.y - prev.1;
            total_len += (dx * dx + dy * dy).sqrt();
            pts.push((wv.x, wv.y));
        }

        runs.push(AnchorRun {
            points_mm: pts,
            length_mm: total_len,
        });
    }

    runs
}

/// Bridge direction = filament travel direction = perpendicular to the longest anchor-edge run,
/// normalized to [0, 180).
fn compute_bridge_direction_deg(runs: &[AnchorRun]) -> f32 {
    let longest = match runs.iter().max_by(|a, b| {
        a.length_mm
            .partial_cmp(&b.length_mm)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Some(r) => r,
        None => return 0.0,
    };

    if longest.points_mm.len() < 2 {
        return 0.0;
    }
    let first = longest.points_mm[0];
    let last = *longest.points_mm.last().unwrap();
    let dx = last.0 - first.0;
    let dy = last.1 - first.1;
    if dx == 0.0 && dy == 0.0 {
        return 0.0;
    }
    let wall_angle_deg = dy.atan2(dx).to_degrees();
    (wall_angle_deg + 90.0).rem_euclid(180.0)
}

/// Anchor width = span of all anchor vertices projected onto the bridge direction axis.
fn compute_anchor_width_mm(runs: &[AnchorRun], bridge_direction_deg: f32) -> f32 {
    if runs.is_empty() {
        return 0.0;
    }
    let dir_rad = bridge_direction_deg.to_radians();
    let dir_x = dir_rad.cos();
    let dir_y = dir_rad.sin();

    let mut min_proj = f32::INFINITY;
    let mut max_proj = f32::NEG_INFINITY;
    for run in runs {
        for &(x, y) in &run.points_mm {
            let p = x * dir_x + y * dir_y;
            min_proj = min_proj.min(p);
            max_proj = max_proj.max(p);
        }
    }
    if min_proj.is_finite() && max_proj.is_finite() {
        (max_proj - min_proj).max(0.0)
    } else {
        0.0
    }
}

/// Union of per-facet XY triangle projections.
///
/// Builds every facet's triangle polygon up front and unions them in a
/// single clipper2 call. Clipper2's union is an n-way sweep, not a pairwise
/// primitive — invoking it once per facet (as an incremental `accum = union(accum, [tri])`
/// loop) turns an O(n) sweep into O(n) full sweeps over a growing accumulator,
/// which is what made `MeshAnalysis` take minutes on meshes with tens of
/// thousands of overhang/bridge facets (e.g. a curved hull like 3D Benchy).
fn compute_xy_footprint(
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    facet_indices: &[u32],
) -> Vec<ExPolygon> {
    use crate::polygon_ops::union_ex;

    let tris: Vec<ExPolygon> = facet_indices
        .iter()
        .map(|&t| {
            let t = t as usize;
            let i0 = mesh.indices[t * 3];
            let i1 = mesh.indices[t * 3 + 1];
            let i2 = mesh.indices[t * 3 + 2];

            let wv0 = apply_transform(transform, get_vertex_unchecked(mesh, i0));
            let wv1 = apply_transform(transform, get_vertex_unchecked(mesh, i1));
            let wv2 = apply_transform(transform, get_vertex_unchecked(mesh, i2));

            ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(wv0.x, wv0.y),
                        Point2::from_mm(wv1.x, wv1.y),
                        Point2::from_mm(wv2.x, wv2.y),
                    ],
                },
                holes: vec![],
            }
        })
        .collect();

    union_ex(&tris)
}

/// XY footprint (union of per-facet triangle projections, transform applied)
/// of every facet classified [`FacetClass::BottomSurface`] in `mesh`, using
/// the per-triangle classification in `facet_classes` (as produced by
/// [`execute_mesh_analysis`]).
///
/// Mirrors how overhang footprints are projected
/// ([`OverhangRegion::xy_footprint`]). The pre-pass intersects this
/// flat-bottom footprint with a layer's unsupported region to detect
/// perfectly-horizontal ("flat") unsupported bridge spans — e.g. a beam
/// spanning a gap between two supports. Such spans never enter the
/// sloped-overhang bridge pipeline because their downward facets classify as
/// `BottomSurface` (normal ≈ straight down), not `Overhang`, and
/// [`crate::algos::prepass_slice::assemble_bridge_areas`] only consumes
/// `Bridge`/`Overhang`-derived clusters.
///
/// Returns an empty vec when no facet is a `BottomSurface`.
pub fn bottom_surface_footprint(
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    facet_classes: &[FacetClass],
) -> Vec<ExPolygon> {
    let tri_count = mesh.indices.len() / 3;
    let bottom_facets: Vec<u32> = facet_classes
        .iter()
        .take(tri_count)
        .enumerate()
        .filter_map(|(i, class)| matches!(class, FacetClass::BottomSurface).then_some(i as u32))
        .collect();
    if bottom_facets.is_empty() {
        return Vec::new();
    }
    compute_xy_footprint(mesh, transform, &bottom_facets)
}

/// Bridge span: extent of facet vertices projected along the axis perpendicular to bridge direction.
fn compute_bridge_length_mm(
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    facet_indices: &[u32],
    bridge_direction_deg: f32,
) -> f32 {
    let dir_rad = (bridge_direction_deg + 90.0).to_radians();
    let dir_x = dir_rad.cos();
    let dir_y = dir_rad.sin();

    let mut min_proj = f32::INFINITY;
    let mut max_proj = f32::NEG_INFINITY;

    for &t in facet_indices {
        let t = t as usize;
        for &idx in &[
            mesh.indices[t * 3],
            mesh.indices[t * 3 + 1],
            mesh.indices[t * 3 + 2],
        ] {
            let wv = apply_transform(transform, get_vertex_unchecked(mesh, idx));
            let proj = wv.x * dir_x + wv.y * dir_y;
            min_proj = min_proj.min(proj);
            max_proj = max_proj.max(proj);
        }
    }

    if min_proj.is_finite() && max_proj.is_finite() {
        (max_proj - min_proj).max(0.0)
    } else {
        0.0
    }
}

/// Compute bridge metrics for all bridge-eligible clusters.
fn compute_bridge_metrics(
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
    facet_classes: &[FacetClass],
    facet_normals: &[[f32; 3]],
    config: &MeshAnalysisConfig,
) -> Vec<BridgeRegion> {
    let clusters = find_bridge_clusters(mesh, facet_classes, facet_normals);

    clusters
        .into_iter()
        .enumerate()
        .map(|(idx, cluster)| {
            let runs = build_anchor_runs(&cluster.anchor_edges, mesh, transform);
            let bridge_direction_deg = compute_bridge_direction_deg(&runs);
            let anchor_width_mm = compute_anchor_width_mm(&runs, bridge_direction_deg);
            let bridge_length_mm = compute_bridge_length_mm(
                mesh,
                transform,
                &cluster.facet_indices,
                bridge_direction_deg,
            );
            let xy_footprint = compute_xy_footprint(mesh, transform, &cluster.facet_indices);

            let is_valid = anchor_width_mm >= config.anchor_width_mm
                && bridge_length_mm >= config.min_bridge_length_mm;

            BridgeRegion {
                id: idx as u64,
                facet_indices: cluster.facet_indices,
                bridge_direction_deg,
                anchor_width_mm,
                bridge_length_mm,
                expansion_margin_mm: config.expansion_margin_mm,
                is_valid,
                xy_footprint,
            }
        })
        .collect()
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
    crate::transform_point3(&t.matrix, *p)
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

    if !nz.is_finite() || (normal[0] == 0.0 && normal[1] == 0.0 && normal[2] == 0.0) {
        return FacetClass::Normal;
    }

    if nz >= 1.0 - TOP_BOTTOM_COSINE_EPSILON {
        return FacetClass::TopSurface;
    }
    if nz <= -(1.0 - TOP_BOTTOM_COSINE_EPSILON) {
        return FacetClass::BottomSurface;
    }

    if nz < 0.0 {
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

    fn identity_transform() -> Transform3d {
        Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// `facet_count` mutually non-overlapping unit triangles, spaced 10mm apart
    /// on the X axis so their XY projections never touch and union can never
    /// merge them into fewer output polygons.
    fn disjoint_triangle_mesh(facet_count: usize) -> IndexedTriangleSet {
        let mut vertices = Vec::with_capacity(facet_count * 3);
        let mut indices = Vec::with_capacity(facet_count * 3);
        for i in 0..facet_count {
            let ox = i as f32 * 10.0;
            let base = vertices.len() as u32;
            vertices.push(Point3 {
                x: ox,
                y: 0.0,
                z: 0.0,
            });
            vertices.push(Point3 {
                x: ox + 1.0,
                y: 0.0,
                z: 0.0,
            });
            vertices.push(Point3 {
                x: ox,
                y: 1.0,
                z: 0.0,
            });
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
        }
        IndexedTriangleSet { vertices, indices }
    }

    /// Regression test for the `PrePass::MeshAnalysis` stage taking 118s+ on
    /// 3D Benchy: `compute_xy_footprint` used to call `union(&accum, &[tri])`
    /// once per facet, growing `accum` every iteration and re-running a full
    /// clipper2 boolean-op on the whole accumulated set each time — O(n)
    /// full clipper2 invocations instead of one batched union. Measured
    /// directly against the old per-facet-union implementation: 1200
    /// disjoint facets took 61ms batched vs. 20.6s incremental (338x). The
    /// threshold below leaves ~50x headroom above the batched runtime while
    /// sitting ~7x under the incremental runtime, so it fails fast and
    /// reliably if this ever regresses back to the incremental pattern.
    #[test]
    fn compute_xy_footprint_is_fast_for_thousands_of_disjoint_facets() {
        const FACET_COUNT: usize = 1200;
        let mesh = disjoint_triangle_mesh(FACET_COUNT);
        let transform = identity_transform();
        let facet_indices: Vec<u32> = (0..FACET_COUNT as u32).collect();

        let start = std::time::Instant::now();
        let footprint = compute_xy_footprint(&mesh, &transform, &facet_indices);
        let elapsed = start.elapsed();

        assert_eq!(
            footprint.len(),
            FACET_COUNT,
            "disjoint triangles must not merge into fewer polygons"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "compute_xy_footprint took {elapsed:?} for {FACET_COUNT} disjoint \
             facets (expected well under 3s; batched call measures ~61ms) — \
             this smells like a regression to per-facet incremental union \
             calls (O(n) separate clipper2 invocations instead of one \
             batched union), which is what made PrePass::MeshAnalysis take \
             118s+ on 3D Benchy"
        );
    }
}
