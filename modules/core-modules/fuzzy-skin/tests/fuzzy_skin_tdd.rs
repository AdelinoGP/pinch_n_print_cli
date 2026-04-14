//! TDD tests for the fuzzy-skin module (TASK-092).
//!
//! Tests verify selective outer-wall perturbation using propagated feature flags
//! while preserving path/flag cardinality for the `Layer::PerimetersPostProcess` stage.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth,
    WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use fuzzy_skin::FuzzySkinModule;

/// Helper: create default WallFeatureFlags with fuzzy_skin set to the given value.
fn flags(fuzzy: bool) -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: fuzzy,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

/// Helper: create a simple outer wall loop with 4 points forming a square.
fn outer_wall(z: f32, fuzzy_flags: &[bool]) -> WallLoop {
    let points = vec![
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 10.0,
            y: 0.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 10.0,
            y: 10.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 0.0,
            y: 10.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
    ];
    let feature_flags: Vec<WallFeatureFlags> = fuzzy_flags.iter().map(|f| flags(*f)).collect();
    let widths = vec![0.4; points.len()];
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points,
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile { widths },
        feature_flags,
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

/// Helper: create an inner wall loop.
fn inner_wall(z: f32) -> WallLoop {
    let points = vec![
        Point3WithWidth {
            x: 1.0,
            y: 1.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 9.0,
            y: 1.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 9.0,
            y: 9.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: 1.0,
            y: 9.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
    ];
    WallLoop {
        perimeter_index: 1,
        loop_type: LoopType::Inner,
        path: ExtrusionPath3D {
            points: points.clone(),
            role: ExtrusionRole::InnerWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4; points.len()],
        },
        feature_flags: vec![flags(false); 4],
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

/// Helper: create a PerimeterRegionView with given wall loops.
fn region_with_walls(walls: Vec<WallLoop>) -> PerimeterRegionView {
    PerimeterRegionView::new("obj-0".to_string(), 0, walls, vec![], vec![])
}

/// Helper: default config (no overrides).
fn default_config() -> ConfigView {
    ConfigView {
        fields: HashMap::new(),
    }
}

/// Helper: config with apply-to-all = true.
fn apply_to_all_config() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("apply-to-all".to_string(), ConfigValue::Bool(true));
    ConfigView { fields }
}

// ============================================================================
// Test 1: Segments with feature_flags.fuzzy_skin = true are perturbed
// ============================================================================

#[test]
fn fuzzy_flagged_segments_are_perturbed() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[true, true, true, true]);
    let regions = vec![region_with_walls(vec![wall.clone()])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    let out_walls = output.wall_loops();
    assert_eq!(out_walls.len(), 1, "should produce one wall loop");
    // Output path must differ from input (perturbation applied)
    assert_ne!(
        out_walls[0].path.points, wall.path.points,
        "fuzzy_skin=true segments should be perturbed"
    );
}

// ============================================================================
// Test 2: Segments with feature_flags.fuzzy_skin = false are unchanged
// ============================================================================

#[test]
fn non_fuzzy_segments_unchanged() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[false, false, false, false]);
    let regions = vec![region_with_walls(vec![wall.clone()])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    let out_walls = output.wall_loops();
    assert_eq!(out_walls.len(), 1);
    assert_eq!(
        out_walls[0].path.points, wall.path.points,
        "fuzzy_skin=false segments should be unchanged"
    );
}

// ============================================================================
// Test 3: apply-to-all = true perturbs all outer wall segments regardless of flags
// ============================================================================

