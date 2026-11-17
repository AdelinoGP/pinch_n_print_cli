//! Deterministic controls for `H-BOOL-SET` and `H-BOOL-CLIP`.
//!
//! Every control writes its input and expected values here, calls the real
//! SUT on the named fixture (which must pass), feeds an analytically written
//! correct answer through the same checker (which must pass), and then feeds a
//! deliberately wrong result through **the same checker the property uses**
//! (which must fail). No synthetic mutation implies a prior SUT failure.

use slicer_core::polygon_ops::clip_polylines;
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::{GeometryCase, TopologyClass};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, signed_area2, validate_ring, PointLocation,
};

use super::boolean::{
    check_general_pair, check_rect_pair, holed_witness, realize, reference_raw_bounds,
    validate_general_output_rings, validate_limits, BoolInput, BoolOutputs, Tagged,
};
use super::clip::{check_clip, near_input};
use super::rect::Rect;

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

fn must_reject<T: std::fmt::Debug>(label: &str, result: Result<T, String>, needle: &str) {
    match result {
        Ok(stats) => panic!("{label}: the checker accepted a wrong result ({stats:?})"),
        Err(message) => assert!(
            message.contains(needle),
            "{label}: rejected for an unexpected reason: {message}"
        ),
    }
}

fn boolean_controls() {
    // ── Fake union returning only operand A (rectangle reference) ─────────
    let (a, b) = (mm(0, 0, 4, 3), mm(2, 1, 6, 5));
    let real = BoolOutputs::compute(&[a.to_expolygon()], &[b.to_expolygon()]);
    let stats = check_rect_pair(a, b, &real).expect("real SUT on the overlapping pair");
    // 3x3 partition (x: 0,2,4,6 mm; y: 0,1,3,5 mm). A covers 4 cells, B 4,
    // they share [2,4]x[1,3]: union 7, intersection 1, A\B 3, xor 6.
    assert_eq!(stats.cells, 9);
    assert_eq!(stats.inside_cells, [7, 1, 3, 6]);
    let u = 10_000;
    let analytic_union = ExPolygon {
        contour: ring(&[
            (0, 0),
            (4 * u, 0),
            (4 * u, u),
            (6 * u, u),
            (6 * u, 5 * u),
            (2 * u, 5 * u),
            (2 * u, 3 * u),
            (0, 3 * u),
        ]),
        holes: Vec::new(),
    };
    let analytic = BoolOutputs {
        union: vec![analytic_union],
        ..real.clone()
    };
    check_rect_pair(a, b, &analytic).expect("the analytically written union passes");
    let fake = BoolOutputs {
        union: vec![a.to_expolygon()],
        ..real.clone()
    };
    must_reject("rect union = A", check_rect_pair(a, b, &fake), "Union");

    // ── Difference that fills the hole (rectangle reference) ──────────────
    let (outer, inner) = (mm(0, 0, 10, 10), mm(2, 2, 4, 4));
    let real = BoolOutputs::compute(&[outer.to_expolygon()], &[inner.to_expolygon()]);
    check_rect_pair(outer, inner, &real).expect("real SUT on the nested pair");
    let filled = BoolOutputs {
        difference: vec![outer.to_expolygon()],
        ..real.clone()
    };
    must_reject(
        "rect difference fills the hole",
        check_rect_pair(outer, inner, &filled),
        "Difference",
    );

    // ── Fake union returning only A (general sample reference) ────────────
    let holed = vec![holed_witness()];
    let bar = vec![mm_units(40, 32, 120, 48)];
    let real = BoolOutputs::compute(&holed, &bar);
    check_general_pair(&holed, &bar, &real, 4096).expect("real SUT: holed vs bar");
    let fake = BoolOutputs {
        union: holed.clone(),
        ..real.clone()
    };
    must_reject(
        "general union = A",
        check_general_pair(&holed, &bar, &fake, 4096),
        "Union",
    );

    // ── Difference that fills the hole (general sample reference) ─────────
    let far = vec![mm_units(200, 200, 240, 240)];
    let real = BoolOutputs::compute(&holed, &far);
    check_general_pair(&holed, &far, &real, 4096).expect("real SUT: holed minus a far bar");
    assert_eq!(real.difference.len(), 1);
    assert_eq!(
        real.difference[0].holes.len(),
        1,
        "the real difference keeps the hole"
    );
    let filled = BoolOutputs {
        difference: vec![ExPolygon {
            contour: holed[0].contour.clone(),
            holes: Vec::new(),
        }],
        ..real.clone()
    };
    must_reject(
        "general difference fills the hole",
        check_general_pair(&holed, &far, &filled, 4096),
        "Difference",
    );
}

