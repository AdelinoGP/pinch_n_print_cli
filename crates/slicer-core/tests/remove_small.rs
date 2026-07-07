//! TDD suite for packet 112 (Track B, T-227): `arachne::remove_small::remove_small_lines`.
//!
//! AC-8: a primary (closed, inset 0) line survives alongside a short odd
//! transition sliver that gets removed.
//! AC-N3: an all-primary input (every line closed & inset 0) has zero
//! removals regardless of length.

use slicer_core::arachne::remove_small_lines;
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

/// AC-8: mixes a primary (closed, inset 0) line with short, odd, open
/// "transition" lines. Expect the primary survives and the short transitions
/// are removed.
#[test]
fn remove_small_lines_preserves_primary() {
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

    // Short odd, open transition sliver: length = 0.05mm.
    let short_transition = ExtrusionLine {
        junctions: vec![junction(50.0, 50.0, 0.4), junction(50.05, 50.0, 0.4)],
        inset_idx: 2,
        is_odd: true,
        is_closed: false,
    };

    let min_length_factor = 0.5;
    let min_width = 0.4; // threshold = 0.2mm; short_transition length 0.05mm < 0.2mm

    let input = vec![primary.clone(), short_transition];
    let result = remove_small_lines(input, min_length_factor, min_width, false);

    assert_eq!(
        result.len(),
        1,
        "short odd/open transition must be removed, primary must remain"
    );
    assert_eq!(
        result[0], primary,
        "primary (closed, inset 0) line must be preserved unchanged"
    );
}

/// AC-N3: when every input line is primary-shaped (closed & inset 0), zero
/// lines are removed regardless of how short any of them are.
#[test]
fn remove_small_lines_all_primary_invariant() {
    let tiny_primary_a = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.01, 0.0, 0.4),
            junction(0.0, 0.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };
    let tiny_primary_b = ExtrusionLine {
        junctions: vec![
            junction(5.0, 5.0, 0.4),
            junction(5.001, 5.0, 0.4),
            junction(5.0, 5.0, 0.4),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };

    let input = vec![tiny_primary_a.clone(), tiny_primary_b.clone()];
    // A large threshold that would remove both lines if the preserve
    // invariant were not checked first.
    let result = remove_small_lines(input, 1000.0, 1000.0, false);

    assert_eq!(
        result.len(),
        2,
        "all-primary input must have zero removals regardless of length/threshold"
    );
    assert!(result.contains(&tiny_primary_a));
    assert!(result.contains(&tiny_primary_b));
}
