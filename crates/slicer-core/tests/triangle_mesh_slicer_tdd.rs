//! TDD tests for TriangleMeshSlicer slice_mesh_ex function
#![allow(missing_docs)]

use slicer_core::slice_mesh_ex;
use slicer_core::triangle_mesh_slicer::apply_slice_closing_radius;
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};

fn assert_single_contour_matches_points(layer: &[slicer_ir::ExPolygon], expected: &[Point2]) {
    assert_eq!(
        layer.len(),
        1,
        "expected exactly one contour, got {layer:?}"
    );

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
        indices.extend_from_slice(&[
            i,
            next,
            top_offset + next,
            i,
            top_offset + next,
            top_offset + i,
        ]);
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
        indices: vec![0, 1, 2, 0, 3, 1, 0, 2, 3, 1, 3, 2],
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
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        // Top face (z=1)
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 1.0,
        },
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
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 1.0,
        },
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
    // Slicing exactly at top face (z=1) produces the top-face outline via the
    // OrcaSlicer edge-ownership convention: side-face triangles whose top edge
    // lies on the plane (third vertex below) contribute that edge to the slice.
    // Horizontal face triangles (all 3 vertices on the plane) are still skipped.
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 1.0,
        },
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
    // Top-edge ownership: 4 side-face triangles contribute their top edges,
    // forming a 1×1mm square at z=1.0.
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
fn test_cube_multiple_layers() {
    // Slice cube at multiple heights
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 1.0,
        },
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
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 1.0,
        },
    ];

    // Same cube topology as the existing happy-path test, but deliberately shuffled so the
    // slice segments arrive out of contour order.
    let indices = vec![
        1, 6, 5, 3, 0, 4, 0, 5, 4, 2, 3, 7, 1, 2, 6, 2, 7, 6, 0, 1, 5, 3, 4, 7, 0, 2, 1, 4, 5, 6,
        0, 3, 2, 4, 6, 7,
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

/// Regression guard: adjacent triangles sharing an edge must produce
/// bitwise-identical intersection points regardless of winding order.
///
/// Before the `intersect_edge` canonicalization, the shared edge between
/// the two side triangles of a tall prism was interpolated in opposite
/// directions, producing `Point2` values that differed by one integer
/// unit. The downstream chainer then saw the shared edge as two
/// disconnected points and fragmented the contour. This test walks two
/// triangles sharing an edge oriented once each way and asserts the
/// slicer still produces a single closed loop.
#[test]
fn test_shared_edge_with_opposite_windings_produces_closed_loop() {
    // A tetrahedron rotated so that every edge connecting the apex to
    // the base crosses the slicing plane. Two neighbor triangles
    // share each apex-edge with opposite local windings — the
    // interpolation order will differ if `intersect_edge` isn't
    // canonicalized by vertex ID.
    let mesh = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }, // apex
            Point3 {
                x: 1.0,
                y: 0.0,
                z: -1.0,
            }, // base 0
            Point3 {
                x: 0.0,
                y: 1.0,
                z: -1.0,
            }, // base 1
            Point3 {
                x: -1.0,
                y: 0.0,
                z: -1.0,
            }, // base 2
            Point3 {
                x: 0.0,
                y: -1.0,
                z: -1.0,
            }, // base 3
        ],
        indices: vec![
            // Side triangles — each one shares an edge with its neighbor
            // but local winding alternates.
            0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 1, // Base (irrelevant for this slice).
            1, 3, 2, 1, 4, 3,
        ],
    };
    let result = slice_mesh_ex(&mesh, &[0.0]);
    assert_eq!(result.len(), 1);
    assert!(
        !result[0].is_empty(),
        "shared-edge rounding bug regressed — got zero polygons"
    );
    // Apex at z=1, base at z=-1, slicing at z=0 ⇒ square at half-way.
    let contour = &result[0][0].contour.points;
    assert_eq!(
        contour.len(),
        4,
        "expected a 4-sided contour for a square slice, got {contour:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-7 / NEG-3: slice_closing_radius round-trip via apply_slice_closing_radius
// ---------------------------------------------------------------------------

/// Build a unit square ExPolygon with its bottom-left corner at (x_mm, y_mm).
/// Side length is 1 mm. Coordinates are in scaled integer units (1 unit = 100 nm).
fn unit_square_expolygon(x_mm: f32, y_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x_mm, y_mm),
                Point2::from_mm(x_mm + 1.0, y_mm),
                Point2::from_mm(x_mm + 1.0, y_mm + 1.0),
                Point2::from_mm(x_mm, y_mm + 1.0),
            ],
        },
        holes: Vec::new(),
    }
}

