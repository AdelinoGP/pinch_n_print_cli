//! Mesh manifold repair — degenerate removal, orientation normalization, open-edge closure.

use slicer_ir::{IndexedTriangleSet, MeshIR, Point3};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Maximum number of vertices in a boundary loop that can be fan-capped.
/// Loops larger than this are skipped with a warning.
pub const MAX_REPAIR_CAP_VERTICES: usize = 256;

/// Result of a mesh repair operation.
#[derive(Debug, Clone, Default)]
pub struct RepairResult {
    /// The repaired mesh.
    pub mesh: MeshIR,
    /// Statistics about what the repair operation changed.
    pub stats: RepairStats,
}

/// Statistics about a mesh repair operation.
#[derive(Debug, Clone, Default)]
pub struct RepairStats {
    /// Number of degenerate (zero-area) triangles removed.
    pub degenerate_removed: usize,
    /// Number of faces whose winding was corrected.
    pub faces_reoriented: usize,
    /// Number of open edges that were closed by fan-capping.
    pub open_edges_closed: usize,
    /// Number of disconnected mesh components found.
    pub components: usize,
    /// Non-fatal warnings encountered during repair.
    pub warnings: Vec<RepairWarning>,
}

/// Non-fatal warnings from the repair process.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairWarning {
    /// A boundary loop was too large to fan-cap reliably.
    LargeCapLoop {
        /// Number of vertices in the skipped boundary loop.
        vertex_count: usize,
    },
    /// The mesh has multiple disconnected components.
    MultipleComponents {
        /// Number of components found.
        count: usize,
    },
}

/// Errors that can occur during mesh repair.
#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    /// The input mesh contains no triangles.
    #[error("input mesh is empty")]
    EmptyMesh,
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Repair a mesh in place. Returns a [`RepairResult`].
///
/// Input mesh may be non-manifold. Output mesh is manifold unless warnings
/// indicate skipped loops.
///
/// Runs three sequential phases per object:
/// 1. Degenerate triangle removal
/// 2. Face orientation normalization (BFS flood-fill)
/// 3. Open-edge closure (fan-cap boundary loops)
pub fn repair(mut mesh: MeshIR) -> Result<RepairResult, RepairError> {
    let mut stats = RepairStats::default();

    for obj in &mut mesh.objects {
        repair_object(&mut obj.mesh, &mut stats);
    }

    Ok(RepairResult { mesh, stats })
}

/// Repair a single object's triangle set.
fn repair_object(its: &mut IndexedTriangleSet, stats: &mut RepairStats) {
    // Phase 1: Remove degenerate triangles.
    phase1_remove_degenerates(its, stats);

    // Phase 2: Normalize face orientation via BFS.
    phase2_normalize_orientation(its, stats);

    // Phase 3: Close open edges via fan-capping.
    phase3_close_open_edges(its, stats);
}

// ---------------------------------------------------------------------------
// Phase 1: Degenerate triangle removal
// ---------------------------------------------------------------------------

