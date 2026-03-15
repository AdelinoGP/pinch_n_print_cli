//! TDD tests for TriangleMeshSlicer slice_mesh_ex function
#![allow(missing_docs)]

use slicer_core::slice_mesh_ex;
use slicer_ir::{IndexedTriangleSet, Point2, Point3};

fn assert_single_contour_matches_points(layer: &[slicer_ir::ExPolygon], expected: &[Point2]) {
    assert_eq!(layer.len(), 1, "expected exactly one contour, got {layer:?}");

    let contour = &layer[0].contour.points;
    assert_eq!(
        contour.len(),
        expected.len(),
        "unexpected contour length for contour {contour:?}"
    );

    for point in expected {
        assert!(
            contour.contains(point),
            "missing expected point {point:?} in contour {contour:?}"
        );
    }
}

fn build_open_strip_mesh(polyline_xy: &[(f32, f32)]) -> IndexedTriangleSet {
    let mut vertices = Vec::with_capacity(polyline_xy.len() * 2);
    for &(x, y) in polyline_xy {
        vertices.push(Point3 { x, y, z: 0.0 });
    }
    for &(x, y) in polyline_xy {
        vertices.push(Point3 { x, y, z: 1.0 });
    }

    let mut indices = Vec::with_capacity((polyline_xy.len().saturating_sub(1)) * 6);
    let top_offset = polyline_xy.len() as u32;
    for i in 0..polyline_xy.len().saturating_sub(1) as u32 {
        let next = i + 1;
        indices.extend_from_slice(&[i, next, top_offset + next, i, top_offset + next, top_offset + i]);
    }

    IndexedTriangleSet { vertices, indices }
}

fn build_vertex_touching_tetrahedron() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 2.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
        ],
        indices: vec![
            0, 1, 2,
            0, 3, 1,
            0, 2, 3,
            1, 3, 2,
        ],
    }
}

#[test]
fn test_empty_mesh_produces_empty_layers() {
    let mesh = IndexedTriangleSet {
        vertices: vec![],
        indices: vec![],
    };
    let zs = vec![0.0, 0.5, 1.0];
    let result = slice_mesh_ex(&mesh, &zs);

    assert_eq!(result.len(), 3);
    assert!(result[0].is_empty());
    assert!(result[1].is_empty());
    assert!(result[2].is_empty());
}

#[test]
fn test_cube_sliced_at_half_height() {
    // Create a unit cube from (0,0,0) to (1,1,1)
    let vertices = vec![
        // Bottom face (z=0)
        Point3 { x: 0.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 1.0, z: 0.0 },
        // Top face (z=1)
        Point3 { x: 0.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 1.0, z: 1.0 },
        Point3 { x: 0.0, y: 1.0, z: 1.0 },
    ];

    // 12 triangles (2 per face)
    let indices = vec![
        // Bottom face (z=0) - 2 triangles (ccw when viewed from below)
        0, 2, 1, // triangle 1
        0, 3, 2, // triangle 2
        // Top face (z=1) - 2 triangles (ccw when viewed from above)
        4, 5, 6, // triangle 3
        4, 6, 7, // triangle 4
        // Side faces
        // Front face (y=0)
        0, 1, 5, // triangle 5
        0, 5, 4, // triangle 6
        // Right face (x=1)
        1, 2, 6, // triangle 7
        1, 6, 5, // triangle 8
        // Back face (y=1)
        2, 3, 7, // triangle 9
        2, 7, 6, // triangle 10
        // Left face (x=0)
        3, 0, 4, // triangle 11
        3, 4, 7, // triangle 12
    ];

    let mesh = IndexedTriangleSet { vertices, indices };
    let zs = vec![0.5];
    let result = slice_mesh_ex(&mesh, &zs);

    // Should produce one layer
    assert_eq!(result.len(), 1);

    // Should contain one polygon (the square cross-section)
    let layer = &result[0];
    assert_eq!(layer.len(), 1);

        // Check the polygon has 4 points (square)
        let expolygon = &layer[0];
        assert_eq!(expolygon.contour.points.len(), 4);
    assert!(expolygon.holes.is_empty());

    // Check points are at correct locations (scaled integers)
    // Unit cube from 0 to 1, sliced at 0.5
    // Should give square from (0,0) to (1,1)
    // Coordinates are scaled by 10_000
    let expected_points = vec![
        Point2::from_mm(0.0, 0.0),
        Point2::from_mm(1.0, 0.0),
        Point2::from_mm(1.0, 1.0),
        Point2::from_mm(0.0, 1.0),
    ];

    // Check if the contour points match (order might vary due to orientation)
    for point in &expolygon.contour.points {
        // The point should be one of the expected corners
        let is_valid = expected_points.iter().any(|p| p == point);
        assert!(is_valid, "Unexpected point: {:?}", point);
    }
}

