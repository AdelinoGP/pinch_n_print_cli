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
            ..Default::default()
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

/// An open inset-1 line tracing a full circle of `radius` (first junction
/// repeated at the end), as `generate_toolpaths` emits each bead of a ring.
fn circle_line(radius: f32, reverse: bool) -> ExtrusionLine {
    let n = 48;
    let mut junctions: Vec<ExtrusionJunction> = (0..=n)
        .map(|k| {
            let a = std::f32::consts::TAU * (k % n) as f32 / n as f32;
            junction(radius * a.cos(), radius * a.sin(), 0.37, 1)
        })
        .collect();
    if reverse {
        junctions.reverse();
    }
    ExtrusionLine {
        junctions,
        inset_idx: 1,
        is_odd: false,
        is_closed: false,
    }
}

/// Canonical `PolylineStitcher::stitch` lets closing a chain on itself
/// compete with extending it: two concentric even bead loops 0.37 mm apart,
/// each already meeting itself, are within the 0.407 mm stitch distance of
/// each other, but each closes on its own (closing distance 0 + 0.01 mm
/// bias beats 0.37 mm). Joining them instead prints a chord between the two
/// beads. The two beads of a ring run in opposite directions, so the
/// even-line reversal gate does not keep them apart.
#[test]
fn stitch_closes_each_loop_before_joining_a_parallel_loop() {
    let outer = circle_line(2.43, false);
    let inner = circle_line(2.06, true);
    let result = stitch_extrusions(vec![outer.clone(), inner.clone()], 0.407);

    assert_eq!(result.len(), 2, "expected two loops, got {result:#?}");
    for (line, expected) in [(&result[0], &outer), (&result[1], &inner)] {
        let radius = |j: &ExtrusionJunction| j.p.x.hypot(j.p.y);
        let want = radius(&expected.junctions[0]);
        assert!(line.is_closed, "each loop closes on itself");
        assert!(
            line.junctions.iter().all(|j| (radius(j) - want).abs() < 1e-3),
            "a loop of radius {want} picked up junctions of the other loop"
        );
    }
}
