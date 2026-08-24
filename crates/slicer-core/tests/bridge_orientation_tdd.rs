#![allow(missing_docs)]

//! TDD tests for the pure port of OrcaSlicer's active inline
//! `detect_bridging_direction` semantics (`BridgeDetector.cpp`), added to
//! `crates/slicer-core/src/algos/prepass_slice.rs` by packet 235 Step 1.
//!
//! All fixtures are built from polygon primitives only (no mesh fixtures).
//! Tie-break behaviour follows ADR-0061 (smallest quantized angle among
//! exact-minimum-cost candidates).

use slicer_core::algos::prepass_slice::{
    detect_bridging_direction_deg, floating_edges_of_gated_area, update_external_bridge_orientation,
};
use slicer_ir::{ExPolygon, Point2, Polygon, SlicedRegion};

/// Axis-aligned rectangle as a CCW `ExPolygon` (coordinates in mm).
fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: vec![],
    }
}

/// Axis-aligned rectangle centered at the origin, rotated by `deg` about the
/// origin. Trig is exact f64; conversion to units happens once per point.
fn rotated_center_rect(half_w: f64, half_h: f64, deg: f64) -> ExPolygon {
    let rad = deg.to_radians();
    let (s, c) = rad.sin_cos();
    let corners = [
        (-half_w, -half_h),
        (half_w, -half_h),
        (half_w, half_h),
        (-half_w, half_h),
    ];
    ExPolygon {
        contour: Polygon {
            points: corners
                .iter()
                .map(|&(x, y)| Point2::from_mm((x * c - y * s) as f32, (x * s + y * c) as f32))
                .collect(),
        },
        holes: vec![],
    }
}

/// General polygon from mm coordinates, rotated by `deg` about the origin.
fn rotated_polygon(points_mm: &[(f64, f64)], deg: f64) -> ExPolygon {
    let rad = deg.to_radians();
    let (s, c) = rad.sin_cos();
    ExPolygon {
        contour: Polygon {
            points: points_mm
                .iter()
                .map(|&(x, y)| Point2::from_mm((x * c - y * s) as f32, (x * s + y * c) as f32))
                .collect(),
        },
        holes: vec![],
    }
}

/// Symmetric plus/cross centered at the origin (CCW contour). Total horizontal
/// edge length equals total vertical edge length exactly, so candidate normals
/// of both families accumulate identical cost in exact arithmetic.
fn plus_polygon_mm(half_w: f64, half_h_bar: f64, half_w_bar: f64, half_h: f64) -> Vec<(f64, f64)> {
    let (a, b, c, d) = (half_w, half_h_bar, half_w_bar, half_h);
    vec![
        (c, d),
        (-c, d),
        (-c, b),
        (-a, b),
        (-a, -b),
        (-c, -b),
        (-c, -d),
        (c, -d),
        (c, -b),
        (a, -b),
        (a, b),
        (c, b),
    ]
}

/// AC-1: 40x10mm span anchored on both ends orients along the span (0 deg).
#[test]
fn two_sided_rect_gap_orients_along_span() {
    let to_cover = vec![rect(-20.0, -5.0, 20.0, 5.0)];
    let anchors = vec![rect(-30.0, -5.0, -18.0, 5.0), rect(18.0, -5.0, 30.0, 5.0)];
    let deg = detect_bridging_direction_deg(&to_cover, &anchors);
    assert!(
        deg.abs() < 1e-9,
        "span-direction rect must orient at exactly 0 deg, got {deg}"
    );
}

/// AC-2a: an anchor edge flush with the gated-area boundary is absorbed by the
/// SCALED_EPSILON (1 unit) anchor growth, leaving 3 floating edges of the rect.
#[test]
fn flush_anchor_edges_absorbed_within_scaled_epsilon() {
    let gated = vec![rect(0.0, 0.0, 40.0, 10.0)];
    // Anchor butted flush against the left edge (shared edge at x = 0).
    let anchors = vec![rect(-2.0, 0.0, 0.0, 10.0)];
    let segs = floating_edges_of_gated_area(&gated, &anchors);
    assert_eq!(
        segs.len(),
        3,
        "flush anchor must absorb the boundary-coincident edge"
    );
}

/// AC-2b: recessing the same anchor 0.5mm behind the boundary keeps all 4
/// boundary edges as floating candidates (0.5mm >> SCALED_EPSILON).
#[test]
fn recessed_anchor_keeps_floating_edge_candidates() {
    let gated = vec![rect(0.0, 0.0, 40.0, 10.0)];
    let anchors = vec![rect(-2.5, 0.0, -0.5, 10.0)];
    let segs = floating_edges_of_gated_area(&gated, &anchors);
    assert_eq!(
        segs.len(),
        4,
        "recessed anchor must leave every boundary edge floating"
    );
}

/// AC-3: a fully edge-anchored 16x2mm island has no floating edges; the
/// principal-component fallback picks the minor axis (90 deg).
#[test]
fn fully_anchored_island_picks_minor_principal_axis() {
    let island = vec![rect(-8.0, -1.0, 8.0, 1.0)];
    // Four anchor blocks overlapping every side by 0.5mm inward; the uncovered
    // interior ([-7.5, 7.5] x [-0.5, 0.5]) is the overhang area.
    let anchors = vec![
        rect(-9.0, -2.0, 9.0, -0.5),
        rect(-9.0, 0.5, 9.0, 2.0),
        rect(-9.0, -2.0, -7.5, 2.0),
        rect(7.5, -2.0, 9.0, 2.0),
    ];
    let deg = detect_bridging_direction_deg(&island, &anchors);
    assert_eq!(
        deg, 90.0,
        "minor principal axis of a 15x1mm overhang must be exactly 90 deg"
    );
}

