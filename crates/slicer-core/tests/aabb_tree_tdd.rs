#![allow(missing_docs)]

use slicer_core::AabbTree;
use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3};

const EPS: f32 = 1.0e-5;

fn assert_close(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(
        delta <= EPS,
        "expected {expected}, got {actual}, delta {delta}"
    );
}

fn assert_point3(actual: Point3, expected_x: f32, expected_y: f32, expected_z: f32) {
    assert!(actual.x.is_finite(), "x must be finite: {actual:?}");
    assert!(actual.y.is_finite(), "y must be finite: {actual:?}");
    assert!(actual.z.is_finite(), "z must be finite: {actual:?}");
    assert_close(actual.x, expected_x);
    assert_close(actual.y, expected_y);
    assert_close(actual.z, expected_z);
}

fn empty_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![],
        indices: vec![],
    }
}

fn unit_cube_mesh() -> IndexedTriangleSet {
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
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2,
        7, 6, 3, 0, 4, 3, 4, 7,
    ];

    IndexedTriangleSet { vertices, indices }
}

fn assert_bounds(bounds: BoundingBox3, min: (f32, f32, f32), max: (f32, f32, f32)) {
    assert_point3(bounds.min, min.0, min.1, min.2);
    assert_point3(bounds.max, max.0, max.1, max.2);
}

#[test]
fn empty_mesh_reports_no_bounds_hits_or_closest_point() {
    let tree = AabbTree::new(empty_mesh());

    assert!(tree.is_empty());
    assert_eq!(tree.bounds(), None);
    assert_eq!(
        tree.raycast_first_hit(
            Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        ),
        None
    );
    assert!(
        tree.raycast_all_hits(
            Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        )
        .is_empty()
    );
    assert_eq!(
        tree.closest_point(Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
        None
    );
}

#[test]
fn bounds_match_unit_cube_vertex_extrema() {
    let tree = AabbTree::new(unit_cube_mesh());

    let bounds = tree.bounds().expect("unit cube should have bounds");
    assert_bounds(bounds, (0.0, 0.0, 0.0), (1.0, 1.0, 1.0));
}

#[test]
fn positive_z_raycast_from_below_hits_cube_bottom_face_first() {
    let tree = AabbTree::new(unit_cube_mesh());

    let hit = tree
        .raycast_first_hit(
            Point3 {
                x: 0.5,
                y: 0.5,
                z: -1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        )
        .expect("ray should hit the cube");

    assert!(hit.distance.is_finite(), "distance must be finite: {hit:?}");
    assert_close(hit.distance, 1.0);
    assert_point3(hit.point, 0.5, 0.5, 0.0);
}

#[test]
fn raycast_all_hits_returns_sorted_entry_and_exit_intersections() {
    let tree = AabbTree::new(unit_cube_mesh());

    let hits = tree.raycast_all_hits(
        Point3 {
            x: 0.5,
            y: 0.5,
            z: -1.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    );

    assert_eq!(hits.len(), 2, "expected entry and exit hits, got {hits:?}");
    assert!(hits[0].distance.is_finite(), "distance must be finite: {hits:?}");
    assert!(hits[1].distance.is_finite(), "distance must be finite: {hits:?}");
    assert!(
        hits[0].distance <= hits[1].distance,
        "hits must be sorted by distance: {hits:?}"
    );
    assert_close(hits[0].distance, 1.0);
    assert_close(hits[1].distance, 2.0);
    assert_point3(hits[0].point, 0.5, 0.5, 0.0);
    assert_point3(hits[1].point, 0.5, 0.5, 1.0);
}

#[test]
fn closest_point_projects_queries_below_and_above_the_cube() {
    let tree = AabbTree::new(unit_cube_mesh());

    let below = tree
        .closest_point(Point3 {
            x: 0.25,
            y: 0.75,
            z: -2.0,
        })
        .expect("below query should project onto cube");
    assert!(
        below.squared_distance.is_finite(),
        "squared distance must be finite: {below:?}"
    );
    assert_point3(below.point, 0.25, 0.75, 0.0);
    assert_close(below.squared_distance, 4.0);

    let above = tree
        .closest_point(Point3 {
            x: 0.25,
            y: 0.75,
            z: 3.0,
        })
        .expect("above query should project onto cube");
    assert!(
        above.squared_distance.is_finite(),
        "squared distance must be finite: {above:?}"
    );
    assert_point3(above.point, 0.25, 0.75, 1.0);
    assert_close(above.squared_distance, 4.0);
}

#[test]
fn ray_miss_returns_no_intersections() {
    let tree = AabbTree::new(unit_cube_mesh());

    assert_eq!(
        tree.raycast_first_hit(
            Point3 {
                x: 2.0,
                y: 2.0,
                z: 0.5,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        ),
        None
    );
    assert!(
        tree.raycast_all_hits(
            Point3 {
                x: 2.0,
                y: 2.0,
                z: 0.5,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        )
        .is_empty()
    );
}
