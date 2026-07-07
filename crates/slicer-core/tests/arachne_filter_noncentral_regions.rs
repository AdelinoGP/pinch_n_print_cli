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

fn dumbbell_polygon() -> ExPolygon {
    let pad_w = 3.0;
    let pad_h = 6.0;
    let neck_w = 0.35;
    let neck_h = 1.0;
    let gap = 0.5;

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

#[test]
fn dumbbell_single_central_region_inset0_ring_pair() {
    let dumbbell = dumbbell_polygon();
    let lines = run_arachne_pipeline(
        std::slice::from_ref(&dumbbell),
        &ArachneParams::default(),
        false,
    )
    .expect("dumbbell polygon should produce Ok(lines)");

    let i0 = inset0_lines(&lines);
    assert!(
        i0.len() >= 2,
        "expected at least one inset-0 ring pair for the dumbbell polygon, got {} inset-0 lines \
         (total lines: {}). A single continuous outer-wall region should produce one clockwise + \
         one counter-clockwise ring pair. Four fragments would indicate the neck's non-central gap \
         was NOT promoted back to central by filter_noncentral_regions.",
        i0.len(),
        lines.len()
    );

    let total_pts: usize = i0.iter().map(|l| l.junctions.len()).sum();
    assert!(
        total_pts >= 8,
        "inset-0 ring pair should have at least 8 points total (4 per ring minimum for a \
         non-degenerate polygon), got {total_pts}"
    );
}
