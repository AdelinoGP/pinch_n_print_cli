#![allow(missing_docs)]

use slicer_core::algos::bridge_over_infill::{
    construct_anchored_polygon, depth_window_start, determine_bridging_angle, filled_window_start,
    gather_areas_w_depth, internal_bridge_angles, presort_bridge_candidates,
    remove_filled_polygons_on_lower_layers, BridgeCandidateLayer, BridgeDepthLayer,
    InternalBridgeAngleInputs,
};
use slicer_core::flow::canonical_bridging_flow;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn square(width: f32, height: f32) -> ExPolygon {
    rect(0.0, 0.0, width, height)
}

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
        holes: Vec::new(),
    }
}

#[test]
fn bridging_angle_unequal_anchor_lengths_are_length_weighted() {
    let anchors = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)],
        vec![
            Point2::from_mm(0.0, 10.0),
            Point2::from_mm(3.0, 10.0 + 3.0 * 10.0_f32.to_radians().tan()),
        ],
    ];
    let area = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        vec![Point2::from_mm(0.0, 10.0), Point2::from_mm(3.0, 10.0)],
    ];
    // Samples are 1 + 2: (90 + 100 + 100) / 3 = 96.6666667 degrees.
    let angle = determine_bridging_angle(&anchors, &area, 0.0);
    // Point2 stores integer native units, so the 10 degree line quantizes here.
    assert!((angle - 96.66691).abs() <= 1e-6, "angle={angle}");
}

#[test]
fn bridging_angle_histogram_wraps_modulo_180_seam() {
    let anchors = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)],
        vec![
            Point2::from_mm(0.0, 10.0),
            Point2::from_mm(-10.0, 10.0 - 10.0 * 1.0_f32.to_radians().tan()),
        ],
    ];
    let area = vec![
        vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        vec![Point2::from_mm(0.0, 10.0), Point2::from_mm(1.0, 10.0)],
    ];
    let angle = determine_bridging_angle(&anchors, &area, 0.0);
    assert!((angle - 90.500145).abs() <= 1e-6, "angle={angle}");
}

#[test]
fn bridging_angle_is_deterministic() {
    let anchors = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)]];
    let area = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(4.0, 0.0)]];
    let angle = determine_bridging_angle(&anchors, &area, 0.0);
    assert_eq!(angle, determine_bridging_angle(&anchors, &area, 0.0));
    assert!(angle.is_finite());
    assert!(
        (angle - 90.0).abs() <= 1e-6,
        "expected perpendicular bridge angle near 90.0°, got {angle}"
    );
}

#[test]
fn bridging_angle_override_is_exactly_45_degrees() {
    let anchors = vec![vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 10.0)]];
    assert_eq!(determine_bridging_angle(&anchors, &[], 45.0), 45.0);
}

#[test]
fn anchored_polygon_line_count_tracks_round_span_over_spacing() {
    let anchors = vec![
        vec![Point2::from_mm(-1.0, 0.0), Point2::from_mm(11.0, 0.0)],
        vec![Point2::from_mm(-1.0, 4.0), Point2::from_mm(11.0, 4.0)],
    ];
    let (_polygons, lines) =
        construct_anchored_polygon(&anchors, &[square(10.0, 4.0)], 90.0, 1.0, 0.4);
    assert!(
        (lines.len() as i32 - 10).abs() <= 1,
        "line count: {}",
        lines.len()
    );
}

#[test]
fn bridging_flow_uses_configured_bridge_line_width() {
    let spec = canonical_bridging_flow(0.6, 1.0, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.6);
    assert!((spec.spacing_mm - 0.65).abs() <= 1e-6);
}

#[test]
fn bridging_flow_uses_nozzle_when_bridge_line_width_is_unset() {
    let spec = canonical_bridging_flow(0.0, 1.0, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.4);
    assert!((spec.spacing_mm - 0.45).abs() <= 1e-6);
}

#[test]
fn bridging_flow_scales_thread_diameter_by_ratio() {
    let spec = canonical_bridging_flow(0.0, 0.25, 0.4);
    assert_eq!(spec.thread_diameter_mm, 0.2);
    assert_eq!(spec.spacing_mm, 0.25);
}

