#![allow(missing_docs)]

use slicer_core::polygon_ops::{clip_polylines, PreparedPolylineClip};
use slicer_core::{difference, intersection, offset, union, xor, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, signed_area2, validate_ring, PointLocation,
};

fn square(min_x_mm: f32, min_y_mm: f32, max_x_mm: f32, max_y_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, max_y_mm),
                Point2::from_mm(min_x_mm, max_y_mm),
            ],
        },
        holes: Vec::new(),
    }
}

fn shape_signature(polys: &[ExPolygon]) -> Vec<Vec<(i64, i64)>> {
    polys
        .iter()
        .map(|poly| poly.contour.points.iter().map(|p| (p.x, p.y)).collect())
        .collect()
}

fn signed_area_twice(points: &[Point2]) -> i128 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(current, next)| {
            i128::from(current.x) * i128::from(next.y) - i128::from(current.y) * i128::from(next.x)
        })
        .sum()
}

fn area_units2(polys: &[ExPolygon]) -> i128 {
    let twice_area: i128 = polys
        .iter()
        .map(|poly| {
            signed_area_twice(&poly.contour.points).abs()
                - poly
                    .holes
                    .iter()
                    .map(|hole| signed_area_twice(&hole.points).abs())
                    .sum::<i128>()
        })
        .sum();
    twice_area.abs() / 2
}

fn contour_bounds(polys: &[ExPolygon]) -> Option<(Point2, Point2)> {
    let mut points = polys.iter().flat_map(|poly| poly.contour.points.iter());
    let first = points.next()?;
    let (min_x, min_y, max_x, max_y) = points.fold(
        (first.x, first.y, first.x, first.y),
        |(min_x, min_y, max_x, max_y), point| {
            (
                min_x.min(point.x),
                min_y.min(point.y),
                max_x.max(point.x),
                max_y.max(point.y),
            )
        },
    );
    Some((Point2 { x: min_x, y: min_y }, Point2 { x: max_x, y: max_y }))
}

#[test]
fn boolean_ops_produce_expected_presence_for_overlapping_squares() {
    let a = square(0.0, 0.0, 10.0, 10.0);
    let b = square(5.0, 0.0, 15.0, 10.0);

    let union_result = union(&[a.clone()], &[b.clone()]);
    let intersection_result = intersection(&[a.clone()], &[b.clone()]);
    let difference_result = difference(&[a.clone()], &[b.clone()]);
    let xor_result = xor(&[a], &[b]);

    assert!(!union_result.is_empty(), "union should return geometry");
    assert!(
        !intersection_result.is_empty(),
        "intersection should return geometry"
    );
    assert!(
        !difference_result.is_empty(),
        "difference should return geometry"
    );
    assert!(!xor_result.is_empty(), "xor should return geometry");

    assert_eq!(union_result.len(), 1);
    assert_eq!(area_units2(&union_result), 15_000_000_000);
    assert_eq!(
        contour_bounds(&union_result),
        Some((Point2::from_mm(0.0, 0.0), Point2::from_mm(15.0, 10.0),))
    );

    assert_eq!(intersection_result.len(), 1);
    assert_eq!(area_units2(&intersection_result), 5_000_000_000);
    assert_eq!(
        contour_bounds(&intersection_result),
        Some((Point2::from_mm(5.0, 0.0), Point2::from_mm(10.0, 10.0),))
    );

    assert_eq!(difference_result.len(), 1);
    assert_eq!(area_units2(&difference_result), 5_000_000_000);
    assert_eq!(
        contour_bounds(&difference_result),
        Some((Point2::from_mm(0.0, 0.0), Point2::from_mm(5.0, 10.0),))
    );

    assert_eq!(xor_result.len(), 2);
    assert_eq!(area_units2(&xor_result), 10_000_000_000);
    assert_eq!(
        contour_bounds(&xor_result),
        Some((Point2::from_mm(0.0, 0.0), Point2::from_mm(15.0, 10.0),))
    );

    let union_signature = shape_signature(&union_result);
    let intersection_signature = shape_signature(&intersection_result);
    let difference_signature = shape_signature(&difference_result);
    let xor_signature = shape_signature(&xor_result);
    assert_ne!(union_signature, intersection_signature);
    assert_ne!(union_signature, difference_signature);
    assert_ne!(union_signature, xor_signature);
    assert_ne!(intersection_signature, difference_signature);
    assert_ne!(intersection_signature, xor_signature);
    assert_ne!(difference_signature, xor_signature);
}

