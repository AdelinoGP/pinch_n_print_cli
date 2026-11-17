//! Charter `H-MESH-QUERY`: transforms and AABB-tree queries against dimensions.
//!
//! `slicer_core::transform_point3` is compared against an independent
//! coordinate sign/swap plus translation, and `AabbTree::{bounds,
//! raycast_first_hit, raycast_all_hits}` against geometry derived from the box's
//! own dimensions. Nothing here re-uses a matrix product or a second tree query
//! as its own oracle.
//!
//! Two ownership notes belong in this charter rather than in the code:
//!
//! * The tree is a mesh-space library contract. No non-test `AabbTree::new`
//!   caller was found in the inspected tree, so this is not a pipeline-wiring
//!   claim.
//! * `RayHit::distance` is the **parametric `t`** along the supplied direction;
//!   the tree never normalizes. Every ray here is unit length, which is what
//!   makes a millimetre claim meaningful. No arbitrary rotation and no
//!   direction-normalization promise is asserted.
//!
//! Ray probes sit at `(x0+1, y0+2)`, which is provably off both face diagonals:
//! `x - x0 == y - y0` needs `1 == 2`, and the other diagonal needs
//! `(w-1)(d-2) == 2`, which has no integer solution for `w, d >= 4`.

use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_core::aabb_tree::AabbTree;
use slicer_core::transform_point3;
use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3};
use slicer_sdk::test_support::property_testing::domains::{
    analytical_meshes, rigid_transforms, MeshCase, RigidTransformCase, SolidKind,
};
use slicer_sdk::test_support::property_testing::oracles::{
    mesh_signed_volume_mm3, validate_indexed_mesh,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{at_least, in_range, read, to_f32_exact, to_i64, to_usize, BoundsError};
use super::envelopes::f32_abs_error_envelope;
use super::mesh_slice::Solid;

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-MESH-QUERY";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_mesh_transform_and_aabb";

/// Smallest box dimension that admits a face-interior probe clear of both
/// diagonals and of the `1e-5` dedup band.
pub const MIN_RAY_EXTENT_MM: u64 = 4;

/// Rounded operations in one Möller–Trumbore evaluation that feeds `distance`:
/// three edge subtractions, the determinant, two barycentric parameters, and the
/// final `t` are at most sixteen rounded operations on magnitudes bounded by the
/// probe's millimetre extent.
pub const RAY_ROUNDED_OPS: u32 = 16;

/// Rounded operations from an exact f64 transform to the f32 result: the final
/// narrowing only, because the matrix entries are `0/+-1`, the inputs are exact
/// f32 values widened to f64, and dyadic translations are exact in f64.
pub const TRANSFORM_ROUNDED_OPS: u32 = 1;

/// Raw `H-MESH-QUERY` profile keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawQueryBounds {
    /// `max_extent_mm`.
    pub max_extent_mm: u64,
    /// `max_translation_mm`.
    pub max_translation_mm: u64,
    /// `max_queries`.
    pub max_queries: u64,
}

impl RawQueryBounds {
    /// Read every `H-MESH-QUERY` key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_extent_mm: read(run, "max_extent_mm")?,
            max_translation_mm: read(run, "max_translation_mm")?,
            max_queries: read(run, "max_queries")?,
        })
    }
}

/// Validated `H-MESH-QUERY` limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryLimits {
    /// Largest box dimension, whole millimetres.
    pub max_extent_mm: i64,
    /// Same bound as `f32`, for the domain constructors.
    pub max_extent_f32: f32,
    /// Largest translation, whole millimetres.
    pub max_translation_mm: i64,
    /// Same bound as `f32`.
    pub max_translation_f32: f32,
    /// Rays and probe points per case.
    pub max_queries: usize,
}

