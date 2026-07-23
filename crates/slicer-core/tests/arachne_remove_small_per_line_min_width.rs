//! AC-2: Validates per-line `min_width` in `remove_small_lines`
//! (WallToolPaths.cpp:838-856).
//!
//! The removal threshold is computed per line from the minimum junction width
//! along that line:
//! - Top/bottom layers (`is_initial_layer == true`): `min_junction_width / 2`
//! - Other layers: `min_junction_width * min_length_factor`
//!
//! This replaces the old caller-supplied constant `min_width`.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::remove_small_lines;
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

/// A line with narrow junctions (0.1mm) gets a smaller removal threshold
/// than one with wide junctions (0.4mm). On non-initial layers, the
/// threshold is `min_junction_width * min_length_factor`.
///
/// A 0.15mm-long line with width 0.4mm: threshold = 0.4 * 0.5 = 0.2mm.
/// 0.15 < 0.2 → removed.
///
/// A 0.15mm-long line with width 0.1mm: threshold = 0.1 * 0.5 = 0.05mm.
/// 0.15 > 0.05 → kept.
#[test]
fn per_line_min_width_narrow_line_survives() {
    // Line with narrow junctions (0.1mm), length ~0.15mm.
    let narrow = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.1), junction(0.15, 0.0, 0.1)],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };

    // threshold = 0.1 * 0.5 = 0.05mm. Length 0.15mm > 0.05mm → kept.
    let result = remove_small_lines(vec![narrow.clone()], 0.5, 0.4, false, false);
    assert_eq!(
        result.len(),
        1,
        "narrow line (0.1mm width) should survive — its per-line threshold (0.05mm) is below its length (0.15mm)"
    );

    // Same line with wide junctions (0.4mm) would be removed:
    // threshold = 0.4 * 0.5 = 0.2mm. Length 0.15mm < 0.2mm → removed.
    let wide = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(0.15, 0.0, 0.4)],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };
    let result_wide = remove_small_lines(vec![wide], 0.5, 0.4, false, false);
    assert!(
        result_wide.is_empty(),
        "wide line (0.4mm width) should be removed — its per-line threshold (0.2mm) exceeds its length (0.15mm)"
    );
}

/// On initial layers (top/bottom), the divisor is `min_width / 2` instead of
/// `min_width * min_length_factor`. This is more conservative (smaller
/// threshold) to prevent top gaps.
///
/// A 0.15mm-long line with width 0.4mm:
/// - Non-initial: threshold = 0.4 * 0.5 = 0.2mm → removed (0.15 < 0.2)
/// - Initial: threshold = 0.4 / 2 = 0.2mm → removed (0.15 < 0.2)
///
/// For min_length_factor = 0.5, both happen to give the same threshold.
/// But the threshold computation is different. Let's use a different
/// min_length_factor to show the divergence.
///
/// A 0.15mm-long line with width 0.4mm, min_length_factor = 0.3:
/// - Non-initial: threshold = 0.4 * 0.3 = 0.12mm → kept (0.15 > 0.12)
/// - Initial: threshold = 0.4 / 2 = 0.2mm → removed (0.15 < 0.2)
#[test]
fn initial_layer_uses_min_width_div_2() {
    // Non-initial: threshold = 0.4 * 0.3 = 0.12mm. Length 0.15 > 0.12 → kept.
    let line = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(0.15, 0.0, 0.4)],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };
    let result = remove_small_lines(vec![line.clone()], 0.3, 0.4, false, false);
    assert_eq!(
        result.len(),
        1,
        "non-initial layer: threshold = 0.4 * 0.3 = 0.12mm, length 0.15 > 0.12 → kept"
    );

    // Initial layer: threshold = 0.4 / 2 = 0.2mm. Length 0.15 < 0.2 → removed.
    let result_initial = remove_small_lines(vec![line], 0.3, 0.4, true, false);
    assert!(
        result_initial.is_empty(),
        "initial layer: threshold = 0.4 / 2 = 0.2mm, length 0.15 < 0.2 → removed"
    );
}

/// Per-line min_width: a line with mixed junction widths uses the minimum.
/// A line with junctions at widths [0.4, 0.1, 0.4] has min_width = 0.1mm.
/// threshold = 0.1 * 0.5 = 0.05mm.
#[test]
fn per_line_min_width_uses_minimum_junction_width() {
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.06, 0.0, 0.1), // narrow junction
            junction(0.12, 0.0, 0.4),
        ],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };

    // min junction width = 0.1mm. threshold = 0.1 * 0.5 = 0.05mm.
    // length ≈ 0.12mm > 0.05mm → kept.
    let result = remove_small_lines(vec![line], 0.5, 0.4, false, false);
    assert_eq!(
        result.len(),
        1,
        "line with mixed widths uses minimum junction width (0.1mm) for threshold"
    );
}

/// Closed lines and even lines are never removed regardless of per-line
/// min_width.
#[test]
fn closed_and_even_lines_always_survive() {
    let closed = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(0.01, 0.0, 0.4)],
        inset_idx: 0,
        is_odd: true,
        is_closed: true,
    };
    let even = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(0.01, 0.0, 0.4)],
        inset_idx: 1,
        is_odd: false,
        is_closed: false,
    };

    let result = remove_small_lines(vec![closed, even], 0.5, 0.4, false, false);
    assert_eq!(result.len(), 2, "closed and even lines always survive");
}

/// An all-primary input (closed, inset 0) has zero removals regardless
/// of per-line width computation.
#[test]
fn all_primary_input_no_removals() {
    let primary = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(10.0, 0.0, 0.4),
            junction(10.0, 10.0, 0.4),
            junction(0.0, 10.0, 0.4),
            junction(0.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };

    let result = remove_small_lines(vec![primary.clone()], 1000.0, 1000.0, false, false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], primary);
}
