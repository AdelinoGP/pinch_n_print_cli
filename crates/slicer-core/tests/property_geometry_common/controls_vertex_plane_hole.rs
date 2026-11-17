//! Deterministic witnesses for the vertex-plane and hole contracts.
//!
//! These are the named witnesses from the packet's "Negative controls"
//! paragraph that belong to this step: the cube's top and bottom ownership
//! planes, the through-vertex analytically expected triangle, and an annulus
//! whose hole must still be observed as a hole after a real Boolean operation.
//!
//! The endpoints-and-midpoint-inside clip counterexample is **not** duplicated
//! here: it lives in `controls_boolean_clip`, which owns the clip checker
//! (`docs/spec_packets/proptest-openrscad_09_pbt-geometry/packet.spec.md`,
//! `## Deviations` item 1). The vertex-only shared-index pair and the
//! one-triangle component belong to the helper step's control binary.

use slicer_core::polygon_ops::{difference, intersection, union, xor};
use slicer_core::slice_mesh_ex;
use slicer_ir::{IndexedTriangleSet, Point2, Point3, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{signed_area2, PointLocation};

use super::boolean::{check_rect_pair, BoolOutputs};
use super::mesh_slice::checked_limits as checked_slice_limits;
use super::mesh_slice::{check_slice, RawSliceBounds, Solid, LATTICE_DIVISIONS};
use super::rect::Rect;

/// Whole millimetres to canonical units, exactly.
fn units(mm: i64) -> i64 {
    mm * 10_000
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

fn limits() -> super::mesh_slice::SliceLimits {
    checked_slice_limits(RawSliceBounds {
        max_extent_mm: 16,
        max_translation_mm: 8,
        max_planes: 8,
    })
    .expect("witness profile")
}

/// The unit cube convention: the bottom plane is empty and the top carries the
/// square, via upper-edge ownership.
fn cube_planes() {
    const ARM: &str = "cube top/bottom ownership";
    let solid = Solid {
        box_solid: true,
        x0: 0,
        y0: 0,
        z0: 0,
        w: 1,
        d: 1,
        h: 1,
    };
    let bottom = slice_mesh_ex(&solid.mesh(), &[0.0]);
    check_slice(&solid, &[0], &bottom, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: bottom: {error}"));
    assert!(
        bottom[0].is_empty(),
        "{ARM}: the bottom plane must be empty, got {:?}",
        bottom[0]
    );
    let top = slice_mesh_ex(&solid.mesh(), &[1.0]);
    let stats = check_slice(&solid, &[1], &top, LATTICE_DIVISIONS)
        .unwrap_or_else(|error| panic!("{ARM}: top: {error}"));
    assert_eq!(
        stats.expected_nonempty, 1,
        "{ARM}: the top plane must be a nonempty layer"
    );
    assert_eq!(
        top[0].len(),
        1,
        "{ARM}: the top plane must yield exactly one square"
    );
    // The upper-edge convention is falsifiable in both directions.
    must_reject(
        ARM,
        check_slice(&solid, &[0], &[solid.section()], LATTICE_DIVISIONS),
        "must be empty",
    );
    must_reject(
        ARM,
        check_slice(&solid, &[1], &[Vec::new()], LATTICE_DIVISIONS),
        "must be nonempty",
    );
}

/// The pre-existing vertex-touching tetrahedron, sliced exactly through a vertex.
fn vertex_plane() {
    const ARM: &str = "through-vertex plane";
    let mesh = IndexedTriangleSet {
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
    };
    let emitted = slice_mesh_ex(&mesh, &[0.5]);
    assert_eq!(emitted.len(), 1, "{ARM}: one plane must produce one layer");
    assert_eq!(
        emitted[0].len(),
        1,
        "{ARM}: the vertex-touching tetrahedron must produce one component"
    );
    let half = units(1) / 2;
    let expected = Polygon {
        points: vec![
            Point2 { x: 0, y: 0 },
            Point2 {
                x: half,
                y: units(1),
            },
            Point2 {
                x: -half,
                y: units(1),
            },
        ],
    };
    let want = signed_area2(&expected).expect("analytic triangle");
    let got = signed_area2(&emitted[0][0].contour).expect("emitted triangle");
    assert_eq!(
        got, want,
        "{ARM}: the analytically expected triangle has double area {want}, got {got}"
    );
    // A wrong expected triangle must be distinguishable from the real one.
    let wrong = Polygon {
        points: vec![
            Point2 { x: 0, y: 0 },
            Point2 {
                x: units(1),
                y: units(1),
            },
            Point2 {
                x: -half,
                y: units(1),
            },
        ],
    };
    assert_ne!(
        signed_area2(&wrong).expect("mutant triangle"),
        want,
        "{ARM}: the mutant expectation must differ from the analytic area"
    );
}

/// An annulus keeps its hole through a real `difference`, and a dropped hole is
/// observably wrong to the shared Boolean checker.
fn annulus_retained_hole() {
    const ARM: &str = "annulus with a retained hole";
    let outer = Rect::from_mm(0, 0, 10, 10).expect("outer rectangle");
    let inner = Rect::from_mm(3, 3, 7, 7).expect("inner rectangle");
    let (a, b) = (vec![outer.to_expolygon()], vec![inner.to_expolygon()]);
    let outputs = BoolOutputs {
        union: union(&a, &b),
        intersection: intersection(&a, &b),
        difference: difference(&a, &b),
        xor: xor(&a, &b),
    };
    assert_eq!(
        outputs.difference.len(),
        1,
        "{ARM}: the real difference must be one component"
    );
    assert_eq!(
        outputs.difference[0].holes.len(),
        1,
        "{ARM}: the real difference must keep one hole"
    );
    check_rect_pair(outer, inner, &outputs)
        .unwrap_or_else(|error| panic!("{ARM}: the real output must pass: {error}"));

    // The mutant: the hole filled in.
    let mut filled = outputs.clone();
    filled.difference[0].holes.clear();
    must_reject(ARM, check_rect_pair(outer, inner, &filled), "Difference");

    // The checker also observes the hole through the independent predicate.
    let centre = Point2 {
        x: units(5),
        y: units(5),
    };
    assert_eq!(
        slicer_sdk::test_support::property_testing::oracles::point_in_region(
            centre,
            &outputs.difference
        )
        .expect("hole membership"),
        PointLocation::Outside,
        "{ARM}: the annulus centre must be outside the material"
    );
}

/// Run every vertex-plane and hole witness.
pub fn run() {
    let _ = limits();
    cube_planes();
    vertex_plane();
    annulus_retained_hole();
}