/// Reject zero, overflowing and out-of-range query bounds before sampling.
pub fn checked_limits(raw: RawQueryBounds) -> Result<QueryLimits, BoundsError> {
    in_range("max_extent_mm", raw.max_extent_mm, 1..=64)?;
    at_least(
        "max_extent_mm",
        raw.max_extent_mm,
        "face-interior ray probe",
        MIN_RAY_EXTENT_MM,
    )?;
    in_range("max_translation_mm", raw.max_translation_mm, 0..=64)?;
    in_range("max_queries", raw.max_queries, 1..=256)?;
    at_least("max_queries", raw.max_queries, "ray and probe sweep", 4)?;
    Ok(QueryLimits {
        max_extent_mm: to_i64("max_extent_mm", raw.max_extent_mm)?,
        max_extent_f32: to_f32_exact("max_extent_mm", raw.max_extent_mm)?,
        max_translation_mm: to_i64("max_translation_mm", raw.max_translation_mm)?,
        max_translation_f32: to_f32_exact("max_translation_mm", raw.max_translation_mm)?,
        max_queries: to_usize("max_queries", raw.max_queries)?,
    })
}

/// Counts proving the checked population was not empty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueryStats {
    /// Rays that were required to hit.
    pub hit_rays: usize,
    /// Rays that were required to miss.
    pub miss_rays: usize,
    /// Transform probes compared.
    pub transform_points: usize,
}

fn to_mm(units: i64) -> f32 {
    units as f32 / 10_000.0
}

/// Whole millimetres to canonical units, exactly.
fn units(mm: i64) -> i64 {
    mm * 10_000
}

/// Whole millimetres as `f32`; every value here is exactly representable.
fn mm_f32(mm: i64) -> f32 {
    mm as f32
}

/// The independent oracle for one quarter-turn plus translation.
///
/// A quarter-turn around Z is a coordinate sign/swap, evaluated here without
/// any matrix arithmetic, so the SUT's column-major product is never its own
/// reference.
#[must_use]
pub fn expected_transform(case: &RigidTransformCase, point: Point3) -> [f64; 3] {
    let (x, y, z) = (f64::from(point.x), f64::from(point.y), f64::from(point.z));
    let (rx, ry) = match case.quarter_turns_z % 4 {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    };
    [
        rx + case.translation_mm[0],
        ry + case.translation_mm[1],
        z + case.translation_mm[2],
    ]
}

/// Probe points on an eighth-millimetre grid, every coordinate exactly f32.
#[must_use]
pub fn probe_points(extent_mm: f32, queries: usize) -> Vec<Point3> {
    let eighths = (extent_mm * 8.0).round() as i32;
    let step = eighths.max(1);
    let eighth = 1.0_f32 / 8.0;
    let mut points = Vec::new();
    for (index, (ix, iy, iz)) in [
        (0, 0, 0),
        (step, 0, 0),
        (0, step, 0),
        (0, 0, step),
        (step, step, step),
        (-step, step, 0),
        (step, 0, -step),
        (0, -step, step),
    ]
    .into_iter()
    .enumerate()
    {
        if index >= queries {
            break;
        }
        points.push(Point3 {
            x: ix as f32 * eighth,
            y: iy as f32 * eighth,
            z: iz as f32 * eighth,
        });
    }
    points
}

/// Compare the SUT transform against the independent sign/swap oracle.
pub fn check_transform(
    case: &RigidTransformCase,
    points: &[Point3],
    stats: &mut QueryStats,
) -> Result<(), String> {
    if case.quarter_turns_z % 4 != case.quarter_turns_z {
        return Err(format!(
            "model precondition: quarter_turns_z {} is not in 0..4",
            case.quarter_turns_z
        ));
    }
    for point in points {
        let exact = expected_transform(case, *point);
        let actual = transform_point3(&case.transform.matrix, *point);
        let magnitude = exact
            .iter()
            .fold(1.0_f64, |acc, value| acc.max(value.abs()));
        let envelope = f32_abs_error_envelope(magnitude, TRANSFORM_ROUNDED_OPS)
            .map_err(|error| format!("transform envelope: {error}"))?;
        let actual = [
            f64::from(actual.x),
            f64::from(actual.y),
            f64::from(actual.z),
        ];
        for axis in 0..3 {
            if (actual[axis] - exact[axis]).abs() > envelope {
                return Err(format!(
                    "transform of {point:?} axis {axis}: got {}, oracle {} (envelope {envelope})",
                    actual[axis], exact[axis]
                ));
            }
        }
        stats.transform_points += 1;
    }
    Ok(())
}