fn mm_units(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
    Rect::new(x0, y0, x1, y1)
        .expect("control rectangle")
        .to_expolygon()
}

/// The first vertex of `ring` that occurs twice at non-consecutive indices,
/// i.e. a pinch point of a weakly simple ring.
fn repeated_vertex(ring: &Polygon) -> Option<Point2> {
    let n = ring.points.len();
    for i in 0..n {
        for j in (i + 2)..n {
            if i == 0 && j == n - 1 {
                continue; // the closing pair is adjacent
            }
            if ring.points[i] == ring.points[j] {
                return Some(ring.points[i]);
            }
        }
    }
    None
}

fn single(ring: Polygon) -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: ring,
        holes: Vec::new(),
    }]
}

/// Assert that every vertex of `ring` but `centre` lies on one of the four
/// diagonal rays `(+-t, +-t)`, `t > 0`, seen from `centre`.
///
/// The two `N2` ring-predicate arms are built from exactly these four incident
/// directions at their repeated vertex; only the way the visits pair them
/// differs, which is what separates a touch from a crossing.
fn assert_diagonal_rays(label: &str, ring: &Polygon, centre: Point2) {
    assert!(
        ring.points.iter().filter(|&&point| point == centre).count() >= 2,
        "{label}: {centre:?} must be a repeated vertex, ring {ring:?}"
    );
    for point in ring.points.iter().filter(|&&point| point != centre) {
        let (dx, dy) = (point.x - centre.x, point.y - centre.y);
        assert_eq!(
            dx.abs(),
            dy.abs(),
            "{label}: {point:?} is not on a diagonal ray from {centre:?}, ring {ring:?}"
        );
        assert_ne!(
            dx, 0,
            "{label}: {point:?} must not coincide with {centre:?}, ring {ring:?}"
        );
    }
}

