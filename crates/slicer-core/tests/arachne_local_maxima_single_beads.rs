//! Red tests encoding finding **N9** of the second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N9).
//!
//! **Finding N9:** PNP's `generate_toolpaths` lacks the
//! `generateLocalMaximaSingleBeads` pass
//! (`SkeletalTrapezoidation.cpp:2383-2413`): for nodes with odd
//! `beading.bead_widths.size()`, `isLocalMaximum(true)`, and not central,
//! canonical emits a 6-segment hexagonal micro-loop (radius `width/8`,
//! `is_odd = true`). Without it, local maxima that never join a domain chain
//! vanish (pinholes at the center of near-square regions with odd bead counts).
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

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

/// AC-1 — hexagonal micro-loop at local maximum.
///
/// A small regular pentagon whose center is a geometric local maximum with
/// odd bead count and no central edges. A pentagon has no parallel edges,
/// so its medial axis has no flat segments — every edge from the center
/// radiates outward with `dR/dD ≈ 1.0 >> sin(5°)`, making all edges
/// non-central under the strict 10° predicate. This mirrors OrcaSlicer's
/// `generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp:2383-2413`):
/// isolated thick spots with odd bead count, non-central, at local maxima
/// get a 6-segment hexagonal micro-loop.
#[test]
fn ac1_local_maximum_emits_hexagonal_micro_loop() {
    // Regular pentagon centered at origin with circumradius 0.7mm
    // (apothem ≈ 0.566mm). The center has R ≈ 0.566mm → width ≈ 1.132mm
    // → optimal_bead_count = round(1.132 / 0.4) = 3 (odd). All edges from
    // the center have dR/dD ≈ 1.0, which exceeds sin(5°) ≈ 0.087, so no
    // edge is central — Gate 3 passes.
    let cr: f64 = 0.7;
    let pentagon = expoly(vec![
        p_mm(0.0, cr),
        p_mm(
            cr * (72.0_f64.to_radians()).sin(),
            cr * (72.0_f64.to_radians()).cos(),
        ),
        p_mm(
            cr * (144.0_f64.to_radians()).sin(),
            cr * (144.0_f64.to_radians()).cos(),
        ),
        p_mm(
            cr * (216.0_f64.to_radians()).sin(),
            cr * (216.0_f64.to_radians()).cos(),
        ),
        p_mm(
            cr * (288.0_f64.to_radians()).sin(),
            cr * (288.0_f64.to_radians()).cos(),
        ),
    ]);

    let (lines, _) = run_arachne_pipeline(
        std::slice::from_ref(&pentagon),
        &ArachneParams::default(),
        false,
    )
    .expect("pentagon should produce Ok(lines)");

    // Find closed is_odd lines with exactly 6 junctions — the micro-loop.
    let micro_loops: Vec<&ExtrusionLine> = lines
        .iter()
        .filter(|l| l.is_odd && l.is_closed && l.junctions.len() == 6)
        .collect();

    assert!(
        !micro_loops.is_empty(),
        "expected at least one closed is_odd ExtrusionLine with 6 junctions (hexagonal \
         micro-loop from generateLocalMaximaSingleBeads) for a regular pentagon with odd bead \
         count and no central edges, got {} total lines with {} closed-is_odd-6j candidates",
        lines.len(),
        micro_loops.len()
    );

    // Verify the micro-loop's geometry: the 6 junctions should form a
    // roughly hexagonal shape.
    let ml = micro_loops[0];
    assert_eq!(
        ml.inset_idx, ml.junctions[0].perimeter_index,
        "micro-loop's inset_idx should match its junctions' perimeter_index (the middle bead)"
    );

    // All 6 junctions should be equidistant from their centroid (within tolerance).
    let cx: f32 = ml.junctions.iter().map(|j| j.p.x).sum::<f32>() / 6.0;
    let cy: f32 = ml.junctions.iter().map(|j| j.p.y).sum::<f32>() / 6.0;

    // All junctions should have the same width (the middle bead's width).
    let first_width = ml.junctions[0].p.width;
    for (i, j) in ml.junctions.iter().enumerate() {
        assert!(
            (j.p.width - first_width).abs() < 1e-3,
            "junction {} width {:.4} differs from first junction width {:.4}",
            i,
            j.p.width,
            first_width
        );
    }

    // The radius (distance from centroid to each junction) should be
    // width/8 (in mm).
    let expected_r_mm = first_width / 8.0;
    for (i, j) in ml.junctions.iter().enumerate() {
        let dx = j.p.x - cx;
        let dy = j.p.y - cy;
        let r = (dx * dx + dy * dy).sqrt();
        assert!(
            (r - expected_r_mm).abs() < 0.01,
            "junction {} radius {:.4} mm differs from expected width/8 = {:.4} mm",
            i,
            r,
            expected_r_mm
        );
    }
}