#[test]
fn test_cube_sliced_at_bottom() {
    // Slicing exactly at bottom face (z=0) should produce empty
    // because horizontal triangles are ignored
    let vertices = vec![
        Point3 { x: 0.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 1.0, z: 1.0 },
        Point3 { x: 0.0, y: 1.0, z: 1.0 },
    ];

    let indices = vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Top
        0, 1, 5, 0, 5, 4, // Front
        1, 2, 6, 1, 6, 5, // Right
        2, 3, 7, 2, 7, 6, // Back
        3, 0, 4, 3, 4, 7, // Left
    ];

    let mesh = IndexedTriangleSet { vertices, indices };
    let zs = vec![0.0];
    let result = slice_mesh_ex(&mesh, &zs);

    assert_eq!(result.len(), 1);
    assert!(result[0].is_empty());
}

#[test]
fn test_cube_sliced_at_top() {
    // Slicing exactly at top face (z=1) should produce empty
    let vertices = vec![
        Point3 { x: 0.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 1.0, z: 1.0 },
        Point3 { x: 0.0, y: 1.0, z: 1.0 },
    ];

    let indices = vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Top
        0, 1, 5, 0, 5, 4, // Front
        1, 2, 6, 1, 6, 5, // Right
        2, 3, 7, 2, 7, 6, // Back
        3, 0, 4, 3, 4, 7, // Left
    ];

    let mesh = IndexedTriangleSet { vertices, indices };
    let zs = vec![1.0];
    let result = slice_mesh_ex(&mesh, &zs);

    assert_eq!(result.len(), 1);
    assert!(result[0].is_empty());
}

#[test]
fn test_cube_multiple_layers() {
    // Slice cube at multiple heights
    let vertices = vec![
        Point3 { x: 0.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 1.0, z: 1.0 },
        Point3 { x: 0.0, y: 1.0, z: 1.0 },
    ];

    let indices = vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Top
        0, 1, 5, 0, 5, 4, // Front
        1, 2, 6, 1, 6, 5, // Right
        2, 3, 7, 2, 7, 6, // Back
        3, 0, 4, 3, 4, 7, // Left
    ];

    let mesh = IndexedTriangleSet { vertices, indices };
    let zs = vec![0.25, 0.5, 0.75];
    let result = slice_mesh_ex(&mesh, &zs);

    // Should produce 3 layers
    assert_eq!(result.len(), 3);

    // Each layer should have one square polygon
    for layer in &result {
        assert_eq!(layer.len(), 1);
        let expolygon = &layer[0];
        assert_eq!(expolygon.contour.points.len(), 4);
        assert!(expolygon.holes.is_empty());
    }
}

#[test]
fn test_unordered_cube_segments_still_chain_to_one_closed_loop() {
    let vertices = vec![
        Point3 { x: 0.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 0.0, z: 0.0 },
        Point3 { x: 1.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 1.0, z: 0.0 },
        Point3 { x: 0.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 0.0, z: 1.0 },
        Point3 { x: 1.0, y: 1.0, z: 1.0 },
        Point3 { x: 0.0, y: 1.0, z: 1.0 },
    ];

    // Same cube topology as the existing happy-path test, but deliberately shuffled so the
    // slice segments arrive out of contour order.
    let indices = vec![
        1, 6, 5,
        3, 0, 4,
        0, 5, 4,
        2, 3, 7,
        1, 2, 6,
        2, 7, 6,
        0, 1, 5,
        3, 4, 7,
        0, 2, 1,
        4, 5, 6,
        0, 3, 2,
        4, 6, 7,
    ];

    let mesh = IndexedTriangleSet { vertices, indices };
    let result = slice_mesh_ex(&mesh, &[0.5]);

    assert_eq!(result.len(), 1);
    assert_single_contour_matches_points(
        &result[0],
        &[
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(1.0, 0.0),
            Point2::from_mm(1.0, 1.0),
            Point2::from_mm(0.0, 1.0),
        ],
    );
}

#[test]
fn test_open_strip_slice_is_not_silently_emitted_as_closed_polygon() {
    let mesh = build_open_strip_mesh(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (2.0, 1.0)]);

    let result = slice_mesh_ex(&mesh, &[0.5]);

    assert_eq!(result.len(), 1);
    assert!(
        result[0].is_empty(),
        "expected no closed polygons for an open chain, got {:?}",
        result[0]
    );
}

#[test]
fn test_slice_through_mesh_vertex_still_forms_closed_triangle() {
    let mesh = build_vertex_touching_tetrahedron();

    let result = slice_mesh_ex(&mesh, &[0.5]);

    assert_eq!(result.len(), 1);
    assert_single_contour_matches_points(
        &result[0],
        &[
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(-0.5, 1.0),
            Point2::from_mm(0.5, 1.0),
        ],
    );
}