/// The zero-matrix identity guard and a true identity matrix, both explicit.
pub fn transform_identity_witnesses() {
    let point = Point3 {
        x: 1.5,
        y: -2.25,
        z: 3.125,
    };
    let zero = transform_point3(&[0.0; 16], point);
    assert_eq!(
        zero, point,
        "H-MESH-QUERY: the all-zero matrix is documented as identity, not a collapse"
    );
    let mut identity = [0.0_f64; 16];
    for index in [0, 5, 10, 15] {
        identity[index] = 1.0;
    }
    let same = transform_point3(&identity, point);
    assert_eq!(
        same, point,
        "H-MESH-QUERY: the identity matrix must return the point unchanged"
    );
    // A translation-only matrix moves the point by exactly the translation.
    let mut translate = identity;
    translate[12] = 2.0;
    translate[13] = -3.0;
    translate[14] = 0.5;
    let moved = transform_point3(&translate, point);
    let envelope =
        f32_abs_error_envelope(8.0, TRANSFORM_ROUNDED_OPS).expect("translation envelope");
    for (got, want) in [
        (f64::from(moved.x), f64::from(point.x) + 2.0),
        (f64::from(moved.y), f64::from(point.y) - 3.0),
        (f64::from(moved.z), f64::from(point.z) + 0.5),
    ] {
        assert!(
            (got - want).abs() <= envelope,
            "H-MESH-QUERY: translation got {got}, expected {want} (envelope {envelope})"
        );
    }
}

/// The dimension-derived bounds of a box solid.
#[must_use]
pub fn expected_bounds(solid: &Solid) -> BoundingBox3 {
    BoundingBox3 {
        min: Point3 {
            x: to_mm(units(solid.x0)),
            y: to_mm(units(solid.y0)),
            z: to_mm(units(solid.z0)),
        },
        max: Point3 {
            x: to_mm(units(solid.x0 + solid.w)),
            y: to_mm(units(solid.y0 + solid.d)),
            z: to_mm(units(solid.z0 + solid.h)),
        },
    }
}

fn check_hits(
    label: &str,
    hits: &[slicer_core::aabb_tree::RayHit],
    want: &[(f64, Point3)],
    envelope: f64,
) -> Result<(), String> {
    if hits.len() != want.len() {
        return Err(format!(
            "{label}: expected {} hit(s), got {} ({hits:?})",
            want.len(),
            hits.len()
        ));
    }
    for (hit, (distance, point)) in hits.iter().zip(want.iter()) {
        if (f64::from(hit.distance) - distance).abs() > envelope {
            return Err(format!(
                "{label}: hit distance {} != analytic {distance} (envelope {envelope})",
                hit.distance
            ));
        }
        for (got, expected, axis) in [
            (hit.point.x, point.x, "x"),
            (hit.point.y, point.y, "y"),
            (hit.point.z, point.z, "z"),
        ] {
            let expected = f64::from(expected);
            if (f64::from(got) - expected).abs() > envelope {
                return Err(format!(
                    "{label}: hit point {axis} {got} != analytic {expected} (envelope {envelope})"
                ));
            }
        }
    }
    Ok(())
}

