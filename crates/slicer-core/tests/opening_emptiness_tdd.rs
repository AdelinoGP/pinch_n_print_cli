#![allow(missing_docs)]
//! `opening_is_empty` is the exact emptiness answer for morphological opening.
//!
//! The oracle is the unchanged two-pass `opening_ex`, which materialises the
//! erode-then-dilate geometry. The candidate answers the same question from the
//! single shared erosion pass while discarding no geometry it would have
//! consumed. A second, independently extracted oracle (`offset_with_miter_limit`
//! with the same parameters) confirms the predicate really is erosion
//! emptiness, not a re-parameterised opening.
//!
//! The fixture family carries both polarities, so a predicate that always
//! answered `true` (or always `false`) fails rather than passing vacuously.

use slicer_core::polygon_ops::{
    offset_with_miter_limit, opening_ex, opening_is_empty, OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn rect_mm(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

fn square_with_hole_mm(outer: f32, hole_min: f32, hole_max: f32) -> ExPolygon {
    ExPolygon {
        contour: rect_mm(-outer / 2.0, -outer / 2.0, outer / 2.0, outer / 2.0).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(hole_min, hole_min),
                Point2::from_mm(hole_max, hole_min),
                Point2::from_mm(hole_max, hole_max),
                Point2::from_mm(hole_min, hole_max),
            ],
        }],
    }
}

const MITER: OffsetJoinType = OffsetJoinType::Miter;
/// Callers pass `perimeter_arc_tolerance` as the *miter limit* argument; keep
/// that exact parameterisation here so the fixtures match production call sites.
const MITER_LIMIT: f64 = 0.0125;

/// (name, polygons, opening distance mm).
fn fixture_family() -> Vec<(&'static str, Vec<ExPolygon>, f64)> {
    vec![
        (
            "wide_square_survives",
            vec![rect_mm(-5.0, -5.0, 5.0, 5.0)],
            0.5,
        ),
        (
            "square_with_hole_outer_survives",
            vec![square_with_hole_mm(10.0, -2.0, 2.0)],
            1.0,
        ),
        (
            "narrow_rib_erodes_to_nothing",
            vec![rect_mm(0.0, 0.0, 0.3, 10.0)],
            0.2,
        ),
        (
            "thin_ring_wall_erodes_to_nothing",
            vec![square_with_hole_mm(10.0, -4.6, 4.6)],
            0.3,
        ),
        (
            "borderline_rib_keeps_a_core",
            vec![rect_mm(0.0, 0.0, 1.2, 10.0)],
            0.5,
        ),
        ("empty_input_has_no_opening", vec![], 0.4),
        (
            "multiple_polygons_one_survives",
            vec![rect_mm(0.0, 0.0, 0.2, 5.0), rect_mm(10.0, 0.0, 20.0, 10.0)],
            0.3,
        ),
        // Zero distance is a legal `smaller_perimeter_threshold_mm` value
        // (manifest min 0.0). The erosion pass then short-circuits to a clone
        // while the old second pass still runs its union cleanup, which drops
        // degenerate rings — so the two-pass opening can be empty here even
        // though the clone is not. This fixture pins that case.
        (
            "degenerate_zero_area_ring_at_zero_distance",
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(1.0, 0.0),
                        Point2::from_mm(2.0, 0.0),
                    ],
                },
                holes: vec![],
            }],
            0.0,
        ),
        (
            "valid_square_at_zero_distance",
            vec![rect_mm(-5.0, -5.0, 5.0, 5.0)],
            0.0,
        ),
        // Negative distances are not reachable through
        // `smaller_perimeter_threshold_mm`, but the predicate is a public API:
        // its contract must hold for every input, so a negative distance belongs
        // in the family even though no production caller supplies one today.
        (
            "negative_distance_degenerate_ring",
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(1.0, 0.0),
                        Point2::from_mm(2.0, 0.0),
                    ],
                },
                holes: vec![],
            }],
            -1.0,
        ),
        (
            "negative_distance_valid_square",
            vec![rect_mm(-5.0, -5.0, 5.0, 5.0)],
            -1.0,
        ),
        // Degenerate rings at positive distance: pass 1's flat output is
        // non-empty here, so the predicate's verdict rests on "a positive
        // dilation of a non-empty erosion cannot extract to empty". These pin
        // that assumption where it is most fragile.
        (
            "collinear_three_points_at_positive_distance",
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(1.0, 0.0),
                        Point2::from_mm(2.0, 0.0),
                    ],
                },
                holes: vec![],
            }],
            0.5,
        ),
        (
            "duplicate_vertex_square_at_positive_distance",
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(10.0, 0.0),
                        Point2::from_mm(10.0, 10.0),
                        Point2::from_mm(0.0, 10.0),
                    ],
                },
                holes: vec![],
            }],
            3.0,
        ),
        (
            "sub_unit_triangle_at_positive_distance",
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(0.0001, 0.0),
                        Point2::from_mm(0.0, 0.0001),
                    ],
                },
                holes: vec![],
            }],
            0.1,
        ),
    ]
}

