//! Independent, exact production wrapper and mesh-slicing comparisons on both targets.
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};

fn cube(x: f32, y: f32, side: f32) -> IndexedTriangleSet {
    let vertices = [
        (0., 0., 0.),
        (1., 0., 0.),
        (1., 1., 0.),
        (0., 1., 0.),
        (0., 0., 1.),
        (1., 0., 1.),
        (1., 1., 1.),
        (0., 1., 1.),
    ]
    .into_iter()
    .map(|(a, b, z)| Point3 {
        x: x + a * side,
        y: y + b * side,
        z,
    })
    .collect();
    IndexedTriangleSet {
        vertices,
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    }
}

fn run() -> u32 {
    let mut comparisons = 0;
    for reverse in [false, true] {
        let mut mesh = cube(0., 0., 10.);
        let inner = cube(3., 3., 4.);
        let offset = mesh.vertices.len() as u32;
        mesh.vertices.extend(inner.vertices);
        mesh.indices
            .extend(inner.indices.into_iter().map(|i| i + offset));
        if reverse {
            for tri in mesh.indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }
        }
        let zs = [0.125, 0.5, 0.875];
        let actual = core_candidate::slice_mesh_ex(&mesh, &zs);
        let expected = core_original::slice_mesh_ex(&mesh, &zs);
        assert_eq!(actual, expected, "ordered nested mesh contours/holes");
        assert_eq!(actual.len(), 3);
        for layer in &actual {
            assert_eq!(layer.len(), 1, "nested solid population");
            assert_eq!(
                layer[0].holes.len(),
                1,
                "nested mesh must exercise hole reconstruction"
            );
            assert_eq!(layer[0].contour.points.len(), 4);
        }
        comparisons += 1;
        for radius in [0.025, 0.125] {
            let a = core_candidate::triangle_mesh_slicer::apply_slice_closing_radius(
                actual[1].clone(),
                radius,
            );
            let b = core_original::triangle_mesh_slicer::apply_slice_closing_radius(
                expected[1].clone(),
                radius,
            );
            assert!(!a.is_empty());
            assert_eq!(a, b, "mesh closing offset/union consumers");
            comparisons += 1;
        }
    }
    for shift in [0, 1, 16_777_217, -16_777_217] {
        let ring = |a, b| Polygon {
            points: vec![
                Point2 { x: shift + a, y: a },
                Point2 { x: shift + b, y: a },
                Point2 { x: shift + b, y: b },
                Point2 { x: shift + a, y: b },
            ],
        };
        let subject = vec![ExPolygon {
            contour: ring(0, 10000),
            holes: vec![Polygon {
                points: ring(2000, 8000).points.into_iter().rev().collect(),
            }],
        }];
        let clip = vec![ExPolygon {
            contour: ring(5000, 12000),
            holes: vec![],
        }];
        for (a, b) in [
            (
                core_candidate::polygon_ops::union_ex(&subject),
                core_original::polygon_ops::union_ex(&subject),
            ),
            (
                core_candidate::polygon_ops::difference_ex(&subject, &clip),
                core_original::polygon_ops::difference_ex(&subject, &clip),
            ),
            (
                core_candidate::polygon_ops::intersection_ex(&subject, &clip),
                core_original::polygon_ops::intersection_ex(&subject, &clip),
            ),
        ] {
            assert!(!a.is_empty(), "wrapper population");
            assert_eq!(a, b, "ordered production boolean wrapper results");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 18);
    comparisons
}

#[no_mangle]
pub extern "C" fn t57_consumer_checks() -> u32 {
    run()
}

#[test]
fn independent_ordered_mesh_and_polygon_consumers() {
    assert_eq!(run(), 18);
}
