//! G20 (Step 4b) TDD lock-in for the distance-gated `ExtrusionLine::simplify`
//! intersection-distance gate (OrcaSlicer `Arachne/utils/ExtrusionLine.cpp:
//! 163-220`, ported in packet 155).
//!
//! These tests pin the behaviour introduced by the G20 code restructure:
//! - AC-7: an interior junction whose `next` is far is *preserved* when the
//!   intersection of the surrounding infinite lines lies farther than
//!   `smallest_line_segment_squared` from the surviving neighbour
//!   (`dist_greater` reject path).
//! - AC-8: the same shape but the intersection is within the gate, so the
//!   previously-pushed junction is popped and replaced by the intersection
//!   carrying `current`'s width/perimeter_index verbatim.
//! - AC-9: the height used at the gate sites is the OrcaSlicer Shoelace formula
//!   `height_2 = area_removed_so_far² / base_length_2` with a non-zero
//!   `accumulated_area_removed` (upstream junction removed), not a naive
//!   per-junction recompute.
//! - AC-N3: a degenerate 2-junction open line is returned unchanged.
//! - AC-N4: a closed line with the minimum 3 junctions is preserved.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::simplify_toolpaths;
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn j(x: f32, y: f32, width: f32, perimeter_index: u32) -> ExtrusionJunction {
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
        perimeter_index,
    }
}

fn line(junctions: Vec<ExtrusionJunction>, is_closed: bool) -> ExtrusionLine {
    ExtrusionLine {
        junctions,
        inset_idx: 0,
        is_odd: false,
        is_closed,
    }
}

/// Routing parameters that send `simplify_toolpaths` down the distance-gated
/// path (both gates positive) with no area-deviation rejection.
const SMALLEST: f64 = 1e-3;
const ALLOWED: f64 = 1.0;
const MAX_AREA_DEV: f64 = f64::INFINITY;

/// AC-7 (preserves_junction): an interior junction that is a candidate for the
/// tier-3 special case (`next` far away) is retained because the intersection
/// of the infinite lines `(prev_prev → prev)` and `(curr → next)` lies farther
/// than `smallest_line_segment_squared` from `prev`. OrcaSlicer
/// `ExtrusionLine.cpp:163-175` (`dist_greater` reject path).
#[test]
fn simplify_intersection_distance_gate_preserves_junction() {
    // Open polyline: J1 retained (short segment, off the J0-J2 chord),
    // J2 is the tier-3 candidate whose surrounding-line intersection is far
    // from J1, so it is preserved.
    let input = line(
        vec![
            j(0.0, 0.0, 0.4, 0),
            j(0.1, 0.01, 0.4, 0),
            j(0.11, 0.0131, 0.4, 0),
            j(0.2, 0.0, 0.4, 0),
        ],
        false,
    );

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    // Junction count is preserved: nothing removed.
    assert_eq!(
        result[0].junctions.len(),
        4,
        "AC-7: dist_greater reject must preserve the interior junction"
    );
    // Endpoints and both interiors are intact.
    assert_eq!(result[0].junctions[0].p.x, 0.0);
    assert_eq!(result[0].junctions[3].p.x, 0.2);
}

/// AC-8 (replacement_moves_to_intersection): the same tier-3 shape but the
/// intersection lies within `smallest_line_segment_squared` of both `prev` and
/// `curr`, so the previously-pushed junction is popped and replaced by the
/// intersection carrying `current`'s width and `perimeter_index` verbatim
/// (OrcaSlicer `ExtrusionLine.cpp:196-220`).
#[test]
fn simplify_junction_replacement_moves_to_intersection() {
    // Lines (J0->J1) and (J2->J3) cross exactly at J1. J2 is a short segment
    // from J1. The tier-3 replace branch fires: J1 is popped and the
    // intersection (coincident with J1) is pushed, carrying J2's width/perim.
    let input = line(
        vec![
            j(0.0, 0.0, 0.4, 0),
            j(0.1, 0.01, 0.4, 0),     // previously-pushed junction (width 0.4)
            j(0.1005, 0.008, 0.9, 7), // the current; width/perim must be carried
            j(0.05, 0.21, 0.4, 0),
        ],
        false,
    );

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    // Pop + push preserves the running count (2 -> 2), so a 4-junction input
    // collapses to 3 (one interior removed via replacement).
    assert_eq!(
        result[0].junctions.len(),
        3,
        "AC-8: replacement must pop the previously-pushed junction"
    );

    let mid = &result[0].junctions[1];
    // The replacement sits at the intersection (= J1 position) within 1e-3 mm.
    assert!(
        (mid.p.x - 0.1).abs() < 1e-3 && (mid.p.y - 0.01).abs() < 1e-3,
        "AC-8: junction moved to intersection (got {:?})",
        mid.p
    );
    // It carries `current`'s (J2) width and perimeter_index verbatim.
    assert_eq!(mid.p.width, 0.9, "AC-8: width must be carried from current");
    assert_eq!(
        mid.perimeter_index, 7,
        "AC-8: perimeter_index must be carried from current"
    );
}