#[test]
fn offset_outward_expands_bounds() {
    let input = square(0.0, 0.0, 10.0, 10.0);
    let expanded = offset(&[input], 0.4, OffsetJoinType::Miter, 0.0);

    assert!(!expanded.is_empty(), "offset should return geometry");

    let first = &expanded[0].contour.points;
    let min_x = first.iter().map(|p| p.x).min().unwrap_or(0);
    let min_y = first.iter().map(|p| p.y).min().unwrap_or(0);
    let max_x = first.iter().map(|p| p.x).max().unwrap_or(0);
    let max_y = first.iter().map(|p| p.y).max().unwrap_or(0);

    assert!(min_x < Point2::from_mm(0.0, 0.0).x);
    assert!(min_y < Point2::from_mm(0.0, 0.0).y);
    assert!(max_x > Point2::from_mm(10.0, 0.0).x);
    assert!(max_y > Point2::from_mm(0.0, 10.0).y);
}

#[test]
fn degenerate_polygon_is_ignored() {
    let degenerate = ExPolygon {
        contour: Polygon {
            points: vec![Point2::from_mm(1.0, 1.0), Point2::from_mm(1.0, 1.0)],
        },
        holes: Vec::new(),
    };
    let normal = square(0.0, 0.0, 1.0, 1.0);

    let result = union(&[degenerate], &[normal]);

    assert_eq!(result.len(), 1, "degenerate geometry should be ignored");
}

#[test]
fn offset_arc_tolerance_reduces_vertex_count() {
    // Square contour offset by +1.0 with Round join, comparing arc_tolerance 0.0 vs 0.5.
    // arc_tolerance 0.5 should produce strictly fewer vertices on the rounded corners.
    let sq = square(0.0, 0.0, 10.0, 10.0);
    let fine = offset(&[sq.clone()], 1.0, OffsetJoinType::Round, 0.0);
    let coarse = offset(&[sq], 1.0, OffsetJoinType::Round, 0.5);
    let fine_count: usize = fine.iter().map(|p| p.contour.points.len()).sum();
    let coarse_count: usize = coarse.iter().map(|p| p.contour.points.len()).sum();
    assert!(
        coarse_count < fine_count,
        "coarse={} not less than fine={}",
        coarse_count,
        fine_count
    );
}

// ---------------------------------------------------------------------------
// clip_polylines (packet 129)
// ---------------------------------------------------------------------------

/// Boundary-endpoint tolerance in integer units (1 unit = 100 nm).
const EDGE_TOL: i64 = 2;

fn polyline(points_mm: &[(f32, f32)]) -> Vec<Point2> {
    points_mm
        .iter()
        .map(|&(x, y)| Point2::from_mm(x, y))
        .collect()
}

fn square_with_hole(min_mm: f32, max_mm: f32, hole_min_mm: f32, hole_max_mm: f32) -> ExPolygon {
    let mut exp = square(min_mm, min_mm, max_mm, max_mm);
    // Hole wound CW (opposite of the CCW contour).
    exp.holes.push(Polygon {
        points: vec![
            Point2::from_mm(hole_min_mm, hole_min_mm),
            Point2::from_mm(hole_min_mm, hole_max_mm),
            Point2::from_mm(hole_max_mm, hole_max_mm),
            Point2::from_mm(hole_max_mm, hole_min_mm),
        ],
    });
    exp
}

