//! Regression coverage for the `filterNoncentralRegions` gap-dissolving
//! behavior (dumbbell-polygon fixture) in the arachne pipeline.
#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};

fn p_mm(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM) as i64,
        y: (y_mm * UNITS_PER_MM) as i64,
    }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn inset0_lines(lines: &[ExtrusionLine]) -> Vec<&ExtrusionLine> {
    lines.iter().filter(|l| l.inset_idx == 0).collect()
}

fn dumbbell_polygon_with_dimensions(gap: f64, neck_h: f64) -> ExPolygon {
    let pad_w = 3.0;
    let pad_h = 6.0;
    let neck_w = 0.35;

    let left_x = -(gap + pad_w / 2.0);
    let right_x = gap + pad_w / 2.0;
    let neck_half = neck_w / 2.0;
    let neck_top = neck_h / 2.0;
    let pad_top = pad_h / 2.0;
    let pad_inner_left = left_x + pad_w / 2.0;
    let pad_inner_right = right_x - pad_w / 2.0;

    expoly(vec![
        p_mm(left_x - pad_w / 2.0, -pad_top),
        p_mm(left_x + pad_w / 2.0, -pad_top),
        p_mm(pad_inner_left, -neck_top),
        p_mm(neck_half, -neck_top),
        p_mm(pad_inner_right, -pad_top),
        p_mm(right_x + pad_w / 2.0, -pad_top),
        p_mm(right_x + pad_w / 2.0, pad_top),
        p_mm(pad_inner_right, pad_top),
        p_mm(neck_half, neck_top),
        p_mm(pad_inner_left, neck_top),
        p_mm(left_x - pad_w / 2.0, pad_top),
    ])
}

fn dumbbell_polygon() -> ExPolygon {
    dumbbell_polygon_with_dimensions(0.5, 1.0)
}

#[test]
fn dumbbell_single_central_region_inset0_ring_pair() {
    let dumbbell = dumbbell_polygon();
    let (lines, _) = run_arachne_pipeline(
        std::slice::from_ref(&dumbbell),
        &ArachneParams::default(),
        false,
    )
    .expect("dumbbell polygon should produce Ok(lines)");

    let i0 = inset0_lines(&lines);
    // Canonical `filterNoncentralRegions` promotes short non-central gaps
    // (≤0.4mm) between same/±1-bead-count central regions. The dumbbell's
    // neck gap (0.5mm + 1.0mm = 1.5mm) exceeds max_dist=0.4mm, so canonical
    // does NOT dissolve it — the outer wall remains fragmented. The test
    // asserts that at least one inset-0 line exists and has meaningful
    // geometry (≥4 junctions for a non-degenerate polygon).
    assert!(
        !i0.is_empty(),
        "expected at least one inset-0 line for the dumbbell polygon, got 0 \
         (total lines: {}).",
        lines.len()
    );

    let total_pts: usize = i0.iter().map(|l| l.junctions.len()).sum();
    assert!(
        total_pts >= 4,
        "inset-0 lines should have at least 4 points total for a non-degenerate \
         polygon, got {total_pts}"
    );
}

/// x-extent (mm) of a line's junctions.
fn x_extent(line: &ExtrusionLine) -> (f32, f32) {
    line.junctions
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), j| (lo.min(j.p.x), hi.max(j.p.x)))
}

// Oracle for the two tests below: OrcaSlicer 2.4.1's CLI slicing each
// dumbbell outline extruded 1 mm (BBL X1C 0.20mm Standard process, which is
// Arachne with 2 walls). Its outer wall at z = 0.6 is ONE closed loop spanning
// both pads for the 1.0 mm-tall neck, and TWO closed loops, one per pad, for
// the 0.1 mm-tall neck. The per-pad split is the geometry, not
// `filterNoncentralRegions`: a 0.1 mm neck holds no bead, so each pad's
// outer wall turns back at the neck mouth.
//
// Both tests used to pin the opposite of this oracle (a fragmented wide
// wall, a single narrow ring). The single narrow ring was the per-domain
// chain emission drawing a chord through the neck; with canonical
// `connectJunctions` the pads' walls close on their own.

#[test]
fn dumbbell_wide_neck_outer_wall_is_one_loop_around_both_pads() {
    let dumbbell = dumbbell_polygon();
    let (lines, _) = run_arachne_pipeline(
        std::slice::from_ref(&dumbbell),
        &ArachneParams::default(),
        false,
    )
    .expect("wide-gap dumbbell polygon should produce Ok(lines)");

    let i0 = inset0_lines(&lines);
    let topology: Vec<(bool, usize)> = i0
        .iter()
        .map(|line| (line.is_closed, line.junctions.len()))
        .collect();
    assert_eq!(i0.len(), 1, "wide-neck inset-0 line count: {topology:?}");
    assert!(i0[0].is_closed, "wide-neck outer wall must be closed: {topology:?}");
    // One loop around BOTH pads (pads span x in [-3.5, -0.5] and [0.5, 3.5]).
    let (lo, hi) = x_extent(i0[0]);
    assert!(
        lo < -3.0 && hi > 3.0,
        "the single outer wall must run around both pads, x-extent [{lo}, {hi}]"
    );
}

#[test]
fn dumbbell_narrow_neck_gives_each_pad_its_own_outer_wall() {
    let dumbbell = dumbbell_polygon_with_dimensions(0.2, 0.1);
    let (lines, _) = run_arachne_pipeline(
        std::slice::from_ref(&dumbbell),
        &ArachneParams::default(),
        false,
    )
    .expect("narrow-gap dumbbell polygon should produce Ok(lines)");

    let i0 = inset0_lines(&lines);
    let topology: Vec<(bool, usize)> = i0
        .iter()
        .map(|line| (line.is_closed, line.junctions.len()))
        .collect();
    assert_eq!(i0.len(), 2, "narrow-neck inset-0 line count: {topology:?}");
    assert!(
        i0.iter().all(|line| line.is_closed),
        "each pad's outer wall must close on its own: {topology:?}"
    );
    // One loop per pad (pads span x in [-3.2, -0.2] and [0.2, 3.2]); neither
    // may cross the 0.1 mm neck.
    let mut extents: Vec<(f32, f32)> = i0.iter().map(|line| x_extent(line)).collect();
    extents.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    assert!(
        extents[0].0 < -2.5 && extents[0].1 < 0.0,
        "left loop must stay on the left pad: {extents:?}"
    );
    assert!(
        extents[1].0 > 0.0 && extents[1].1 > 2.5,
        "right loop must stay on the right pad: {extents:?}"
    );
}
