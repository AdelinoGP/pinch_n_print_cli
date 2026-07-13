//! Packet 153 (Step 3): port OrcaSlicer's `canReverse` parity gate into
//! `stitch_extrusions`.
//!
//! Even (`is_odd == false`) line groups must NOT allow merges that would
//! reverse a chain: the same-side endpoint pairs `(E,E)` and `(S,S)` are
//! rejected. Odd (`is_odd == true`) groups keep the full 4-way merge, so an
//! odd-line reversal is still permitted.
//!
//! Coordinates are in slicer units (1 unit = 100 nm = 10⁻⁴ mm), matching
//! `Point3WithWidth`'s coordinate convention in this pipeline.

use slicer_core::arachne::stitch::stitch_extrusions;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn j(x: i64, y: i64) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x: x as f32,
            y: y as f32,
            z: 0.0,
            width: 20.0,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index: 0,
    }
}

fn line(a: (i64, i64), b: (i64, i64), is_odd: bool) -> ExtrusionLine {
    ExtrusionLine {
        junctions: vec![j(a.0, a.1), j(b.0, b.1)],
        inset_idx: 0,
        is_odd,
        is_closed: false,
    }
}

/// Geometry: two back-to-back lines whose ONLY within-gap endpoint pair is a
/// reversal pair `(E,E)`.
///
/// - Line A: (0,0) → (10_000, 0)
/// - Line B: (12_000, 100_000) → (12_000, 0)
///
/// Closest pair is A.End(10_000,0) to B.End(12_000,0): 2_000 units (reversal).
/// Every non-reversal pair (E,S)/(S,E)/(S,S) is ≥ 12_000 units away.
const MAX_GAP: f64 = 5_000.0;

/// AC-6: even lines whose only valid join is a reversal must remain unjoined.
#[test]
fn even_line_reversal_is_rejected() {
    let lines = vec![
        line((0, 0), (10_000, 0), false),
        line((12_000, 100_000), (12_000, 0), false),
    ];
    let out = stitch_extrusions(lines, MAX_GAP);

    assert_eq!(out.len(), 2, "even reversal pair must NOT merge");
    for l in &out {
        assert!(!l.is_closed, "unjoined even lines stay open");
    }
}

/// AC-N2: odd lines whose only valid join is a reversal are still permitted to
/// merge.
#[test]
fn odd_line_reversal_is_permitted() {
    let lines = vec![
        line((0, 0), (10_000, 0), true),
        line((12_000, 100_000), (12_000, 0), true),
    ];
    let out = stitch_extrusions(lines, MAX_GAP);

    assert_eq!(out.len(), 1, "odd reversal pair must merge");
    let merged = &out[0];
    // A (2 junctions) + reversed B (2 junctions) = 4 junctions; not closed.
    assert_eq!(merged.junctions.len(), 4, "merged line keeps both chains");
    assert!(!merged.is_closed, "back-to-back lines do not close");
}