/// Controls for the weakly-simple ring predicate of the general Boolean family.
///
/// Each arm feeds a named shape through **the same predicate** the `H-BOOL-SET`
/// general reference uses (`validate_general_output_rings`): a shape it must
/// accept, and a shape it must reject for the named reason. The predicate is
/// weaker than #2 `validate_ring` (a pinch repeats a vertex), so every arm also
/// pins the boundary of that relaxation rather than trusting it.
fn ring_predicate_controls() {
    // ── Arm 1: the archived `xor` pinch is accepted ───────────────────────
    // This is also M2 arm 1 (the xor defect fix has a regression): the input
    // below is the archived extended-tier `H-BOOL-SET` failure
    // (`target/property-testing/38b3e4d08b55800c4437fa370e73a06d/H-BOOL-SET/
    // failure.json`), whose message was "general pair: Xor[0] contour invalid:
    // invalid domain input: a ring cannot repeat a vertex". The operands share
    // corners and edges; `xor` merges two lobes that meet at one point, so the
    // contour visits that point twice.
    //
    // M2 arm 2 (the `H-MESH-SLICE` thin-prism case) is **not** here: it is
    // owned by the sibling fix in `controls_slice_query.rs`. This ring
    // predicate does not reach the slice family — `H-MESH-SLICE` validates
    // sections with #2 `validate_ring` (see `mesh_slice.rs`), and the only
    // callers of `validate_general_output_rings` are this file's general
    // Boolean reference and `scad_boolean.rs`.
    {
        const ARM: &str = "M1 arm 1 / M2 arm 1 (pinch accepted)";
        let limits = validate_limits(reference_raw_bounds()).expect("H-BOOL-SET reference profile");
        let archived = BoolInput {
            rects: (
                Rect::new(0, 0, 10_000, 10_000).expect("archived rect A"),
                Rect::new(0, 0, 10_000, 10_000).expect("archived rect B"),
            ),
            a: Tagged {
                class: TopologyClass::Holed,
                case: GeometryCase {
                    polygons: vec![
                        ExPolygon {
                            contour: ring(&[(-14, -1), (0, -1), (0, 2), (-14, 2)]),
                            holes: vec![ring(&[(-13, 0), (-13, 1), (-12, 1), (-12, 0)])],
                        },
                        ExPolygon {
                            contour: ring(&[(1, -1), (2, -1), (2, 0), (1, 0)]),
                            holes: Vec::new(),
                        },
                    ],
                },
            },
            b: Tagged {
                class: TopologyClass::Concave,
                case: GeometryCase {
                    polygons: vec![ExPolygon {
                        contour: ring(&[(-3, -2), (3, -2), (3, 1), (2, 1), (2, 2), (-3, 2)]),
                        holes: vec![
                            ring(&[(-2, -1), (-2, 0), (-1, 0), (-1, -1)]),
                            ring(&[(0, -1), (0, 0), (1, 0), (1, -1)]),
                        ],
                    }],
                },
            },
            rel_b: (11, 0),
            copy: (0, 0),
            base: (0, 0),
        };
        let realized = realize(&archived, &limits)
            .unwrap_or_else(|error| panic!("{ARM}: archived input must realise: {error}"));
        let out = BoolOutputs::compute(&realized.a, &realized.b);
        let pinch = out
            .xor
            .iter()
            .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
            .find_map(|ring| repeated_vertex(ring).map(|vertex| (ring, vertex)))
            .unwrap_or_else(|| panic!("{ARM}: the SUT output no longer pinches: {:?}", out.xor));
        let (pinched, vertex) = pinch;
        // The pinch is real: the strict simple-ring predicate rejects this very
        // ring with the archived failure's message, so the acceptance below is
        // the relaxation, not a ring that was always simple.
        let strict = validate_ring(pinched)
            .expect_err("the pinched ring repeats a vertex, so #2 `validate_ring` must reject it");
        assert!(
            strict.to_string().contains("repeat a vertex"),
            "{ARM}: unexpected strict rejection: {strict}"
        );
        // The good case passes the same predicate, and the whole previously
        // failing reference (`general pair`, all four operations) passes.
        validate_general_output_rings("Xor", &out.xor)
            .unwrap_or_else(|error| panic!("{ARM}: the pinch must be accepted: {error}"));
        check_general_pair(&realized.a, &realized.b, &out, 4096)
            .unwrap_or_else(|error| panic!("{ARM}: the archived failure must pass: {error}"));
        // The pinch vertex is independently derivable: B's second hole occupies
        // `[112,120] x [8,16]` and the notch that A's second component cuts out
        // of B occupies `[120,128] x [0,8]`, so the two excluded regions meet at
        // the single point (120, 8) and the xor material is pinched there: the
        // `Xor[0]` contour visits that point at indices 8 and 12, four edges
        // apart.
        assert_eq!(
            vertex,
            Point2 { x: 120, y: 8 },
            "{ARM}: the archived input's pinch vertex moved: {pinched:?}"
        );
    }

    // ── Arm 2: a bowtie is rejected by a proper crossing ──────────────────
    {
        const ARM: &str = "M1 arm 2 (bowtie rejected)";
        let bowtie = ring(&[(0, 0), (10, 10), (10, 0), (0, 20)]);
        // Double area +100 is positive, so the winding gate alone accepts it:
        // only the crossing rule can reject this ring.
        assert_eq!(
            signed_area2(&bowtie),
            Ok(100),
            "{ARM}: the bowtie must clear the area and winding gates"
        );
        must_reject(
            ARM,
            validate_general_output_rings("bowtie", &single(bowtie)),
            "properly cross",
        );
        // The same four vertices in a crossing-free order pass the predicate.
        let untangled = ring(&[(0, 0), (10, 0), (10, 10), (0, 20)]);
        assert_eq!(signed_area2(&untangled), Ok(300));
        validate_general_output_rings("bowtie (untangled)", &single(untangled))
            .expect("the crossing-free order of the same four vertices passes");
    }

    // ── Arm 3: a collinear backtracking spike is rejected ────────────────
    {
        const ARM: &str = "M1 arm 3 (spike rejected)";
        // (10,0) is a spike tip: (0,0) -> (10,0) -> (5,0) has zero turn and
        // reverses, so the boundary covers [5,10]x{0} twice.
        let spiked = ring(&[(0, 0), (10, 0), (5, 0), (5, 10), (0, 10)]);
        assert_eq!(
            signed_area2(&spiked),
            Ok(100),
            "{ARM}: the spiked ring must clear the area and winding gates"
        );
        must_reject(
            ARM,
            validate_general_output_rings("spike", &single(spiked)),
            "backtracks",
        );
        // The same ring without the spike tip, with the same double area, passes.
        let square = ring(&[(0, 0), (5, 0), (5, 10), (0, 10)]);
        assert_eq!(signed_area2(&square), Ok(100));
        validate_general_output_rings("spike (no tip)", &single(square))
            .expect("the same ring without the spike passes");
        // A collinear vertex that goes *forward* is a redundant vertex, not a
        // spike, and must stay accepted: the rule rejects reversal, not
        // collinearity.
        let collinear = ring(&[(0, 0), (3, 0), (5, 0), (5, 10), (0, 10)]);
        assert_eq!(signed_area2(&collinear), Ok(100));
        validate_general_output_rings("spike (forward collinear vertex)", &single(collinear))
            .expect("a forward collinear vertex is accepted");
    }

    // ── Arm 4: a figure-8 whose crossing point is a *vertex* is rejected ──
    // Every edge pair of this ring either shares the repeated vertex `(0,0)` or
    // is disjoint, so a per-edge-pair test sees no proper crossing at all: the
    // four edges at `(0,0)` are `(-1,-1)->(0,0)`, `(0,0)->(2,2)`,
    // `(1,-1)->(0,0)` and `(0,0)->(-3,3)`, and their far endpoints lie on the
    // four diagonal rays `(+-1,+-1)` in the cyclic order 45, 135, 225, 315. The
    // visits pair those directions **alternately** — visit 0 uses `45` and
    // `225`, visit 1 uses `135` and `315` — so the boundary crosses at the
    // vertex: the lobes have opposite winding and unequal sizes (double areas
    // `-4` and `+6`), which leaves a nonzero total double area of `+2`, so the
    // area and winding gates accept the ring and only the interleaving rule can
    // reject it.
    {
        const ARM: &str = "N2 arm 4 (vertex crossing rejected)";
        let crossing = ring(&[(0, 0), (2, 2), (1, -1), (0, 0), (-3, 3), (-1, -1)]);
        assert_eq!(
            signed_area2(&crossing),
            Ok(2),
            "{ARM}: the crossing must clear the area and winding gates"
        );
        assert_diagonal_rays(ARM, &crossing, pt(0, 0));
        must_reject(
            ARM,
            validate_general_output_rings("vertex crossing", &single(crossing)),
            "crosses at vertex",
        );
    }

    // ── Arm 5: the same four directions, paired without interleaving, pass ──
    // This is the legitimate pinch: the two lobes of the region meet at the
    // single vertex `(0,0)`, each visit keeps its own pair of adjacent
    // directions (`135`/`225` and `315`/`45`), so no pass crosses the other and
    // the ring is weakly simple. It repeats the vertex, so the strict #2
    // `validate_ring` still rejects it — the acceptance below is the
    // relaxation, not a ring that was always simple.
    {
        const ARM: &str = "N2 arm 5 (non-interleaved pinch accepted)";
        let pinch = ring(&[(0, 0), (-1, -1), (1, -1), (0, 0), (1, 1), (-1, 1)]);
        assert_eq!(
            signed_area2(&pinch),
            Ok(4),
            "{ARM}: the pinch must clear the area and winding gates"
        );
        assert_diagonal_rays(ARM, &pinch, pt(0, 0));
        let strict = validate_ring(&pinch)
            .expect_err("the pinch repeats a vertex, so #2 `validate_ring` must reject it");
        assert!(
            strict.to_string().contains("repeat a vertex"),
            "{ARM}: unexpected strict rejection: {strict}"
        );
        validate_general_output_rings("pinch", &single(pinch)).unwrap_or_else(|error| {
            panic!("{ARM}: a non-interleaved repeated vertex must be accepted: {error}")
        });
    }
}

