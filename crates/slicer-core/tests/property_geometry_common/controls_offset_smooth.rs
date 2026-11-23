//! Deterministic controls for `H-OFFSET-MITER` and `H-OFFSET-SMOOTH`.
//!
//! Every arm writes its input and expected values here, runs the real SUT on
//! the named fixture (which must pass the shared checker), feeds an
//! analytically written correct answer through the same checker (which must
//! pass), and then feeds a deliberately wrong result through **the same
//! checker the property uses** (which must fail, for the named reason). No
//! synthetic mutation implies a prior SUT failure.

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, RngSeed, TestRunner};

use slicer_core::polygon_ops::{offset, offset2_ex, OffsetJoinType};
use slicer_core::smooth_outward::smooth_outward_polygon;
use slicer_ir::{ExPolygon, Point2, Polygon};

use super::bounds::BoundsError;
use super::offset::{
    self, check_offset, eroding_annulus, expected_single_pass, expected_two_pass, order_square,
    run_single_pass, run_two_pass, Expected, RawOffsetBounds, Shape, ARC_TOLERANCE_MM, MITER_LIMIT,
};
use super::rect::Rect;
use super::smooth::{
    self, check_convex_unchanged, check_l_chamfer, check_tiny_cw_deleted, convex_witness,
    l_witness, tiny_cw_square, RawSmoothBounds, WITNESS_CHORD,
};

fn pt(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ring(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| pt(x, y)).collect(),
    }
}

