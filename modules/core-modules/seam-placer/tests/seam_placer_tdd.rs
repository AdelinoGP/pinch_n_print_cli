//! TDD tests for the seam placer module (TASK-084).
//!
//! Tests verify seam candidate selection and resolved seam output
//! for the `Layer::WallPostProcess` stage.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth,
    SeamCandidate, SeamReason, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

/// Helper: create a seam candidate at given position with score and reason.
fn candidate(x: f32, y: f32, z: f32, score: f32, reason: SeamReason) -> SeamCandidate {
    SeamCandidate {
        position: Point3WithWidth {
            x,
            y,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        score,
        reason,
    }
}

/// Helper: create a minimal wall loop at given z.
fn wall_at_z(z: f32) -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                },
                Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4, 0.4],
        },
        feature_flags: vec![],
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

/// Helper: create a PerimeterRegionView with given candidates and walls.
fn region_with_candidates(candidates: Vec<SeamCandidate>, z: f32) -> PerimeterRegionView {
    PerimeterRegionView::new(
        "obj-0".to_string(),
        0,
        vec![wall_at_z(z)],
        vec![],
        candidates,
    )
}

// ============================================================================
// Test 1: on_print_start defaults
// ============================================================================

#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
    let config = ConfigView::from_map(HashMap::new(),);
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
