//! Connected-component splitting for `IndexedTriangleSet`.
//!
//! Two triangle faces are adjacent iff they share a **directed** edge with opposite
//! winding: face A has directed edge (a→b) and face B contains the reversed directed
//! edge (b→a). Vertex-only contact does NOT create adjacency.
//!
//! This mirrors the OrcaSlicer `its_split` algorithm.

use slicer_ir::IndexedTriangleSet;
use std::collections::HashMap;

/// Split `set` into its connected components.
///
/// Adjacency is defined by shared directed edges with opposite winding (i.e. a
/// manifold shared edge). Vertex-only contacts produce separate components.
///
/// Every component is emitted, including single-triangle fragments — there is no
/// minimum-size threshold.
///
/// **Deterministic order**: components are seeded by ascending face index. Within
/// a component, vertices are remapped in first-seen order.
pub fn split_connected_components(set: &IndexedTriangleSet) -> Vec<IndexedTriangleSet> {
    let tri_count = set.indices.len() / 3;
    if tri_count == 0 {
        return vec![];
    }

    // Build directed-edge → face index map.
    // Key: (from_vertex, to_vertex) directed edge.
    // Value: list of face indices that contain that directed edge.
    let mut directed_edge_to_faces: HashMap<(u32, u32), Vec<usize>> =
        HashMap::with_capacity(tri_count * 3);

    for t in 0..tri_count {
        let i0 = set.indices[t * 3];
        let i1 = set.indices[t * 3 + 1];
        let i2 = set.indices[t * 3 + 2];
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            directed_edge_to_faces.entry((a, b)).or_default().push(t);
        }
    }

    // For each face, find neighbors via reversed directed edges.
    // Adjacency list: face_adjacency[t] = list of face indices adjacent to t.
    let mut face_adjacency: Vec<Vec<usize>> = vec![Vec::new(); tri_count];
    for t in 0..tri_count {
        let i0 = set.indices[t * 3];
        let i1 = set.indices[t * 3 + 1];
        let i2 = set.indices[t * 3 + 2];
        // For each directed edge (a→b) in face t, look up reversed edge (b→a).
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            if let Some(neighbors) = directed_edge_to_faces.get(&(b, a)) {
                for &nbr in neighbors {
                    if nbr != t {
                        face_adjacency[t].push(nbr);
                    }
                }
            }
        }
    }

    // DFS to label connected components, seeding by ascending face index.
    let mut component_id: Vec<Option<usize>> = vec![None; tri_count];
    let mut num_components = 0usize;

    for seed in 0..tri_count {
        if component_id[seed].is_some() {
            continue;
        }
        // New component — DFS from seed.
        let cid = num_components;
        num_components += 1;
        let mut stack = vec![seed];
        component_id[seed] = Some(cid);
        while let Some(face) = stack.pop() {
            for &nbr in &face_adjacency[face] {
                if component_id[nbr].is_none() {
                    component_id[nbr] = Some(cid);
                    stack.push(nbr);
                }
            }
        }
    }

    // Build output IndexedTriangleSets, one per component.
    // Within each component, remap vertices in first-seen order (ascending face index,
    // then vertex order within each face: v0, v1, v2).
    let mut components: Vec<IndexedTriangleSet> = (0..num_components)
        .map(|_| IndexedTriangleSet {
            vertices: Vec::new(),
            indices: Vec::new(),
        })
        .collect();

    // vertex_map[cid][original_vertex_idx] = new_local_idx
    let mut vertex_maps: Vec<HashMap<u32, u32>> =
        (0..num_components).map(|_| HashMap::new()).collect();

    // Iterate faces in ascending order so first-seen vertex ordering is deterministic.
    for t in 0..tri_count {
        let cid = component_id[t].expect("every face must be assigned a component");
        let comp = &mut components[cid];
        let vmap = &mut vertex_maps[cid];

        let i0 = set.indices[t * 3];
        let i1 = set.indices[t * 3 + 1];
        let i2 = set.indices[t * 3 + 2];

        let mut remap = |orig: u32| -> u32 {
            let next = vmap.len() as u32;
            *vmap.entry(orig).or_insert_with(|| {
                comp.vertices.push(set.vertices[orig as usize]);
                next
            })
        };

        let r0 = remap(i0);
        let r1 = remap(i1);
        let r2 = remap(i2);
        comp.indices.push(r0);
        comp.indices.push(r1);
        comp.indices.push(r2);
    }

    components
}
