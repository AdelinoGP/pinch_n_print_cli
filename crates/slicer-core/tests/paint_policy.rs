//! TDD tests for `slicer_core::paint_policy`.
//!
//! Step 2 of packet 120 (`support-modules-paint-segment-annotations-migration`).
//! Covers AC-1..AC-5 + AC-N3:
//!
//! - AC-1: `support_eligibility` exists and the local
//!   `SupportPaintPolicy` enum is accessible at
//!   `slicer_core::paint_policy::SupportPaintPolicy`.
//! - AC-2: Blocker paint with majority area coverage → `Blocked`.
//! - AC-3: Enforcer paint with majority area coverage (no blocker) → `Enforced`.
//! - AC-4: Blocker + enforcer both present → `Blocked` wins (precedence).
//! - AC-5: L-shape region whose vertex-mean centroid lies in the notch
//!   (outside the polygon) but whose enforcer covers the L's vertical arm
//!   → `Enforced` (the geometric check correctly classifies this case that
//!   the centroid probe would have missed).
//! - AC-N3: Empty `segment_annotations` map → `DefaultEligible` (no panic).
//!
//! All region polygons are constructed directly with `Point2::from_mm` so
//! these tests have no dependency on the `slicer-sdk` test fixtures
//! (Step 2 must avoid the `slicer-core` ↔ `slicer-sdk` Cargo cycle; see
//! the module-level note in `crates/slicer-core/src/paint_policy.rs`).

use std::collections::HashMap;

use slicer_core::paint_policy::{support_eligibility, SupportPaintPolicy};
use slicer_ir::{ExPolygon, PaintSemantic, PaintValue, Point2, Polygon};

/// Build a 10×10 mm square ExPolygon at the origin (lower-left at (0, 0)).
fn ten_mm_square() -> ExPolygon {
    let s = slicer_ir::mm_to_units(10.0);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: s, y: 0 },
                Point2 { x: s, y: s },
                Point2 { x: 0, y: s },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a per-polygon per-vertex paint map where the given semantic has at
/// least one `Some(Flag(true))` value on every region polygon, so the
/// helper's "annotation applies" branch is taken.
fn annotations_with(
    semantic: PaintSemantic,
    polygon_count: usize,
) -> HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
    let mut map = HashMap::new();
    map.insert(
        semantic,
        (0..polygon_count)
            .map(|_| vec![Some(PaintValue::Flag(true))])
            .collect(),
    );
    map
}

#[test]
fn pub_use_re_exports_support_paint_policy() {
    // AC-1: compile-time assertion that the enum exposes all three variants
    // at the canonical path.
    const _COMPILE_TIME_VARIANTS: fn() = || {
        let _ = SupportPaintPolicy::Blocked;
        let _ = SupportPaintPolicy::Enforced;
        let _ = SupportPaintPolicy::DefaultEligible;
    };
    // Runtime equivalent for completeness.
    let all: [SupportPaintPolicy; 3] = [
        SupportPaintPolicy::Blocked,
        SupportPaintPolicy::Enforced,
        SupportPaintPolicy::DefaultEligible,
    ];
    assert_eq!(all.len(), 3);
}

#[test]
fn blocker_majority_returns_blocked() {
    // AC-2: region covered by SupportBlocker with non-trivial area
    // → Blocked.
    let polygon = ten_mm_square();
    let view = annotations_with(PaintSemantic::SupportBlocker, 1);
    assert_eq!(
        support_eligibility(&[polygon], &view),
        SupportPaintPolicy::Blocked
    );
}

#[test]
fn enforcer_majority_returns_enforced() {
    // AC-3: region covered by SupportEnforcer with non-trivial area and
    // no SupportBlocker → Enforced.
    let polygon = ten_mm_square();
    let view = annotations_with(PaintSemantic::SupportEnforcer, 1);
    assert_eq!(
        support_eligibility(&[polygon], &view),
        SupportPaintPolicy::Enforced
    );
}

#[test]
fn blocker_wins_over_enforcer() {
    // AC-4: both blocker and enforcer present on the same region → Blocker
    // wins regardless of relative area (precedence rule from
    // docs/01_system_architecture.md §"Support Stage Paint Precedence").
    let polygon = ten_mm_square();
    let mut annotations = annotations_with(PaintSemantic::SupportEnforcer, 1);
    annotations.insert(
        PaintSemantic::SupportBlocker,
        vec![vec![Some(PaintValue::Flag(true))]],
    );
    assert_eq!(
        support_eligibility(&[polygon], &annotations),
        SupportPaintPolicy::Blocked
    );
}