/// AC-4: an empty overhang difference falls back to {1, 0} -> exactly 0.0 deg,
/// never NaN, never a panic.
#[test]
fn degenerate_overhang_falls_back_to_x_axis() {
    let cover = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let deg = detect_bridging_direction_deg(&cover, &cover.clone());
    assert!(deg.is_finite(), "fallback angle must be finite, got {deg}");
    assert_eq!(deg, 0.0, "fully-degenerate fallback must be exactly 0 deg");
}

/// AC-5: the AC-1 fixture rigidly rotated by k*11.25 deg (k = 0..=16) stays in
/// the half-open range [0, 180) and matches (k*11.25) mod 180 within 1e-3.
#[test]
fn rotated_rect_sweep_stays_in_half_open_range() {
    for k in 0..=16 {
        let ang = f64::from(k) * 11.25;
        let to_cover = vec![rotated_center_rect(20.0, 5.0, ang)];
        // Same anchors as AC-1: overlapping the left/right ends by 2mm, full
        // height, rigidly rotated with the cover rect.
        let left = rotated_polygon(
            &[(-30.0, -5.0), (-18.0, -5.0), (-18.0, 5.0), (-30.0, 5.0)],
            ang,
        );
        let right = rotated_polygon(&[(18.0, -5.0), (30.0, -5.0), (30.0, 5.0), (18.0, 5.0)], ang);
        let anchors = vec![left, right];
        let deg = detect_bridging_direction_deg(&to_cover, &anchors);
        assert!(
            (0.0..180.0).contains(&deg),
            "k={k}: output {deg} outside [0, 180)"
        );
        let expect = ang % 180.0;
        let err = (f64::from(deg) - expect).abs();
        assert!(
            err < 1e-3,
            "k={k}: expected {expect} deg, got {deg} (err {err})"
        );
    }
}

/// AC-N1: equal-cost tie between the horizontal and vertical candidate normals
/// of a symmetric cross resolves to the smallest quantized angle key
/// (ceil(atan2 * 1000) = -1570 for normal (0, -1)), yielding exactly 0 deg —
/// never 90 deg, never order-dependent.
#[test]
fn equal_cost_tie_resolves_smallest_quantized_angle() {
    // Symmetric plus: horizontal-bar 30x10, vertical-bar 10x30. Total edge
    // length per axis family is exactly 60mm, so both candidate normal
    // families tie at identical cost in exact integer arithmetic.
    let cross = vec![ExPolygon {
        contour: Polygon {
            points: plus_polygon_mm(15.0, 5.0, 5.0, 15.0)
                .iter()
                .map(|&(x, y)| Point2::from_mm(x as f32, y as f32))
                .collect(),
        },
        holes: vec![],
    }];
    let deg = detect_bridging_direction_deg(&cross, &[]);
    assert_eq!(
        deg, 0.0,
        "ADR-0061 tie-break must pick the smallest quantized angle (-1570) -> 0 deg"
    );
}

/// AC-N2: the cross rotated 7 deg must yield 7.0 deg within 1e-3 — a legacy
/// 5 deg snapping sweep would emit 5 or 10. The horizontal edge family is
/// dominant (60mm vs 20mm total length), so the minimal-cost winner is
/// perpendicular to it and unique modulo the ADR-0061 tie between its two
/// opposite normals (both flip to a direction congruent to 7 deg mod 180).
#[test]
fn rotated_cross_rejects_legacy_five_degree_snap() {
    // Asymmetric plus: horizontal-bar 30x6, vertical-bar 6x10.
    let cross = vec![rotated_polygon(&plus_polygon_mm(15.0, 3.0, 3.0, 5.0), 7.0)];
    let deg = detect_bridging_direction_deg(&cross, &[]);
    let err = (f64::from(deg) - 7.0).abs();
    assert!(
        err < 1e-3,
        "rotated cross must orient at 7 deg within 1e-3, got {deg} (err {err})"
    );
}

/// AC-6a: `update_external_bridge_orientation` writes
/// `region.bridge_orientation_deg` from the GATED bridge areas + RAW lower
/// contours — exactly `detect_bridging_direction_deg` on the same inputs.
#[test]
fn orientation_written_from_gated_geometry() {
    // Gated 40x10mm span; raw lower contours anchor both ends (2mm overlap).
    let bridge_areas = vec![rect(0.0, 0.0, 40.0, 10.0)];
    let lower = vec![rect(-2.0, 0.0, 0.0, 10.0), rect(40.0, 0.0, 42.0, 10.0)];
    let mut region = SlicedRegion {
        bridge_areas: bridge_areas.clone(),
        // Sentinel from the retired Slice-stage heuristic; must be overwritten.
        bridge_orientation_deg: 123.0,
        ..Default::default()
    };
    let expected = detect_bridging_direction_deg(&bridge_areas, &lower);
    update_external_bridge_orientation(&mut region, Some(&lower));
    assert_eq!(
        region.bridge_orientation_deg, expected,
        "orientation must derive from the gated geometry + raw lower contours"
    );
}

/// AC-6b: empty gated bridge areas leave a pre-existing orientation value
/// untouched (the gate cleared the candidates; there is nothing to orient).
#[test]
fn empty_bridge_areas_leave_orientation_untouched() {
    const SENTINEL: f32 = 42.5;
    let lower = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let mut region = SlicedRegion {
        bridge_areas: Vec::new(),
        bridge_orientation_deg: SENTINEL,
        ..Default::default()
    };
    update_external_bridge_orientation(&mut region, Some(&lower));
    assert_eq!(
        region.bridge_orientation_deg, SENTINEL,
        "empty gated areas must be a no-op on the orientation field"
    );
}
