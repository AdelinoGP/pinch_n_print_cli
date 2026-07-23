//! TDD coverage for aligned-mode missing-plan degradation (packet 180).

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

fn config_with_mode(mode: &str) -> ConfigView {
    let mut map = HashMap::new();
    map.insert(
        "seam_mode".to_string(),
        ConfigValue::String(mode.to_string()),
    );
    ConfigView::from_map(map)
}

fn ir_point(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn ir_flags(count: usize) -> Vec<WallFeatureFlags> {
    vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        };
        count
    ]
}

fn ir_wall(layer_z: f32, points: &[(f32, f32)]) -> WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
    let path = ExtrusionPath3D {
        points: path_points.clone(),
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(
            path,
            ir_flags(path_points.len()),
            WallBoundaryType::ExteriorSurface,
        )
        .build()
        .wall_loops()[0]
        .clone()
}

fn ir_candidate(x: f32, y: f32, z: f32, score: f32, reason: SeamReason) -> SeamCandidate {
    seam_candidate(ir_point(x, y, z), score, reason)
}

fn aligned_region(
    object_id: &str,
    region_id: u64,
    walls: Vec<WallLoop>,
    candidates: Vec<SeamCandidate>,
    resolved: Option<Point3WithWidth>,
) -> PerimeterRegionView {
    let mut region = PerimeterRegionView::default();
    region.set_object_id(object_id.to_string());
    region.set_region_id(region_id);
    region.set_wall_loops(walls);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(candidates);
    region.set_resolved_seam(resolved.map(|point| SeamPosition {
        point,
        wall_index: 0,
    }));
    region
}

#[test]
fn missing_plan_emits_non_fatal_and_preserves_walls() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let input_wall = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    let regions = vec![aligned_region(
        "obj-missing",
        7,
        vec![input_wall.clone()],
        vec![],
        None,
    )];
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_wall_postprocess(3, &regions, &mut output, &config);
    let error = result.expect_err("missing aligned plan entry must be observable");

    assert!(!error.fatal, "missing plan entry must be non-fatal");
    assert_eq!(error.code, 6, "code 6 documents a missing seam plan entry");
    assert!(error
        .message
        .contains("(layer=3, object=obj-missing, region_id=7, variant_chain=[])"));
    assert_eq!(
        output.rotated_wall_loops().len(),
        1,
        "wall must be preserved"
    );
    assert_eq!(
        output.rotated_wall_loops()[0].2.path.points,
        input_wall.path.points
    );
}

#[test]
fn aligned_with_resolved_seam_does_not_emit_non_fatal() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let seam = ir_point(1.0, 0.0, 0.2);
    let regions = vec![aligned_region(
        "obj-resolved",
        8,
        vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])],
        vec![],
        Some(seam),
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(3, &regions, &mut output, &config)
        .expect("resolved aligned seam must succeed");

    assert_eq!(
        output.resolved_seam().expect("seam must be set").point,
        seam
    );
}

#[test]
fn nearest_mode_does_not_emit_non_fatal_on_missing_plan() {
    let config = config_with_mode("nearest");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![aligned_region(
        "obj-nearest",
        9,
        vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])],
        vec![],
        None,
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(3, &regions, &mut output, &config)
        .expect("nearest mode must not report a missing aligned plan");

    assert!(output.resolved_seam().is_none());
    assert_eq!(output.rotated_wall_loops().len(), 1);
}

#[test]
fn aligned_missing_plan_uses_nearest_candidate_as_fallback() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![aligned_region(
        "obj-candidate",
        10,
        vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])],
        vec![
            ir_candidate(0.0, 0.0, 0.2, 0.8, SeamReason::Aligned),
            ir_candidate(1.0, 0.0, 0.2, 0.2, SeamReason::Aligned),
        ],
        None,
    )];
    let mut output = PerimeterOutputBuilder::new();

    let error = module
        .run_wall_postprocess(3, &regions, &mut output, &config)
        .expect_err("missing plan must remain observable after local fallback");

    assert!(!error.fatal);
    assert_eq!(
        output
            .resolved_seam()
            .expect("fallback seam must be set")
            .point
            .x,
        1.0
    );
    assert_eq!(output.rotated_wall_loops().len(), 1);
}