/// Cross product of two 3D vectors represented as (f32, f32, f32).
fn cross(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

/// Squared length of a 3D vector.
fn len_sq(v: (f32, f32, f32)) -> f32 {
    v.0 * v.0 + v.1 * v.1 + v.2 * v.2
}

/// A triangle is degenerate if the squared magnitude of its cross product < 2e-16.
fn is_degenerate(v0: &Point3, v1: &Point3, v2: &Point3) -> bool {
    let e1 = (v1.x - v0.x, v1.y - v0.y, v1.z - v0.z);
    let e2 = (v2.x - v0.x, v2.y - v0.y, v2.z - v0.z);
    let c = cross(e1, e2);
    len_sq(c) < 2e-16
}

/// Remove degenerate triangles from the index buffer.
fn phase1_remove_degenerates(its: &mut IndexedTriangleSet, stats: &mut RepairStats) {
    let tri_count = its.indices.len() / 3;
    let mut kept = Vec::with_capacity(its.indices.len());
    let mut removed = 0usize;

    for t in 0..tri_count {
        let i0 = its.indices[t * 3] as usize;
        let i1 = its.indices[t * 3 + 1] as usize;
        let i2 = its.indices[t * 3 + 2] as usize;
        let v0 = &its.vertices[i0];
        let v1 = &its.vertices[i1];
        let v2 = &its.vertices[i2];
        if is_degenerate(v0, v1, v2) {
            removed += 1;
        } else {
            kept.push(its.indices[t * 3]);
            kept.push(its.indices[t * 3 + 1]);
            kept.push(its.indices[t * 3 + 2]);
        }
    }

    its.indices = kept;
    stats.degenerate_removed += removed;
}

// ---------------------------------------------------------------------------
// Phase 2: Face orientation normalization
// ---------------------------------------------------------------------------

/// Canonical (sorted) edge key for adjacency lookup.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Normalize face orientation via BFS flood-fill from the triangle with
/// the most-negative Z centroid. Handles multiple disconnected components.
///
/// Exposed crate-internally so [`crate::decimate::decimate`] can run a single
/// Phase 2 pass over each object after `meshopt::simplify`, per
/// `docs/13_slicer_helpers_crate.md` §Decimation Algorithm step 4.
pub(crate) fn phase2_normalize_orientation(its: &mut IndexedTriangleSet, stats: &mut RepairStats) {
    let tri_count = its.indices.len() / 3;
    if tri_count == 0 {
        return;
    }

    // Build edge→triangle adjacency.
    // For each canonical edge, store (triangle_index, directed_edge_a, directed_edge_b).
    let mut edge_to_tris: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for t in 0..tri_count {
        let i0 = its.indices[t * 3];
        let i1 = its.indices[t * 3 + 1];
        let i2 = its.indices[t * 3 + 2];
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            edge_to_tris.entry(edge_key(a, b)).or_default().push(t);
        }
    }

    let mut visited = vec![false; tri_count];
    let mut components = 0usize;

    // Process all components.
    loop {
        // Find the unvisited triangle with the most-negative Z centroid.
        let seed = (0..tri_count).filter(|&t| !visited[t]).min_by(|&a, &b| {
            let centroid_z = |t: usize| -> f32 {
                let v0 = &its.vertices[its.indices[t * 3] as usize];
                let v1 = &its.vertices[its.indices[t * 3 + 1] as usize];
                let v2 = &its.vertices[its.indices[t * 3 + 2] as usize];
                (v0.z + v1.z + v2.z) / 3.0
            };
            centroid_z(a)
                .partial_cmp(&centroid_z(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let seed = match seed {
            Some(s) => s,
            None => break,
        };

        components += 1;
        visited[seed] = true;
        let mut queue = VecDeque::new();
        queue.push_back(seed);

        while let Some(tri) = queue.pop_front() {
            let i0 = its.indices[tri * 3];
            let i1 = its.indices[tri * 3 + 1];
            let i2 = its.indices[tri * 3 + 2];

            // For each edge of the current triangle, check neighbors.
            for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
                let key = edge_key(a, b);
                let neighbors = match edge_to_tris.get(&key) {
                    Some(n) => n,
                    None => continue,
                };
                for &nbr in neighbors {
                    if visited[nbr] {
                        continue;
                    }
                    visited[nbr] = true;

                    // Check winding consistency: if both triangles share the
                    // same directed edge (a→b), they have inconsistent winding.
                    // Consistent winding means one has a→b and the other has b→a.
                    let nbr_has_same_direction = has_directed_edge(its, nbr, a, b);
                    if nbr_has_same_direction {
                        // Flip the neighbor's winding.
                        flip_triangle(its, nbr);
                        stats.faces_reoriented += 1;
                    }

                    queue.push_back(nbr);
                }
            }
        }
    }

    stats.components = components;
    if components > 1 {
        stats
            .warnings
            .push(RepairWarning::MultipleComponents { count: components });
    }
}

/// Check if triangle `t` has the directed edge `a → b` (in that order).
fn has_directed_edge(its: &IndexedTriangleSet, t: usize, a: u32, b: u32) -> bool {
    let i0 = its.indices[t * 3];
    let i1 = its.indices[t * 3 + 1];
    let i2 = its.indices[t * 3 + 2];
    (i0 == a && i1 == b) || (i1 == a && i2 == b) || (i2 == a && i0 == b)
}

/// Flip a triangle's winding by swapping the second and third indices.
fn flip_triangle(its: &mut IndexedTriangleSet, t: usize) {
    its.indices.swap(t * 3 + 1, t * 3 + 2);
}

// ---------------------------------------------------------------------------
// Phase 3: Open-edge closure
// ---------------------------------------------------------------------------

/// Close open edges by fan-capping boundary loops.
fn phase3_close_open_edges(its: &mut IndexedTriangleSet, stats: &mut RepairStats) {
    // Find open (boundary) edges: edges referenced by exactly one triangle.
    let tri_count = its.indices.len() / 3;
    let mut edge_counts: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();

    for t in 0..tri_count {
        let i0 = its.indices[t * 3];
        let i1 = its.indices[t * 3 + 1];
        let i2 = its.indices[t * 3 + 2];
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            let key = edge_key(a, b);
            edge_counts.entry(key).or_default().push((a, b));
        }
    }

    // Collect boundary edges (directed, as seen by the single triangle).
    // Use BTreeMap for deterministic iteration order.
    let mut boundary_edges: Vec<(u32, u32)> = Vec::new();
    for dirs in edge_counts.values() {
        if dirs.len() == 1 {
            // Single triangle references this edge — it's open.
            // Store the directed edge from the triangle's perspective.
            boundary_edges.push(dirs[0]);
        }
    }

    if boundary_edges.is_empty() {
        return;
    }

    // Group boundary edges into loops by chaining shared vertices.
    let loops = chain_boundary_loops(&boundary_edges);

    for boundary_loop in &loops {
        if boundary_loop.len() > MAX_REPAIR_CAP_VERTICES {
            stats.warnings.push(RepairWarning::LargeCapLoop {
                vertex_count: boundary_loop.len(),
            });
            continue;
        }

        // Fan-cap: add a centroid vertex, then connect each edge in the loop
        // to the centroid with a triangle.
        let centroid = compute_centroid(its, boundary_loop);
        let centroid_idx = its.vertices.len() as u32;
        its.vertices.push(centroid);

        let edges_closed = boundary_loop.len();

        for i in 0..boundary_loop.len() {
            let curr = boundary_loop[i];
            let next = boundary_loop[(i + 1) % boundary_loop.len()];
            // The boundary edge goes curr→next (as seen from the existing triangle).
            // The cap triangle should have opposite winding to close the hole:
            // centroid, next, curr (so the cap's edge between curr and next is next→curr).
            its.indices.push(centroid_idx);
            its.indices.push(next);
            its.indices.push(curr);
        }

        stats.open_edges_closed += edges_closed;
    }
}

/// Chain boundary edges into closed loops by following shared vertices.
/// Returns a list of loops, each being a vector of vertex indices in order.
fn chain_boundary_loops(edges: &[(u32, u32)]) -> Vec<Vec<u32>> {
    // Build adjacency: from_vertex → to_vertex.
    let mut adj: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in edges {
        adj.entry(a).or_default().push(b);
    }

    let mut used_edges: HashMap<(u32, u32), bool> = HashMap::new();
    for &(a, b) in edges {
        used_edges.insert((a, b), false);
    }

    let mut loops = Vec::new();

    for &(start_a, start_b) in edges {
        if used_edges[&(start_a, start_b)] {
            continue;
        }

        let mut chain = Vec::new();
        let mut current = start_a;

        loop {
            chain.push(current);

            // Find an unused outgoing edge from current.
            let next = {
                let targets = match adj.get(&current) {
                    Some(t) => t,
                    None => break,
                };
                let mut found = None;
                for &t in targets {
                    if let Some(used) = used_edges.get(&(current, t)) {
                        if !used {
                            found = Some(t);
                            break;
                        }
                    }
                }
                found
            };

            match next {
                Some(n) => {
                    used_edges.insert((current, n), true);
                    if n == chain[0] {
                        // Loop closed.
                        break;
                    }
                    current = n;
                }
                None => break,
            }
        }

        if chain.len() >= 3 {
            loops.push(chain);
        }
    }

    loops
}

/// Compute the centroid of a set of vertices referenced by a loop.
fn compute_centroid(its: &IndexedTriangleSet, loop_verts: &[u32]) -> Point3 {
    let n = loop_verts.len() as f32;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    let mut cz = 0.0f32;
    for &vi in loop_verts {
        let v = &its.vertices[vi as usize];
        cx += v.x;
        cy += v.y;
        cz += v.z;
    }
    Point3 {
        x: cx / n,
        y: cy / n,
        z: cz / n,
    }
}
