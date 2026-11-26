#![allow(missing_docs)]
//! Regression tests for bed placement.
//!
//! `tmp/base.stl`-shaped models are authored far from the origin and below the
//! plate (that model sits at X ~ -728, z_min = -75.7). Before placement the
//! slicer emitted those raw coordinates, so the print was both off the plate in
//! XY and truncated at z = 0 — 116 layers instead of 495.

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
};
use slicer_model_io::{bed_center_mm, bed_overflow_mm, place_bare_mesh_on_bed};
use std::collections::HashMap;

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

/// A hand-built object using the all-zero identity convention these fixtures
/// share. The production loader's `assemble_object` instead writes a real
/// identity matrix; the `identity_matrix_placement_...` regression covers that
/// shape.
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

/// Raw-vertex AABB, valid for the all-zero fixture convention and for objects
/// whose placement shifted the vertices.
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

fn identity_transform() -> slicer_ir::Transform3d {
    let mut matrix = [0.0f64; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    slicer_ir::Transform3d { matrix }
}

/// World-space AABB, applying the zero-matrix-is-identity convention. This is
/// the space the prepass slices (`object_world_mesh`) and `MeshIR::build_volume`
/// is required to track.
fn world_aabb_transformed(mesh: &MeshIR) -> (Point3, Point3) {
    let mut lo = p3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut hi = p3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for object in &mesh.objects {
        let m = &object.transform.matrix;
        let identity = m.iter().all(|value| *value == 0.0);
        for vertex in &object.mesh.vertices {
            let world = if identity {
                *vertex
            } else {
                let (x, y, z) = (
                    f64::from(vertex.x),
                    f64::from(vertex.y),
                    f64::from(vertex.z),
                );
                p3(
                    (m[0] * x + m[4] * y + m[8] * z + m[12]) as f32,
                    (m[1] * x + m[5] * y + m[9] * z + m[13]) as f32,
                    (m[2] * x + m[6] * y + m[10] * z + m[14]) as f32,
                )
            };
            lo = p3(lo.x.min(world.x), lo.y.min(world.y), lo.z.min(world.z));
            hi = p3(hi.x.max(world.x), hi.y.max(world.y), hi.z.max(world.z));
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
            vec![p3(40.0, 0.0, 0.0), p3(50.0, 0.0, 0.0), p3(40.0, 10.0, 10.0)],
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

/// Regression: an identity-matrix object's placement must update
/// `MeshIR::build_volume` to the **post-placement world-space** AABB union.
///
/// Root cause: `place_bare_mesh_on_bed` recomputed `build_volume` with
/// `compute_bounding_box_union` over **raw** mesh vertices. The loader
/// assembles every object with a *real* identity matrix (`assemble_object`),
/// not the all-zero fixture convention, so placement folds the translation
/// into `ObjectMesh::transform.matrix` and leaves the vertices untouched. The
/// raw-vertex recompute therefore preserved the pre-placement box, while the
/// prepass slices world-space geometry
/// (`slicer_core::algos::prepass_slice::object_world_mesh`) and
/// `crates/pnp-cli/src/visual_debug.rs`'s `mesh_xy_bounds` reads
/// `build_volume` as the world-space model extent.
///
/// Formerly failing input: this single identity-matrix object at
/// x[-500,-490], y[-500,-490], z[-10,0], placed at bed centre (125,125),
/// reported `build_volume.min.x == -500` and `build_volume.max.z == 0`
/// instead of the placed world union x[120,130], y[120,130], z[0,10].
#[test]
fn identity_matrix_placement_updates_build_volume_to_the_world_union() {
    let vertices = vec![
        p3(-500.0, -500.0, -10.0),
        p3(-490.0, -500.0, -10.0),
        p3(-500.0, -490.0, 0.0),
    ];
    let mut mesh = mesh_of(vec![object("identity-matrix", vertices.clone())]);
    mesh.objects[0].transform = identity_transform();
    let (pre_lo, pre_hi) = world_aabb_transformed(&mesh);
    mesh.build_volume = BoundingBox3 {
        min: pre_lo,
        max: pre_hi,
    };

    let placement = place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");
    assert_eq!(
        (placement.dx, placement.dy, placement.dz),
        (620.0, 620.0, 10.0)
    );
    assert_eq!(
        mesh.objects[0].mesh.vertices, vertices,
        "the real identity-matrix branch folds the shift into transform.matrix"
    );
    assert_eq!(
        (
            mesh.objects[0].transform.matrix[12],
            mesh.objects[0].transform.matrix[13],
            mesh.objects[0].transform.matrix[14],
        ),
        (620.0, 620.0, 10.0)
    );

    let (lo, hi) = world_aabb_transformed(&mesh);
    assert_eq!((lo.x, lo.y, lo.z), (120.0, 120.0, 0.0));
    assert_eq!((hi.x, hi.y, hi.z), (130.0, 130.0, 10.0));
    assert_eq!(
        mesh.build_volume,
        BoundingBox3 { min: lo, max: hi },
        "build_volume must be the post-placement world AABB union; the former raw-vertex \
         recompute kept x[-500,-490], y[-500,-490], z[-10,0]"
    );
}

#[test]
fn zero_shift_placement_still_refreshes_build_volume() {
    // Formerly failing input: an already-centred identity mesh whose
    // `build_volume` was stale, placed with an exactly-zero translation. The
    // old early return skipped the recompute and left the stale box; the
    // world-space contract is refreshed on every successful placement.
    //
    // The x coordinates are signed zeros, so the same test pins that a
    // zero-shift placement does not rewrite `-0.0` to `+0.0`.
    let vertices = vec![p3(-0.0, -1.0, 0.0), p3(0.0, 1.0, 0.0), p3(-0.0, 0.0, 2.0)];
    let mut mesh = mesh_of(vec![object("centred", vertices)]);
    mesh.objects[0].transform = identity_transform();
    mesh.build_volume = BoundingBox3 {
        min: p3(-9.0, -9.0, -9.0),
        max: p3(9.0, 9.0, 9.0),
    };

    let placement = place_bare_mesh_on_bed(&mut mesh, (0.0, 0.0)).expect("placeable");
    assert_eq!(
        (placement.dx, placement.dy, placement.dz),
        (0.0, 0.0, 0.0),
        "the test input must be exactly centred so the old early return fired"
    );
    assert_eq!(
        mesh.objects[0].mesh.vertices[0].x.to_bits(),
        (-0.0_f32).to_bits(),
        "a zero-shift placement must preserve the `-0.0` bit pattern"
    );
    let (lo, hi) = world_aabb_transformed(&mesh);
    assert_eq!(
        mesh.build_volume,
        BoundingBox3 { min: lo, max: hi },
        "a zero-shift placement must still refresh build_volume; the old early return kept \
         the stale [-9,9] box"
    );
}

#[test]
fn extreme_same_sign_extents_do_not_overflow_the_midpoint() {
    // Formerly failing input: every vertex at x = +2e38. The f32 expression
    // `(min.x + max.x) / 2.0` overflowed to `+inf`, so `dx` became `-inf`, the
    // shifted vertices became non-finite, and the post-shift union was `None`
    // (a panic on the debug-build arm). The f64 midpoint keeps the shift
    // finite and the world-space box is still refreshed.
    let vertices = vec![
        p3(2.0e38, -1.0, 0.0),
        p3(2.0e38, 1.0, 0.0),
        p3(2.0e38, 0.0, 2.0),
    ];
    let mut mesh = mesh_of(vec![object("extreme", vertices)]);
    mesh.objects[0].transform = identity_transform();
    mesh.build_volume = BoundingBox3 {
        min: p3(-9.0, -9.0, -9.0),
        max: p3(9.0, 9.0, 9.0),
    };

    let placement = place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("finite geometry");
    assert!(
        placement.dx.is_finite() && placement.dy.is_finite() && placement.dz.is_finite(),
        "the f64 midpoint must keep an extreme same-sign shift finite, got {placement:?}"
    );
    let (lo, hi) = world_aabb_transformed(&mesh);
    assert_eq!(
        mesh.build_volume,
        BoundingBox3 { min: lo, max: hi },
        "an extreme-coordinate placement must still refresh build_volume"
    );
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
        vec![p3(0.0, 0.0, 0.0), p3(400.0, 0.0, 0.0), p3(0.0, 300.0, 10.0)],
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
        vec![p3(0.0, 0.0, 0.0), p3(20.0, 0.0, 0.0), p3(0.0, 20.0, 5.0)],
    )]);
    place_bare_mesh_on_bed(&mut mesh, (125.0, 125.0)).expect("placeable");

    assert_eq!(bed_overflow_mm(&mesh, &square), None);
}
