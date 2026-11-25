#![allow(missing_docs)]

//! Exactness oracles for the `PrePass::ShellClassification` accelerations
//! (perf-vs-orca ticket 184).
//!
//! Every accelerated routine here must return the *bit-identical* value of the
//! brute-force computation it replaces. The brute-force versions below are the
//! pre-acceleration algorithms, kept verbatim as oracles, and each test also
//! carries a negative control or a non-vacuity count showing the fixture can
//! tell a wrong answer from a right one (`docs/22_test_quality.md`).

use slicer_core::algos::bridge_over_infill::{
    boundary_anchor_polylines, determine_bridging_angle, grow_box, internal_bridge_angles,
    polygons_box, polygons_touching_box, qualify_internal_bridge_surface, unsupported_span_areas,
    unsupported_span_areas_near, InternalBridgeAngleInputs,
};
use slicer_core::algos::prepass_slice::{
    min_cost_bridging_candidate, min_cost_bridging_candidate_reference,
};
use slicer_core::polygon_ops::{
    closing_ex, difference, intersection, offset, union, OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon};
use std::f32::consts::PI;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Deterministic xorshift64* generator; fixtures never depend on a seed the
/// test does not state.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn coord(&mut self, span: i64, grid: i64) -> i64 {
        (self.below((span / grid) as u64 + 1) as i64) * grid
    }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

/// Clockwise rectangle ring, for use as a hole.
fn hole(x0: f32, y0: f32, x1: f32, y1: f32) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(x0, y0),
            Point2::from_mm(x0, y1),
            Point2::from_mm(x1, y1),
            Point2::from_mm(x1, y0),
        ],
    }
}

fn polyline(points: &[(i64, i64)]) -> Vec<Point2> {
    points.iter().map(|&(x, y)| Point2 { x, y }).collect()
}

// ---------------------------------------------------------------------------
// Brute-force oracles (the pre-acceleration algorithms)
// ---------------------------------------------------------------------------

const SAMPLE_STEP: f64 = 20_000.0;
const SCALED_EPSILON_MM: f64 = 0.0001;

/// Pre-acceleration nearest-anchor scan. Also reports whether the minimum was
/// shared by more than one segment, so tests can prove tie-breaking ran.
fn reference_nearest_anchor_angle(anchors: &[Vec<Point2>], p: (f64, f64)) -> (Option<f64>, bool) {
    let mut scanned: Vec<(f64, f64)> = Vec::new(); // (distance2, angle) in scan order
    let mut best: Option<(f64, f64, f64)> = None;
    for line in anchors {
        for pair in line.windows(2) {
            let ax = pair[0].x as f64;
            let ay = pair[0].y as f64;
            let dx = (pair[1].x - pair[0].x) as f64;
            let dy = (pair[1].y - pair[0].y) as f64;
            let len2 = dx * dx + dy * dy;
            if len2 == 0.0 {
                continue;
            }
            let t = (((p.0 - ax) * dx + (p.1 - ay) * dy) / len2).clamp(0.0, 1.0);
            let distance2 = (p.0 - ax - t * dx).powi(2) + (p.1 - ay - t * dy).powi(2);
            let angle = (dy.atan2(dx) + PI as f64 / 2.0).rem_euclid(PI as f64);
            scanned.push((distance2, angle));
            if best.is_none_or(|(d, _, _): (f64, f64, f64)| distance2 < d) {
                best = Some((distance2, angle, len2));
            }
        }
    }
    // A tie that matters: another segment at the same minimal distance with a
    // different angle, which only the first-segment rule decides.
    let tied = best.is_some_and(|(d, angle, _)| {
        scanned
            .iter()
            .any(|&(other_d, other_angle)| other_d == d && other_angle != angle)
    });
    (best.map(|(_, angle, _)| angle), tied)
}

