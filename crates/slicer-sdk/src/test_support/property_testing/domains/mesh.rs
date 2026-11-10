//! Construction-first dyadic mesh and quarter-turn parameters.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3, Transform3d};

use super::{DomainError, InvalidMeshKind, MeshCase, RigidTransformCase, SolidKind};

// Both the grid indices and their products fit the exactness proof in mod.rs.
const MAX_MM: f32 = 10_000.0;
const ORIGIN_GRID_LIMIT: i32 = 16;

// For small positive bounds, choose the largest power of two at or below the
// bound, including subnormals. For larger bounds, use eighth-millimeter units.
fn grid(bound: f32) -> (f32, i32) {
    if bound >= 0.125 {
        (0.125, (bound * 8.0).floor() as i32)
    } else {
        let bits = bound.to_bits();
        let exponent = bits & 0x7f80_0000;
        let power_bits = if exponent == 0 {
            1_u32 << (31 - bits.leading_zeros())
        } else {
            exponent
        };
        (f32::from_bits(power_bits), 1)
    }
}

fn point(x: i32, y: i32, z: i32, unit: f32) -> Point3 {
    Point3 {
        x: x as f32 * unit,
        y: y as f32 * unit,
        z: z as f32 * unit,
    }
}

fn solid(kind: SolidKind, dims: (i32, i32, i32), origin: (i32, i32, i32), q: f32) -> MeshCase {
    let (a, b, c) = dims;
    let (x, y, z) = origin;
    let at = |dx, dy, dz| point(x + dx, y + dy, z + dz, q);
    let (vertices, indices) = match kind {
        SolidKind::Box => (
            vec![
                at(0, 0, 0),
                at(a, 0, 0),
                at(a, b, 0),
                at(0, b, 0),
                at(0, 0, c),
                at(a, 0, c),
                at(a, b, c),
                at(0, b, c),
            ],
            vec![
                0, 2, 1, 0, 3, 2, // bottom
                4, 5, 6, 4, 6, 7, // top
                0, 1, 5, 0, 5, 4, // front
                1, 2, 6, 1, 6, 5, // right
                2, 3, 7, 2, 7, 6, // back
                3, 0, 4, 3, 4, 7, // left
            ],
        ),
        SolidKind::TriangularPrism => (
            vec![
                at(0, 0, 0),
                at(a, 0, 0),
                at(0, b, 0),
                at(0, 0, c),
                at(a, 0, c),
                at(0, b, c),
            ],
            vec![
                0, 2, 1, // bottom
                3, 4, 5, // top
                0, 1, 4, 0, 4, 3, // front
                1, 2, 5, 1, 5, 4, // diagonal
                2, 0, 3, 2, 3, 5, // left
            ],
        ),
    };
    let (a, b, c) = (
        f64::from(a) * f64::from(q),
        f64::from(b) * f64::from(q),
        f64::from(c) * f64::from(q),
    );
    let expected_volume_mm3 = match kind {
        SolidKind::Box => a * b * c,
        SolidKind::TriangularPrism => a * b * 0.5 * c,
    };
    MeshCase {
        mesh: IndexedTriangleSet { vertices, indices },
        expected_bounds: BoundingBox3 {
            min: at(0, 0, 0),
            max: at(dims.0, dims.1, dims.2),
        },
        expected_volume_mm3,
    }
}

pub(super) fn analytical_meshes(
    kind: SolidKind,
    max_extent_mm: f32,
) -> Result<BoxedStrategy<MeshCase>, DomainError> {
    if !max_extent_mm.is_finite() || max_extent_mm <= 0.0 || max_extent_mm > MAX_MM {
        return Err(DomainError::InvalidBounds(
            "mesh extent must be finite and in (0, 10000] mm".into(),
        ));
    }
    let (q, cap) = grid(max_extent_mm);
    Ok((
        1..=cap,
        1..=cap,
        1..=cap,
        -ORIGIN_GRID_LIMIT..=ORIGIN_GRID_LIMIT,
        -ORIGIN_GRID_LIMIT..=ORIGIN_GRID_LIMIT,
        -ORIGIN_GRID_LIMIT..=ORIGIN_GRID_LIMIT,
    )
        .prop_map(move |(a, b, c, x, y, z)| solid(kind, (a, b, c), (x, y, z), q))
        .boxed())
}

pub(super) fn rigid_transforms(
    max_translation_mm: f32,
) -> Result<BoxedStrategy<RigidTransformCase>, DomainError> {
    if !max_translation_mm.is_finite() || !(0.0..=MAX_MM).contains(&max_translation_mm) {
        return Err(DomainError::InvalidBounds(
            "translation must be finite and in [0, 10000] mm".into(),
        ));
    }
    let (q, cap) = if max_translation_mm == 0.0 {
        (0.125, 0)
    } else {
        grid(max_translation_mm)
    };
    Ok((0_u8..=3, -cap..=cap, -cap..=cap, -cap..=cap)
        .prop_map(move |(quarter_turns_z, x, y, z)| {
            let (c, s) = match quarter_turns_z {
                0 => (1.0, 0.0),
                1 => (0.0, 1.0),
                2 => (-1.0, 0.0),
                _ => (0.0, -1.0),
            };
            let translation_mm = [x, y, z].map(|n| f64::from(n) * f64::from(q));
            let [tx, ty, tz] = translation_mm;
            RigidTransformCase {
                transform: Transform3d {
                    matrix: [
                        c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0,
                    ],
                },
                translation_mm,
                quarter_turns_z,
            }
        })
        .boxed())
}

pub(super) fn invalid_meshes(kind: InvalidMeshKind) -> BoxedStrategy<IndexedTriangleSet> {
    (1_i16..=1024)
        .prop_map(move |n| {
            let x = f32::from(n);
            let mut vertices = vec![
                Point3 { x, y: 0.0, z: 0.0 },
                Point3 {
                    x: x + 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 { x, y: 1.0, z: 0.0 },
            ];
            let indices = match kind {
                InvalidMeshKind::BadIndex => vec![0, 1, 3],
                InvalidMeshKind::TruncatedTriangle => vec![0, 1, 2, 0],
                InvalidMeshKind::NonFiniteVertex => {
                    vertices[0].x = f32::NAN;
                    vec![0, 1, 2]
                }
                InvalidMeshKind::DegenerateTriangle => {
                    vertices[2].x = x + 2.0;
                    vertices[2].y = 0.0;
                    vec![0, 1, 2]
                }
            };
            IndexedTriangleSet { vertices, indices }
        })
        .boxed()
}
