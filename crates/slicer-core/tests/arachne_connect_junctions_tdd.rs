//! Canonical `connectJunctions` (`SkeletalTrapezoidation.cpp`) regressions on
//! `run_arachne_pipeline`: every quad connects its own junction fans, oddness
//! is decided per segment, and no line bridges a stretch where its bead is
//! absent.
//!
//! The former per-domain chain emission concatenated one bead's junctions
//! around a whole domain walk: it drew chords across stretches where the bead
//! was absent, never marked an odd-count centre line odd, and left walls and
//! inner-contour markers open. Each outline below reproduces one of those
//! failures on a shape small enough to reason about by hand.
#![cfg(feature = "host-algos")]

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};

fn pt(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM).round() as i64,
        y: (y_mm * UNITS_PER_MM).round() as i64,
    }
}

/// Axis-aligned rectangle, counter-clockwise (outline) or clockwise (hole).
fn rect(x0: f64, y0: f64, x1: f64, y1: f64, hole: bool) -> Polygon {
    let mut points = vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)];
    if hole {
        points.reverse();
    }
    Polygon { points }
}

fn params() -> ArachneParams {
    ArachneParams {
        optimal_width: 0.40708,
        preferred_bead_width_outer: 0.40708,
        max_bead_count: 6,
        ..ArachneParams::default()
    }
}

fn run(part: ExPolygon) -> (Vec<ExtrusionLine>, Vec<ExtrusionLine>) {
    run_arachne_pipeline(&[part], &params(), false).expect("outline produces walls")
}

fn polyline_len(line: &ExtrusionLine) -> f64 {
    line.junctions
        .windows(2)
        .map(|w| f64::from(w[1].p.x - w[0].p.x).hypot(f64::from(w[1].p.y - w[0].p.y)))
        .sum()
}

/// A 24 x 10 mm plate with two 5 x 4 mm holes separated by a 1 mm web at
/// x in (11, 12).
fn two_hole_plate() -> ExPolygon {
    ExPolygon {
        contour: rect(0.0, 0.0, 24.0, 10.0, false),
        holes: vec![rect(6.0, 3.0, 11.0, 7.0, true), rect(12.0, 3.0, 17.0, 7.0, true)],
    }
}

/// T1: a 1.3 mm-wide strip holds an odd bead count (3 beads between the
/// canonical 2->3 and 3->4 transitions at ~1.15 and ~1.49 mm for 0.407 mm
/// beads and min_bead_width 85%); its centre bead is ONE odd, open line along
/// the 3.7 mm medial axis (5.0 - 2 x 0.65), emitted once — not a doubled,
/// closed, zero-length loop.
#[test]
fn odd_centre_bead_of_a_narrow_strip_is_one_open_odd_line() {
    let (lines, _) = run(ExPolygon {
        contour: rect(0.0, 0.0, 1.3, 5.0, false),
        holes: Vec::new(),
    });
    let inset1: Vec<&ExtrusionLine> = lines.iter().filter(|l| l.inset_idx == 1).collect();
    let summary: Vec<(bool, bool, f64)> = inset1
        .iter()
        .map(|l| (l.is_odd, l.is_closed, polyline_len(l)))
        .collect();
    assert_eq!(inset1.len(), 1, "inset-1 lines (odd, closed, len): {summary:?}");
    let centre = inset1[0];
    assert!(centre.is_odd, "the centre bead of an odd count is odd: {summary:?}");
    assert!(!centre.is_closed, "the centre bead is an open line: {summary:?}");
    let len = polyline_len(centre);
    assert!(
        (len - 3.70).abs() <= 0.05,
        "centre line length {len:.3} mm, expected 3.70 mm (the medial axis)"
    );
}

/// T2: no inner wall may cut across the 1 mm web between the two holes.
#[test]
fn no_inner_wall_chord_crosses_the_web_between_two_holes() {
    let (lines, _) = run(two_hole_plate());
    let mut chords = Vec::new();
    for line in lines.iter().filter(|l| l.inset_idx >= 1) {
        for w in line.junctions.windows(2) {
            let (mx, my) = (
                f64::from(w[0].p.x + w[1].p.x) / 2.0,
                f64::from(w[0].p.y + w[1].p.y) / 2.0,
            );
            if mx > 11.0 && mx < 12.0 && my > 3.2 && my < 6.8 {
                chords.push((line.inset_idx, (w[0].p.x, w[0].p.y), (w[1].p.x, w[1].p.y)));
            }
        }
    }
    assert!(
        chords.is_empty(),
        "inner-wall segments crossing the web (inset, from, to): {chords:?}"
    );
}

/// T3: a 10 x 10 mm square with a square hole leaving a 1.7 mm wall all
/// around. Both the outline-side and the hole-side outer and second walls
/// are complete rings.
#[test]
fn outer_and_second_walls_close_around_a_thin_walled_square_frame() {
    let (lines, _) = run(ExPolygon {
        contour: rect(0.0, 0.0, 10.0, 10.0, false),
        holes: vec![rect(1.7, 1.7, 8.3, 8.3, true)],
    });
    let open: Vec<(u32, f64)> = lines
        .iter()
        .filter(|l| l.inset_idx <= 1 && !l.is_closed)
        .map(|l| (l.inset_idx, polyline_len(l)))
        .collect();
    assert!(
        lines.iter().any(|l| l.inset_idx == 0) && lines.iter().any(|l| l.inset_idx == 1),
        "the frame must have inset-0 and inset-1 walls"
    );
    assert!(open.is_empty(), "open inset-0/1 lines (inset, length mm): {open:?}");
}

/// T4: the zero-width inner-contour markers of the two-hole plate (the
/// infill boundary) are all closed.
#[test]
fn inner_contour_markers_of_a_two_hole_plate_are_closed() {
    let (_, inner_contour) = run(two_hole_plate());
    let markers: Vec<&ExtrusionLine> = inner_contour.iter().filter(|l| !l.is_odd).collect();
    assert!(!markers.is_empty(), "the plate must leave inner-contour markers");
    let open: Vec<f64> = markers
        .iter()
        .filter(|l| !l.is_closed)
        .map(|l| polyline_len(l))
        .collect();
    assert!(open.is_empty(), "open inner-contour markers (length mm): {open:?}");
}