/// `[0,1000]²` with the hole `[200,400] x [400,600]` (canonical units).
fn holed_clip() -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: Rect::new(0, 0, 1000, 1000).expect("outer").ring(),
        holes: vec![Rect::new(200, 400, 400, 600).expect("hole").hole_ring()],
    }]
}

/// `[0,1000]² \ [500,1000] x [500,1000]` (canonical units, concave).
fn l_clip() -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: ring(&[
            (0, 0),
            (1000, 0),
            (1000, 500),
            (500, 500),
            (500, 1000),
            (0, 1000),
        ]),
        holes: Vec::new(),
    }]
}

fn assert_naively_inside(label: &str, clip: &[ExPolygon], points: &[Point2]) {
    for &p in points {
        assert_eq!(
            point_in_region(p, clip),
            Ok(PointLocation::Inside),
            "{label}: {p:?} must be inside for the counterexample to bite"
        );
    }
}

fn clip_controls() {
    // ── Holed: endpoints and midpoint inside, an interval exits ──────────
    let clip = holed_clip();
    let input = vec![vec![pt(100, 500), pt(900, 500)]];
    assert_naively_inside(
        "holed counterexample",
        &clip,
        &[pt(100, 500), pt(900, 500), pt(500, 500)],
    );
    let real = clip_polylines(&input, &clip);
    let stats = check_clip(&input, &clip, &real, 32).expect("real SUT through the hole");
    // [100,201] and [399,900] retained; (203,397) certainly in the hole.
    assert!(stats.keep_len > 0 && stats.drop_len > 0, "{stats:?}");
    // R' contracts the hole by one unit: [201, 399].
    let analytic = vec![
        vec![pt(100, 500), pt(201, 500)],
        vec![pt(399, 500), pt(900, 500)],
    ];
    check_clip(&input, &clip, &analytic, 32).expect("the analytic split passes");
    must_reject(
        "shortcut through the hole",
        check_clip(&input, &clip, &input, 32),
        "emitted span",
    );
    must_reject(
        "omitted interior span",
        check_clip(&input, &clip, &analytic[..1], 32),
        "emitted span",
    );
    // An endpoint more than ±2 units past the R' crossing (x = 201) is
    // rejected: 205 is strictly inside the hole of offset(R, INFLATE + BAND).
    let overshoot = vec![
        vec![pt(100, 500), pt(205, 500)],
        vec![pt(399, 500), pt(900, 500)],
    ];
    must_reject(
        "endpoint past the band",
        check_clip(&input, &clip, &overshoot, 32),
        "allowance",
    );
    // At the band edge (203, i.e. R' crossing + 2) the allowance accepts it.
    let within = vec![
        vec![pt(100, 500), pt(203, 500)],
        vec![pt(397, 500), pt(900, 500)],
    ];
    check_clip(&input, &clip, &within, 32).expect("endpoints within ±2 of R' pass");

    // ── Concave: a polyline around the notch, and a shortcut chord ────────
    let clip = l_clip();
    let around = vec![vec![pt(100, 900), pt(100, 100), pt(900, 100)]];
    let real = clip_polylines(&around, &clip);
    check_clip(&around, &clip, &real, 32).expect("real SUT around the notch");
    check_clip(&around, &clip, &around, 32).expect("the whole inside polyline passes");
    let chord = vec![vec![pt(100, 900), pt(900, 100)]];
    must_reject(
        "shortcut chord across the concavity",
        check_clip(&around, &clip, &chord, 32),
        "lies on no input segment",
    );
    // Straight through the notch: only [100, 501] is retained.
    let across = vec![vec![pt(100, 700), pt(900, 700)]];
    let real = clip_polylines(&across, &clip);
    check_clip(&across, &clip, &real, 32).expect("real SUT across the notch");
    // Its far endpoint lies inside the notch, so the allowance check fires.
    must_reject(
        "notch not removed",
        check_clip(&across, &clip, &across, 32),
        "allowance",
    );

    // ── Concave, non-axis: endpoints and midpoint inside, an interval exits
    // Parameter t in (50/540, 480/970) lies in the notch x > 500, y > 500.
    let diagonal = vec![vec![pt(450, 980), pt(990, 10)]];
    assert_naively_inside(
        "L counterexample",
        &clip,
        &[pt(450, 980), pt(990, 10), pt(720, 495)],
    );
    let real = clip_polylines(&diagonal, &clip);
    assert_eq!(
        real.len(),
        2,
        "the diagonal leaves and re-enters the L: {real:?}"
    );
    let stats = check_clip(&diagonal, &clip, &real, 64).expect("real SUT on the L diagonal");
    assert!(
        stats.diag_keep_samples > 0 && stats.diag_drop_samples > 0,
        "{stats:?}"
    );
    must_reject(
        "diagonal shortcut through the notch",
        check_clip(&diagonal, &clip, &diagonal, 64),
        "leaves R'",
    );
    let mut first_only = real.clone();
    first_only.truncate(1);
    must_reject(
        "diagonal omitted span",
        check_clip(&diagonal, &clip, &first_only, 64),
        "dropped span",
    );
}