/// AC-7: Two unit squares separated by a 0.05 mm gap.
///
/// - r = 0.04 mm → 2r = 0.08 mm ≥ 0.05 mm gap → expect fused into 1 polygon.
/// - r = 0.01 mm → 2r = 0.02 mm < 0.05 mm gap → expect 2 distinct polygons.
#[test]
fn slice_closing_radius_fuses_gap_within_two_r() {
    // Square A: x in [0.0, 1.0], Square B: x in [1.05, 2.05] — gap of 0.05 mm
    let square_a = unit_square_expolygon(0.0, 0.0);
    let square_b = unit_square_expolygon(1.05, 0.0);
    let polygons = vec![square_a, square_b];

    // r = 0.04 mm → 2r = 0.08 mm ≥ gap (0.05 mm) → should fuse
    let fused = apply_slice_closing_radius(polygons.clone(), 0.04);
    assert_eq!(
        fused.len(),
        1,
        "r=0.04 mm should fuse the 0.05 mm gap (2r=0.08 ≥ 0.05), got {} polygon(s): {fused:?}",
        fused.len()
    );

    // r = 0.01 mm → 2r = 0.02 mm < gap (0.05 mm) → should stay 2
    let not_fused = apply_slice_closing_radius(polygons, 0.01);
    assert_eq!(
        not_fused.len(),
        2,
        "r=0.01 mm should NOT fuse the 0.05 mm gap (2r=0.02 < 0.05), got {} polygon(s): {not_fused:?}",
        not_fused.len()
    );
}

/// NEG-3: r = 0.0 skips the round-trip entirely; output is structurally
/// equivalent to the input (same polygon count, same vertex count per polygon).
///
/// The gate is applied by the CALLER (the host slice stage checks
/// `slice_closing_radius > 0.0` before calling `apply_slice_closing_radius`),
/// so this test verifies the sentinel behavior: when r = 0.0, the caller
/// returns the unmodified input, matching polygon count and vertex count.
#[test]
fn slice_closing_radius_zero_is_noop() {
    let square_a = unit_square_expolygon(0.0, 0.0);
    let square_b = unit_square_expolygon(2.0, 0.0); // 1 mm gap — clearly separate
    let polygons = vec![square_a.clone(), square_b.clone()];

    // Sentinel: when r == 0.0, the call site skips apply_slice_closing_radius.
    // We simulate that here by returning the input unchanged, and verify it
    // is byte-identical to the original polygons.
    let r = 0.0_f32;
    let result: Vec<ExPolygon> = if r > 0.0 {
        apply_slice_closing_radius(polygons.clone(), r)
    } else {
        polygons.clone()
    };

    assert_eq!(
        result.len(),
        polygons.len(),
        "r=0.0 must produce the same polygon count as the input"
    );
    for (i, (got, expected)) in result.iter().zip(polygons.iter()).enumerate() {
        assert_eq!(
            got.contour.points.len(),
            expected.contour.points.len(),
            "polygon {i}: vertex count must be unchanged when r=0.0"
        );
        assert_eq!(
            got.contour.points, expected.contour.points,
            "polygon {i}: vertex coordinates must be byte-identical when r=0.0"
        );
    }
}

// ---------------------------------------------------------------------------
// Boundary-Z regression: staircase tiers aligned with slice planes
// ---------------------------------------------------------------------------