fn near(p: Point2, q: Point2, tol: i64) -> bool {
    (p.x - q.x).abs() <= tol && (p.y - q.y).abs() <= tol
}

fn polyline_len(poly: &[Point2]) -> f64 {
    poly.windows(2)
        .map(|w| {
            let dx = (w[1].x - w[0].x) as f64;
            let dy = (w[1].y - w[0].y) as f64;
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

/// AC-1: polyline strictly inside → returned whole, unsplit, order preserved.
#[test]
fn clip_polylines_line_fully_inside_returned_whole() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 2.0), (5.0, 3.0), (8.0, 8.0)]);

    let result = clip_polylines(&[input.clone()], &clip);

    assert_eq!(result.len(), 1, "expected exactly one polyline");
    assert_eq!(
        result[0], input,
        "inside polyline must be returned verbatim"
    );
}

/// AC-2: single boundary crossing → one polyline covering only the inside
/// portion; crossing endpoint on the boundary within ±2 units.
#[test]
fn clip_polylines_line_crossing_once_split() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(5.0, 5.0), (15.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 1, "expected exactly one polyline");
    let out = &result[0];
    assert!(out.len() >= 2);
    let inside_pt = Point2::from_mm(5.0, 5.0);
    let boundary_pt = Point2::from_mm(10.0, 5.0);
    let (first, last) = (out[0], *out.last().unwrap());
    let matches = (near(first, inside_pt, EDGE_TOL) && near(last, boundary_pt, EDGE_TOL))
        || (near(first, boundary_pt, EDGE_TOL) && near(last, inside_pt, EDGE_TOL));
    assert!(
        matches,
        "endpoints {:?}..{:?} must be the inside point and the boundary crossing",
        first, last
    );
    // Only the inside portion: nothing beyond x = 10 mm (+tol).
    for p in out {
        assert!(
            p.x <= boundary_pt.x + EDGE_TOL,
            "point {:?} outside clip",
            p
        );
    }
}