/// Compare the tree's bounds and rays against the box's dimensions.
pub fn check_rays(
    solid: &Solid,
    tree: &AabbTree,
    queries: usize,
    stats: &mut QueryStats,
) -> Result<(), String> {
    if !solid.box_solid || solid.w < MIN_RAY_EXTENT_MM as i64 || solid.d < MIN_RAY_EXTENT_MM as i64
    {
        return Err(format!(
            "model precondition: ray box {solid:?} must be a box of at least {MIN_RAY_EXTENT_MM} mm per side"
        ));
    }
    let want_bounds = expected_bounds(solid);
    match tree.bounds() {
        Some(bounds) if bounds == want_bounds => {}
        other => {
            return Err(format!(
                "bounds: got {other:?}, dimension-derived {want_bounds:?}"
            ))
        }
    }
    let envelope = f32_abs_error_envelope(
        (solid.h.max(solid.w).max(solid.d) as f64) + 2.0,
        RAY_ROUNDED_OPS,
    )
    .map_err(|error| format!("ray envelope: {error}"))?;
    let probe = |x: i64, y: i64, z: i64| Point3 {
        x: mm_f32(x),
        y: mm_f32(y),
        z: mm_f32(z),
    };
    let (px, py) = (solid.x0 + 1, solid.y0 + 2);
    let (z0, z1) = (solid.z0, solid.z0 + solid.h);
    let mut rays = 0_usize;

    let up_origin = probe(px, py, z0 - 1);
    let up = tree.raycast_all_hits(
        up_origin,
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    );
    check_hits(
        "+Z through the bottom face",
        &up,
        &[
            (1.0, probe(px, py, z0)),
            (1.0 + solid.h as f64, probe(px, py, z1)),
        ],
        envelope,
    )?;
    let first = tree
        .raycast_first_hit(
            up_origin,
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        )
        .ok_or_else(|| String::from("first hit: +Z ray reported no hit"))?;
    if (f64::from(first.distance) - 1.0).abs() > envelope {
        return Err(format!(
            "first hit: distance {} != the analytic entry 1.0",
            first.distance
        ));
    }
    stats.hit_rays += 1;
    rays += 1;

    if queries >= 2 {
        let down = tree.raycast_all_hits(
            probe(px, py, z1 + 1),
            Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        );
        check_hits(
            "-Z through the top face",
            &down,
            &[
                (1.0, probe(px, py, z1)),
                (1.0 + solid.h as f64, probe(px, py, z0)),
            ],
            envelope,
        )?;
        stats.hit_rays += 1;
        rays += 1;
    }

    if queries >= 3 {
        let miss = tree.raycast_all_hits(
            probe(solid.x0 + solid.w + 1, solid.y0 + 1, z0 - 1),
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        );
        if !miss.is_empty() {
            return Err(format!("miss ray: expected no hits, got {miss:?}"));
        }
        if tree
            .raycast_first_hit(
                probe(solid.x0 + solid.w + 1, solid.y0 + 1, z0 - 1),
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
            )
            .is_some()
        {
            return Err(String::from("miss ray: first hit fabricated a hit"));
        }
        stats.miss_rays += 1;
        rays += 1;
    }

    if rays > queries {
        return Err(format!("{rays} rays exceeded the query budget {queries}"));
    }
    Ok(())
}

/// The empty tree is a distinct documented contract.
pub fn empty_tree_witnesses() {
    let tree = AabbTree::new(IndexedTriangleSet::default());
    assert!(
        tree.bounds().is_none(),
        "H-MESH-QUERY: an empty tree has no bounds"
    );
    assert!(
        tree.raycast_all_hits(
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0
            }
        )
        .is_empty(),
        "H-MESH-QUERY: an empty tree returns no hits"
    );
    assert!(
        tree.raycast_first_hit(
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0
            }
        )
        .is_none(),
        "H-MESH-QUERY: an empty tree returns no first hit"
    );
}

/// One generated query scene.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryScene {
    /// An analytical box from the #2 `analytical_meshes` catalog.
    Analytical(Box<MeshCase>),
    /// A locally constructed ray-friendly box.
    Rays(Box<Solid>),
    /// An explicit quarter-turn plus translation.
    Transform(Box<RigidTransformCase>),
}

fn scene_strategy(limits: QueryLimits) -> impl Strategy<Value = QueryScene> {
    let analytical = analytical_meshes(SolidKind::Box, limits.max_extent_f32)
        .expect("H-MESH-QUERY analytical mesh domain")
        .prop_map(|case| QueryScene::Analytical(Box::new(case)))
        .boxed();
    let rays = (
        MIN_RAY_EXTENT_MM as i64..=limits.max_extent_mm,
        MIN_RAY_EXTENT_MM as i64..=limits.max_extent_mm,
        1_i64..=limits.max_extent_mm,
        -limits.max_translation_mm..=limits.max_translation_mm,
        -limits.max_translation_mm..=limits.max_translation_mm,
        -limits.max_translation_mm..=limits.max_translation_mm,
    )
        .prop_map(|(w, d, h, x0, y0, z0)| {
            QueryScene::Rays(Box::new(Solid {
                box_solid: true,
                x0,
                y0,
                z0,
                w,
                d,
                h,
            }))
        })
        .boxed();
    let transforms = rigid_transforms(limits.max_translation_f32)
        .expect("H-MESH-QUERY rigid transform domain")
        .prop_map(|case| QueryScene::Transform(Box::new(case)))
        .boxed();
    prop_oneof![analytical, rays, transforms]
}