/// Pre-acceleration `determine_bridging_angle`: `O(S × A)` scan, `O(S²)` vote.
/// Returns the angle and the number of samples whose nearest anchor was tied.
fn reference_bridging_angle(
    anchors: &[Vec<Point2>],
    area_edges: &[Vec<Point2>],
    override_deg: f32,
) -> (f32, usize) {
    if override_deg > 0.0 {
        return (override_deg, 0);
    }
    let mut samples = Vec::new();
    let mut ties = 0;
    for edge in area_edges {
        for pair in edge.windows(2) {
            let dx = (pair[1].x - pair[0].x) as f64;
            let dy = (pair[1].y - pair[0].y) as f64;
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            let count = (length / SAMPLE_STEP).ceil().max(1.0) as usize;
            for n in 0..count {
                let t = ((n as f64 + 0.5) / count as f64).min(1.0);
                let p = (pair[0].x as f64 + dx * t, pair[0].y as f64 + dy * t);
                let (angle, tied) = reference_nearest_anchor_angle(anchors, p);
                ties += usize::from(tied);
                if let Some(angle) = angle {
                    samples.push(angle);
                }
            }
        }
    }
    if samples.is_empty() {
        return (0.001, ties);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    let window = 18.0_f64.to_radians();
    let mut best_score = 0usize;
    let mut best_angle = 0.0;
    for &candidate in &samples {
        let mut score = 0usize;
        let mut sum = 0.0;
        for &direction in &samples {
            let mut delta = (direction - candidate).rem_euclid(PI as f64);
            if delta > PI as f64 / 2.0 {
                delta -= PI as f64;
            }
            if delta.abs() <= window {
                score += 1;
                sum += candidate + delta;
            }
        }
        if score > best_score {
            best_score = score;
            best_angle = (sum / score as f64).rem_euclid(PI as f64);
        }
    }
    (
        (best_angle.to_degrees() as f32)
            .rem_euclid(180.0)
            .max(0.001),
        ties,
    )
}

/// Pre-acceleration `internal_bridge_angles`. `update_expansion = false` is
/// the negative control: it skips the per-candidate expansion-area update.
fn reference_internal_bridge_angles(
    candidates: &[ExPolygon],
    inputs: &InternalBridgeAngleInputs<'_>,
    update_expansion: bool,
) -> Vec<f32> {
    let spacing = inputs.spacing_mm;
    let total_fill_area = closing_ex(
        inputs.total_fill_area,
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    let mut expansion_area = intersection(
        &closing_ex(
            inputs.expansion_area,
            SCALED_EPSILON_MM,
            OffsetJoinType::Miter,
        ),
        inputs.deep_infill_clip_area,
    );
    let mut angles: Vec<f32> = Vec::new();
    let mut bridge_areas: Vec<Vec<ExPolygon>> = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let candidate = std::slice::from_ref(candidate);
        let mut area: Vec<ExPolygon> = intersection(
            &offset(candidate, spacing, OffsetJoinType::Miter, 0.0),
            inputs.deep_infill_clip_area,
        )
        .into_iter()
        .filter(|part| {
            !intersection(std::slice::from_ref(part), inputs.internal_unsupported_area).is_empty()
        })
        .collect();
        if area.is_empty() {
            area = candidate.to_vec();
        }
        let limiting_area = union(&area, &expansion_area);
        let anchors = boundary_anchor_polylines(&total_fill_area, &limiting_area, spacing);
        let edges: Vec<Vec<Point2>> = area
            .iter()
            .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
            .map(|ring| ring.points.clone())
            .collect();
        let (mut angle, _) = reference_bridging_angle(&anchors, &edges, inputs.override_deg);
        let grown = offset(&area, 3.0 * spacing, OffsetJoinType::Miter, 0.0);
        if let Some(earlier) = bridge_areas[..index]
            .iter()
            .position(|earlier| !intersection(earlier, &grown).is_empty())
        {
            angle = angles[earlier];
        }
        angles.push(angle);
        if update_expansion {
            expansion_area = difference(&expansion_area, &area);
        }
        bridge_areas.push(area);
    }
    angles
}

/// Candidate normals exactly as `detect_bridging_direction_deg` builds them.
fn production_candidates(edges: &[(f64, f64)]) -> Vec<(i64, f64, f64)> {
    let mut candidates: Vec<(i64, f64, f64)> = Vec::new();
    for &(dx, dy) in edges {
        let len = dx.hypot(dy);
        if len == 0.0 {
            continue;
        }
        let nx = dy / len;
        let ny = -dx / len;
        let nx = if nx == 0.0 { 0.0 } else { nx };
        let ny = if ny == 0.0 { 0.0 } else { ny };
        let key = (ny.atan2(nx) * 1000.0).ceil() as i64;
        if !candidates.iter().any(|&(k, _, _)| k == key) {
            candidates.push((key, nx, ny));
        }
    }
    candidates
}

fn brute_costs(edges: &[(f64, f64)], candidates: &[(i64, f64, f64)]) -> Vec<f64> {
    candidates
        .iter()
        .map(|&(_, nx, ny)| {
            edges
                .iter()
                .map(|&(dx, dy)| (dx * nx + dy * ny).abs())
                .sum::<f64>()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// determine_bridging_angle: BVH nearest anchor + windowed vote
// ---------------------------------------------------------------------------

#[test]
fn fast_bridging_angle_matches_brute_force_on_random_anchor_sets() {
    let mut rng = Rng(0x0184_5eed_0000_0001);
    let mut tied_samples = 0usize;
    let mut large_anchor_sets = 0usize;
    for case in 0..400 {
        // Coarse grids force equidistant segments, collinear runs and
        // duplicate points; fine grids exercise general positions.
        let grid = [1, 10, 2_500, 10_000][case % 4];
        let span = 200_000;
        let mut anchors = Vec::new();
        let mut segment_count = 0;
        for _ in 0..1 + rng.below(6) {
            let len = 2 + rng.below(24) as usize;
            let mut line = Vec::with_capacity(len);
            for _ in 0..len {
                // One point in eight repeats its predecessor: a degenerate
                // segment the scan must skip.
                if !line.is_empty() && rng.below(8) == 0 {
                    let last = *line.last().unwrap();
                    line.push(last);
                } else {
                    line.push(Point2 {
                        x: rng.coord(span, grid),
                        y: rng.coord(span, grid),
                    });
                }
            }
            segment_count += line.len() - 1;
            anchors.push(line);
        }
        large_anchor_sets += usize::from(segment_count > 16);
        let mut edges = Vec::new();
        for _ in 0..1 + rng.below(3) {
            let len = 3 + rng.below(8) as usize;
            edges.push(
                (0..len)
                    .map(|_| Point2 {
                        x: rng.coord(span, grid),
                        y: rng.coord(span, grid),
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let (want, ties) = reference_bridging_angle(&anchors, &edges, 0.0);
        tied_samples += ties;
        let got = determine_bridging_angle(&anchors, &edges, 0.0);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "case {case}: fast angle {got} != brute-force {want}"
        );
    }
    assert!(
        tied_samples > 0,
        "no sample had an equidistant nearest-anchor tie; the tie rule went unexercised"
    );
    assert!(
        large_anchor_sets > 100,
        "too few anchor sets were large enough to need hierarchy pruning ({large_anchor_sets})"
    );
}

#[test]
fn fast_bridging_angle_keeps_the_first_of_two_equidistant_anchors() {
    // One sample at the origin, equidistant (1 mm) from a horizontal and a
    // vertical anchor; the first segment scanned must win.
    let horizontal = polyline(&[(-10_000, 10_000), (10_000, 10_000)]);
    let vertical = polyline(&[(10_000, -10_000), (10_000, 10_000)]);
    let edge = vec![polyline(&[(-1, 0), (1, 0)])];
    let horizontal_first = vec![horizontal.clone(), vertical.clone()];
    let vertical_first = vec![vertical, horizontal];

    let (want_h, ties_h) = reference_bridging_angle(&horizontal_first, &edge, 0.0);
    let (want_v, _) = reference_bridging_angle(&vertical_first, &edge, 0.0);
    assert_eq!(ties_h, 1, "fixture must produce exactly one tied sample");
    assert_ne!(
        want_h.to_bits(),
        want_v.to_bits(),
        "the anchor order must decide this fixture, or it cannot detect a wrong tie rule"
    );
    assert_eq!(
        determine_bridging_angle(&horizontal_first, &edge, 0.0).to_bits(),
        want_h.to_bits()
    );
    assert_eq!(
        determine_bridging_angle(&vertical_first, &edge, 0.0).to_bits(),
        want_v.to_bits()
    );
}

#[test]
fn fast_bridging_angle_votes_across_the_zero_and_180_degree_seam() {
    // Anchors at roughly +-3 degrees off horizontal: their perpendicular
    // directions straddle 90 degrees; rotating by 90 puts the votes on both
    // sides of the 0/180 seam, where the window must wrap.
    for (rotate, label) in [(false, "straddling 90"), (true, "straddling 0/180")] {
        let mut anchors = Vec::new();
        for i in 0..12i64 {
            let tilt = if i % 2 == 0 { 520 } else { -520 };
            let (x0, y0, x1, y1) = (i * 15_000, 0, i * 15_000 + 10_000, tilt);
            let line = if rotate {
                polyline(&[(y0, x0), (y1, x1)])
            } else {
                polyline(&[(x0, y0), (x1, y1)])
            };
            anchors.push(line);
        }
        let edge = if rotate {
            vec![polyline(&[(20_000, 0), (20_000, 180_000)])]
        } else {
            vec![polyline(&[(0, 20_000), (180_000, 20_000)])]
        };
        let (want, _) = reference_bridging_angle(&anchors, &edge, 0.0);
        let got = determine_bridging_angle(&anchors, &edge, 0.0);
        assert_eq!(got.to_bits(), want.to_bits(), "{label}: {got} != {want}");
    }
}

// ---------------------------------------------------------------------------
// internal_bridge_angles: hoisted anchors, skipped dead operations
// ---------------------------------------------------------------------------

#[test]
fn internal_bridge_angles_match_brute_force_with_expansion_updates_between_candidates() {
    let big = rect(0.0, 0.0, 60.0, 30.0);
    let field = [big];
    // exhaustive: InternalBridgeAngleInputs has no Default impl; every field is intentional here
    let inputs = InternalBridgeAngleInputs {
        deep_infill_clip_area: &field,
        internal_unsupported_area: &field,
        expansion_area: &field,
        total_fill_area: &field,
        spacing_mm: 0.45,
        override_deg: 0.0,
    };
    // A tall candidate, then a wide one 4 mm to its right (beyond the
    // collision radius) whose nearest anchors become the hole the first
    // candidate's area leaves in the expansion area, and a third far away.
    let candidates = vec![
        rect(10.0, 5.0, 20.0, 25.0),
        rect(24.0, 10.0, 34.0, 14.0),
        rect(45.0, 20.0, 55.0, 24.0),
    ];
    let want = reference_internal_bridge_angles(&candidates, &inputs, true);
    let without_update = reference_internal_bridge_angles(&candidates, &inputs, false);
    assert_ne!(
        want[1].to_bits(),
        without_update[1].to_bits(),
        "the second candidate's angle must depend on the first candidate's expansion update"
    );
    let got = internal_bridge_angles(&candidates, &inputs);
    let bits = |angles: &[f32]| angles.iter().map(|a| a.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&got), bits(&want), "got {got:?}, want {want:?}");

    // One candidate: the skipped final update and collision offset are the
    // only differences from the brute force, and neither is observable.
    let single = &candidates[..1];
    assert_eq!(
        bits(&internal_bridge_angles(single, &inputs)),
        bits(&reference_internal_bridge_angles(single, &inputs, true))
    );
}

// ---------------------------------------------------------------------------
// detect_bridging_direction_deg's candidate selector
// ---------------------------------------------------------------------------

#[test]
fn pruned_bridging_selector_matches_brute_force_including_exact_ties() {
    let mut rng = Rng(0x0184_5eed_0000_0002);
    let mut exact_ties = 0usize;
    let mut cases = Vec::new();
    // Shapes symmetric under a quarter turn: pairs of candidates have equal
    // real-number cost and differ, if at all, only in summation rounding.
    for scale in [1i64, 3, 7, 10_000, 123_457] {
        let side = [
            (2, 0),
            (1, 1),
            (0, 2),
            (-1, 1),
            (-2, 0),
            (-1, -1),
            (0, -2),
            (1, -1),
        ];
        let edges: Vec<(f64, f64)> = side
            .iter()
            .flat_map(|&(x, y)| {
                let (x, y) = ((x * scale) as f64, (y * scale) as f64);
                [(x / 2.0, y / 2.0), (x / 2.0, y / 2.0)]
            })
            .collect();
        cases.push(edges);
    }
    // Random edge sets closed under a quarter turn, shuffled: the four rotated
    // copies of the best normal tie in exact arithmetic, and their summed
    // costs differ only in rounding. These are the cases where a selector that
    // trusted its prefix-sum estimate would pick a different candidate.
    for _ in 0..300 {
        let base: Vec<(i64, i64)> = (0..4 + rng.below(20))
            .map(|_| {
                (
                    rng.coord(200_000, 1) - 100_000,
                    rng.coord(200_000, 1) - 100_000,
                )
            })
            .collect();
        let mut edges: Vec<(f64, f64)> = base
            .iter()
            .flat_map(|&(x, y)| [(x, y), (-y, x), (-x, -y), (y, -x)])
            .map(|(x, y)| (x as f64, y as f64))
            .collect();
        for i in (1..edges.len()).rev() {
            edges.swap(i, rng.below(i as u64 + 1) as usize);
        }
        cases.push(edges);
    }
    // Random integer edge sets, coarse and fine.
    for case in 0..300 {
        let grid = [1, 100, 10_000][case % 3];
        let count = 9 + rng.below(60) as usize;
        let edges: Vec<(f64, f64)> = (0..count)
            .map(|_| {
                let dx = rng.coord(400_000, grid) - 200_000;
                let dy = rng.coord(400_000, grid) - 200_000;
                (dx as f64, dy as f64)
            })
            .collect();
        cases.push(edges);
    }
    let mut pruned_path = 0usize;
    let mut rounding_ties = 0usize;
    for (index, edges) in cases.iter().enumerate() {
        let candidates = production_candidates(edges);
        if candidates.is_empty() {
            continue;
        }
        let costs = brute_costs(edges, &candidates);
        let min = costs.iter().copied().fold(f64::INFINITY, f64::min);
        exact_ties += usize::from(costs.iter().filter(|&&c| c == min).count() > 1);
        rounding_ties += usize::from(
            costs
                .iter()
                .any(|&c| c != min && (c - min) <= 1e-12 * min.abs()),
        );
        pruned_path += usize::from(candidates.len() > 4 && edges.len() > 8);
        let want = min_cost_bridging_candidate_reference(edges, &candidates);
        let got = min_cost_bridging_candidate(edges, &candidates);
        assert_eq!(
            (got.0, got.1.to_bits(), got.2.to_bits()),
            (want.0, want.1.to_bits(), want.2.to_bits()),
            "case {index}"
        );
    }
    assert!(
        exact_ties > 0,
        "no fixture had an exact-cost tie; the key tie-break went unexercised"
    );
    assert!(
        rounding_ties > 0,
        "no fixture had a rounding-level near tie; the exact re-scoring went unexercised"
    );
    assert!(
        pruned_path > 500,
        "too few fixtures reached the pruned path ({pruned_path})"
    );
}

// ---------------------------------------------------------------------------
// unsupported_span_areas_near: exact polygon/hole filtering
// ---------------------------------------------------------------------------

/// Lower-layer fills and solids with geometry near the surfaces (holes and a
/// solid patch close enough to change the unsupported set over them, an acute
/// spike straddling the window boundary) and far away (holes and islands that
/// cannot).
fn lower_layer_fixture() -> (Vec<ExPolygon>, Vec<ExPolygon>, Vec<ExPolygon>) {
    let mut plate = rect(0.0, 0.0, 200.0, 200.0);
    // Near: 1 mm from the surface block's right side.
    plate.holes.push(hole(51.0, 22.0, 54.0, 26.0));
    // Far: a grid of holes the surfaces cannot see.
    for i in 0..6 {
        for j in 0..5 {
            let (x, y) = (130.0 + 10.0 * i as f32, 20.0 + 30.0 * j as f32);
            plate.holes.push(hole(x, y, x + 4.0, y + 4.0));
        }
    }
    let spike = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(20.0, 2.0),
                Point2::from_mm(110.0, 4.0),
                Point2::from_mm(20.0, 3.0),
            ],
        },
        holes: Vec::new(),
    };
    let fills = vec![
        plate,
        rect(260.0, 0.0, 300.0, 40.0),
        rect(0.0, 250.0, 30.0, 290.0),
    ];
    let solids = vec![
        // Near: its shrink-grow reaches into the surface block.
        rect(29.0, 51.0, 40.0, 58.0),
        spike,
        // Far.
        rect(150.0, 150.0, 180.0, 180.0),
        rect(270.0, 10.0, 290.0, 30.0),
    ];
    let surfaces = vec![
        rect(20.0, 20.0, 50.0, 50.0),
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(25.0, 25.0),
                    Point2::from_mm(45.0, 26.0),
                    Point2::from_mm(25.0, 27.0),
                ],
            },
            holes: Vec::new(),
        },
    ];
    (fills, solids, surfaces)
}

