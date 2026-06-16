//! TDD tests for the seam placer module (TASK-084).
//!
//! Tests verify seam candidate selection and resolved seam output
//! for the `Layer::PerimetersPostProcess` stage.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamCandidate,
    SeamPosition, SeamReason, WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{seam_candidate, PerimeterRegionViewBuilder};
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

/// Helper: create a seam candidate at given position with score and reason.
fn candidate(x: f32, y: f32, z: f32, score: f32, reason: SeamReason) -> SeamCandidate {
    seam_candidate(
        Point3WithWidth {
            x,
            y,
            z,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        score,
        reason,
    )
}

/// Helper: create a minimal wall loop at given z.
fn wall_at_z(z: f32) -> WallLoop {
    let p = |x| Point3WithWidth {
        x,
        y: 0.0,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };
    let path = ExtrusionPath3D {
        points: vec![p(0.0), p(1.0), p(2.0)],
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    let flags = vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new()
        };
        3
    ];
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}

fn wall_from_candidates(candidates: &[SeamCandidate], z: f32) -> WallLoop {
    if candidates.is_empty() {
        return wall_at_z(z);
    }
    let points: Vec<_> = candidates
        .iter()
        .map(|c| Point3WithWidth {
            x: c.position.x,
            y: c.position.y,
            z,
            width: c.position.width,
            flow_factor: c.position.flow_factor,
            overhang_quartile: c.position.overhang_quartile,
        })
        .collect();
    let flags = vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new()
        };
        points.len()
    ];
    let path = ExtrusionPath3D {
        points,
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}

/// Helper: create a PerimeterRegionView with given candidates and walls.
fn region_with_candidates(candidates: Vec<SeamCandidate>, z: f32) -> PerimeterRegionView {
    {
        let mut tmp = PerimeterRegionView::default();
        tmp.set_object_id("obj-0".to_string());
        tmp.set_region_id(0);
        tmp.set_wall_loops(vec![wall_from_candidates(&candidates, z)]);
        tmp.set_infill_areas(vec![]);
        tmp.set_seam_candidates(candidates);
        tmp.set_resolved_seam(None);
        tmp
    }
}

// ============================================================================
// Test 1: on_print_start defaults
// ============================================================================

#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();
    assert_eq!(module.seam_mode(), "nearest");
}

// ============================================================================
// Test 2: on_print_start custom config
// ============================================================================

#[test]
fn on_print_start_custom() {
    let mut fields = HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        ConfigValue::String("rear".to_string()),
    );
    let config = ConfigView::from_map(fields);
    let module = SeamPlacer::on_print_start(&config).unwrap();
    assert_eq!(module.seam_mode(), "rear");
}

// ============================================================================
// Test 3: picks lowest score candidate
// ============================================================================