fn check_analytical(case: &MeshCase) -> Result<(), String> {
    validate_indexed_mesh(&case.mesh, true)
        .map_err(|error| format!("analytical mesh precondition: {error}"))?;
    let tree = AabbTree::new(case.mesh.clone());
    match tree.bounds() {
        Some(bounds) if bounds == case.expected_bounds => {}
        other => {
            return Err(format!(
                "analytical bounds: got {other:?}, expected {:?}",
                case.expected_bounds
            ))
        }
    }
    let volume =
        mesh_signed_volume_mm3(&case.mesh).map_err(|error| format!("signed volume: {error}"))?;
    let mut magnitude = 0.0_f64;
    for point in [case.expected_bounds.min, case.expected_bounds.max] {
        for value in [point.x, point.y, point.z] {
            magnitude = magnitude.max(f64::from(value.abs()));
        }
    }
    let envelope = 128.0 * f64::EPSILON * magnitude.powi(3);
    if (volume - case.expected_volume_mm3).abs() > envelope {
        return Err(format!(
            "analytical volume {volume} != dimension-derived {} (envelope {envelope})",
            case.expected_volume_mm3
        ));
    }
    Ok(())
}

/// Check one generated scene end to end.
pub fn check_scene(scene: &QueryScene, limits: QueryLimits) -> TestCaseResult {
    let mut stats = QueryStats::default();
    let outcome = match scene {
        QueryScene::Analytical(case) => check_analytical(case),
        QueryScene::Rays(solid) => {
            let tree = AabbTree::new(solid.mesh());
            check_rays(solid, &tree, limits.max_queries, &mut stats)
        }
        QueryScene::Transform(case) => {
            let points = probe_points(limits.max_extent_f32, limits.max_queries);
            check_transform(case, &points, &mut stats)
        }
    };
    outcome.map_err(TestCaseError::fail)
}

/// Deterministic witnesses: identity guards, the empty tree, named rays and an
/// explicit quarter-turn, all independent of the RNG.
pub fn deterministic_witnesses(limits: QueryLimits) {
    transform_identity_witnesses();
    empty_tree_witnesses();

    let solid = Solid {
        box_solid: true,
        x0: -1,
        y0: 2,
        z0: -3,
        w: 4,
        d: 5,
        h: 2,
    };
    let tree = AabbTree::new(solid.mesh());
    let mut stats = QueryStats::default();
    check_rays(&solid, &tree, limits.max_queries, &mut stats)
        .unwrap_or_else(|error| panic!("H-MESH-QUERY ray witness failed: {error}"));
    assert!(
        stats.hit_rays >= 2 && stats.miss_rays >= 1,
        "H-MESH-QUERY: ray witnesses must exercise hits and a miss, got {stats:?}"
    );

    // An explicit quarter-turn: +X maps to +Y for a single turn about Z.
    let mut matrix = [0.0_f64; 16];
    matrix[0] = 0.0;
    matrix[4] = -1.0;
    matrix[1] = 1.0;
    matrix[5] = 0.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    let case = RigidTransformCase {
        transform: slicer_ir::Transform3d { matrix },
        translation_mm: [0.0, 0.0, 0.0],
        quarter_turns_z: 1,
    };
    let point = Point3 {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    };
    let mut stats = QueryStats::default();
    check_transform(&case, &[point], &mut stats)
        .unwrap_or_else(|error| panic!("H-MESH-QUERY quarter-turn witness failed: {error}"));
    assert_eq!(
        stats.transform_points, 1,
        "H-MESH-QUERY: the quarter-turn witness must compare exactly one point"
    );
}

/// Run the registry-selected `H-MESH-QUERY` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-MESH-QUERY registry context");
    let raw = RawQueryBounds::read(&run).expect("H-MESH-QUERY bound keys");
    let limits = checked_limits(raw).expect("H-MESH-QUERY checked profile bounds");
    deterministic_witnesses(limits);
    let strategy = scene_strategy(limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, limits))
        .expect("H-MESH-QUERY transform and AABB contracts");
}
