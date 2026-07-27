//! TDD suite for packet 112 (Track B, T-226): `arachne::simplify::simplify_toolpaths`.
//!
//! AC-7: a line with collinear-ish junctions simplifies to strictly fewer
//! junctions, and the widths of the retained junctions are unchanged
//! (no averaging/interpolation across dropped runs).

use slicer_core::arachne::simplify_toolpaths;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn junction(x: f32, y: f32, width: f32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.2,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
        },
        perimeter_index: 0,
    }
}

/// AC-7: a nearly-straight line (small collinear wobble under the
/// Visvalingam area threshold) simplifies to fewer junctions, and each
/// retained junction keeps its original width value untouched.
#[test]
fn simplify_toolpaths_vertex_count() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.000, 0.0, 0.40),
            junction(0.002, 0.0, 0.41),
            junction(0.004, 0.0, 0.42),
            junction(0.006, 0.0, 0.43),
            junction(0.008, 0.0, 0.44),
            junction(0.010, 0.0, 0.45),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let original_len = line.junctions.len();

    // Real distance gates (meshfix_maximum_resolution 0.05mm, deviation
    // 0.005mm, canonical area deviation 2e-6 mm²). The 2µm spacing above is
    // below canonical's unconditional 5µm ultra-short bypass, so the interior
    // junctions are removed by that branch. Anchoring on the ultra-short gate
    // keeps this test independent of the `height_2` accumulator.
    let result = simplify_toolpaths(vec![line], 0.0025, 0.000025, 2e-6);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    assert!(
        simplified.junctions.len() < original_len,
        "vertex count must strictly drop: {} -> {}",
        original_len,
        simplified.junctions.len()
    );
    assert!(
        simplified.junctions.len() >= 2,
        "must never reduce below 2 junctions, got {}",
        simplified.junctions.len()
    );

    // Endpoints are always retained; their widths must be exactly the
    // original values (no averaging/interpolation).
    assert_eq!(simplified.junctions.first().unwrap().p.width, 0.40);
    assert_eq!(simplified.junctions.last().unwrap().p.width, 0.45);

    // Every retained junction's width must match one of the original
    // per-vertex widths exactly (proves widths are preserved, not derived).
    let original_widths = [0.40f32, 0.41, 0.42, 0.43, 0.44, 0.45];
    for j in &simplified.junctions {
        assert!(
            original_widths.contains(&j.p.width),
            "retained junction width {} must match an original width exactly",
            j.p.width
        );
    }
}

/// AC-N1: a junction that sits far enough off the chord is preserved. A-B-C
/// form a triangle with legs 2 mm and 0.2 mm, so B is 0.2 mm off the line A-C
/// — two orders of magnitude beyond every removal gate. No branch may drop it.
#[test]
fn simplify_toolpaths_width_weighted_gate_preserves_junctions() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.40),
            junction(2.0, 0.2, 0.40),
            junction(4.0, 0.0, 0.40),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let original_len = line.junctions.len();

    let result = simplify_toolpaths(vec![line], 0.0025, 0.000025, 2e-6);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    assert_eq!(
        simplified.junctions.len(),
        original_len,
        "B lies 0.2mm off the chord A-C; no gate may remove it"
    );

    // Endpoints and the kept middle junction must retain original widths.
    assert_eq!(simplified.junctions[0].p.width, 0.40);
    assert_eq!(simplified.junctions[1].p.width, 0.40);
    assert_eq!(simplified.junctions[2].p.width, 0.40);
}

/// Zero distance gates disable the primary gate. Canonical has no area-only
/// fallback, so the polyline survives essentially intact.
///
/// Only tier 3 reads the distance gates; tier 1 (5µm ultra-short) and tier 2
/// (5µm colinearity band) use hardcoded constants, so index 10 — which sits
/// exactly on the chord at the sine's inflection — is still legitimately
/// removable here. Everything else must survive.
///
/// This is the direct regression guard for the defect where the deleted
/// `simplify_area_only` sweep, driven by canonical's
/// `calculateExtrusionAreaDeviationError`, collapsed uniform-width polylines
/// to their endpoints: that function returns exactly 0.0 when all three widths
/// are equal, so it removed every interior junction regardless of geometry.
#[test]
fn zero_gates_retain_every_junction() {
    // 20-point sine arc, 1.0 mm amplitude, uniform 0.4 mm width. Under the
    // deleted sweep this collapsed from 20 junctions to 2.
    let junctions: Vec<_> = (0..20)
        .map(|i| {
            let x = i as f32 * 0.5;
            let y = (i as f32 * std::f32::consts::PI / 10.0).sin();
            junction(x, y, 0.40)
        })
        .collect();
    let original_len = junctions.len();
    let line = ExtrusionLine {
        junctions,
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(vec![line], 0.0, 0.0, 0.0);

    assert_eq!(result.len(), 1);
    let retained = result[0].junctions.len();
    assert!(
        retained >= original_len - 1,
        "zero gates must retain the arc: {retained} of {original_len}"
    );
}

/// The same uniform-width arc must survive real production gates too: its
/// 0.5 mm spacing and 1.0 mm amplitude are far outside every removal gate, so
/// uniform widths must not by themselves cause any removal.
///
/// Exactly one junction is legitimately dropped: index 10 sits at the sine's
/// inflection, where `sin(0.9π)`, `sin(π)` and `sin(1.1π)` are `+0.309`, `0`
/// and `−0.309`, so it lies exactly on the chord between its neighbours. The
/// bound below is deliberately loose about that single colinear point while
/// still failing hard on the 20 → 2 collapse this guards.
#[test]
fn uniform_width_arc_survives_production_gates() {
    let junctions: Vec<_> = (0..20)
        .map(|i| {
            let x = i as f32 * 0.5;
            let y = (i as f32 * std::f32::consts::PI / 10.0).sin();
            junction(x, y, 0.40)
        })
        .collect();
    let original_len = junctions.len();
    let line = ExtrusionLine {
        junctions,
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(vec![line], 0.0025, 0.000025, 2e-6);

    assert_eq!(result.len(), 1);
    let retained = result[0].junctions.len();
    assert!(
        retained >= original_len - 1,
        "1mm-amplitude arc must not be simplified away: {retained} of {original_len}"
    );
}
