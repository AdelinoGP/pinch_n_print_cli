//! Shared mesh-property observations and hand-worked structural fixtures.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3};
use slicer_sdk::test_support::property_testing::domains::{
    MeshCase, RigidTransformCase, SolidKind,
};
use slicer_sdk::test_support::property_testing::oracles::{
    mesh_signed_volume_mm3, validate_indexed_mesh,
};

fn p(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

pub fn worked_box() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            p(0.0, 0.0, 0.0),
            p(2.0, 0.0, 0.0),
            p(2.0, 3.0, 0.0),
            p(0.0, 3.0, 0.0),
            p(0.0, 0.0, 4.0),
            p(2.0, 0.0, 4.0),
            p(2.0, 3.0, 4.0),
            p(0.0, 3.0, 4.0),
        ],
        // Use the other diagonal on each quadrilateral from the generator.
        indices: vec![
            0, 3, 1, 1, 3, 2, 4, 5, 7, 5, 6, 7, 0, 1, 4, 1, 5, 4, 1, 2, 5, 2, 6, 5, 2, 3, 6, 3, 7,
            6, 3, 0, 7, 0, 4, 7,
        ],
    }
}

pub fn worked_prism() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            p(0.0, 0.0, 0.0),
            p(3.0, 0.0, 0.0),
            p(0.0, 4.0, 0.0),
            p(0.0, 0.0, 5.0),
            p(3.0, 0.0, 5.0),
            p(0.0, 4.0, 5.0),
        ],
        indices: vec![
            0, 2, 1, 3, 4, 5, 0, 1, 3, 1, 4, 3, 1, 2, 4, 2, 5, 4, 2, 0, 5, 0, 3, 5,
        ],
    }
}

pub fn vertex_bounds(mesh: &IndexedTriangleSet) -> BoundingBox3 {
    let first = *mesh.vertices.first().expect("nonempty mesh fixture");
    mesh.vertices.iter().fold(
        BoundingBox3 {
            min: first,
            max: first,
        },
        |mut bounds, p| {
            bounds.min.x = bounds.min.x.min(p.x);
            bounds.min.y = bounds.min.y.min(p.y);
            bounds.min.z = bounds.min.z.min(p.z);
            bounds.max.x = bounds.max.x.max(p.x);
            bounds.max.y = bounds.max.y.max(p.y);
            bounds.max.z = bounds.max.z.max(p.z);
            bounds
        },
    )
}

fn volume_budget(mesh: &IndexedTriangleSet) -> f64 {
    let magnitude = mesh.vertices.iter().fold(0.0_f64, |m, p| {
        m.max(f64::from(p.x).abs())
            .max(f64::from(p.y).abs())
            .max(f64::from(p.z).abs())
    });
    // The analytical_meshes documentation derives this bound from the 72
    // monomials, their rounded operations, f64 summation and division. For the
    // transformed cases here, both grids are eighth-mm: vertex indices stay
    // below 160,016, still exactly representable in f32 and f64, so the same
    // error bound applies and the original dimensional volume stays exact.
    128.0 * f64::EPSILON * magnitude.powi(3)
}

pub fn check_mesh(case: &MeshCase, kind: SolidKind, max_extent: f32) -> Result<(), TestCaseError> {
    let (vertices, triangles) = match kind {
        SolidKind::Box => (8, 12),
        SolidKind::TriangularPrism => (6, 8),
    };
    prop_assert_eq!(case.mesh.vertices.len(), vertices);
    prop_assert_eq!(case.mesh.indices.len(), 3 * triangles);
    prop_assert_eq!(validate_indexed_mesh(&case.mesh, true), Ok(()));
    let bounds = vertex_bounds(&case.mesh);
    prop_assert_eq!(bounds, case.expected_bounds);
    let [dx, dy, dz] = [
        f64::from(bounds.max.x) - f64::from(bounds.min.x),
        f64::from(bounds.max.y) - f64::from(bounds.min.y),
        f64::from(bounds.max.z) - f64::from(bounds.min.z),
    ];
    prop_assert!([dx, dy, dz]
        .into_iter()
        .all(|d| d > 0.0 && d <= f64::from(max_extent)));
    if max_extent >= 0.125 {
        prop_assert!([dx, dy, dz].into_iter().all(|d| (d * 8.0).fract() == 0.0));
    } else {
        // The documented small-bound grid is the greatest power of two <= bound.
        prop_assert!(
            [dx, dy, dz].into_iter().all(|d| {
                d.to_bits() & 0x000f_ffff_ffff_ffff == 0 && 2.0 * d > f64::from(max_extent)
            }),
            "dimensions must use the small-bound dyadic grid"
        );
    }
    let dimensional = match kind {
        SolidKind::Box => dx * dy * dz,
        SolidKind::TriangularPrism => dx * dy * 0.5 * dz,
    };
    prop_assert_eq!(case.expected_volume_mm3, dimensional);
    let volume = mesh_signed_volume_mm3(&case.mesh)
        .map_err(|error| TestCaseError::fail(error.to_string()))?;
    prop_assert!(volume > 0.0);
    prop_assert!(
        (volume - dimensional).abs() <= volume_budget(&case.mesh),
        "volume {volume} vs dimensional {dimensional}; bounds {bounds:?}"
    );
    Ok(())
}