#[test]
fn apply_to_all_perturbs_all_outer() {
    let config = apply_to_all_config();
    let module = FuzzySkinModule::on_print_start(&config).unwrap();
    // All flags false, but apply-to-all overrides
    let wall = outer_wall(0.3, &[false, false, false, false]);
    let regions = vec![region_with_walls(vec![wall.clone()])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .unwrap();

    let out_walls = output.wall_loops();
    assert_eq!(out_walls.len(), 1);
    assert_ne!(
        out_walls[0].path.points, wall.path.points,
        "apply-to-all should perturb even when flags are false"
    );
}

// ============================================================================
// Test 4: No perturbation on inner walls when apply-to-all is false
// ============================================================================

#[test]
fn inner_walls_not_perturbed() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let inner = inner_wall(0.3);
    let regions = vec![region_with_walls(vec![inner.clone()])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    let out_walls = output.wall_loops();
    assert_eq!(out_walls.len(), 1);
    assert_eq!(
        out_walls[0].path.points, inner.path.points,
        "inner walls should never be perturbed"
    );
}

// ============================================================================
// Test 5: Path point count and feature_flags remain parallel after perturbation
// ============================================================================

#[test]
fn feature_flags_parallel_with_points() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[true, true, true, true]);
    let regions = vec![region_with_walls(vec![wall])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    let out_walls = output.wall_loops();
    assert_eq!(out_walls.len(), 1);
    let w = &out_walls[0];
    assert_eq!(
        w.feature_flags.len(),
        w.path.points.len(),
        "feature_flags.len() must equal path.points.len() after perturbation"
    );
    // Width profile must also be parallel
    assert_eq!(
        w.width_profile.widths.len(),
        w.path.points.len(),
        "width_profile.widths.len() must equal path.points.len() after perturbation"
    );
}

// ============================================================================
// Test 6: Property test — all output points have finite coordinates
// ============================================================================

#[test]
fn all_output_points_finite() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[true, true, true, true]);
    let regions = vec![region_with_walls(vec![wall])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    for w in output.wall_loops() {
        for pt in &w.path.points {
            assert!(pt.x.is_finite(), "x must be finite, got {}", pt.x);
            assert!(pt.y.is_finite(), "y must be finite, got {}", pt.y);
            assert!(pt.z.is_finite(), "z must be finite, got {}", pt.z);
            assert!(pt.width.is_finite(), "width must be finite");
            assert!(pt.flow_factor.is_finite(), "flow_factor must be finite");
        }
    }
}

// ============================================================================
// Test 7: on_print_start with default config succeeds
// ============================================================================

#[test]
fn on_print_start_defaults() {
    let module = FuzzySkinModule::on_print_start(&default_config());
    assert!(module.is_ok());
}

// ============================================================================
// Test 8: on_print_start with custom config succeeds
// ============================================================================

#[test]
fn on_print_start_custom_config() {
    let mut fields = HashMap::new();
    fields.insert("thickness".to_string(), ConfigValue::Float(1.0));
    fields.insert("point-distance".to_string(), ConfigValue::Float(0.5));
    fields.insert("apply-to-all".to_string(), ConfigValue::Bool(true));
    let config = ConfigView { fields };
    let module = FuzzySkinModule::on_print_start(&config);
    assert!(module.is_ok());
}

// ============================================================================
// Test 9: Empty regions produce no output wall loops
// ============================================================================

#[test]
fn empty_regions_no_output() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    assert!(
        output.wall_loops().is_empty(),
        "empty regions should produce no wall loops"
    );
}

// ============================================================================
// Test 10: Deterministic — same inputs produce same outputs (seeded RNG)
// ============================================================================

#[test]
fn deterministic_output() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[true, true, true, true]);
    let regions = vec![region_with_walls(vec![wall])];

    let mut output1 = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &regions, &mut output1, &default_config())
        .unwrap();

    let mut output2 = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &regions, &mut output2, &default_config())
        .unwrap();

    assert_eq!(
        output1.wall_loops()[0].path.points,
        output2.wall_loops()[0].path.points,
        "same inputs must produce identical outputs (seeded RNG)"
    );
}

#[test]
fn mixed_fuzzy_loops_keep_unflagged_tail_vertices() {
    let module = FuzzySkinModule::on_print_start(&default_config()).unwrap();
    let wall = outer_wall(0.3, &[true, false, false, false]);
    let regions = vec![region_with_walls(vec![wall.clone()])];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &default_config())
        .unwrap();

    let points = &output.wall_loops()[0].path.points;
    let last = points.last().expect("mixed fuzzy loop should keep points");

    assert_eq!(
        *last, wall.path.points[3],
        "unflagged tail segments should retain the original final vertex"
    );
    assert!(
        points.contains(&wall.path.points[2]),
        "unflagged vertices must remain present on mixed loops"
    );
}