/// AC-3: enter-exit-enter → exactly 2 disjoint inside sub-polylines.
#[test]
fn clip_polylines_line_crossing_twice_two_segments() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    // Starts inside, exits through the top edge, re-enters: W shape.
    let input = polyline(&[(2.0, 5.0), (4.0, 15.0), (6.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 2, "expected two inside sub-polylines");
    let top = Point2::from_mm(0.0, 10.0).y;
    for poly in &result {
        assert!(poly.len() >= 2);
        for p in poly {
            assert!(p.y <= top + EDGE_TOL, "point {:?} above clip top", p);
        }
    }
}

/// AC-4: polyline straight through a hole → 2 sub-polylines, no returned
/// point strictly inside the hole.
#[test]
fn clip_polylines_line_through_hole_split_around_hole() {
    let clip = [square_with_hole(0.0, 10.0, 4.0, 6.0)];
    let input = polyline(&[(1.0, 5.0), (9.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(
        result.len(),
        2,
        "expected the polyline split around the hole"
    );
    let hole_min = Point2::from_mm(4.0, 4.0);
    let hole_max = Point2::from_mm(6.0, 6.0);
    for poly in &result {
        for p in poly {
            let strictly_inside_hole = p.x > hole_min.x + EDGE_TOL
                && p.x < hole_max.x - EDGE_TOL
                && p.y > hole_min.y + EDGE_TOL
                && p.y < hole_max.y - EDGE_TOL;
            assert!(!strictly_inside_hole, "point {:?} lies inside the hole", p);
        }
    }
}

/// AC-5: polyline collinear with a contour edge → on-edge span is inside.
#[test]
fn clip_polylines_line_along_edge_inside() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 0.0), (8.0, 0.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 1, "edge-coincident span must be returned");
    let out = &result[0];
    for p in out {
        assert!(
            p.y.abs() <= EDGE_TOL,
            "point {:?} not on the bottom edge",
            p
        );
    }
    let expected_len = (Point2::from_mm(8.0, 0.0).x - Point2::from_mm(2.0, 0.0).x) as f64;
    let len = polyline_len(out);
    assert!(
        (len - expected_len).abs() <= 2.0 * EDGE_TOL as f64,
        "edge span length {} != expected {}",
        len,
        expected_len
    );
}

/// AC-6: 3 inputs (inside, outside, crossing) → exactly 2 outputs.
#[test]
fn clip_polylines_multi_polyline_clip() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let inside = polyline(&[(2.0, 2.0), (8.0, 8.0)]);
    let outside = polyline(&[(20.0, 20.0), (30.0, 30.0)]);
    let crossing = polyline(&[(5.0, 5.0), (15.0, 5.0)]);

    let result = clip_polylines(&[inside.clone(), outside, crossing], &clip);

    assert_eq!(
        result.len(),
        2,
        "expected inside whole + crossing's inside part"
    );
    // Set membership, not index positions: one output is the inside polyline
    // verbatim (possibly reversed), the other ends at the boundary.
    let mut reversed_inside = inside.clone();
    reversed_inside.reverse();
    let whole_count = result
        .iter()
        .filter(|p| **p == inside || **p == reversed_inside)
        .count();
    assert_eq!(
        whole_count, 1,
        "exactly one output is the untouched inside polyline"
    );
    let boundary_pt = Point2::from_mm(10.0, 5.0);
    let clipped_count = result
        .iter()
        .filter(|p| {
            near(p[0], boundary_pt, EDGE_TOL) || near(*p.last().unwrap(), boundary_pt, EDGE_TOL)
        })
        .count();
    assert_eq!(
        clipped_count, 1,
        "exactly one output ends on the clip boundary"
    );
    // All returned points inside the clip.
    for poly in &result {
        for p in poly {
            assert!(
                p.x <= boundary_pt.x + EDGE_TOL,
                "point {:?} outside clip",
                p
            );
        }
    }
}

/// AC-N1: fully outside → dropped.
#[test]
fn clip_polylines_line_fully_outside_dropped() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(20.0, 20.0), (30.0, 25.0)]);

    let result = clip_polylines(&[input], &clip);

    assert!(result.is_empty(), "fully outside polyline must be dropped");
}

/// AC-N2: empty polylines / empty clip / both → empty Vec, no panic.
#[test]
fn clip_polylines_empty_input_returns_empty() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 2.0), (8.0, 8.0)]);

    assert!(clip_polylines(&[], &clip).is_empty());
    assert!(clip_polylines(&[input.clone()], &[]).is_empty());
    assert!(clip_polylines(&[], &[]).is_empty());
}

/// Representation/reuse regression: holes, islands, concavity, dense rings,
/// all contour edges, hole edges, outside paths and <2-point subjects. The
/// expected vertices/order were captured from the pre-hoist one-shot function.
#[test]
fn prepared_clip_repeated_calls_match_pre_hoist_one_shot_outputs() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/clip_polylines_prepared_reference.json"
    ))
    .unwrap();
    let cases = reference["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 5);
    for case in cases {
        let clip: Vec<ExPolygon> = serde_json::from_value(case["clip"].clone()).unwrap();
        let prepared = PreparedPolylineClip::new(&clip);
        let calls = case["calls"].as_array().unwrap();
        assert_eq!(calls.len(), 14);
        // Revisit calls in reverse order on the same object to catch leaked
        // Clipper state after an outside/empty result or a multi-subject call.
        for call in calls.iter().chain(calls.iter().rev()) {
            let input: Vec<Vec<Point2>> = serde_json::from_value(call["input"].clone()).unwrap();
            let expected: Vec<Vec<Point2>> =
                serde_json::from_value(call["expected"].clone()).unwrap();
            assert_eq!(prepared.clip(&input), expected, "prepared {}", case["name"]);
            assert_eq!(
                clip_polylines(&input, &clip),
                expected,
                "one-shot {}",
                case["name"]
            );
        }
    }
}