/// Pins the ±`BAND` (2-unit) perpendicular attribution band from both sides.
///
/// (a) The `H-BOOL-CLIP` wide-pilot witness (seed 4242): Clipper2 emits the
/// notch crossing at (0, 84004), 1.41 units off the input line and 0 units
/// from the original boundary x = 0 — inside the documented ±2 allowance — so
/// the checker must accept the real output. (b) A piece displaced just over 2
/// units is rejected by the attribution arm. (c) Short and empty paths still
/// yield nothing.
fn clip_band_controls() {
    let clip = vec![ExPolygon {
        contour: ring(&[
            (-218_750, -218_750),
            (187_500, -218_750),
            (187_500, 0),
            (0, 0),
            (0, 187_500),
            (-218_750, 187_500),
        ]),
        holes: Vec::new(),
    }];
    let witness = vec![pt(-55_783, 146_645), pt(279_653, -230_017)];
    let real = clip_polylines(&[witness.clone()], &clip);
    assert_eq!(
        real,
        vec![
            vec![pt(-55_783, 146_645), pt(0, 84_004)],
            vec![pt(74_811, 1), pt(187_501, -126_539)],
        ],
        "(a) finding witness: the SUT output this control pins has changed"
    );
    let stats = check_clip(&[witness.clone()], &clip, &real, 64)
        .unwrap_or_else(|error| panic!("(a) finding witness must be accepted: {error}"));
    assert!(
        stats.diag_keep_samples > 0 && stats.diag_drop_samples > 0 && stats.matched_pieces == 2,
        "(a) finding witness populations: {stats:?}"
    );

    // (b) Displace the first real piece by (2, 1): its start is then
    // |2·(-376662) - 1·335436| / |(335436, -376662)| = 1088760 / 504382.3...
    // ≈ 2.159 units from the input line, beyond the 2-unit band. Both
    // vertices stay inside offset(R, 3), so only the attribution arm can fire.
    let displaced = vec![
        vec![pt(-55_781, 146_646), pt(2, 84_005)],
        vec![pt(74_811, 1), pt(187_501, -126_539)],
    ];
    must_reject(
        "(b) piece displaced 2.159 units from its input line",
        check_clip(&[witness.clone()], &clip, &displaced, 64),
        "lies on no input segment",
    );
    // The band edge itself, on a 3-4-5 line where perpendicular distances are
    // exact multiples of 1/5 unit (2.5 is not representable for integer
    // points off any non-axis integer line: it needs an even primitive
    // hypotenuse). Offset (1, -2) is exactly 2.0 units off, (2, -1) is 2.2.
    let (p0, p1) = (pt(10_000, 10_000), pt(40_000, 50_000));
    assert_eq!(
        near_input(p0, p1, pt(25_001, 29_998)),
        Ok(true),
        "(b) a point exactly 2.0 units from the line is inside the band"
    );
    assert_eq!(
        near_input(p0, p1, pt(25_002, 29_999)),
        Ok(false),
        "(b) a point 2.2 units from the line is outside the band"
    );

    // (c) A one-point path and an empty path yield no output, alone and
    // alongside the witness.
    let inside = pt(-100_000, -100_000);
    assert!(
        clip_polylines(&[Vec::new(), vec![inside]], &clip).is_empty(),
        "(c) short and empty paths must produce no output"
    );
    let batch = vec![witness, Vec::new(), vec![inside]];
    let real = clip_polylines(&batch, &clip);
    assert_eq!(
        real.len(),
        2,
        "(c) only the witness produces pieces: {real:?}"
    );
    check_clip(&batch, &clip, &real, 64)
        .unwrap_or_else(|error| panic!("(c) batch with short/empty paths: {error}"));
}

/// Body of `geometry_controls_boolean_and_clip_mutations`.
pub fn run() {
    boolean_controls();
    ring_predicate_controls();
    clip_controls();
    clip_band_controls();
}
