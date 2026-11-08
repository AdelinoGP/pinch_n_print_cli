//! Batched attributed clipping tests (wayfinder ticket 65).
//!
//! Contract under test: `PreparedPolylineClip::clip_batched` must return,
//! for every input polyline, exactly what a single-subject
//! `PreparedPolylineClip::clip` call returns for that polyline alone —
//! same spans, same order, same omissions — because
//! `link_paths_without_offset` swapped the per-path engine round trip for
//! one batched engine + execute while every downstream step (short
//! filtering, metadata, source segments, connectivity) still assumes
//! per-path-chain-equivalent batches.

use slicer_core::polygon_ops::{clip_polylines, PreparedPolylineClip};
use slicer_ir::{ExPolygon, Polygon};

fn polygon(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points
            .iter()
            .map(|(x, y)| slicer_ir::Point2 { x: *x, y: *y })
            .collect(),
    }
}

fn expoly(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
    ExPolygon {
        contour: polygon(contour),
        holes: holes.iter().map(|hole| polygon(hole)).collect(),
    }
}

fn polyline(points: &[(i64, i64)]) -> Vec<slicer_ir::Point2> {
    points
        .iter()
        .map(|(x, y)| slicer_ir::Point2 { x: *x, y: *y })
        .collect()
}

fn square() -> ExPolygon {
    expoly(&[(0, 0), (1000, 0), (1000, 1000), (0, 1000)], &[])
}

fn square_with_hole() -> ExPolygon {
    expoly(
        &[(0, 0), (1000, 0), (1000, 1000), (0, 1000)],
        &[&[(400, 400), (600, 400), (600, 600), (400, 600)]],
    )
}

/// The batched call must agree with the per-subject chain exactly: the
/// per-subject chain IS the production identity oracle (its output is what
/// the linker shipped before the batch candidate).
fn batch_matches_per_path(boundary: &[ExPolygon], subjects: &[Vec<slicer_ir::Point2>]) {
    let prepared = PreparedPolylineClip::new(boundary);
    let batched = prepared.clip_batched(subjects);
    assert_eq!(
        batched.len(),
        subjects.len(),
        "one result slot per input position"
    );
    for (subject, polylines) in subjects.iter().enumerate() {
        let per_path = prepared.clip(&[polylines.clone()]);
        assert_eq!(
            batched[subject], per_path,
            "subject {} differs from the per-path chain (boundary {:?}, subject {:?})",
            subject, boundary, polylines
        );
    }
}