#[test]
fn depth_window_preserves_full_gather_at_cutoffs_and_filtered_layer_gaps() {
    let height = 0.2_f32;
    let factor = 2.0_f32;
    let current_z = 1.0_f32;
    let cutoff = current_z - height * factor - 0.0001_f32;
    let below = f32::from_bits(cutoff.to_bits() - 1);
    let above = f32::from_bits(cutoff.to_bits() + 1);
    // Gaps model the already-matched region timeline, not the unfiltered stack.
    let cases = [
        (vec![current_z], 0),
        (vec![0.1, current_z], 0),
        (vec![0.1, 0.2, current_z], 1), // retain the far-below predecessor
        (vec![0.1, below, cutoff, above, 0.9, current_z], 2),
        (vec![0.1, 0.7, current_z], 1),
    ];
    for (zs, expected_start) in cases {
        let index = zs.len() - 1;
        let start = depth_window_start(&zs, index, height, factor);
        assert_eq!(start, expected_start, "Z timeline: {zs:?}");
        let layers: Vec<_> = zs
            .iter()
            .enumerate()
            .map(|(i, &print_z)| BridgeDepthLayer {
                print_z,
                sparse_infill: vec![square(10.0 + i as f32, 10.0)],
                not_sparse_infill: vec![square(1.0 + i as f32, 1.0)],
            })
            .collect();
        let full = gather_areas_w_depth(&layers, index, height, factor);
        let trimmed = gather_areas_w_depth(&layers[start..], index - start, height, factor);
        assert_eq!(full, trimmed, "Z timeline: {zs:?}");
        assert_eq!(full.is_empty(), index == 0, "non-vacuous gather: {zs:?}");
    }
    assert_eq!(depth_window_start(&[], 0, height, factor), 0);
    assert!(gather_areas_w_depth(&[], 0, height, factor).is_empty());
}

#[test]
fn filled_window_preserves_removal_at_inclusive_cutoff_and_empty_windows() {
    let height = 0.2_f32;
    let current_z = 1.0_f32;
    let cutoff = current_z - height - 0.0001_f32;
    let below = f32::from_bits(cutoff.to_bits() - 1);
    let above = f32::from_bits(cutoff.to_bits() + 1);
    for (zs, expected_start) in [
        (vec![], 0),
        (vec![0.1, below], 2),
        (vec![cutoff, above], 0),
        (vec![0.1, below, cutoff, above], 2),
    ] {
        let start = filled_window_start(&zs, current_z, height);
        assert_eq!(start, expected_start, "Z timeline: {zs:?}");
        let layers: Vec<_> = zs
            .iter()
            .enumerate()
            .map(|(i, &print_z)| BridgeCandidateLayer {
                print_z,
                new_polys: vec![square(1.0 + i as f32, 1.0)],
            })
            .collect();
        let sparse = vec![square(10.0, 10.0)];
        let full = remove_filled_polygons_on_lower_layers(&sparse, &layers, current_z, height);
        let trimmed =
            remove_filled_polygons_on_lower_layers(&sparse, &layers[start..], current_z, height);
        assert_eq!(full, trimmed, "Z timeline: {zs:?}");
        assert_eq!(
            full == sparse,
            start == zs.len(),
            "non-vacuous removal: {zs:?}"
        );
    }
}