#[test]
fn result_order_is_deterministic_for_same_input() {
    let a = square(0.0, 0.0, 4.0, 4.0);
    let b = square(3.0, 0.0, 7.0, 4.0);

    let first = union(&[a.clone()], &[b.clone()]);
    let second = union(&[a], &[b]);

    assert_eq!(shape_signature(&first), shape_signature(&second));
}

// ---------------------------------------------------------------------------
// Zero-width excursion (collinear backtracking spike) regression
// ---------------------------------------------------------------------------

/// Ring from explicit integer units (1 unit = 100 nm).
fn units_ring(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| Point2 { x, y }).collect(),
    }
}

/// Solid component from explicit integer units (no holes).
fn units_component(points: &[(i64, i64)]) -> ExPolygon {
    ExPolygon {
        contour: units_ring(points),
        holes: Vec::new(),
    }
}

/// `Some(dot)` when `a -> b -> c` is a collinear **backtracking spike**: zero
/// turn (`cross == 0`) and a reversal (`dot < 0`), i.e. the boundary runs out to
/// `b` and comes straight back over itself. `None` for any other vertex,
/// including a forward collinear vertex (`dot > 0`), which is a legitimate
/// straight-run vertex.
fn backtracking_spike_dot(a: Point2, b: Point2, c: Point2) -> Option<i128> {
    let sub = |to: i64, from: i64| i128::from(to) - i128::from(from);
    let (in_x, in_y) = (sub(b.x, a.x), sub(b.y, a.y));
    let (out_x, out_y) = (sub(c.x, b.x), sub(c.y, b.y));
    let cross = in_x * out_y - in_y * out_x;
    if cross != 0 {
        return None;
    }
    let dot = in_x * out_x + in_y * out_y;
    (dot < 0).then_some(dot)
}

/// Render an `i128` double area as exact integer text (never a float).
fn describe_area(twice_area: i128) -> String {
    format!("{twice_area} (double area, squared units)")
}