fn mm(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect::from_mm(x0, y0, x1, y1).expect("whole-mm control rectangle")
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

fn miter2(shape: Shape, d1: f64, d2: f64) -> Vec<ExPolygon> {
    offset2_ex(
        &[shape.to_expolygon()],
        d1,
        d2,
        OffsetJoinType::Miter,
        MITER_LIMIT,
    )
}

/// Arm 1: a result expanded where the delta demanded contraction.
fn reversed_sign() {
    const ARM: &str = "arm 1 (reversed sign)";
    // [0,4]² mm offset by -1 mm is [1,3]² mm.
    let shape = Shape::Rect(mm(0, 0, 4, 4));
    let expected = expected_single_pass(shape, -1).expect("model");
    assert_eq!(expected, Expected::Shape(Shape::Rect(mm(1, 1, 3, 3))));
    run_single_pass(shape, -1).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    let analytic = vec![ExPolygon {
        contour: ring(&[
            (10_000, 10_000),
            (30_000, 10_000),
            (30_000, 30_000),
            (10_000, 30_000),
        ]),
        holes: Vec::new(),
    }];
    check_offset(ARM, expected, &analytic).expect("the analytic contraction passes");
    let expanded = offset(
        &[shape.to_expolygon()],
        1.0,
        OffsetJoinType::Miter,
        ARC_TOLERANCE_MM,
    );
    must_reject(
        ARM,
        check_offset(ARM, expected, &expanded),
        "contour rectangle",
    );
}

/// Arm 2: the deterministic one-mm-square pass-order swap.
fn reversed_order() {
    const ARM: &str = "arm 2 (reversed pass order)";
    let square = order_square();
    // -1 then +2: the first pass collapses the 1 mm square, so empty.
    let want_empty = expected_two_pass(square, -1, 2).expect("model");
    assert_eq!(want_empty, Expected::Empty);
    let real = run_two_pass(square, -1, 2).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    assert_eq!(real.components, 0, "{ARM}: -1 then +2 must be empty");
    // A swapped-pass implementation computes +2 then -1 and survives.
    let swapped = miter2(square, 2.0, -1.0);
    assert!(
        !swapped.is_empty(),
        "{ARM}: the swapped mutant must survive"
    );
    must_reject(
        ARM,
        check_offset(ARM, want_empty, &swapped),
        "expected empty",
    );
    // +2 then -1: the analytic survivor is [-1,2]² mm.
    let want_survivor = expected_two_pass(square, 2, -1).expect("model");
    assert_eq!(
        want_survivor,
        Expected::Shape(Shape::Rect(mm(-1, -1, 2, 2)))
    );
    run_two_pass(square, 2, -1).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    let analytic = vec![ExPolygon {
        contour: ring(&[
            (-10_000, -10_000),
            (20_000, -10_000),
            (20_000, 20_000),
            (-10_000, 20_000),
        ]),
        holes: Vec::new(),
    }];
    check_offset(ARM, want_survivor, &analytic).expect("the analytic survivor passes");
    // The swapped implementation of this call collapses first: empty.
    let swapped = miter2(square, -1.0, 2.0);
    assert!(
        swapped.is_empty(),
        "{ARM}: the swapped mutant must collapse"
    );
    must_reject(
        ARM,
        check_offset(ARM, want_survivor, &swapped),
        "component count 0",
    );
}

/// Arm 3: an annulus output whose hole was dropped.
fn lost_hole() {
    const ARM: &str = "arm 3 (lost hole)";
    // [0,12]² minus [5,7]², -2 then +1 mm: cumulative -1 mm gives
    // [1,11]² minus [4,8]².
    let shape = eroding_annulus();
    let expected = expected_two_pass(shape, -2, 1).expect("model");
    let real = miter2(shape, -2.0, 1.0);
    let stats =
        check_offset(ARM, expected, &real).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    assert_eq!(stats.holes, 1, "{ARM}: the real result keeps the hole");
    let analytic = vec![ExPolygon {
        contour: mm(1, 1, 11, 11).ring(),
        holes: vec![mm(4, 4, 8, 8).hole_ring()],
    }];
    check_offset(ARM, expected, &analytic).expect("the analytic annulus passes");
    let mut dropped = real.clone();
    dropped[0].holes.clear();
    must_reject(ARM, check_offset(ARM, expected, &dropped), "hole count 0");
    // A hole left where it was (moved with the contour's delta of zero
    // instead of opposite to it) also fails.
    let wrong_way = vec![ExPolygon {
        contour: mm(1, 1, 11, 11).ring(),
        holes: vec![mm(5, 5, 7, 7).hole_ring()],
    }];
    must_reject(
        ARM,
        check_offset(ARM, expected, &wrong_way),
        "hole rectangle",
    );
}

/// Arm 4: an L whose chamfer sits at `chord` along each arm instead of
/// `chord / sqrt(2)`; also a missing chamfer and an off-by-one setback.
fn chord_setback() {
    const ARM: &str = "arm 4 (incorrect chord setback)";
    let case = l_witness(0, 0);
    assert_eq!(
        case.ring,
        ring(&[
            (0, 0),
            (40_000, 0),
            (40_000, 20_000),
            (20_000, 20_000),
            (20_000, 40_000),
            (0, 40_000),
        ])
    );
    let real = smooth_outward_polygon(&case.ring, case.chord);
    check_l_chamfer(&case, &real).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    // floor(5000 / sqrt(2)) = 3535: chamfer points (23535, 20000), (20000, 23535).
    let analytic = ring(&[
        (0, 0),
        (40_000, 0),
        (40_000, 20_000),
        (23_535, 20_000),
        (20_000, 23_535),
        (20_000, 40_000),
        (0, 40_000),
    ]);
    check_l_chamfer(&case, &analytic).expect("the analytic chamfer passes");
    let chord_as_setback = ring(&[
        (0, 0),
        (40_000, 0),
        (40_000, 20_000),
        (20_000 + WITNESS_CHORD, 20_000),
        (20_000, 20_000 + WITNESS_CHORD),
        (20_000, 40_000),
        (0, 40_000),
    ]);
    must_reject(
        ARM,
        check_l_chamfer(&case, &chord_as_setback),
        "chamfer setback 5000",
    );
    let rounded_up = ring(&[
        (0, 0),
        (40_000, 0),
        (40_000, 20_000),
        (23_536, 20_000),
        (20_000, 23_536),
        (20_000, 40_000),
        (0, 40_000),
    ]);
    must_reject(
        ARM,
        check_l_chamfer(&case, &rounded_up),
        "chamfer setback 3536",
    );
    let missing = ring(&[
        (0, 0),
        (40_000, 0),
        (40_000, 20_000),
        (20_000, 40_000),
        (0, 40_000),
    ]);
    must_reject(ARM, check_l_chamfer(&case, &missing), "vertex count");
}

/// Arm 5: the tiny CW ring is deleted as documented.
fn tiny_cw_deletion() {
    const ARM: &str = "arm 5 (tiny CW deletion)";
    let tiny = tiny_cw_square();
    let real = smooth_outward_polygon(&tiny, WITNESS_CHORD);
    check_tiny_cw_deleted(&real).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    must_reject(ARM, check_tiny_cw_deleted(&tiny), "expected deletion");
}

/// The convex contract compares cyclically and rejects a reversed ring.
fn convex_unchanged() {
    const ARM: &str = "convex control";
    let convex = convex_witness();
    let real = smooth_outward_polygon(&convex, WITNESS_CHORD);
    check_convex_unchanged(&convex, &real).unwrap_or_else(|e| panic!("{ARM}: real SUT: {e}"));
    let rotated = ring(&[(30_000, 20_000), (0, 20_000), (0, 0), (30_000, 0)]);
    check_convex_unchanged(&convex, &rotated).expect("a rotated ring start passes");
    let moved = ring(&[(0, 0), (30_000, 0), (30_000, 20_000), (1, 20_000)]);
    must_reject(
        ARM,
        check_convex_unchanged(&convex, &moved),
        "cyclic rotation",
    );
}

fn expect_bounds_err<T: std::fmt::Debug>(label: &str, result: Result<T, BoundsError>) {
    if let Ok(value) = result {
        panic!("{label}: invalid profile accepted as {value:?}");
    }
}

/// Invalid profiles fail before any strategy is built.
fn profile_bounds() {
    let good = offset::reference_raw_bounds();
    let limits = offset::validate_limits(good).expect("reference offset profile");
    assert_eq!(limits.max_abs_units, 120_000);
    assert_eq!(limits.half_span_mm, 12);
    for (index, raw) in [
        RawOffsetBounds {
            max_abs_units: 0,
            ..good
        },
        RawOffsetBounds {
            max_abs_units: 1_000_001,
            ..good
        },
        // 2 * 9 < 4 * 3 + 8: no placement room.
        RawOffsetBounds {
            max_abs_units: 90_000,
            ..good
        },
        RawOffsetBounds {
            max_side_mm: 0,
            ..good
        },
        RawOffsetBounds {
            max_side_mm: 2,
            ..good
        },
        RawOffsetBounds {
            max_side_mm: u64::MAX,
            ..good
        },
        RawOffsetBounds {
            max_offset_mm: 0,
            ..good
        },
        RawOffsetBounds {
            max_offset_mm: 1,
            ..good
        },
        RawOffsetBounds {
            max_offset_mm: u64::MAX,
            ..good
        },
    ]
    .into_iter()
    .enumerate()
    {
        expect_bounds_err(
            &format!("offset profile {index}"),
            offset::validate_limits(raw),
        );
    }
    let good = smooth::reference_raw_bounds();
    let limits = smooth::validate_limits(good).expect("reference smoothing profile");
    assert_eq!(limits.max_clip_units, 8_000);
    for (index, raw) in [
        RawSmoothBounds {
            max_abs_units: 0,
            ..good
        },
        RawSmoothBounds {
            max_abs_units: 1_000_001,
            ..good
        },
        RawSmoothBounds {
            max_side_units: 0,
            ..good
        },
        // 2 * (8000 + 1000) = 18000 needed.
        RawSmoothBounds {
            max_side_units: 17_999,
            ..good
        },
        RawSmoothBounds {
            max_side_units: 120_001,
            ..good
        },
        RawSmoothBounds {
            max_clip_units: 0,
            ..good
        },
        RawSmoothBounds {
            max_clip_units: 15,
            ..good
        },
        RawSmoothBounds {
            max_clip_units: 4_999,
            ..good
        },
        RawSmoothBounds {
            max_clip_units: u64::MAX,
            ..good
        },
    ]
    .into_iter()
    .enumerate()
    {
        expect_bounds_err(
            &format!("smoothing profile {index}"),
            smooth::validate_limits(raw),
        );
    }
}

/// Arm 6: the annulus generator must produce moving holes (delta-first drawing).
///
/// The generator defect this arm covers: drawing the annulus **shape** first and
/// clamping the deltas afterwards leaves 1–2 mm bands and holes, which admit only
/// cumulative delta `0`, so almost every annulus came out with `d1 = d2 = 0` and
/// the hole never moved — the two-pass annulus comparison was then a comparison
/// of the identity. `annulus_input` draws the deltas first and derives the bands
/// and hole from them; this arm samples **the real strategy** (`offset::strategy`,
/// not a copy of the algorithm) and requires that at least a quarter of the
/// sampled annuli have a nonzero cumulative delta and a hole that actually moves.
///
/// Determinism: the strategy is driven by proptest's tree API with a fixed
/// `RngSeed` and no cases beyond the documented sample count, so the arm needs no
/// `PropertyRun` context, no environment and no registry profile, and runs
/// identically on every invocation. Using the real strategy rather than a
/// test-local re-implementation of `annulus_input` is deliberate: a local copy
/// would be a self-referential oracle (`docs/22_test_quality.md` §2.1) and would
/// stay green through exactly the shape-first regression this arm exists to catch.
fn annulus_generator_delta_first() {
    const ARM: &str = "arm 6 (annulus generator: shape-first drawing)";
    const SAMPLES: usize = 256;
    let limits =
        offset::validate_limits(offset::reference_raw_bounds()).expect("reference offset profile");
    let strategy = offset::strategy(&limits);
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(0x5eed_0ffe_5e7a_1111),
        failure_persistence: None,
        ..Config::default()
    });
    let mut annuli = 0_usize;
    let mut moving = 0_usize;
    for _ in 0..SAMPLES {
        let input = strategy
            .new_tree(&mut runner)
            .expect("seeded offset tree")
            .current();
        if input.hole.is_none() {
            continue;
        }
        annuli += 1;
        let shape = input.shape().expect("generated annulus");
        let Shape::Annulus { hole: before, .. } = shape else {
            panic!("{ARM}: a `hole: Some` input must build an annulus: {input:?}");
        };
        let expected = expected_two_pass(shape, input.d1, input.d2)
            .unwrap_or_else(|error| panic!("{ARM}: generated annulus outside the family: {error}"));
        let Expected::Shape(Shape::Annulus { hole: after, .. }) = expected else {
            panic!("{ARM}: a generated annulus must survive both passes: {input:?}");
        };
        // The hole of an annulus moves opposite to the cumulative delta, so it
        // ends where it started exactly when `d1 + d2 == 0`.
        let cumulative = input.d1 + input.d2;
        assert_eq!(
            before != after,
            cumulative != 0,
            "{ARM}: the hole must move exactly when the cumulative delta is nonzero: \
             {input:?} -> {after:?}"
        );
        if cumulative != 0 {
            moving += 1;
            // The real SUT's hole is compared against `after` by `check_offset`,
            // and `after != before`, so this proves the production hole moved.
            let stats = run_two_pass(shape, input.d1, input.d2)
                .unwrap_or_else(|error| panic!("{ARM}: real SUT: {error}"));
            assert_eq!(stats.holes, 1, "{ARM}: the moving result keeps its hole");
        }
    }
    // Nonempty populations: the arm can never pass by sampling no annulus and no
    // moving pair (the shape-first regression produces the latter).
    assert!(
        annuli * 4 >= SAMPLES,
        "{ARM}: only {annuli} of {SAMPLES} sampled inputs were annuli"
    );
    // A quarter of the annuli must move. The shape-first design pins almost
    // every annulus at `d1 = d2 = 0` (one- and two-millimetre bands and holes
    // admit only cumulative delta zero), so it cannot reach this floor, while
    // the delta-first design clears it by a wide margin.
    assert!(
        moving * 4 >= annuli,
        "{ARM}: only {moving} of {annuli} sampled annuli had a nonzero cumulative delta: the \
         generator drew the shape before its deltas and the holes are pinned"
    );
}

/// Body of `geometry_controls_offset_and_smooth_mutations`.
pub fn run() {
    reversed_sign();
    reversed_order();
    lost_hole();
    chord_setback();
    tiny_cw_deletion();
    convex_unchanged();
    annulus_generator_delta_first();
    profile_bounds();
}