/// Canonical `PrintObject::bridge_over_infill` builds
/// `area_to_be_bridge = expand(candidate, spacing) ∩ deep_infill`, constructs
/// the anchored bridging area from it, then detects collisions with
/// `expand(bridging_area, 3 × spacing)`. Two neighbours therefore share one
/// direction when their *area-class* regions come within `3 × spacing` — not
/// when their raw candidate polygons do.
///
/// Fixture: A (40 × 10 mm, long along x) and B (10 × 40 mm, long along y)
/// whose raw footprints are 4.5 mm apart edge-to-edge, with spacing 1.0 mm.
/// Each candidate's area-class region grows 1 mm outward, so the area gap is
/// 2.5 mm < 3 mm: **canonical reuses A's direction for B**. Growing the raw
/// polygon instead (the previous behaviour) gives a gap of 4.5 mm > 3 mm, so
/// B keeps its own geometry-derived direction. The two angles differ (~90°
/// vs ~0°), which is what makes the operand observable.
#[test]
fn bridge_angle_collision_uses_the_area_class_region_not_the_raw_candidate() {
    // A: 40 x 10 at the origin, long along x -> lines run across at ~90 deg.
    let a = rect(0.0, 0.0, 40.0, 10.0);
    // B: 10 x 40, long along y, raw edge 4.5 mm to the right of A.
    let b = rect(44.5, 0.0, 54.5, 40.0);

    let spacing = 1.0_f32;
    let mut candidates = vec![a.clone(), b.clone()];
    presort_bridge_candidates(&mut candidates);

    // The deep-infill clip must contain each candidate grown by one spacing
    // for `area_to_be_bridge` to survive, but stay out of the other
    // candidate's way so neither's area reaches the other's raw footprint.
    let deep_clip = vec![rect(-1.0, -1.0, 41.0, 11.0), rect(43.5, -1.0, 55.5, 41.0)];
    // `internal_unsupported_area` is the `area_to_be_bridge` filter: the area
    // must reach it, so it spans both candidates' grown footprints.
    let unsupported = vec![rect(-1.0, -1.0, 55.5, 41.0)];
    // `expansion_area` seeds `limiting_area`; keep it away from both
    // candidates so the anchors come from the fill boundary, not from a
    // shared expansion region.
    let expansion: Vec<ExPolygon> = Vec::new();
    // `total_fill_area` produces the boundary anchors: two rectangles whose
    // long sides run along the respective candidate's long axis, so each
    // candidate's own geometry picks a different direction.
    let fill = vec![rect(-2.0, 0.0, 42.0, 10.0), rect(44.5, 0.0, 56.5, 40.0)];

    // exhaustive: InternalBridgeAngleInputs has no Default impl; every field is intentional here
    let inputs = InternalBridgeAngleInputs {
        deep_infill_clip_area: &deep_clip,
        internal_unsupported_area: &unsupported,
        expansion_area: &expansion,
        total_fill_area: &fill,
        spacing_mm: spacing,
        override_deg: 0.0,
    };
    let angles = internal_bridge_angles(&candidates, &inputs);

    assert_eq!(angles.len(), 2);
    // Which of the two sorted candidates is A is the presort's business; get
    // each candidate's angle by its geometry.
    let angle_over = |target: &ExPolygon| {
        let index = candidates
            .iter()
            .position(|c| std::ptr::eq(c, target))
            .or_else(|| {
                candidates
                    .iter()
                    .position(|c| c.contour.points == target.contour.points)
            })
            .expect("both candidates survive the presort");
        angles[index]
    };
    let line_diff = |angle: f32, expected: f32| {
        let diff = (angle - expected).rem_euclid(180.0);
        diff.min(180.0 - diff)
    };
    let angle_a = angle_over(&a);
    let angle_b = angle_over(&b);

    // A picks its own geometry's direction (long sides are x-parallel, so
    // the bridge lines run in y: ~90 deg).
    assert!(
        line_diff(angle_a, 90.0) <= 10.0,
        "A must run across its long axis (~90 deg), got {angle_a}"
    );
    // B inherits A's direction through the 3 x spacing collision at the
    // AREA-class radius (2.5 mm gap). Without the operand fix B's raw gap is
    // 4.5 mm > 3 mm, so B picked ~0 deg and this assertion fails.
    assert!(
        line_diff(angle_b, angle_a) <= 1.0,
        "B is within 3 x spacing of A's area-class region, so it must reuse \
         A's direction; A={angle_a}, B={angle_b}"
    );
    assert!(
        line_diff(angle_b, 0.0) > 10.0,
        "B must NOT keep its own ~0 deg direction (it is inside the collision \
         radius), got {angle_b}"
    );
}