#[test]
fn windowed_unsupported_spans_equal_the_full_computation_over_every_surface() {
    let (fills, solids, surfaces) = lower_layer_fixture();
    let spacing = 0.45;
    for multiplier in [1.0, 3.0] {
        let full = unsupported_span_areas(&fills, &solids, spacing, multiplier);
        let near = unsupported_span_areas_near(&fills, &solids, spacing, multiplier, &surfaces);
        assert_ne!(
            full, near,
            "m={multiplier}: nothing was dropped, so nothing was tested"
        );
        for (index, surface) in surfaces.iter().enumerate() {
            let surface = std::slice::from_ref(surface);
            assert_eq!(
                intersection(surface, &near),
                intersection(surface, &full),
                "m={multiplier}: surface {index}: unsupported set over the surface changed"
            );
            for nofilter in [false, true] {
                assert_eq!(
                    qualify_internal_bridge_surface(&surface[0], &near, spacing, nofilter),
                    qualify_internal_bridge_surface(&surface[0], &full, spacing, nofilter),
                    "m={multiplier}: surface {index}: qualification changed (nofilter={nofilter})"
                );
            }
        }
    }
}

#[test]
fn windowed_unsupported_spans_need_the_full_offset_reach() {
    // Negative control: a window grown by only half a spacing drops the near
    // hole and near solid, and the unsupported set over the surfaces moves.
    // The real margin keeps them, which is what the equality test relies on.
    let (fills, solids, surfaces) = lower_layer_fixture();
    let spacing = 0.45;
    let full = unsupported_span_areas(&fills, &solids, spacing, 3.0);
    let tight = grow_box(
        polygons_box(&surfaces).expect("surfaces"),
        0.5 * f64::from(spacing),
    );
    let tight_fills = polygons_touching_box(&fills, tight);
    let tight_solids = polygons_touching_box(&solids, tight);
    let too_tight = unsupported_span_areas(&tight_fills, &tight_solids, spacing, 3.0);
    let over = |set: &[ExPolygon]| intersection(&surfaces[..1], set);
    assert_ne!(
        over(&too_tight),
        over(&full),
        "the fixture's near geometry must matter over the surface"
    );
}

#[test]
fn touching_box_filter_drops_only_whole_rings_outside_the_window() {
    let mut plate = rect(0.0, 0.0, 100.0, 100.0);
    plate.holes.push(hole(10.0, 10.0, 12.0, 12.0));
    plate.holes.push(hole(80.0, 80.0, 82.0, 82.0));
    let island = rect(150.0, 0.0, 160.0, 10.0);
    let input = vec![plate.clone(), island];
    let window = grow_box(polygons_box(&[rect(5.0, 5.0, 15.0, 15.0)]).unwrap(), 1.0);
    let kept = polygons_touching_box(&input, window);
    assert_eq!(kept.len(), 1, "the island misses the window");
    assert_eq!(
        kept[0].contour, plate.contour,
        "kept contours are passed through unchanged"
    );
    assert_eq!(
        kept[0].holes,
        vec![plate.holes[0].clone()],
        "only the far hole is dropped"
    );
    let all = grow_box(polygons_box(&input).unwrap(), 0.0);
    assert!(
        matches!(
            polygons_touching_box(&input, all),
            std::borrow::Cow::Borrowed(_)
        ),
        "a window covering everything borrows the input"
    );
}
