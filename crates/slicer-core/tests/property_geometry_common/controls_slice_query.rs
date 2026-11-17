//! Deterministic controls for `H-MESH-SLICE` and `H-MESH-QUERY`.
//!
//! Every arm runs the real SUT on a named fixture (which must pass the shared
//! checker) and then feeds a deliberately wrong result through **the same
//! checker the property uses** (which must fail for the named reason). No
//! synthetic mutation implies a prior SUT failure.

use slicer_core::aabb_tree::AabbTree;
use slicer_core::slice_mesh_ex;
use slicer_ir::{IndexedTriangleSet, Point3};
use slicer_sdk::test_support::property_testing::domains::RigidTransformCase;

use super::mesh_query::{
    check_rays, check_transform, checked_limits as checked_query_limits, probe_points, QueryStats,
    RawQueryBounds,
};
use super::mesh_slice::{
    check_slice, checked_limits as checked_slice_limits, RawSliceBounds, Solid, LATTICE_DIVISIONS,
};

fn slice_limits() -> super::mesh_slice::SliceLimits {
    checked_slice_limits(RawSliceBounds {
        max_extent_mm: 16,
        max_translation_mm: 8,
        max_planes: 8,
    })
    .expect("control slice profile")
}

fn query_limits() -> super::mesh_query::QueryLimits {
    checked_query_limits(RawQueryBounds {
        max_extent_mm: 16,
        max_translation_mm: 8,
        max_queries: 8,
    })
    .expect("control query profile")
}

fn must_reject<T: std::fmt::Debug>(arm: &str, result: Result<T, String>, needle: &str) {
    match result {
        Ok(stats) => panic!("{arm}: the checker accepted a wrong result ({stats:?})"),
        Err(message) => assert!(
            message.contains(needle),
            "{arm}: rejected for an unexpected reason (wanted {needle:?}): {message}"
        ),
    }
}

fn witness_solid() -> Solid {
    Solid {
        box_solid: true,
        x0: -1,
        y0: 2,
        z0: -3,
        w: 6,
        d: 8,
        h: 3,
    }
}

