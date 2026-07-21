//! AC-3: Validates simplify distance gates (ExtrusionLine.cpp:56-243).
//!
//! The canonical single linear pass is gated by `smallest_line_segment_squared`
//! / `allowed_error_distance_squared` (from `meshfix_maximum_resolution`/
//! `_deviation`), with `calculateExtrusionAreaDeviationError` as an extra guard
//! on the near-colinear fast path only.
//!
//! Key property: a long low-curvature arc that the old iterative area-only
//! sweep would consume survives because the distance gates reject the collapse.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::simplify_toolpaths;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn junction(x: f32, y: f32, width: f32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.0,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
        },
        perimeter_index: 0,
    }
}

/// A long low-curvature arc (gentle curve, many junctions along a shallow arc)
/// survives the distance-gated simplify. Under the old area-only sweep, this
/// arc would be aggressively simplified because each junction's area deviation
/// is tiny. Under the new distance gates, the `smallest_line_segment_squared`
/// gate prevents removal of segments that are longer than the threshold.
#[test]
fn long_low_curvature_arc_survives_distance_gates() {
    // 20 junctions along a moderate arc (y deviation = 1.0mm over 10mm).
    // Each segment is ~0.5mm long, well above a 0.05mm resolution gate.
    // The arc's curvature is too high for the near-colinear fast path, and
    // segments are too long for the primary distance gate. The old area-only
    // sweep would remove many because the area deviation per junction is small.
    let mut junctions = Vec::new();
    for i in 0..20 {
        let x = i as f32 * 0.5;
        let y = 1.0 * (i as f32 * std::f32::consts::PI / 10.0).sin();
        junctions.push(junction(x, y, 0.4));
    }
    let original_len = junctions.len();

    let line = ExtrusionLine {
        junctions,
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    // Distance gates: resolution = 0.05mm (squared = 0.0025), deviation = 0.005mm (squared = 0.000025).
    // Area threshold = 0.01mm² (typical).
    let result = simplify_toolpaths(
        vec![line],
        0.01,     // visvalingam_area_threshold
        0.0025,   // smallest_line_segment_squared (0.05mm²)
        0.000025, // allowed_error_distance_squared (0.005mm²)
        0.005,    // maximum_extrusion_area_deviation
    );

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    // The arc should retain most of its junctions because segments are longer
    // than smallest_line_segment_squared and the curvature exceeds the
    // near-colinear threshold. Allow some removal, but the arc shape must be
    // preserved (at least 70% retained).
    let min_expected = (original_len as f64 * 0.7) as usize;
    assert!(
        simplified.junctions.len() >= min_expected,
        "long low-curvature arc should retain most junctions under distance gates: \
         got {} of {} (expected >= {})",
        simplified.junctions.len(),
        original_len,
        min_expected
    );
}

/// Under the old area-only sweep (distance gates = 0), the same arc would be
/// aggressively simplified because each junction's area deviation is small.
/// This test demonstrates the difference between the two modes.
#[test]
fn area_only_sweep_aggressively_simplifies_arc() {
    // Same arc as above (amplitude 1.0mm).
    let mut junctions = Vec::new();
    for i in 0..20 {
        let x = i as f32 * 0.5;
        let y = 1.0 * (i as f32 * std::f32::consts::PI / 10.0).sin();
        junctions.push(junction(x, y, 0.4));
    }
    let original_len = junctions.len();

    let line = ExtrusionLine {
        junctions,
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    // Legacy mode: distance gates = 0 → falls back to area-only sweep.
    let result = simplify_toolpaths(vec![line], 0.01, 0.0, 0.0, 0.0);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    // The area-only sweep should simplify more aggressively than the
    // distance-gated version.
    assert!(
        simplified.junctions.len() < original_len,
        "area-only sweep should simplify the arc (got {} of {})",
        simplified.junctions.len(),
        original_len
    );
}

/// Ultra-short segments (< 0.005mm, i.e. 5µm) are always removed by the
/// distance-gated pass, regardless of area deviation.
#[test]
fn ultra_short_segments_always_removed() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.001, 0.0, 0.4), // 1µm — ultra-short
            junction(5.0, 0.0, 0.4),
            junction(10.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(vec![line], 0.01, 0.0025, 0.000025, 0.005);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    // The ultra-short segment should be removed.
    assert!(
        simplified.junctions.len() < 4,
        "ultra-short segment (1µm) should be removed by tier 1 bypass"
    );
}

/// Near-colinear junctions with tiny area deviation are removed by the
/// near-colinear fast path (tier 2).
#[test]
fn near_colinear_junctions_removed_by_fast_path() {
    // Three nearly-colinear points: B is 0.0001mm off the A-C chord.
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(1.0, 0.0001, 0.4), // 0.1µm off the chord
            junction(2.0, 0.0, 0.4),
            junction(3.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(vec![line], 0.01, 0.0025, 0.000025, 0.005);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    // The near-colinear junction should be removed.
    assert!(
        simplified.junctions.len() < 4,
        "near-colinear junction (0.1µm off chord) should be removed by tier 2"
    );
}

/// Junctions that violate both distance gates and the area deviation guard
/// are retained. A short segment with high curvature keeps its junction.
#[test]
fn high_curvature_junction_retained() {
    // A-B-C: B is 1mm off the A-C chord (high curvature).
    // Segment A-B is 0.04mm (< smallest_line_segment_squared threshold of 0.05mm),
    // but height is large (1mm > allowed_error_distance_squared).
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.04, 1.0, 0.4), // short segment, high deviation
            junction(2.0, 0.0, 0.4),
            junction(4.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(
        vec![line],
        0.01,
        0.0025,   // smallest_line_segment_squared = 0.05mm²
        0.000025, // allowed_error_distance_squared = 0.005mm²
        0.005,
    );

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    // B should be retained because the height (1mm²) exceeds the error gate.
    assert_eq!(
        simplified.junctions.len(),
        4,
        "high-curvature junction (1mm off chord) must be retained"
    );
}

/// Both endpoints are always retained regardless of distance gates.
#[test]
fn endpoints_always_retained() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.5, 0.0001, 0.4), // near-colinear
            junction(1.0, 0.0002, 0.4), // near-colinear
            junction(1.5, 0.0001, 0.4), // near-colinear
            junction(2.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = simplify_toolpaths(vec![line], 0.01, 0.0025, 0.000025, 0.005);

    assert_eq!(result.len(), 1);
    let simplified = &result[0];

    assert!(
        simplified.junctions.len() >= 2,
        "must never reduce below 2 junctions"
    );
    // First and last junctions must be the original endpoints.
    assert_eq!(simplified.junctions.first().unwrap().p.x, 0.0);
    assert_eq!(simplified.junctions.last().unwrap().p.x, 2.0);
}
