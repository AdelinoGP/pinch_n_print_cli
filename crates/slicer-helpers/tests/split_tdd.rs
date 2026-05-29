//! TDD tests for `split_connected_components` (packet 71, Step 1, AC-5 / AC-N2).

use slicer_helpers::split_connected_components;
use slicer_ir::{IndexedTriangleSet, Point3};

// ---------------------------------------------------------------------------
// Inline mesh helpers
// ---------------------------------------------------------------------------

/// Build a regular tetrahedron with outward CCW winding.
/// Vertex layout:
///   0: (0, 0, 0), 1: (1, 0, 0), 2: (0.5, 1, 0), 3: (0.5, 0.5, 1)
fn tetrahedron(offset_x: f32, offset_y: f32, offset_z: f32) -> IndexedTriangleSet {
    let v = |x: f32, y: f32, z: f32| Point3 {
        x: x + offset_x,
        y: y + offset_y,
        z: z + offset_z,
    };
    let vertices = vec![
        v(0.0, 0.0, 0.0), // 0
        v(1.0, 0.0, 0.0), // 1
        v(0.5, 1.0, 0.0), // 2
        v(0.5, 0.5, 1.0), // 3
    ];
    // Four faces, outward CCW winding.
    #[rustfmt::skip]
    let indices = vec![
        0, 2, 1, // bottom (normal -Z)
        0, 1, 3, // front
        1, 2, 3, // right
        2, 0, 3, // left
    ];
    IndexedTriangleSet { vertices, indices }
}

/// Merge two `IndexedTriangleSet`s into one (vertices concatenated, indices offset).
fn merge(a: IndexedTriangleSet, b: IndexedTriangleSet) -> IndexedTriangleSet {
    let offset = a.vertices.len() as u32;
    let mut vertices = a.vertices;
    vertices.extend(b.vertices);
    let mut indices = a.indices;
    indices.extend(b.indices.iter().map(|&i| i + offset));
    IndexedTriangleSet { vertices, indices }
}

/// Build a valid closed cube mesh (8 vertices, 12 triangles).
/// Mirrors the helper in `repair_tdd.rs`.
fn valid_cube() -> IndexedTriangleSet {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }, // 0
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        }, // 1
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 0.0,
        }, // 2
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 0.0,
        }, // 3
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        }, // 4
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 10.0,
        }, // 5
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 10.0,
        }, // 6
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 10.0,
        }, // 7
    ];
    #[rustfmt::skip]
    let indices = vec![
        // Back face  (z=0,  normal -Z)
        0, 2, 1,
        0, 3, 2,
        // Front face (z=10, normal +Z)
        4, 5, 6,
        4, 6, 7,
        // Bottom     (y=0,  normal -Y)
        0, 1, 5,
        0, 5, 4,
        // Top        (y=10, normal +Y)
        3, 6, 2,
        3, 7, 6,
        // Left       (x=0,  normal -X)
        0, 4, 7,
        0, 7, 3,
        // Right      (x=10, normal +X)
        1, 2, 6,
        1, 6, 5,
    ];
    IndexedTriangleSet { vertices, indices }
}

/// Two tetrahedra sharing exactly ONE vertex (no shared edge).
///
/// Tet A uses vertices 0-3. Tet B uses vertices 4-6 plus vertex 0 (shared).
/// They touch at vertex 0 only — no directed-edge pairs are reversed mirrors.
fn two_tets_vertex_only_contact() -> IndexedTriangleSet {
    let vertices = vec![
        // Tet A
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }, // 0  ← shared vertex
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }, // 1
        Point3 {
            x: 0.5,
            y: 1.0,
            z: 0.0,
        }, // 2
        Point3 {
            x: 0.5,
            y: 0.5,
            z: 1.0,
        }, // 3
        // Tet B (spatially distinct, only shares vertex 0)
        Point3 {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        }, // 4
        Point3 {
            x: -0.5,
            y: 1.0,
            z: 0.0,
        }, // 5
        Point3 {
            x: -0.5,
            y: 0.5,
            z: 1.0,
        }, // 6
    ];
    // Tet A faces (CCW outward)
    // Tet B faces (CCW outward), note: tet B uses vertex 0 as its "origin"
    #[rustfmt::skip]
    let indices = vec![
        // Tet A
        0, 2, 1,
        0, 1, 3,
        1, 2, 3,
        2, 0, 3,
        // Tet B  (vertices: 0, 4, 5, 6 — shares vertex 0)
        0, 5, 4,
        0, 4, 6,
        4, 5, 6,
        5, 0, 6,
    ];
    IndexedTriangleSet { vertices, indices }
}

/// Single isolated triangle (one face, three unique vertices).
fn single_triangle(offset_x: f32) -> IndexedTriangleSet {
    let vertices = vec![
        Point3 {
            x: offset_x,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: offset_x + 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: offset_x + 0.5,
            y: 1.0,
            z: 0.0,
        },
    ];
    IndexedTriangleSet {
        vertices,
        indices: vec![0, 1, 2],
    }
}

// ---------------------------------------------------------------------------
// AC-5: split_component_counts
// ---------------------------------------------------------------------------

/// AC-5: component-count assertions for three mesh configurations.
#[test]
fn split_component_counts() {
    // (a) Two spatially-disjoint tetrahedra → 2 components.
    let two_tets = merge(tetrahedron(0.0, 0.0, 0.0), tetrahedron(100.0, 0.0, 0.0));
    let components = split_connected_components(&two_tets);
    assert_eq!(
        components.len(),
        2,
        "two disjoint tetrahedra must produce 2 components, got {}",
        components.len()
    );

    // (b) One watertight cube (12 triangles) → 1 component.
    let cube = valid_cube();
    let components = split_connected_components(&cube);
    assert_eq!(
        components.len(),
        1,
        "a single watertight cube must produce 1 component, got {}",
        components.len()
    );

    // (c) Two tetrahedra touching at a single shared vertex only → 2 components.
    let touching = two_tets_vertex_only_contact();
    let components = split_connected_components(&touching);
    assert_eq!(
        components.len(),
        2,
        "two tets sharing only a vertex must produce 2 components, got {}",
        components.len()
    );
}

// ---------------------------------------------------------------------------
// AC-N2: split_keeps_tiny_fragment
// ---------------------------------------------------------------------------

/// AC-N2: a single-triangle fragment must not be dropped (no size threshold).
#[test]
fn split_keeps_tiny_fragment() {
    // Large solid: a cube at origin (12 triangles).
    // Tiny fragment: a single triangle far away (spatially disjoint).
    let fragment = single_triangle(1000.0);
    let combined = merge(valid_cube(), fragment);

    let components = split_connected_components(&combined);

    assert_eq!(
        components.len(),
        2,
        "cube + single-triangle fragment must produce 2 components, got {}",
        components.len()
    );

    // Find the component that is the single-triangle fragment.
    let tiny = components
        .iter()
        .find(|c| c.indices.len() == 3)
        .expect("one component must have exactly 1 triangle (3 indices)");

    assert_eq!(
        tiny.vertices.len(),
        3,
        "single-triangle component must have exactly 3 vertices, got {}",
        tiny.vertices.len()
    );
}