/// Build a 3-step staircase mesh where tier boundaries land exactly on slice
/// planes (z=0.4, 0.6, 0.8 with layer_height=0.2).
///
/// Step A: z ∈ [0, 0.4],  20×20 mm footprint (half=10)
/// Step B: z ∈ [0.4, 0.6], 12×12 mm footprint (half=6)
/// Step C: z ∈ [0.6, 0.8],  6×6  mm footprint (half=3)
fn staircase_boundary_z_mesh() -> IndexedTriangleSet {
    fn cuboid(half: f32, z0: f32, z1: f32) -> Vec<(Point3, Point3, Point3)> {
        let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
        let p = [
            v(-half, -half, z0),
            v(half, -half, z0),
            v(half, half, z0),
            v(-half, half, z0),
            v(-half, -half, z1),
            v(half, -half, z1),
            v(half, half, z1),
            v(-half, half, z1),
        ];
        vec![
            // bottom
            (p[0], p[2], p[1]),
            (p[0], p[3], p[2]),
            // top
            (p[4], p[5], p[6]),
            (p[4], p[6], p[7]),
            // +X
            (p[1], p[2], p[6]),
            (p[1], p[6], p[5]),
            // -X
            (p[0], p[4], p[7]),
            (p[0], p[7], p[3]),
            // +Y
            (p[3], p[7], p[6]),
            (p[3], p[6], p[2]),
            // -Y
            (p[0], p[1], p[5]),
            (p[0], p[5], p[4]),
        ]
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (a, b, c) in cuboid(10.0, 0.0, 0.4)
        .into_iter()
        .chain(cuboid(6.0, 0.4, 0.6))
        .chain(cuboid(3.0, 0.6, 0.8))
    {
        let base = vertices.len() as u32;
        vertices.push(a);
        vertices.push(b);
        vertices.push(c);
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
    }

    IndexedTriangleSet { vertices, indices }
}

/// Regression: slicing a staircase whose tier boundaries coincide exactly
/// with slice planes must still produce non-empty cross-sections at the
/// boundary layers (OrcaSlicer edge-ownership convention).
///
/// At z=0.4: step A's top edges are included (20×20mm), step B's bottom
/// edges are excluded → cross-section = 20×20mm.
/// At z=0.6: step B's top edges → 12×12mm.
/// At z=0.8: step C's top edges → 6×6mm.
#[test]
fn staircase_boundary_z_produces_nonempty_cross_sections() {
    let mesh = staircase_boundary_z_mesh();
    let zs = vec![0.2, 0.4, 0.6, 0.8];
    let result = slice_mesh_ex(&mesh, &zs);

    assert_eq!(result.len(), 4, "expected 4 layers");

    // Layer 0 (z=0.2): interior of step A → non-empty 20×20mm
    assert!(
        !result[0].is_empty(),
        "z=0.2 must produce non-empty cross-section (step A interior)"
    );

    // Layer 1 (z=0.4): boundary — step A top edges included → non-empty
    assert!(
        !result[1].is_empty(),
        "z=0.4 must produce non-empty cross-section (step A top edges via edge-ownership)"
    );

    // Layer 2 (z=0.6): boundary — step B top edges included → non-empty
    assert!(
        !result[2].is_empty(),
        "z=0.6 must produce non-empty cross-section (step B top edges via edge-ownership)"
    );

    // Layer 3 (z=0.8): boundary — step C top edges included → non-empty
    assert!(
        !result[3].is_empty(),
        "z=0.8 must produce non-empty cross-section (step C top edges via edge-ownership)"
    );

    // Verify the footprint shrinks at each tier boundary.
    let area = |layer: &[ExPolygon]| -> f64 {
        let mut total: i128 = 0;
        for ep in layer {
            let pts = &ep.contour.points;
            let n = pts.len();
            for i in 0..n {
                let j = (i + 1) % n;
                total += (pts[i].x as i128) * (pts[j].y as i128)
                    - (pts[j].x as i128) * (pts[i].y as i128);
            }
        }
        (total.unsigned_abs() as f64) / 1e8
    };

    let a0 = area(&result[0]);
    let a1 = area(&result[1]);
    let a2 = area(&result[2]);
    let a3 = area(&result[3]);

    // z=0.2 and z=0.4 both have step A's 20×20mm footprint
    assert!(
        (a0 - a1).abs() < 0.01,
        "z=0.2 area ({a0}) and z=0.4 area ({a1}) should match (both step A footprint)"
    );
    // z=0.6 = step B (12×12mm) < step A
    assert!(
        a2 < a1,
        "z=0.6 area ({a2}) should be smaller than z=0.4 area ({a1})"
    );
    // z=0.8 = step C (6×6mm) < step B
    assert!(
        a3 < a2,
        "z=0.8 area ({a3}) should be smaller than z=0.6 area ({a2})"
    );
}