/// Arm 1: the cube bottom treated as a filled square.
fn bottom_filled() {
    const ARM: &str = "arm 1 (bottom treated as filled)";
    let solid = witness_solid();
    let emitted = slice_mesh_ex(&solid.mesh(), &[solid.z0 as f32]);
    check_slice(&solid, &[solid.z0], &emitted, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    // The mutant: the bottom layer carries the interior section.
    let wrong = vec![solid.section()];
    must_reject(
        ARM,
        check_slice(&solid, &[solid.z0], &wrong, LATTICE_DIVISIONS),
        "must be empty",
    );
}

/// Arm 2: the cube top treated as empty.
fn top_empty() {
    const ARM: &str = "arm 2 (top treated as empty)";
    let solid = witness_solid();
    let top = solid.z0 + solid.h;
    let emitted = slice_mesh_ex(&solid.mesh(), &[top as f32]);
    check_slice(&solid, &[top], &emitted, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    must_reject(
        ARM,
        check_slice(&solid, &[top], &[Vec::new()], LATTICE_DIVISIONS),
        "must be nonempty",
    );
}

/// Arm 3: an omitted interior cross-section.
fn omitted_section() {
    const ARM: &str = "arm 3 (omitted cross-section)";
    let solid = witness_solid();
    let interior = solid.z0 + 1;
    let emitted = slice_mesh_ex(&solid.mesh(), &[interior as f32]);
    check_slice(&solid, &[interior], &emitted, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    must_reject(
        ARM,
        check_slice(&solid, &[interior], &[Vec::new()], LATTICE_DIVISIONS),
        "must be nonempty",
    );
}

/// Arm 4: a filled (spurious) cross-section on an outside plane.
fn filled_section() {
    const ARM: &str = "arm 4 (filled outside section)";
    let solid = witness_solid();
    let outside = solid.z0 + solid.h + 1;
    let emitted = slice_mesh_ex(&solid.mesh(), &[outside as f32]);
    check_slice(&solid, &[outside], &emitted, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    must_reject(
        ARM,
        check_slice(&solid, &[outside], &[solid.section()], LATTICE_DIVISIONS),
        "must be empty",
    );
}

/// Arm 5: a transposed transform.
fn transposed_transform() {
    const ARM: &str = "arm 5 (transposed transform)";
    let mut matrix = [0.0_f64; 16];
    matrix[4] = -1.0; // correct +90 degrees about Z: x' = -y, y' = x
    matrix[1] = 1.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    let case = RigidTransformCase {
        transform: slicer_ir::Transform3d { matrix },
        translation_mm: [0.0, 0.0, 0.0],
        quarter_turns_z: 1,
    };
    let points = probe_points(4.0, 8);
    let mut stats = QueryStats::default();
    check_transform(&case, &points, &mut stats)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    // The mutant: the transposed matrix (a -90 degree turn) with the +90 oracle.
    let mut transposed = matrix;
    transposed[1] = -1.0;
    transposed[4] = 1.0;
    let wrong = RigidTransformCase {
        transform: slicer_ir::Transform3d { matrix: transposed },
        ..case
    };
    let mut stats = QueryStats::default();
    must_reject(ARM, check_transform(&wrong, &points, &mut stats), "axis");
}

/// A box mesh with its top face removed, so the exit hit is missing.
fn open_box(solid: &Solid) -> IndexedTriangleSet {
    let mut mesh = solid.mesh();
    // The top face is the second pair of triangles in `Solid::mesh`.
    mesh.indices.drain(6..12);
    mesh
}

/// A box mesh with an extra interior sheet that a +Z ray also hits.
///
/// The sheet stays inside the box, so the tree's bounds are unchanged and the
/// fabricated hit is the only difference from the honest fixture.
fn box_with_false_roof(solid: &Solid) -> IndexedTriangleSet {
    let mut mesh = solid.mesh();
    let next = u32::try_from(mesh.vertices.len()).expect("vertex count fits u32");
    let (x0, y0) = (solid.x0, solid.y0);
    let (x1, y1) = (solid.x0 + solid.w, solid.y0 + solid.d);
    let z = solid.z0 + 1;
    let point = |x: i64, y: i64| Point3 {
        x: x as f32,
        y: y as f32,
        z: z as f32,
    };
    mesh.vertices.push(point(x0, y0));
    mesh.vertices.push(point(x1, y0));
    mesh.vertices.push(point(x1, y1));
    mesh.vertices.push(point(x0, y1));
    mesh.indices
        .extend([next, next + 1, next + 2, next, next + 2, next + 3]);
    mesh
}

/// Arm 6: a missing exit hit.
fn missing_exit() {
    const ARM: &str = "arm 6 (missing exit hit)";
    let solid = witness_solid();
    let limits = query_limits();
    let tree = AabbTree::new(solid.mesh());
    let mut stats = QueryStats::default();
    check_rays(&solid, &tree, limits.max_queries, &mut stats)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    let broken = AabbTree::new(open_box(&solid));
    let mut stats = QueryStats::default();
    must_reject(
        ARM,
        check_rays(&solid, &broken, limits.max_queries, &mut stats),
        "expected 2 hit(s)",
    );
}

/// Arm 7: a fabricated hit.
fn fabricated_hit() {
    const ARM: &str = "arm 7 (fabricated hit)";
    let solid = witness_solid();
    let limits = query_limits();
    let tree = AabbTree::new(solid.mesh());
    let mut stats = QueryStats::default();
    check_rays(&solid, &tree, limits.max_queries, &mut stats)
        .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
    let extra = AabbTree::new(box_with_false_roof(&solid));
    let mut stats = QueryStats::default();
    must_reject(
        ARM,
        check_rays(&solid, &extra, limits.max_queries, &mut stats),
        "expected 2 hit(s), got 3",
    );
}

/// Run every slice and query mutation arm.
pub fn run() {
    // The shared profiles must themselves be valid before any arm uses them.
    let _ = slice_limits();
    bottom_filled();
    top_empty();
    omitted_section();
    filled_section();
    transposed_transform();
    missing_exit();
    fabricated_hit();
}