#[test]
fn batch_matches_per_path_on_independent_shapes() {
    let boundary = [square()];
    let subjects = vec![
        polyline(&[(100, 100), (900, 100)]),   // fully inside
        polyline(&[(-500, 500), (1500, 500)]), // crossing once
        polyline(&[(-500, 500), (500, 5), (900, 900), (1500, 200)]), // multi-bend
        polyline(&[(-900, -900), (-800, -800)]), // fully outside
        polyline(&[(-500, 1500), (1500, 1500)]), // beyond the boundary
        polyline(&[(100, 100)]),               // invalid (<2 points), skipped
        Vec::new(),                            // invalid, skipped
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_with_holes_and_multi_span() {
    let boundary = [square_with_hole()];
    let subjects = vec![
        // Passes straight through the hole: two spans, hole excluded.
        polyline(&[(0, 500), (1000, 500)]),
        // Zig-zag across two hole edges: multiple spans per subject.
        polyline(&[(300, 450), (500, 450), (500, 550), (300, 550)]),
        // Entirely inside the hole: nothing.
        polyline(&[(450, 480), (550, 520)]),
        polyline(&[(650, 650), (900, 700)]),
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_on_subjects_touching_at_corners() {
    let boundary = [square()];
    // Two subjects meeting exactly at an interior corner plus one touching
    // the boundary edge: the one behavioural adjacency risk of a single
    // execute (the engine may concatenate abutting chains; attribution must
    // reconstitute each subject's own pieces).
    let subjects = vec![
        polyline(&[(100, 500), (500, 500)]),
        polyline(&[(500, 500), (500, 900)]),
        polyline(&[(500, 200), (500, 500)]),
        polyline(&[(200, 700), (700, 700)]),
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_on_boundary_riding_subjects() {
    let boundary = [square()];
    // Spans collinear with (and on) the clip boundary itself and its
    // 1-unit-inflated replacement: AC-5's on-boundary inclusion is
    // side-dependent, so both directions exercise the boundary rider.
    let subjects = vec![
        polyline(&[(-100, 0), (1100, 0)]),
        polyline(&[(-100, 1000), (1100, 1000)]),
        polyline(&[(0, -100), (0, 1100)]),
        polyline(&[(1000, -100), (1000, 1100)]),
        polyline(&[(0, 0), (1000, 0)]),
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_on_collinear_touching_subjects() {
    let boundary = [square()];
    // Two subjects sharing exactly one collinear endpoint mid-region: the
    // clearest case for the engine's join logic and the attribution split.
    // Whether the engine concatenates them into one emitted path or not,
    // per-subject results must match the per-path chain.
    let subjects = vec![
        polyline(&(0..=600).map(|x| (x, 500)).collect::<Vec<_>>()),
        polyline(&[(500, 500), (600, 500), (700, 550)]),
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_on_overlapping_segments_within_one_subject() {
    let boundary = [square()];
    let subjects = vec![
        polyline(&[(100, 400), (500, 400), (500, 400), (900, 400)]), // duplicate point
        polyline(&[(100, 600), (900, 600), (900, 600)]),             // trailing duplicate
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_preserves_subject_major_ordering() {
    let boundary = [square()];
    let subjects = vec![
        polyline(&[(100, 100), (900, 100)]),
        polyline(&[(-500, 500), (1500, 500)]),
        Vec::new(), // invalid slot stays empty at its position
        polyline(&[(100, 800), (900, 800)]),
    ];
    let prepared = PreparedPolylineClip::new(&boundary);
    let batched = prepared.clip_batched(&subjects);
    assert_eq!(batched[2].len(), 0, "invalid slot stays empty");
    for (subject, polylines) in subjects.iter().enumerate() {
        assert_eq!(
            batched[subject],
            prepared.clip(&[polylines.clone()]),
            "position {} must carry that subject's own clipped spans",
            subject
        );
    }
}

#[test]
fn batch_matches_per_path_against_multi_polygon_boundary() {
    // Two disjoint islands: subjects crossing both are split per island,
    // as the one-shot `clip_polylines` reference does.
    let boundary = [
        expoly(&[(0, 0), (1000, 0), (1000, 1000), (0, 1000)], &[]),
        expoly(&[(2000, 0), (3000, 0), (3000, 1000), (2000, 1000)], &[]),
    ];
    let subjects = vec![
        polyline(&[(-500, 500), (3500, 500)]),
        polyline(&[(-500, 1500), (3500, 1500)]),
        polyline(&[(2500, 200), (2500, 800)]),
        polyline(&[(400, 200), (400, 800)]),
    ];
    batch_matches_per_path(&boundary, &subjects);
}

#[test]
fn batch_matches_per_path_on_the_one_shot_reference_shapes() {
    // The one-shot `clip_polylines` call and the batch must agree subject
    // for subject on every shape, including the dropped-whole and skipped
    // corners (order across subjects is deliberately NOT compared — the
    // one-shot's across-subject order is unspecified by AC-6).
    let boundary = [square()];
    let subjects = vec![
        polyline(&[(100, 100), (900, 100)]),
        polyline(&[(-500, 500), (1500, 500)]),
        polyline(&[(-900, -900), (-800, -800)]),
    ];
    let one_shot = clip_polylines(&subjects, &boundary);
    let per_subject: Vec<Vec<Vec<slicer_ir::Point2>>> = subjects
        .iter()
        .map(|subject| {
            let prepared = PreparedPolylineClip::new(&boundary);
            prepared.clip(&[subject.clone()])
        })
        .collect();
    let sorted_one_shot = sorted_paths(one_shot);
    let mut sorted_per_subject: Vec<Vec<slicer_ir::Point2>> =
        per_subject.iter().flat_map(|spans| spans.clone()).collect();
    sorted_per_subject = sorted_paths(sorted_per_subject);
    assert_eq!(
        sorted_one_shot, sorted_per_subject,
        "batch and one-shot must carve the same set of spans"
    );
}

/// The realistic-scale regression: several hundred scan-line subjects
/// against a jagged boundary, all of them batched in one execute, compared
/// subject-for-subject against the per-path chain — the matched cells'
/// invocation shape (~30.6 subjects/invocation at up to ~6.5k universe
/// points), deterministic (fixed LCG), no RNG dependency.
#[test]
fn batch_matches_per_path_on_a_realistic_scan_field() {
    let boundary = [expoly(
        &[
            (0, 0),
            (9000, 0),
            (9000, 2000),
            (7000, 2200),
            (4500, 2400),
            (2400, 2100),
            (600, 2300),
            (0, 1900),
        ],
        &[&[(3600, 900), (4400, 900), (4400, 1500), (3600, 1500)]],
    )];
    // Deterministic jagged scan field: horizontal waves with staircase
    // steps and deliberate corner kisses between consecutive spans.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let next = |state: &mut u64| {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*state >> 33) as i64) % 2200
    };
    let mut subjects: Vec<Vec<slicer_ir::Point2>> = Vec::new();
    for row in 0..64 {
        let y = 80 + (row as i64) * 34 + next(&mut state) % 3;
        let mut x = -400;
        while x < 9400 {
            let width = 300 + (next(&mut state).unsigned_abs() as i64 % 900);
            let points = vec![
                (x, y),
                (x + width / 2, y + (next(&mut state) % 3) - 1),
                (x + width, y),
            ];
            subjects.push(polyline(&points));
            x += width; // next span starts at this span's end: a kiss
        }
    }
    assert!(
        subjects.len() >= 300,
        "the scan field must be realistic-scale, got {}",
        subjects.len()
    );
    batch_matches_per_path(&boundary, &subjects);
}

fn sorted_paths(mut paths: Vec<Vec<slicer_ir::Point2>>) -> Vec<Vec<slicer_ir::Point2>> {
    paths.sort_by_key(|path| {
        path.iter()
            .map(|point| (point.x, point.y))
            .collect::<Vec<_>>()
    });
    paths
}