#[test]
fn picks_lowest_score() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![
        candidate(1.0, 0.0, 1.0, 0.8, SeamReason::Sharp),
        candidate(2.0, 0.0, 1.0, 0.2, SeamReason::Concave),
        candidate(3.0, 0.0, 1.0, 0.5, SeamReason::Aligned),
    ];
    let regions = vec![region_with_candidates(candidates, 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let seam = output.resolved_seam().expect("should have resolved seam");
    assert!(
        (seam.point.x - 2.0).abs() < 0.001,
        "should pick x=2.0 (score 0.2)"
    );
}

// ============================================================================
// Test 4: concave preferred over same-score aligned
// ============================================================================

#[test]
fn concave_preferred() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![
        candidate(1.0, 0.0, 1.0, 0.5, SeamReason::Aligned),
        candidate(2.0, 0.0, 1.0, 0.5, SeamReason::Concave),
    ];
    let regions = vec![region_with_candidates(candidates, 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let seam = output.resolved_seam().expect("should have resolved seam");
    assert!(
        (seam.point.x - 2.0).abs() < 0.001,
        "concave should win over aligned at same score"
    );
}

// ============================================================================
// Test 5: no candidates => no resolved seam
// ============================================================================

#[test]
fn no_candidates_no_seam() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let regions = vec![region_with_candidates(vec![], 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    assert!(output.resolved_seam().is_none(), "no candidates => no seam");
}

// ============================================================================
// Test 6: rear mode prefers max-Y candidate
// ============================================================================

#[test]
fn rear_mode_prefers_back() {
    let mut fields = HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        ConfigValue::String("rear".to_string()),
    );
    let config = ConfigView::from_map(fields);
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![
        candidate(0.0, 1.0, 1.0, 0.5, SeamReason::Sharp),
        candidate(0.0, 5.0, 1.0, 0.9, SeamReason::Sharp), // max Y, worst score
        candidate(0.0, 3.0, 1.0, 0.1, SeamReason::Concave), // best score but not max Y
    ];
    let regions = vec![region_with_candidates(candidates, 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let seam = output.resolved_seam().expect("should have resolved seam");
    assert!(
        (seam.point.y - 5.0).abs() < 0.001,
        "rear mode should select max-Y"
    );
}

// ============================================================================
// Test 7: random mode produces some seam
// ============================================================================

#[test]
fn random_mode_produces_seam() {
    let mut fields = HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        ConfigValue::String("random".to_string()),
    );
    let config = ConfigView::from_map(fields);
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![
        candidate(1.0, 0.0, 1.0, 0.5, SeamReason::Sharp),
        candidate(2.0, 0.0, 1.0, 0.3, SeamReason::Concave),
    ];
    let regions = vec![region_with_candidates(candidates, 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    assert!(
        output.resolved_seam().is_some(),
        "random mode should produce a seam"
    );
}

// ============================================================================
// Test 8: seam at correct Z
// ============================================================================

#[test]
fn seam_at_correct_z() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![candidate(1.0, 0.0, 1.5, 0.3, SeamReason::Concave)];
    let regions = vec![region_with_candidates(candidates, 1.5)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let seam = output.resolved_seam().expect("should have resolved seam");
    assert!((seam.point.z - 1.5).abs() < 0.001);
}

// ============================================================================
// Test 9: multiple regions each get resolved seam
// ============================================================================

#[test]
fn multiple_regions() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let regions = vec![
        region_with_candidates(
            vec![candidate(1.0, 0.0, 1.0, 0.3, SeamReason::Concave)],
            1.0,
        ),
        region_with_candidates(vec![candidate(2.0, 0.0, 1.0, 0.4, SeamReason::Sharp)], 1.0),
    ];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    // With multiple regions, the last region's resolved seam wins
    // (or the module processes all and keeps the best overall)
    assert!(
        output.resolved_seam().is_some(),
        "should have resolved seam from regions"
    );
}

// ============================================================================
// Test 10: empty regions => no output
// ============================================================================

#[test]
fn empty_regions_no_output() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    assert!(output.resolved_seam().is_none());
}

// ============================================================================
// Test 11: wall_index is always 0
// ============================================================================

#[test]
fn wall_index_zero() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    let candidates = vec![candidate(1.0, 2.0, 1.0, 0.3, SeamReason::Concave)];
    let regions = vec![region_with_candidates(candidates, 1.0)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let seam = output.resolved_seam().expect("should have resolved seam");
    assert_eq!(seam.wall_index, 0);
}

// ============================================================================
// HIGH-2 regression: region without any seam information must still emit walls
// ============================================================================

/// Contract post-HIGH-2: a region with empty `seam_candidates` AND
/// `resolved_seam = None` is the "no seam info at all" branch — historically
/// a `continue` here dropped the region's walls from the output, which then
/// corrupted the `(object_id, region_id)` pairing in
/// `layer_executor::commit_layer_outputs`. The refactored loop must emit the
/// walls pristine and leave `resolved_seam` unset.
#[test]
fn region_without_seam_candidates_or_resolved_seam_preserves_walls() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    // Build a region with three concrete wall vertices, NO candidates,
    // NO pre-resolved seam.
    let input_wall = wall_at_z(0.2);
    let mut region = PerimeterRegionView::default();
    region.set_object_id("obj-no-seam".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![input_wall.clone()]);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(None);
    let regions = vec![region];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("region with no seam info must not fail the layer");

    assert!(
        output.resolved_seam().is_none(),
        "no candidates and no pre-resolved seam => no resolved seam emitted"
    );
    let rotated = output.rotated_wall_loops();
    assert_eq!(
        rotated.len(),
        1,
        "wall must still be present even when no seam can be placed"
    );
    let (_, emitted_wall_index, emitted_loop) = &rotated[0];
    assert_eq!(*emitted_wall_index, 0);
    assert_eq!(
        emitted_loop.path.points, input_wall.path.points,
        "wall must be emitted byte-identical to its input (no rotation)"
    );
}

// ============================================================================
// HIGH-2 regression: multi-region mix where one resolves and one misses must
// preserve BOTH regions' walls
// ============================================================================

/// Two regions in the same dispatch: region A's pre-resolved seam matches a
/// wall vertex (the success path → rotates that wall), region B's pre-resolved
/// seam is at mesh-corner coordinates that DON'T match any wall vertex (the
/// `find_seam_location` returned `None` branch → must still emit pristine
/// walls). Pre-HIGH-2 this dropped region B entirely; the regression check is
/// that the output contains exactly TWO wall loops, that A's first vertex is
/// the seam point (rotation applied), and that B's wall is byte-identical to
/// its input.
#[test]
fn multi_region_mixed_seam_match_preserves_all_walls() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SeamPlacer::on_print_start(&config).unwrap();

    // Region A: seam at (1.0, 0.0) — a vertex that exists on the wall.
    let wall_a_points = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)];
    let wall_a = make_wall(0.2, &wall_a_points);
    let seam_a_point = Point3WithWidth {
        x: 1.0,
        y: 0.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };
    let mut region_a = PerimeterRegionView::default();
    region_a.set_object_id("obj-a".to_string());
    region_a.set_region_id(0);
    region_a.set_wall_loops(vec![wall_a.clone()]);
    region_a.set_infill_areas(vec![]);
    region_a.set_seam_candidates(vec![]);
    region_a.set_resolved_seam(Some(SeamPosition {
        point: seam_a_point,
        wall_index: 0,
    }));

    // Region B: seam at (99.0, 99.0) — does NOT match any wall vertex
    // (mesh-corner-vs-inset gap simulated).
    let wall_b_points = [(10.0, 10.0), (11.0, 10.0), (11.0, 11.0)];
    let wall_b = make_wall(0.2, &wall_b_points);
    let seam_b_point = Point3WithWidth {
        x: 99.0,
        y: 99.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };
    let mut region_b = PerimeterRegionView::default();
    region_b.set_object_id("obj-b".to_string());
    region_b.set_region_id(0);
    region_b.set_wall_loops(vec![wall_b.clone()]);
    region_b.set_infill_areas(vec![]);
    region_b.set_seam_candidates(vec![]);
    region_b.set_resolved_seam(Some(SeamPosition {
        point: seam_b_point,
        wall_index: 0,
    }));

    let regions = vec![region_a, region_b];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("mixed match/miss across regions must not fail the layer");

    let rotated = output.rotated_wall_loops();
    assert_eq!(
        rotated.len(),
        2,
        "both regions' walls must reach the output (HIGH-2 regression: dropping \
         region B's walls here corrupts the (object_id, region_id) pairing)"
    );

    // Region A: emitted as the first wall, rotated so the seam vertex (1.0, 0.0)
    // becomes the start vertex.
    let (_, _, emitted_a) = &rotated[0];
    let a_first = emitted_a
        .path
        .points
        .first()
        .expect("region A wall must have at least one point");
    assert!(
        (a_first.x - 1.0).abs() < 1e-4 && (a_first.y - 0.0).abs() < 1e-4,
        "region A's wall must be rotated so seam (1.0, 0.0) is the first vertex, \
         got ({}, {})",
        a_first.x,
        a_first.y,
    );

    // Region B: emitted second, byte-identical to its input (no rotation).
    let (_, _, emitted_b) = &rotated[1];
    assert_eq!(
        emitted_b.path.points, wall_b.path.points,
        "region B's wall must be emitted pristine when its seam doesn't match \
         any vertex (HIGH-2: emit walls even when find_seam_location returns None)"
    );

    // Only ONE resolved seam should be present — region A's. The
    // PerimeterOutputBuilder aggregates `set_resolved_seam` calls into a single
    // `Option<SeamPosition>`, so we assert structurally that the surviving seam
    // matches region A's target coordinates.
    let resolved = output
        .resolved_seam()
        .expect("region A's matching seam must be committed");
    assert!(
        (resolved.point.x - 1.0).abs() < 1e-4 && (resolved.point.y - 0.0).abs() < 1e-4,
        "resolved seam must be region A's (1.0, 0.0), got ({}, {}) — region B \
         must not have committed a resolved seam",
        resolved.point.x,
        resolved.point.y,
    );
}

/// Helper for the multi-region test: build a single wall loop at `z` from
/// explicit (x, y) vertex coordinates. Mirrors the dispatch-test `ir_wall`
/// helper so the two test files agree on wall geometry shape.
fn make_wall(z: f32, points: &[(f32, f32)]) -> WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| Point3WithWidth {
            x: *x,
            y: *y,
            z,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        })
        .collect();
    let flags = vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new()
        };
        path_points.len()
    ];
    let path = ExtrusionPath3D {
        points: path_points,
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}
