#![allow(missing_docs)]

//! TDD red tests for packet 193: the `overhang_distance_mm` carrier on
//! `Point3WithWidth` (AC-4, AC-5, AC-N1, AC-N2).
//!
//! Normative contract (canonical `estimate_points_properties`,
//! OrcaSlicerDocumented `GCode/ExtrusionProcessor.hpp`, G-code speed path):
//! `overhang_distance_mm` is the SIGNED perpendicular distance in mm from the
//! point to the previous layer's slice boundary, already normalised by adding
//! `boundary_offset = 0.5 * width` where `width` is the point's own stamped
//! extrusion width. Negative => inside (over) the previous layer by more than
//! `boundary_offset`; zero (within 1e-5) => exactly on the offset boundary;
//! positive => overhangs beyond it. `None` => no distance measured.
//!
//! These tests reference symbols that do not exist yet
//! (`Point3WithWidth::overhang_distance_mm`,
//! `slicer_core::perimeter_utils::signed_distance_to_boundary`, and
//! `expolygon_to_path3d`'s future previous-layer-boundary parameter) — this
//! binary MUST fail to compile until the production half of the packet lands.

use slicer_core::perimeter_utils::expolygon_to_path3d;
use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon};

/// Axis-aligned square contour (100nm integer units) centred on the origin.
fn square_contour(half_mm: f32) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(-half_mm, -half_mm),
            Point2::from_mm(half_mm, -half_mm),
            Point2::from_mm(half_mm, half_mm),
            Point2::from_mm(-half_mm, half_mm),
        ],
    }
}

/// Previous-layer slice boundary: one hole-free ExPolygon square.
fn square_boundary(half_mm: f32) -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: square_contour(half_mm),
        holes: Vec::new(),
    }]
}

fn quartiles_of(pts: &[Point3WithWidth]) -> Vec<Option<u8>> {
    pts.iter().map(|p| p.overhang_quartile).collect()
}

/// AC-4 (PRIMARY): the stamped value is signed, and equals
/// `signed_distance_to_boundary(x, y, prev) + 0.5 * width`.
#[test]
fn overhang_distance_is_signed_and_boundary_offset_normalised() {
    let boundary = square_boundary(10.0);
    let width = 0.4_f32;

    // Supported wall: 10mm square deep inside the 20mm previous layer.
    let supported = expolygon_to_path3d(&square_contour(5.0), 0.2, width, &[], &boundary);
    assert!(!supported.is_empty());
    for pt in &supported {
        let raw = slicer_core::perimeter_utils::signed_distance_to_boundary(pt.x, pt.y, &boundary);
        let stamped = pt
            .overhang_distance_mm
            .expect("previous-layer boundary present => Some(distance)");
        assert!(
            (stamped - (raw + 0.5 * pt.width)).abs() < 1e-4,
            "stamped overhang_distance_mm ({}) must equal signed_distance ({}) + 0.5 * width ({})",
            stamped,
            raw,
            pt.width
        );
        assert!(
            stamped < 0.0,
            "vertex inside the previous layer by more than boundary_offset must stamp negative; got {}",
            stamped
        );
    }

    // Overhanging wall: 30mm square surrounding the 20mm previous layer.
    let overhanging = expolygon_to_path3d(&square_contour(15.0), 0.2, width, &[], &boundary);
    assert!(!overhanging.is_empty());
    for pt in &overhanging {
        let stamped = pt
            .overhang_distance_mm
            .expect("previous-layer boundary present => Some(distance)");
        assert!(
            stamped > 0.0,
            "vertex overhanging beyond the offset boundary must stamp positive; got {}",
            stamped
        );
    }

    // Exactly on the offset boundary: vertices 0.2mm inside the previous
    // layer and boundary_offset = 0.5 * 0.4 = 0.2 => stamped == 0 (1e-5).
    let on_offset = expolygon_to_path3d(&square_contour(9.8), 0.2, width, &[], &boundary);
    assert!(!on_offset.is_empty());
    for pt in &on_offset {
        let stamped = pt
            .overhang_distance_mm
            .expect("previous-layer boundary present => Some(distance)");
        assert!(
            stamped.abs() < 1e-5,
            "vertex exactly on the offset boundary must stamp zero within 1e-5; got {}",
            stamped
        );
    }
}

/// AC-5: `expolygon_to_path3d` stamps the signed normalised distance when a
/// previous-layer boundary is supplied, and `None` when it is empty.
#[test]
fn expolygon_to_path3d_stamps_signed_distance_and_none_on_empty_boundary() {
    let boundary = square_boundary(10.0);
    let contour = square_contour(15.0);
    let width = 0.42_f32;

    let with_boundary = expolygon_to_path3d(&contour, 0.3, width, &[], &boundary);
    assert!(!with_boundary.is_empty());
    for pt in &with_boundary {
        let raw = slicer_core::perimeter_utils::signed_distance_to_boundary(pt.x, pt.y, &boundary);
        let stamped = pt
            .overhang_distance_mm
            .expect("non-empty previous-layer boundary => Some(distance)");
        assert!(
            (stamped - (raw + 0.5 * pt.width)).abs() < 1e-4,
            "stamped ({}) must equal signed_distance ({}) + 0.5 * width ({})",
            stamped,
            raw,
            pt.width
        );
    }

    let without_boundary = expolygon_to_path3d(&contour, 0.3, width, &[], &[]);
    assert_eq!(with_boundary.len(), without_boundary.len());
    for pt in &without_boundary {
        assert_eq!(
            pt.overhang_distance_mm, None,
            "empty previous-layer boundary => None on every vertex"
        );
    }
}

/// AC-N1: absent previous layer stamps `None` — never a sentinel like
/// `Some(0.0)`, `Some(-1.0)`, or `Some(f32::MAX)`.
#[test]
fn no_previous_layer_stamps_none_not_zero() {
    let pts = expolygon_to_path3d(&square_contour(5.0), 0.2, 0.4, &[], &[]);
    assert!(!pts.is_empty());
    for pt in &pts {
        assert_eq!(
            pt.overhang_distance_mm, None,
            "absent previous layer => None, never a sentinel value"
        );
        assert!(
            !matches!(
                pt.overhang_distance_mm,
                Some(0.0) | Some(-1.0) | Some(f32::MAX)
            ),
            "sentinel values are prohibited for the absent-boundary case"
        );
    }
}

/// AC-N2: carrying a previous-layer boundary must not change
/// `overhang_quartile` stamping — values are bit-identical with and without
/// the carrier present.
#[test]
fn quartile_stamping_is_unchanged_by_the_distance_carrier() {
    // One quartile-2 band covering a 12mm square: every vertex of the 10mm
    // contour below falls inside it, so the fixture exercises real bands.
    let bands = vec![slicer_ir::slice_ir::QuartileBand {
        quartile: 2,
        polygons: vec![ExPolygon {
            contour: square_contour(6.0),
            holes: Vec::new(),
        }],
    }];

    let contour = square_contour(5.0);
    let baseline = expolygon_to_path3d(&contour, 0.2, 0.4, &bands, &[]);
    let carried = expolygon_to_path3d(&contour, 0.2, 0.4, &bands, &square_boundary(6.0));

    let q_baseline = quartiles_of(&baseline);
    let q_carried = quartiles_of(&carried);
    assert!(
        q_baseline.iter().any(|q| q.is_some()),
        "fixture must exercise real quartile bands (at least one Some)"
    );
    assert_eq!(
        q_baseline, q_carried,
        "overhang_quartile values must be bit-identical with the distance carrier present"
    );
}