/// Regression: `xor` emitted a **zero-width excursion** when one operand had a
/// redundant collinear vertex on a boundary segment coincident with the other
/// operand, and a component gap forced the boundary to split there.
///
/// Measured fixture (integer units, 1 unit = 100 nm):
///
/// * subject A = this chamfered octagon
///   ```text
///   (-95848,-88869) (-95840,-88869) (-95632,-88869) (-95632,-88669)
///   (-95640,-88661) (-95840,-88661) (-95848,-88669)
///   ```
///   in which `(-95840,-88869)` is a **redundant collinear** vertex of the
///   bottom edge, plus a detached `8 x 208` component
///   `[-95624,-95616] x [-88869,-88661]`;
/// * clip B = the enclosing rectangle `[-95880,-95608] x [-88869,-88661]`.
///
/// A is a strict subset of B sharing B's whole y band, so `xor(A, B) = B \ A`
/// and the expected result is **analytic**, not captured: the band is
/// `272 x 208 = 56576` units^2, the octagon is `216 x 208` minus two 8x8
/// corner chamfers `= 44864`, the detached 8x208 component is `1664`, so
/// `xor` encloses exactly `56576 - 44864 - 1664 = 10048` units^2, a double
/// area of `20096`. Because the octagon and the detached component both span
/// the band's full height, the result is exactly three disjoint slivers (no
/// holes): left `x in [-95880,-95848]`, gap `x in [-95632,-95624]`, right
/// `x in [-95616,-95608]`, with the two octagon corner chamfers cut out of the
/// gap's and the left sliver's top edges.
///
/// Pre-fix the gap sliver closed
/// `... (-95632,-88869) -> (-95840,-88869) -> (-95624,-88869)`, retracing
/// `x in [-95840,-95632]` at `y = -88869`: three consecutive output vertices
/// with zero orientation and a negative dot product. That ring fails the strict
/// `validate_ring` ("ring edges intersect beyond their shared endpoint": the
/// retraced span is a positive-length collinear overlap) and the
/// weakly-simple predicate that the H-BOOL-SET property uses. Removing the
/// redundant input vertex makes the output clean, which is what pins the
/// trigger.
#[test]
fn xor_redundant_collinear_vertex_emits_no_backtracking_spike() {
    let subject = vec![
        units_component(&[
            (-95848, -88869),
            (-95840, -88869),
            (-95632, -88869),
            (-95632, -88669),
            (-95640, -88661),
            (-95840, -88661),
            (-95848, -88669),
        ]),
        units_component(&[
            (-95624, -88869),
            (-95616, -88869),
            (-95616, -88661),
            (-95624, -88661),
        ]),
    ];
    let clip = vec![units_component(&[
        (-95880, -88869),
        (-95608, -88869),
        (-95608, -88661),
        (-95880, -88661),
    ])];

    let result = xor(&subject, &clip);

    // 1. Every ring is a genuine simple closed ring, and none backtracks.
    for (index, polygon) in result.iter().enumerate() {
        let labelled = std::iter::once((String::from("contour"), &polygon.contour)).chain(
            polygon
                .holes
                .iter()
                .enumerate()
                .map(|(hole_index, hole)| (format!("hole[{hole_index}]"), hole)),
        );
        for (name, ring) in labelled {
            let label = format!("result[{index}].{name}");
            validate_ring(ring).unwrap_or_else(|error| {
                panic!("{label} fails the strict ring validation: {error}; ring {ring:?}")
            });
            let points = &ring.points;
            let n = points.len();
            for vertex in 0..n {
                let (prev, here, next) = (
                    points[(vertex + n - 1) % n],
                    points[vertex],
                    points[(vertex + 1) % n],
                );
                if let Some(dot) = backtracking_spike_dot(prev, here, next) {
                    panic!(
                        "{label} backtracks at vertex {here:?}: the boundary runs out to a \
                         collinear spike and returns over itself (dot {dot}); ring {ring:?}"
                    );
                }
            }
        }
    }

    // 2. The region is unchanged: exactly three components, no holes, and the
    //    analytic double area of `B \ A`.
    assert_eq!(
        result.len(),
        3,
        "xor must split into three slivers, got {result:?}"
    );
    for (index, polygon) in result.iter().enumerate() {
        assert!(
            polygon.holes.is_empty(),
            "result[{index}] must have no holes, got {:?}",
            polygon.holes
        );
    }
    let double_area: i128 = result
        .iter()
        .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
        .map(|ring| signed_area2(ring).expect("ring double area"))
        .sum();
    assert_eq!(
        double_area,
        2 * (272 * 208 - (216 * 208 - 2 * 32) - 8 * 208),
        "xor double area must equal the analytic B \\ A area; result {result:?}"
    );
    assert_eq!(
        double_area, 20_096,
        "analytic double area recomputed by hand"
    );

    // 2 (continued). Membership truth table at integer points that are clear of
    //    every operand edge and of every output edge, so the classification is
    //    exact and `Boundary` is a failure, not a skip.
    let truth = [
        // Inside B, outside A: the three slivers.
        ((-95860, -88700), true, "left sliver"),
        (
            (-95628, -88760),
            true,
            "gap between octagon and detached component",
        ),
        ((-95612, -88700), true, "right sliver"),
        ((-95847, -88662), true, "octagon's top-left chamfer notch"),
        // Inside A, so outside the xor.
        ((-95700, -88700), false, "inside the octagon"),
        ((-95620, -88700), false, "inside the detached component"),
        // Outside B, so outside the xor.
        ((-95870, -88600), false, "above the band"),
        ((-95600, -88700), false, "right of the band"),
        ((-95890, -88700), false, "left of the band"),
    ];
    for &((x, y), want, what) in truth.iter() {
        let point = Point2 { x, y };
        let got = point_in_region(point, &result).expect("point classification");
        assert!(
            matches!(
                (want, got),
                (true, PointLocation::Inside) | (false, PointLocation::Outside)
            ),
            "{what}: point {point:?} classified {got:?}, expected {}; result {result:?}",
            if want { "Inside" } else { "Outside" }
        );
    }

    // 3. Winding: contours CCW, holes CW.
    for (index, polygon) in result.iter().enumerate() {
        let contour_area = signed_area2(&polygon.contour).expect("contour double area");
        assert!(
            contour_area > 0,
            "result[{index}] contour must be counter-clockwise, double area {}",
            describe_area(contour_area)
        );
        for (hole_index, hole) in polygon.holes.iter().enumerate() {
            let hole_area = signed_area2(hole).expect("hole double area");
            assert!(
                hole_area < 0,
                "result[{index}].hole[{hole_index}] must be clockwise, double area {}",
                describe_area(hole_area)
            );
        }
    }

    // 4. The normalisation removes only reversals. A *forward* collinear vertex
    //    is a legitimate straight-run vertex: `(5,0)` sits on the bottom edge of
    //    a 10x10 square, Clipper2 re-emits it, and it must survive untouched.
    let straight_run = units_component(&[(0, 0), (5, 0), (10, 0), (10, 10), (0, 10)]);
    let kept = union(&[straight_run], &[]);
    assert_eq!(kept.len(), 1, "union of one square must yield one ring");
    let ring = &kept[0].contour;
    assert!(
        ring.points.contains(&Point2 { x: 5, y: 0 }),
        "a forward collinear vertex must not be removed; ring {ring:?}"
    );
    assert_eq!(
        ring.points.len(),
        5,
        "a forward collinear vertex must not be removed as a spike; ring {ring:?}"
    );
    assert_eq!(
        signed_area2(ring).expect("square double area"),
        200,
        "removing a forward collinear vertex must not change the area; ring {ring:?}"
    );
}