// Independent observation: no matrix element participates in this sign/swap.
fn sign_swap(point: [f64; 3], case: &RigidTransformCase) -> [f64; 3] {
    let [x, y, z] = point;
    let [x, y] = match case.quarter_turns_z {
        0 => [x, y],
        1 => [-y, x],
        2 => [-x, -y],
        3 => [y, -x],
        _ => panic!("quarter turn outside the contract"),
    };
    let [tx, ty, tz] = case.translation_mm;
    [x + tx, y + ty, z + tz]
}

fn matrix_point(point: [f64; 3], case: &RigidTransformCase) -> [f64; 3] {
    let [x, y, z] = point;
    let m = case.transform.matrix;
    [
        m[0] * x + m[4] * y + m[8] * z + m[12],
        m[1] * x + m[5] * y + m[9] * z + m[13],
        m[2] * x + m[6] * y + m[10] * z + m[14],
    ]
}

pub fn check_transform(case: &RigidTransformCase, bound: f32) -> Result<(), TestCaseError> {
    prop_assert!(case.quarter_turns_z <= 3);
    prop_assert!(case
        .translation_mm
        .iter()
        .all(|t| t.is_finite() && t.abs() <= f64::from(bound)));
    if bound >= 0.125 {
        prop_assert!(case.translation_mm.iter().all(|t| (t * 8.0).fract() == 0.0));
    } else if bound > 0.0 {
        prop_assert!(
            case.translation_mm.iter().all(|t| {
                let magnitude = t.abs();
                magnitude == 0.0
                    || (magnitude.to_bits() & 0x000f_ffff_ffff_ffff == 0
                        && 2.0 * magnitude > f64::from(bound))
            }),
            "translations must use the small-bound dyadic grid"
        );
    }
    let rotations = [
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        [
            0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        [
            -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        [
            0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    ];
    let mut expected = rotations[usize::from(case.quarter_turns_z)];
    expected[12..15].copy_from_slice(&case.translation_mm);
    prop_assert_eq!(case.transform.matrix, expected);
    // Use the bound's scale for very small grids. With at most 24 significant
    // f32 bits plus these small multipliers, sign/swaps and additions are
    // exactly representable in f64, including subnormal f32 bounds.
    let scale = if bound > 0.0 && bound < 0.125 {
        f64::from(bound)
    } else {
        0.125
    };
    let point = [10.0 * scale, -20.0 * scale, 30.0 * scale];
    prop_assert_eq!(matrix_point(point, case), sign_swap(point, case));
    Ok(())
}

pub fn check_transformed_mesh(
    case: &MeshCase,
    kind: SolidKind,
    transform: &RigidTransformCase,
    max_extent: f32,
) -> Result<(), TestCaseError> {
    let mut transformed = case.mesh.clone();
    for vertex in &mut transformed.vertices {
        let point = [
            f64::from(vertex.x),
            f64::from(vertex.y),
            f64::from(vertex.z),
        ];
        let actual = matrix_point(point, transform);
        prop_assert_eq!(actual, sign_swap(point, transform));
        *vertex = p(actual[0] as f32, actual[1] as f32, actual[2] as f32);
    }
    let bounds = case.expected_bounds;
    let min = [
        f64::from(bounds.min.x),
        f64::from(bounds.min.y),
        f64::from(bounds.min.z),
    ];
    let max = [
        f64::from(bounds.max.x),
        f64::from(bounds.max.y),
        f64::from(bounds.max.z),
    ];
    let [tx, ty, tz] = transform.translation_mm;
    let (lo, hi) = match transform.quarter_turns_z {
        0 => ([min[0], min[1]], [max[0], max[1]]),
        1 => ([-max[1], min[0]], [-min[1], max[0]]),
        2 => ([-max[0], -max[1]], [-min[0], -min[1]]),
        3 => ([min[1], -max[0]], [max[1], -min[0]]),
        _ => return Err(TestCaseError::fail("invalid quarter turn")),
    };
    let expected_bounds = BoundingBox3 {
        min: p(
            (lo[0] + tx) as f32,
            (lo[1] + ty) as f32,
            (min[2] + tz) as f32,
        ),
        max: p(
            (hi[0] + tx) as f32,
            (hi[1] + ty) as f32,
            (max[2] + tz) as f32,
        ),
    };
    check_mesh(
        &MeshCase {
            mesh: transformed,
            expected_bounds,
            expected_volume_mm3: case.expected_volume_mm3,
        },
        kind,
        max_extent,
    )
}
