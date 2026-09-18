//! Wall-continuity regressions for `run_arachne_pipeline` on small outlines
//! that reproduce the benchy-hull wall defects (layers 29-53 of
//! `3dbenchy.stl`, 3 walls):
//!
//! - A thin gap's beading climbed the whole medial spine (propagation never
//!   overwrote it), so the third wall vanished from the wide region next to
//!   the gap.
//! - Per-bead junctions were concatenated across a whole domain walk, which
//!   left inner-contour markers open or closed them with chords.
//! - Transition ribs ended on their own spine node instead of the outline,
//!   so every quad next to a bead-count transition lost its walls.
//!
//! Each assertion is measured against the outline's own geometry, not a
//! captured baseline.
#![cfg(feature = "host-algos")]

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};

fn pt(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM).round() as i64,
        y: (y_mm * UNITS_PER_MM).round() as i64,
    }
}

fn ring(points: &[(f64, f64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| pt(x, y)).collect(),
    }
}

/// The benchy default config's wall widths: `line_width` auto (1.125 x 0.4 mm
/// nozzle = 0.45 mm) turned into spacing at 0.2 mm layers, 3 walls.
fn three_wall_params() -> ArachneParams {
    let spacing = 0.45 - 0.2 * (1.0 - std::f64::consts::FRAC_PI_4);
    ArachneParams {
        optimal_width: spacing,
        preferred_bead_width_outer: spacing,
        max_bead_count: 6,
        ..ArachneParams::default()
    }
}

fn polyline_len(line: &ExtrusionLine) -> f64 {
    line.junctions
        .windows(2)
        .map(|w| f64::from(w[1].p.x - w[0].p.x).hypot(f64::from(w[1].p.y - w[0].p.y)))
        .sum()
}

fn inset_len(lines: &[ExtrusionLine], inset: u32) -> f64 {
    lines
        .iter()
        .filter(|l| l.inset_idx == inset)
        .map(polyline_len)
        .sum()
}

/// Absolute shoelace area (mm^2) of a closed line.
fn closed_area(line: &ExtrusionLine) -> f64 {
    let j = &line.junctions;
    let twice: f64 = (0..j.len())
        .map(|i| {
            let (a, b) = (&j[i].p, &j[(i + 1) % j.len()].p);
            f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y)
        })
        .sum();
    twice.abs() / 2.0
}

/// A 40 x 20 mm slab with a 1 mm-radius hole whose edge is 1.5 mm from the
/// left end — the benchy stern ring next to the transom. The 1.5 mm gap holds
/// 3 beads; the rest of the slab holds all 6. The gap's 3-bead beading is
/// copied up the spine, and must be overwritten by the 6-bead beading
/// propagated down from the slab's wide middle once it is farther than the
/// transition distance from the gap. When it was not, the third wall
/// (inset 2) vanished from the whole left half.
#[test]
fn thin_gap_beading_does_not_starve_the_wide_region_of_its_third_wall() {
    let hole: Vec<(f64, f64)> = (0..32)
        .rev()
        .map(|k| {
            let t = std::f64::consts::TAU * f64::from(k) / 32.0;
            (2.5 + t.cos(), 10.0 + t.sin())
        })
        .collect();
    let slab = ExPolygon {
        contour: ring(&[(0.0, 0.0), (40.0, 0.0), (40.0, 20.0), (0.0, 20.0)]),
        holes: vec![ring(&hole)],
    };
    let (lines, _) = run_arachne_pipeline(&[slab], &three_wall_params(), false)
        .expect("slab with hole produces walls");

    let outer = inset_len(&lines, 0);
    let third = inset_len(&lines, 2);
    assert!(outer > 100.0, "the outer wall must run around the slab, got {outer:.1} mm");
    // The third wall follows the outer one everywhere except where the gap
    // beside the hole has room for only 3 beads.
    assert!(
        third >= 0.85 * outer,
        "third wall is {third:.1} mm against {outer:.1} mm of outer wall — \
         it is missing from the region next to the hole"
    );
}

