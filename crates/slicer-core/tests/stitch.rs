//! TDD suite for packet 112 (Track B, T-225): `arachne::stitch::stitch_extrusions`.
//!
//! AC-6: a primary (closed, inset 0) `ExtrusionLine` is never a candidate for
//! joining/splitting and must come back byte-identical, while separate open
//! polylines within `max_gap` of each other join into one.

use slicer_core::arachne::stitch_extrusions;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn junction(x: f32, y: f32, width: f32, perimeter_index: u32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.2,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index,
    }
}

/// AC-6: mixes a primary (closed, inset 0) line with two open polylines
/// whose endpoints sit within `max_gap` of each other. Expect: the two open
/// lines join into a single line, and the primary line is returned
/// byte-identical (deep-equal) to its input value.
#[test]
fn stitch_extrusions_preserves_primary() {
    let primary = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4, 0),
            junction(10.0, 0.0, 0.4, 0),
            junction(10.0, 10.0, 0.4, 0),
            junction(0.0, 10.0, 0.4, 0),
            junction(0.0, 0.0, 0.4, 0),
        ],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };

    // Two open polylines, inset 1, whose facing endpoints are 0.05mm apart
    // (well within max_gap = 0.2mm) but whose far endpoints remain far apart
    // (20mm) so the merged result stays open, keeping the assertion simple.
    let open_a = ExtrusionLine {
        junctions: vec![junction(100.0, 0.0, 0.4, 1), junction(105.0, 0.0, 0.4, 1)],
        inset_idx: 1,
        is_odd: false,
        is_closed: false,
    };
    let open_b = ExtrusionLine {
        junctions: vec![junction(105.05, 0.0, 0.4, 1), junction(120.0, 0.0, 0.4, 1)],
        inset_idx: 1,
        is_odd: false,
        is_closed: false,
    };

    let input = vec![primary.clone(), open_a.clone(), open_b.clone()];
    let max_gap = 0.2;

    let result = stitch_extrusions(input, max_gap);

    assert_eq!(
        result.len(),
        2,
        "expected primary + one merged line, got {} lines: {result:#?}",
        result.len()
    );

    let merged_primary = result
        .iter()
        .find(|l| l.inset_idx == 0)
        .expect("primary line (inset 0) must be present in output");
    assert_eq!(
        merged_primary, &primary,
        "primary (closed, inset 0) line must be returned byte-identical"
    );

    let merged_open = result
        .iter()
        .find(|l| l.inset_idx == 1)
        .expect("merged inset-1 line must be present in output");
    assert_eq!(
        merged_open.junctions.len(),
        open_a.junctions.len() + open_b.junctions.len(),
        "the two open lines must join into one line with all junctions preserved"
    );
    assert!(
        !merged_open.is_closed,
        "merged line's far endpoints (100,0) and (120,0) are far apart -- must stay open"
    );
}