/// Regression: the zero-width-excursion normalisation left a **consecutive
/// duplicate** behind when the excursion closed exactly on its own start,
/// `a -> b -> a`.
///
/// Measured fixture (`difference`, integer units, 1 unit = 100 nm):
///
/// * subject A = `[2,5] x [2,3]` together with `[1,4] x [1,4]`;
/// * clip B = `[2,4] x [1,2]`, which lies inside A's second block.
///
/// Clipper2 emits the raw ring
/// `(4,2) (5,2) (5,3) (4,3) (4,4) (1,4) (1,1) (2,1) (2,2) (4,2) (4,1)`:
/// `(4,1)` is B's bottom-right corner, reached from `(4,2)` and followed by
/// `(4,2)` again, so it is the apex of a **needle** whose base is the single
/// point `(4,2)` — the `a -> b -> a` shape (`a = (4,2)`, `b = (4,1)`). Deleting
/// the apex leaves `(4,2), (4,2)` adjacent, which is a consecutive duplicate,
/// not an edge: pre-fix the ring has ten vertices and fails the strict
/// `validate_ring` with `a ring cannot repeat a vertex`, even though the needle
/// encloses no area and the region is correct. The fix drops the duplicate
/// inside the same fixed-point pass, with the same rule as `remove_duplicates`.
///
/// In this witness the duplicate lands **on the seam** (the raw path starts at
/// `(4,2)`, the needle's base), so `Vec::dedup` alone cannot see it and neither
/// can a `windows(2)` scan: the normalisation must apply the duplicate rule to
/// the `last -> first` pair as well, which is why the seam pair is handled
/// explicitly rather than left to `remove_duplicates`.
///
/// The region is analytic, not captured. `|A| = 3 + 9 - 2`, because the
/// `3 x 1` bar and the `3 x 3` block overlap in `[2,4] x [2,3]` (area 2);
/// `B` lies inside the block, so `|A \ B| = 10 - 2 = 8` units^2, a double area
/// of `16`, as one component with no hole.
#[test]
fn needle_excursion_returning_to_its_start_leaves_no_duplicate_vertex() {
    let subject = vec![
        units_component(&[(2, 3), (2, 2), (5, 2), (5, 3)]),
        units_component(&[(4, 1), (4, 4), (1, 4), (1, 1)]),
    ];
    let clip = vec![units_component(&[(2, 1), (4, 1), (4, 2), (2, 2)])];

    let result = difference(&subject, &clip);

    // 1. One ring, no hole: the needle encloses nothing, so removing it cannot
    //    change the component structure.
    assert_eq!(result.len(), 1, "one component, got {result:?}");
    assert_eq!(result[0].holes.len(), 0, "no hole, got {result:?}");
    let ring = &result[0].contour;
    assert_eq!(
        ring.points.len(),
        9,
        "the needle apex is the only vertex the normalisation removes; ring {ring:?}"
    );

    // 2. No consecutive duplicate, including across the seam (which the needle
    //    apex sat on before the fix: the raw ring ended `... (2,2) (4,2) (4,1)`).
    let points = &ring.points;
    let n = points.len();
    for index in 0..n {
        let next = points[(index + 1) % n];
        assert_ne!(
            points[index], next,
            "ring repeats a vertex consecutively at index {index}: {ring:?}"
        );
    }
    assert!(
        !ring.points.contains(&Point2 { x: 4, y: 1 }),
        "the needle apex must be gone; ring {ring:?}"
    );

    // 3. No backtracking spike anywhere, and the ring is strictly simple.
    for vertex in 0..n {
        let (prev, here, next) = (
            points[(vertex + n - 1) % n],
            points[vertex],
            points[(vertex + 1) % n],
        );
        if let Some(dot) = backtracking_spike_dot(prev, here, next) {
            panic!(
                "ring backtracks at vertex {here:?}: the boundary runs out to a collinear \
                 spike and returns over itself (dot {dot}); ring {ring:?}"
            );
        }
    }
    validate_ring(ring).unwrap_or_else(|error| {
        panic!("the normalised ring must be strictly simple: {error}; ring {ring:?}")
    });

    // 4. The same region: the analytic double area, and membership on both
    //    sides of every boundary the needle touched.
    let double_area: i128 = std::iter::once(ring)
        .chain(result[0].holes.iter())
        .map(|ring| signed_area2(ring).expect("ring double area"))
        .sum();
    assert_eq!(
        double_area,
        2 * (3 + 9 - 2 - 2),
        "difference double area must equal the analytic |A \\ B|; result {result:?}"
    );
    assert_eq!(
        double_area, 16,
        "analytic double area recomputed by hand (8 units^2)"
    );
    let truth = [
        ((3, 1), false, "inside the clipped-away corner of B"),
        ((3, 3), true, "inside the bar/block overlap"),
        ((2, 3), true, "inside the block above B"),
        ((6, 0), false, "outside the subject"),
    ];
    for &((x, y), want, what) in truth.iter() {
        let point = Point2 { x, y };
        let got = point_in_region(point, &result).expect("point classification");
        assert!(
            matches!(
                (want, got),
                (true, PointLocation::Inside) | (false, PointLocation::Outside)
            ),
            "{what}: point {point:?} classified {got:?}, expected {}; result {result:?}",
            if want { "Inside" } else { "Outside" }
        );
    }

    // 5. The needle is B's, not the subject's: `A` alone unions into one ring of
    //    its own eight corners and the analytic double area `2 * |A| = 20`, so
    //    nothing here is a normalisation of the subject.
    let untouched = union(&subject, &[]);
    assert_eq!(untouched.len(), 1, "union of the subject: {untouched:?}");
    assert_eq!(
        untouched[0].contour.points.len(),
        8,
        "the subject's own ring has eight vertices and no needle: {:?}",
        untouched[0].contour
    );
    assert_eq!(
        signed_area2(&untouched[0].contour).expect("subject double area"),
        2 * (3 + 9 - 2),
        "the subject is `|A| = 10` units^2: 3 + 9 minus the 2-unit overlap"
    );
}
