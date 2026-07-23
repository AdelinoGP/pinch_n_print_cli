//! Packet 153 (Step 4): port OrcaSlicer's tiny-polygon non-closure rule into
//! `finalize_chain`.
//!
//! A chain is left open (`is_closed = false`) when the total polyline length
//! plus the closing-segment distance is `< 3 * max_gap`, or when the chain has
//! `<= 2` junctions. This prevents small/short polylines from being folded into
//! tiny closed loops (they may still extend into a longer open polyline).
//!
//! # Unit convention
//!
//! Everything here is in **millimeters**: junction coordinates
//! (`Point3WithWidth`'s documented unit), `max_gap` (`stitch_extrusions`'s
//! documented contract), and the derived `3 * max_gap` threshold. This is the
//! convention the production call site (`arachne/pipeline.rs`, which passes
//! `preferred_bead_width_outer - 1e-6`) uses.
//!
//! Corrected 2026-07-16 (D-147-STITCH-TINY-POLY-UNITS): this file used to pass
//! `max_gap` in slicer units and rely on `finalize_chain` dividing it back down
//! by `UNITS_PER_MM`. The scale-up and the divide-down cancelled exactly, so
//! these tests stayed green while the same division defeated the rule in
//! production (threshold 1.2mm -> 0.00012mm) — this suite could not observe the
//! quantity it exists to test. The fixtures and their asserted outcomes are
//! unchanged; only the `max_gap` input's unit is corrected, so `3 * max_gap` is
//! still 1.2mm.

use slicer_core::arachne::stitch::stitch_extrusions;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn j(x: f32, y: f32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.0,
            width: 20.0,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
        },
        perimeter_index: 0,
    }
}

fn line(pts: &[(f32, f32)], is_odd: bool) -> ExtrusionLine {
    ExtrusionLine {
        junctions: pts.iter().map(|&(x, y)| j(x, y)).collect(),
        inset_idx: 0,
        is_odd,
        is_closed: false,
    }
}

/// `max_gap` in mm, matching the production call site
/// (`arachne/pipeline.rs`: `preferred_bead_width_outer - 1e-6`).
/// `3 * MAX_GAP` = 1.2mm is the tiny-poly threshold every fixture below is
/// reasoned against.
const MAX_GAP: f64 = 0.4;

/// AC-7: a single even `ExtrusionLine` whose total polyline length plus
/// closing-segment distance is `< 3 * max_gap` (in mm) must stay open.
///
/// Junctions (mm): (0,0)-(1,0)-(2,0). Polyline length = 1 + 1 = 2 mm.
/// Closing-segment distance = 2 mm. Sum = 4 mm.
/// `max_gap = 0.4 mm` => `3 * max_gap = 1.2 mm`. Sum 4 >= 1.2, so this
/// specific fixture is NOT tiny — see the deliberately-tiny fixture below.
///
/// To exercise the rule we instead use a genuinely tiny polyline:
/// (0,0)-(0.1,0)-(0.2,0): polyline = 0.2 mm, closing = 0.2 mm, sum = 0.4 mm
/// < 1.2 mm => rule fires. Closing distance 0.2 mm <= max_gap 0.4 mm, so
/// WITHOUT the rule it would close; WITH the rule it stays open.
#[test]
fn tiny_polygon_below_threshold_stays_open() {
    let poly = line(&[(0.0, 0.0), (0.1, 0.0), (0.2, 0.0)], false);
    let out = stitch_extrusions(vec![poly], MAX_GAP);

    assert_eq!(out.len(), 1, "single input line -> single output line");
    let l = &out[0];
    assert!(!l.is_closed, "sub-3*max_gap chain must NOT close");
    // No closing-junction duplicate appended.
    assert_eq!(
        l.junctions.len(),
        3,
        "junction count must be unchanged (no closing dup)"
    );
}

/// AC-N3: a single even `ExtrusionLine` whose total polyline length plus
/// closing-segment distance is `>= 3 * max_gap` (and closing-segment <=
/// max_gap) must close into a loop.
///
/// Junctions (mm): (0,0)-(5,0)-(10,0)-(1,0). Polyline length = 5 + 5 + 9 = 19
/// mm. Closing-segment distance = 1 mm. Sum = 20 mm. `3 * max_gap = 1.2 mm`.
/// Sum 20 >= 1.2 => rule does NOT fire. Closing-segment 1 mm > max_gap 0.4 mm
/// would NOT close either, so shrink the closing gap: use (0,0)-(5,0)-(10,0)-
/// (0.1,0): polyline = 5 + 5 + 9.9 = 19.9 mm, closing = 0.1 mm, sum = 20 mm
/// >= 1.2 mm => rule does not fire; closing 0.1 mm <= 0.4 mm => closes.
#[test]
fn large_polygon_at_or_above_threshold_closes() {
    let poly = line(&[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (0.1, 0.0)], false);
    let out = stitch_extrusions(vec![poly], MAX_GAP);

    assert_eq!(out.len(), 1, "single input line -> single output line");
    let l = &out[0];
    assert!(l.is_closed, ">= 3*max_gap chain within max_gap must close");
    // Original 4 junctions + 1 closing duplicate.
    assert_eq!(
        l.junctions.len(),
        5,
        "closed line gains one closing-junction dup"
    );
}

/// `junctions.len() <= 2` guard: a 2-junction chain whose endpoints are within
/// `max_gap` must still stay open (OrcaSlicer rejects 2-vertex polygons).
#[test]
fn two_junction_chain_never_closes() {
    let poly = line(&[(0.0, 0.0), (0.1, 0.0)], false);
    let out = stitch_extrusions(vec![poly], MAX_GAP);

    assert_eq!(out.len(), 1);
    assert!(!out[0].is_closed, "2-junction chain must stay open");
    assert_eq!(
        out[0].junctions.len(),
        2,
        "no closing dup for 2-junction chain"
    );
}
