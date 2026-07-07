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
            junction(0.0, 0.0, 0.40),
            junction(2.0, 0.001, 0.41), // ~1um off the chord -- well under threshold
            junction(4.0, -0.001, 0.42),
            junction(6.0, 0.0005, 0.43),
            junction(8.0, 0.0, 0.44),
            junction(10.0, 0.0, 0.45),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let original_len = line.junctions.len();

    let visvalingam_area_threshold = 0.01; // mm² -- typical bead-width-weighted default
    let result = simplify_toolpaths(vec![line], visvalingam_area_threshold, 0.0, 0.0, 0.0);

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

/// AC-N1 (NEW): a junction whose removal would produce a width-weighted area
/// deviation above the threshold is preserved. This is the negative case for
/// the Visvalingam gate: the middle vertex is *not* dropped because its
/// deviation exceeds `visvalingam_area_threshold`.
#[test]
fn simplify_toolpaths_width_weighted_gate_preserves_junctions() {
    // A-B-C form a right triangle with legs 2 mm and 0.2 mm.
    // Area = 0.5 * 2.0 * 0.2 = 0.2 mm².
    // With B's width = 0.4 mm, the width-weighted deviation is
    // 0.5 * 0.4 * |cross(AB, AC)| / |AC| = 0.2 * 0.4 / |AC|.
    // |AC| = sqrt(2.0² + 0.2²) ≈ 2.00998, so deviation ≈ 0.0398 mm²,
    // well above a 0.01 mm² threshold. The middle vertex must survive.
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

    let visvalingam_area_threshold = 0.01; // mm²
    let result = simplify_toolpaths(vec![line], visvalingam_area_threshold, 0.0, 0.0, 0.0);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    assert_eq!(
        simplified.junctions.len(),
        original_len,
        "width-weighted area deviation exceeds threshold; middle junction must be kept"
    );

    // Endpoints and the kept middle junction must retain original widths.
    assert_eq!(simplified.junctions[0].p.width, 0.40);
    assert_eq!(simplified.junctions[1].p.width, 0.40);
    assert_eq!(simplified.junctions[2].p.width, 0.40);
}
