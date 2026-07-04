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

/// AC-7: a nearly-straight line (small collinear wobble under the DP
/// epsilon) simplifies to fewer junctions, and each retained junction keeps
/// its original width value untouched.
#[test]
fn simplify_toolpaths_vertex_count() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.40),
            junction(2.0, 0.001, 0.41), // ~1um off the chord -- well under epsilon
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

    let dp_epsilon = 0.025; // mm -- matches OrcaSlicer/Cura's default maximum_deviation
    let result = simplify_toolpaths(vec![line], dp_epsilon);

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