#[test]
fn opening_is_empty_agrees_with_two_pass_opening_on_fixture_family() {
    let mut seen_true = false;
    let mut seen_false = false;
    for (name, polygons, distance) in fixture_family() {
        let predicate = opening_is_empty(&polygons, distance, MITER, MITER_LIMIT);
        let two_pass = opening_ex(&polygons, distance, MITER, MITER_LIMIT);
        assert_eq!(
            predicate,
            two_pass.is_empty(),
            "{name}: predicate {predicate} disagrees with the two-pass opening \
             ({} polygon(s))",
            two_pass.len()
        );
        if predicate {
            seen_true = true;
        } else {
            seen_false = true;
        }
    }
    assert!(
        seen_true && seen_false,
        "fixture family must carry both polarities; \
         got true={seen_true} false={seen_false} — a constant predicate would pass"
    );
}

#[test]
fn opening_is_empty_is_exactly_erosion_emptiness_for_positive_distance() {
    let mut checked = 0usize;
    for (name, polygons, distance) in fixture_family() {
        if distance <= 0.0 {
            continue; // documented boundaries, pinned separately below
        }
        let predicate = opening_is_empty(&polygons, distance, MITER, MITER_LIMIT);
        // Independently extracted erosion: tree-based extraction, same join,
        // same miter limit, same zero arc tolerance as the shared first pass.
        let eroded = offset_with_miter_limit(
            &polygons,
            -(distance as f32),
            MITER,
            MITER_LIMIT as f32,
            0.0,
        );
        assert_eq!(
            predicate,
            eroded.is_empty(),
            "{name}: predicate {predicate} must equal erosion emptiness \
             ({} polygon(s) survived the erode)",
            eroded.len()
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "positive-distance coverage collapsed: only {checked} fixtures checked"
    );
}

/// The `delta <= 0` boundary is deliberate, so pin it: for a zero delta the raw
/// erosion clones a degenerate ring (non-empty) while the real opening's union
/// cleanup drops it (empty); for a negative delta the opposite-sign second pass
/// empties a non-empty first pass. A refactor that deletes the delegation would
/// make this test red rather than silently changing `classify_narrow_island`'s
/// verdict for zero-area rings at `smaller_perimeter_threshold_mm = 0`.
#[test]
fn nonpositive_distance_delegates_to_the_two_pass_opening() {
    let fixtures: Vec<_> = fixture_family()
        .into_iter()
        .filter(|(_, _, distance)| *distance <= 0.0)
        .collect();
    assert!(
        !fixtures.is_empty(),
        "non-positive-distance fixtures must exist"
    );
    for (name, polygons, distance) in fixtures {
        let predicate = opening_is_empty(&polygons, distance, MITER, MITER_LIMIT);
        let two_pass = opening_ex(&polygons, distance, MITER, MITER_LIMIT);
        assert_eq!(
            predicate,
            two_pass.is_empty(),
            "{name}: non-positive-distance predicate must equal the two-pass \
             opening's emptiness"
        );
    }
    // The degenerate fixtures are the ones where erosion emptiness alone would
    // be wrong; assert each divergence exists so the delegation cannot be
    // removed without this test noticing.
    let degenerate = vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(1.0, 0.0),
                Point2::from_mm(2.0, 0.0),
            ],
        },
        holes: vec![],
    }];
    let eroded_at_zero = offset_with_miter_limit(&degenerate, -0.0, MITER, MITER_LIMIT as f32, 0.0);
    assert!(
        !eroded_at_zero.is_empty() && opening_ex(&degenerate, 0.0, MITER, MITER_LIMIT).is_empty(),
        "the degenerate zero-distance divergence no longer exists; the \
         delegation's justification needs re-derivation"
    );
    let eroded_at_neg = offset_with_miter_limit(&degenerate, 1.0, MITER, MITER_LIMIT as f32, 0.0);
    assert!(
        !eroded_at_neg.is_empty() && opening_ex(&degenerate, -1.0, MITER, MITER_LIMIT).is_empty(),
        "the degenerate negative-distance divergence no longer exists; the \
         delegation's justification needs re-derivation"
    );
}

/// Falsification the other way: for a positive distance, the dilation of a
/// non-empty erosion stays non-empty, so a caller reading only `.is_empty()`
/// cannot see a difference — this is the direction the fast path relies on.
#[test]
fn surviving_erosion_always_yields_a_non_empty_opening_for_positive_distance() {
    for (name, polygons, distance) in fixture_family() {
        if distance <= 0.0 {
            continue;
        }
        let eroded = offset_with_miter_limit(
            &polygons,
            -(distance as f32),
            MITER,
            MITER_LIMIT as f32,
            0.0,
        );
        if eroded.is_empty() {
            continue;
        }
        let two_pass = opening_ex(&polygons, distance, MITER, MITER_LIMIT);
        assert!(
            !two_pass.is_empty(),
            "{name}: non-empty erosion ({}) opened to nothing — the emptiness \
             identity does not hold for this fixture",
            eroded.len()
        );
    }
}