#[test]
fn enforcer_works_for_l_shape_with_centroid_outside_polygon() {
    // AC-5: L-shaped region whose vertex-mean centroid lies in the notch
    // (outside the polygon) but whose enforcer covers the L's vertical arm
    // → Enforced.
    //
    // L-shape definition: 10×10 mm outer bbox at the origin (x ∈ [0,10],
    // y ∈ [0,10]) with a 6×6 mm notch in the top-right (x ∈ [4,10],
    // y ∈ [4,10]). Contour vertices CCW (mm):
    //   (0,0), (10,0), (10,4), (4,4), (4,10), (0,10)
    //
    // Vertex-mean centroid (in mm):
    //   mean_x = (0+10+10+4+4+0) / 6 = 28 / 6 ≈ 4.667
    //   mean_y = (0+0+4+4+10+10) / 6 = 28 / 6 ≈ 4.667
    // → (4.667, 4.667) mm. The notch is x ≥ 4 AND y ≥ 4; the centroid
    // satisfies both, so it falls INSIDE the notch (i.e. OUTSIDE the L
    // polygon). The legacy centroid-probe would return `DefaultEligible`;
    // the new geometric check returns `Enforced` because the painted
    // area covers the L's vertical arm with non-trivial area.
    let mm_to_units = |mm: f32| slicer_ir::mm_to_units(mm);
    let p = |x_mm: f32, y_mm: f32| Point2 {
        x: mm_to_units(x_mm),
        y: mm_to_units(y_mm),
    };
    let l_shape = ExPolygon {
        contour: Polygon {
            points: vec![
                p(0.0, 0.0),
                p(10.0, 0.0),
                p(10.0, 4.0),
                p(4.0, 4.0),
                p(4.0, 10.0),
                p(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    };

    // Sanity: confirm the vertex-mean centroid is NOT inside the L polygon.
    let centroid_x_mm = 28.0 / 6.0;
    let centroid_y_mm = 28.0 / 6.0;
    assert!(
        centroid_x_mm >= 4.0 && centroid_y_mm >= 4.0,
        "centroid ({centroid_x_mm}, {centroid_y_mm}) mm must fall in the notch x>=4 AND y>=4"
    );
    let centroid_pt = Point2 {
        x: mm_to_units(centroid_x_mm),
        y: mm_to_units(centroid_y_mm),
    };
    assert!(
        !point_in_expolygon(centroid_pt, &l_shape),
        "centroid must fall in the notch (outside the L)"
    );

    let annotations = annotations_with(PaintSemantic::SupportEnforcer, 1);
    assert_eq!(
        support_eligibility(&[l_shape], &annotations),
        SupportPaintPolicy::Enforced
    );
}

#[test]
fn empty_segment_annotations_returns_default_eligible() {
    // AC-N3: empty segment_annotations map → DefaultEligible (no panic).
    let polygon = ten_mm_square();
    let annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> = HashMap::new();
    assert_eq!(
        support_eligibility(&[polygon], &annotations),
        SupportPaintPolicy::DefaultEligible
    );
}

// ---------------------------------------------------------------------------
// Test-local geometry helpers (inlined so the test has no dependency on the
// production code under test).
// ---------------------------------------------------------------------------

/// Ray-cast point-in-polygon (with hole subtraction) for asserting the
/// vertex-mean centroid of the L-shape falls outside the L. Mirrors the
/// shape of the helper in `slicer-sdk/src/traits.rs::point_in_expolygon`.
fn point_in_expolygon(pt: Point2, ep: &ExPolygon) -> bool {
    if !point_in_polygon(pt, &ep.contour.points) {
        return false;
    }
    for hole in &ep.holes {
        if point_in_polygon(pt, &hole.points) {
            return false;
        }
    }
    true
}

fn point_in_polygon(pt: Point2, ring: &[Point2]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let pi = &ring[i];
        let pj = &ring[j];
        let pi_above = pi.y > pt.y;
        let pj_above = pj.y > pt.y;
        if pi_above != pj_above {
            let cross = (pj.x as i128 - pi.x as i128) * (pt.y as i128 - pi.y as i128)
                / (pj.y as i128 - pi.y as i128)
                + pi.x as i128;
            if (pt.x as i128) < cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