/// AC-9 (uses_shoelace_height_2): an upstream junction (J1) is removed, making
/// `accumulated_area_removed != 0`. The next interior junction (J2) is
/// near-colinear with a tiny local triangle, so a naive per-junction recompute
/// would drop it at the tier-2 near-colinear gate — but the Shoelace formula
/// folds in `accumulated_area_removed`, raising `height_2` above the
/// near-colinear threshold and keeping the junction
/// (OrcaSlicer `ExtrusionLine.cpp:151`).
#[test]
fn simplify_distance_gated_uses_shoelace_height_2() {
    // J1 removed (short segment, zero height). J2 is near-colinear with J0-J3
    // with a negligible local triangle; the carried-over accumulated area keeps
    // it under the Shoelace formula.
    let input = line(
        vec![
            j(0.0, 0.0, 0.4, 0),
            j(0.02, 0.02, 0.4, 0),      // removed upstream -> accumulated != 0
            j(0.04, 0.0, 0.4, 0),       // near-colinear; Shoelace keeps it
            j(0.06, 0.0000003, 0.4, 0), // last (always retained)
        ],
        false,
    );

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    // J1 is removed, J2 is retained by the Shoelace height (naive would also
    // have removed J2). Expect [J0, J2, J3].
    assert_eq!(
        result[0].junctions.len(),
        3,
        "AC-9: accumulated-area Shoelace height must retain J2"
    );
    let kept = &result[0].junctions[1];
    assert!(
        (kept.p.x - 0.04).abs() < 1e-4 && (kept.p.y - 0.0).abs() < 1e-4,
        "AC-9: retained junction must be J2 (got {:?})",
        kept.p
    );
}

/// AC-N3 (degenerate_two_junctions): an open `ExtrusionLine` with exactly two
/// junctions is returned unchanged (both endpoints always retained, no interior
/// to simplify).
#[test]
fn simplify_degenerate_two_junctions_unchanged() {
    let junctions = vec![j(0.0, 0.0, 0.4, 0), j(5.0, 2.0, 0.4, 0)];
    let input = line(junctions.clone(), false);

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].junctions, junctions,
        "AC-N3: a 2-junction open line must be returned unchanged"
    );
}

/// AC-N4 (closed_line_minimum_size): a closed `ExtrusionLine` with the minimum
/// 3 junctions cannot be simplified and is returned intact.
#[test]
fn simplify_closed_line_minimum_size_preserved() {
    let junctions = vec![
        j(0.0, 0.0, 0.4, 0),
        j(1.0, 0.0, 0.4, 0),
        j(0.5, 0.866, 0.4, 0),
    ];
    let input = line(junctions.clone(), true);

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].junctions.len(),
        3,
        "AC-N4: a closed 3-junction line must preserve all 3 junctions"
    );
    assert!(result[0].is_closed);
}

/// AC-N4 (closed_line_minimum_size, falsifying case): unlike
/// `simplify_closed_line_minimum_size_preserved` above (whose equilateral
/// triangle happens to survive the generic tier checks regardless of whether
/// the guard is `is_closed`-aware), this fixture is near-colinear and WOULD be
/// collapsed to 2 junctions by the generic tier-2 near-colinear path if the
/// minimum-size guard did not distinguish closed from open lines
/// (`min_path_size = is_closed ? 3 : 2`, `ExtrusionLine.cpp:63-65`). A guard
/// that only checked `n <= 2` (ignoring `is_closed`) would let this 3-junction
/// closed triangle fall through into the walk and lose its middle junction.
#[test]
fn simplify_closed_line_near_colinear_minimum_size_preserved() {
    let junctions = vec![
        j(0.0, 0.0, 0.4, 0),
        j(5.0, 0.0000001, 0.4, 0),
        j(10.0, 0.0, 0.4, 0),
    ];
    let input = line(junctions.clone(), true);

    let result = simplify_toolpaths(vec![input], 0.0, SMALLEST, ALLOWED, MAX_AREA_DEV);

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].junctions.len(),
        3,
        "AC-N4: a closed 3-junction line must be preserved intact even when \
         near-colinear, because the is_closed-aware minimum-size guard fires \
         before the generic tier-2 near-colinear removal path can run"
    );
    assert!(result[0].is_closed);
}
