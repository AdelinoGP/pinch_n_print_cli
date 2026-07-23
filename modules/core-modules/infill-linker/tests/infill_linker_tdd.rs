#![allow(missing_docs)]

use infill_linker::offset::{clip_to_offset_boundary, remove_short_polylines, ExPolygonWithOffset};
use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon};

fn square(size_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(size_mm, 0.0),
                Point2::from_mm(size_mm, size_mm),
                Point2::from_mm(0.0, size_mm),
            ],
        },
        holes: vec![],
    }
}

fn bounds(expolygon: &ExPolygon) -> (i64, i64, i64, i64) {
    expolygon.contour.points.iter().fold(
        (i64::MAX, i64::MIN, i64::MAX, i64::MIN),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    )
}

#[test]
fn re_clip_to_offset_boundary() {
    let overlap = ExPolygonWithOffset::for_infill_overlap(&[square(10.0)], 0.45, 1.0);
    let paths = vec![vec![Point2::from_mm(-1.0, 5.0), Point2::from_mm(11.0, 5.0)]];

    let clipped = clip_to_offset_boundary(&paths, overlap.polygons_outer());
    assert_eq!(clipped.len(), 1);

    let min_boundary = mm_to_units(0.05);
    let max_boundary = mm_to_units(9.95);
    for point in &clipped[0] {
        assert!((min_boundary - 2..=max_boundary + 2).contains(&point.x));
        assert!((min_boundary - 2..=max_boundary + 2).contains(&point.y));
    }
}

#[test]
fn expolygon_with_offset_matches_orca_square_case() {
    // FillRectilinear.cpp:388-490: OrcaSlicer `aoffset1 = overlap - 0.5*spacing` is passed to Clipper `offset()`; when `overlap < 0.5*spacing` the signed value is NEGATIVE and Clipper semantics INSET the source polygon. The linker uses `polygons_outer` (= `polygons_inner` when `aoffset1 == aoffset2`) as the re-clip boundary.
    const INFILL_OVERLAP: f32 = 0.45;
    const SPACING_MM: f32 = 1.0;
    let overlap =
        ExPolygonWithOffset::for_infill_overlap(&[square(10.0)], INFILL_OVERLAP, SPACING_MM);
    let expected_inset = mm_to_units((0.5 - INFILL_OVERLAP) * SPACING_MM);
    let expected = (
        expected_inset,
        mm_to_units(10.0) - expected_inset,
        expected_inset,
        mm_to_units(10.0) - expected_inset,
    );

    assert_eq!(overlap.polygons_outer().len(), 1);
    assert_eq!(bounds(&overlap.polygons_outer()[0]), expected);
    assert_eq!(overlap.polygons_inner(), overlap.polygons_outer());
}

#[test]
fn short_segment_filter() {
    let paths = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(0.79, 0.0)],
        vec![Point2::from_mm(0.0, 1.0), Point2::from_mm(0.8, 1.0)],
    ];

    let filtered = remove_short_polylines(&paths, 1.0);

    assert_eq!(filtered, vec![paths[1].clone()]);
}
