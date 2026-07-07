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
/// A small regular hexagon whose center is a local maximum with odd bead
/// count and no central edges (the hexagon is small enough that the
/// centrality predicate filters out all edges). After `generate_toolpaths`
/// runs, the output must contain at least one closed `ExtrusionLine` with
/// `is_odd = true` and exactly 6 junctions — the hexagonal micro-loop
/// emitted by `generateLocalMaximaSingleBeads`.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidation.cpp:2383-2413`.
#[test]
fn ac1_local_maximum_emits_hexagonal_micro_loop() {
    // A "T-shape": a wide horizontal bar with a narrow vertical protrusion
    // (bump). The bar is large enough to have a central skeleton with
    // bead_counts. The bump is narrow enough that its edges are non-central
    // (dR/dD ratio fails the strict 10° centrality predicate). The tip of
    // the bump is a local maximum with odd bead count (propagated from the
    // bar) and no central edges — triggering the micro-loop.
    //
    // Bar: 20mm × 4mm centered at origin, bump: 2mm × 6mm protruding upward
    // from the center of the bar.
    let t_shape = expoly(vec![
        p_mm(-10.0, -2.0), // bottom-left
        p_mm(10.0, -2.0),  // bottom-right
        p_mm(10.0, 2.0),   // top-right of bar
        p_mm(1.0, 2.0),    // right side of bump base
        p_mm(1.0, 8.0),    // top-right of bump
        p_mm(-1.0, 8.0),   // top-left of bump
        p_mm(-1.0, 2.0),   // left side of bump base
        p_mm(-10.0, 2.0),  // top-left of bar
    ]);

    let lines = run_arachne_pipeline(
        std::slice::from_ref(&t_shape),
        &ArachneParams::default(),
        false,
    )
    .expect("T-shape should produce Ok(lines)");

    // Find closed is_odd lines with exactly 6 junctions — the micro-loop.
    let micro_loops: Vec<&ExtrusionLine> = lines
        .iter()
        .filter(|l| l.is_odd && l.is_closed && l.junctions.len() == 6)
        .collect();

    assert!(
        !micro_loops.is_empty(),
        "expected at least one closed is_odd ExtrusionLine with 6 junctions (hexagonal \
         micro-loop from generateLocalMaximaSingleBeads) for a small hexagon with odd bead \
         count, got {} total lines with {} closed-is_odd-6j candidates",
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
