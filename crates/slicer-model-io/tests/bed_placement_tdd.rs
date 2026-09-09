#![allow(missing_docs)]
//! Regression tests for bed placement.
//!
//! `tmp/base.stl`-shaped models are authored far from the origin and below the
//! plate (that model sits at X ~ -728, z_min = -75.7). Before placement the
//! slicer emitted those raw coordinates, so the print was both off the plate in
//! XY and truncated at z = 0 — 116 layers instead of 495.

use slicer_ir::{BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer};
use slicer_model_io::{bed_center_mm, bed_overflow_mm, place_bare_mesh_on_bed};
use std::collections::HashMap;

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

/// `assemble_object` leaves transforms as the all-zero identity convention.
fn object(id: &str, vertices: Vec<Point3>) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            indices: (0..vertices.len() as u32).collect(),
            vertices,
        },
        transform: slicer_ir::Transform3d { matrix: [0.0; 16] },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        ..Default::default()
    }
}

fn mesh_of(objects: Vec<ObjectMesh>) -> MeshIR {
    MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects,
        build_volume: BoundingBox3 {
            min: p3(0.0, 0.0, 0.0),
            max: p3(0.0, 0.0, 0.0),
        },
    }
}

fn world_aabb(mesh: &MeshIR) -> (Point3, Point3) {
    let mut lo = p3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut hi = p3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for o in &mesh.objects {
        for v in &o.mesh.vertices {
            lo = p3(lo.x.min(v.x), lo.y.min(v.y), lo.z.min(v.z));
            hi = p3(hi.x.max(v.x), hi.y.max(v.y), hi.z.max(v.z));
        }
    }
    (lo, hi)
}

/// The `tmp/base.stl` case: far off-origin in XY and sunk below the plate.
#[test]
fn far_off_origin_model_is_centred_and_dropped_onto_the_bed() {
    let mut mesh = mesh_of(vec![object(
        "base",
        vec![
            p3(-780.0, 25.0, -75.0),
            p3(-680.0, 70.0, 20.0),
            p3(-730.0, 50.0, -30.0),
        ],
    )]);

    let placement = place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");

    let (lo, hi) = world_aabb(&mesh);
    assert!(
        (lo.z - 0.0).abs() < 1e-3,
        "model must rest on the plate, got z_min {}",
        lo.z
    );
    assert!(
        ((lo.x + hi.x) / 2.0 - 125.0).abs() < 1e-3,
        "X centre should be the bed centre, got {}",
        (lo.x + hi.x) / 2.0
    );
    assert!(
        ((lo.y + hi.y) / 2.0 - 125.0).abs() < 1e-3,
        "Y centre should be the bed centre, got {}",
        (lo.y + hi.y) / 2.0
    );
    assert!((placement.dz - 75.0).abs() < 1e-3, "dz lifts z_min to 0");
}

/// Placement is a rigid-body move: multi-object scenes keep their arrangement.
#[test]
fn multi_object_relative_positions_are_preserved() {
    let mut mesh = mesh_of(vec![
        object(
            "a",
            vec![p3(0.0, 0.0, 0.0), p3(10.0, 0.0, 0.0), p3(0.0, 10.0, 10.0)],
        ),
        object(
            "b",
            vec![
                p3(40.0, 0.0, 0.0),
                p3(50.0, 0.0, 0.0),
                p3(40.0, 10.0, 10.0),
            ],
        ),
    ]);

    let before = mesh.objects[1].mesh.vertices[0].x - mesh.objects[0].mesh.vertices[0].x;
    place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");
    let after = mesh.objects[1].mesh.vertices[0].x - mesh.objects[0].mesh.vertices[0].x;

    assert!(
        (before - after).abs() < 1e-4,
        "inter-object spacing changed: {before} -> {after}"
    );
}

/// An already-centred, already-grounded model is left alone.
#[test]
fn already_placed_model_is_not_moved() {
    let mut mesh = mesh_of(vec![object(
        "centred",
        vec![
            p3(120.0, 120.0, 0.0),
            p3(130.0, 120.0, 0.0),
            p3(125.0, 130.0, 5.0),
        ],
    )]);

    let placement = place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");
    assert!(placement.dx.abs() < 1e-4);
    assert!(placement.dy.abs() < 1e-4);
    assert!(placement.dz.abs() < 1e-4);
}

/// `build_volume` is the model AABB, so it must follow the model.
#[test]
fn build_volume_tracks_the_placed_model() {
    let mut mesh = mesh_of(vec![object(
        "m",
        vec![
            p3(-500.0, -500.0, -10.0),
            p3(-490.0, -500.0, -10.0),
            p3(-500.0, -490.0, 0.0),
        ],
    )]);

    place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");

    let (lo, hi) = world_aabb(&mesh);
    assert!((mesh.build_volume.min.x - lo.x).abs() < 1e-3);
    assert!((mesh.build_volume.max.z - hi.z).abs() < 1e-3);
    assert!((mesh.build_volume.min.z - 0.0).abs() < 1e-3);
}

#[test]
fn bed_centre_is_read_from_the_bed_shape_polygon() {
    // Default 250 x 250 square from `ResolvedConfig::bed_shape`.
    let square = vec![0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0];
    assert_eq!(bed_center_mm(&square), Some((125.0, 125.0)));
}

#[test]
fn unusable_bed_shapes_are_rejected_rather_than_guessed() {
    assert_eq!(bed_center_mm(&[]), None, "empty");
    assert_eq!(bed_center_mm(&[0.0, 0.0, 1.0, 1.0]), None, "two points");
    assert_eq!(
        bed_center_mm(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        None,
        "zero area"
    );
    assert_eq!(
        bed_center_mm(&[0.0, 0.0, f64::NAN, 0.0, 1.0, 1.0]),
        None,
        "non-finite"
    );
}

/// A model larger than the plate must be reported, not silently sliced.
#[test]
fn oversized_model_reports_bed_overflow() {
    let square = vec![0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0];
    let mut mesh = mesh_of(vec![object(
        "huge",
        vec![
            p3(0.0, 0.0, 0.0),
            p3(400.0, 0.0, 0.0),
            p3(0.0, 300.0, 10.0),
        ],
    )]);
    place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");

    let (over_x, over_y) = bed_overflow_mm(&mesh, &square).expect("must report overflow");
    // 400 mm wide on a 250 mm bed, centred: 75 mm past each edge.
    assert!((over_x - 75.0).abs() < 1e-3, "over_x was {over_x}");
    assert!((over_y - 25.0).abs() < 1e-3, "over_y was {over_y}");
}

/// A model that fits reports no overflow.
#[test]
fn fitting_model_reports_no_overflow() {
    let square = vec![0.0, 0.0, 250.0, 0.0, 250.0, 250.0, 0.0, 250.0];
    let mut mesh = mesh_of(vec![object(
        "small",
        vec![
            p3(0.0, 0.0, 0.0),
            p3(20.0, 0.0, 0.0),
            p3(0.0, 20.0, 5.0),
        ],
    )]);
    place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");

    assert_eq!(bed_overflow_mm(&mesh, &square), None);
}