/// syn2: a 20 x 20 mm square with a centred 10 x 10 mm hole. The walls leave
/// an annulus of infill between the two inner-contour markers, and both
/// markers must come out closed so the infill boundary exists.
#[test]
fn inner_contour_markers_close_around_a_square_hole() {
    let part = ExPolygon {
        contour: ring(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
        holes: vec![ring(&[(5.0, 15.0), (15.0, 15.0), (15.0, 5.0), (5.0, 5.0)])],
    };
    let (_, inner_contour) = run_arachne_pipeline(&[part], &three_wall_params(), false)
        .expect("square with hole produces walls");

    let markers: Vec<&ExtrusionLine> = inner_contour.iter().filter(|l| !l.is_odd).collect();
    let open: Vec<usize> = markers
        .iter()
        .filter(|l| !l.is_closed)
        .map(|l| l.junctions.len())
        .collect();
    assert!(open.is_empty(), "open inner-contour markers (junction counts): {open:?}");
    assert_eq!(markers.len(), 2, "one marker per outline ring");

    // Both markers sit 3 bead widths (3 x 0.40708 = 1.2212 mm) in from
    // their ring: sides 20 - 2 x 1.2212 and 10 + 2 x 1.2212 mm.
    let mut areas: Vec<f64> = markers.iter().map(|l| closed_area(l)).collect();
    areas.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let annulus = areas[0] - areas[1];
    let expected = 17.557_f64.powi(2) - 12.443_f64.powi(2);
    assert!(
        (annulus - expected).abs() <= 0.02 * expected,
        "infill annulus {annulus:.1} mm^2, expected {expected:.1} mm^2 (areas {areas:?})"
    );
}

/// syn3: a 30 x 12 mm ring whose hole leaves thin 1.5 mm sides and an 8 mm
/// thick right end. Only the thick end has room for infill; its marker must
/// enclose that whole end, not a sliver cut off by a chord.
#[test]
fn inner_contour_marker_covers_the_thick_end_of_a_thin_ring() {
    let part = ExPolygon {
        contour: ring(&[(0.0, 0.0), (30.0, 0.0), (30.0, 12.0), (0.0, 12.0)]),
        holes: vec![ring(&[(1.5, 10.5), (22.0, 10.5), (22.0, 1.5), (1.5, 1.5)])],
    };
    let (_, inner_contour) = run_arachne_pipeline(&[part], &three_wall_params(), false)
        .expect("thin ring produces walls");

    let markers: Vec<&ExtrusionLine> = inner_contour.iter().filter(|l| !l.is_odd).collect();
    assert!(!markers.is_empty(), "the thick end must leave an infill marker");
    assert!(
        markers.iter().all(|l| l.is_closed),
        "every inner-contour marker must be closed"
    );
    // The thick end's infill region is about [23.22, 28.78] x [1.22, 10.78]
    // = 53.2 mm^2.
    let area: f64 = markers.iter().map(|l| closed_area(l)).sum();
    assert!(area >= 45.0, "marker area {area:.1} mm^2, expected about 53 mm^2");
}

/// A 30 mm strip tapering from 1.8 to 2.2 mm: the bead count steps from 4
/// to 5 along it, which inserts transition nodes with ribs. The outer wall
/// must still run all the way around the strip.
#[test]
fn bead_count_transition_keeps_the_outer_wall_along_a_tapered_strip() {
    let strip = ExPolygon {
        contour: ring(&[(0.0, 0.0), (30.0, 0.0), (30.0, 2.2), (0.0, 1.8)]),
        holes: Vec::new(),
    };
    let (lines, _) = run_arachne_pipeline(&[strip], &three_wall_params(), false)
        .expect("tapered strip produces walls");

    let perimeter = 30.0 + 30.0_f64.hypot(0.4) + 1.8 + 2.2;
    let outer = inset_len(&lines, 0);
    assert!(
        outer >= 0.9 * perimeter,
        "outer wall is {outer:.1} mm for a {perimeter:.1} mm outline — walls are missing \
         around the bead-count transition"
    );
}
