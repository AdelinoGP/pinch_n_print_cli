//! Regression coverage for optional inner-wall seam staggering.

use std::collections::HashMap;

use seam_placer::SeamPlacer;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth, SeamReason,
    WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{seam_candidate, PerimeterRegionViewBuilder};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::traits::LayerModule;

fn point(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        ..Default::default()
    }
}

fn closed_wall(role: ExtrusionRole, coordinates: &[(f32, f32)]) -> WallLoop {
    let mut points: Vec<_> = coordinates.iter().map(|&(x, y)| point(x, y)).collect();
    points.push(points[0]);
    let path = ExtrusionPath3D {
        points: points.clone(),
        ..extrusion_path3d_base(role.clone())
    };
    let mut wall = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(
            path,
            vec![WallFeatureFlags::default(); points.len()],
            WallBoundaryType::ExteriorSurface,
        )
        .build()
        .wall_loops()[0]
        .clone();
    wall.path.role = role.clone();
    if role == ExtrusionRole::InnerWall {
        wall.perimeter_index = 1;
        wall.loop_type = LoopType::Inner;
    }
    wall.width_profile.widths = vec![0.4; points.len()];
    wall
}

fn run_region(
    enabled: Option<bool>,
    region: slicer_sdk::views::PerimeterRegionView,
) -> PerimeterOutputBuilder {
    let mut fields = HashMap::new();
    if let Some(enabled) = enabled {
        fields.insert(
            "staggered_inner_seams".to_string(),
            ConfigValue::Bool(enabled),
        );
    }
    let config = ConfigView::from_map(fields);
    let module = SeamPlacer::from_config(&config).expect("module config must parse");
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &[region], &mut output, &config)
        .expect("wall postprocess must succeed");
    output
}

fn standard_region() -> slicer_sdk::views::PerimeterRegionView {
    let outer = closed_wall(
        ExtrusionRole::OuterWall,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    );
    let inner = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)],
    );
    let unrelated = closed_wall(
        ExtrusionRole::ThinWall,
        &[(20.0, 20.0), (21.0, 20.0), (21.0, 21.0)],
    );
    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer, inner, unrelated]);
    region.set_seam_candidates(vec![seam_candidate(
        point(0.0, 0.0),
        0.0,
        SeamReason::Sharp,
    )]);
    region
}

fn run(enabled: Option<bool>) -> Vec<WallLoop> {
    run_region(enabled, standard_region())
        .rotated_wall_loops()
        .iter()
        .map(|(_, _, loop_)| loop_.clone())
        .collect()
}

#[test]
fn inner_wall_seam_target_does_not_activate_staggering_or_leave_a_stale_seam() {
    let mut region = standard_region();
    region.set_seam_candidates(vec![seam_candidate(
        point(1.0, 1.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let baseline = run_region(Some(false), region.clone());
    let staggered = run_region(Some(true), region);

    assert_eq!(
        staggered.rotated_wall_loops(),
        baseline.rotated_wall_loops(),
        "a non-outer target must retain the ordinary seam-placement behavior"
    );
    let seam = staggered
        .resolved_seam()
        .expect("the target seam must remain resolved");
    let target = &staggered.rotated_wall_loops()[seam.wall_index as usize].2;
    assert_eq!(target.path.points.first(), Some(&seam.point));
}

#[test]
fn width_clamp_offsets_first_inner_seam_by_profile_width() {
    let outer = closed_wall(
        ExtrusionRole::OuterWall,
        &[
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ],
    );
    let mut inner = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 0.1), (9.0, 0.1), (9.0, 9.0), (1.0, 9.0)],
    );
    for point in &mut inner.path.points {
        point.width = 0.8;
    }
    inner.width_profile.widths.fill(0.8);

    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer, inner]);
    region.set_seam_candidates(vec![seam_candidate(
        point(5.0, 0.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let output = run_region(Some(true), region);
    let staggered = &output.rotated_wall_loops()[1].2;
    let first = staggered.path.points[0];

    assert!((first.x - 5.8).abs() < 1e-5);
    assert!((first.y - 0.1).abs() < 1e-5);
    assert!((first.x - 5.0 - first.width).abs() < 1e-5);
    assert!((first.width - 0.8).abs() < 1e-5);
    assert_eq!(staggered.path.points.first(), staggered.path.points.last());
    assert_eq!(
        staggered.path.points.len(),
        staggered.width_profile.widths.len()
    );
    assert_eq!(staggered.path.points.len(), staggered.feature_flags.len());
}

#[test]
fn final_to_first_projection_advances_across_edge_with_parallel_arrays() {
    let outer = closed_wall(
        ExtrusionRole::OuterWall,
        &[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 5.0),
        ],
    );
    let inner = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)],
    );
    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer, inner]);
    region.set_seam_candidates(vec![seam_candidate(
        point(0.0, 5.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let output = run_region(Some(true), region);
    let staggered = &output.rotated_wall_loops()[1].2;
    let first = staggered.path.points[0];
    let expected_y = 5.0 - 1.0 / std::f32::consts::SQRT_2;

    assert!((first.x - 1.0).abs() < 1e-5);
    assert!((first.y - expected_y).abs() < 1e-5);
    assert!(first.y > 1.0 && first.y < 9.0);
    assert_eq!(staggered.path.points.first(), staggered.path.points.last());
    assert_eq!(
        staggered.path.points.len(),
        staggered.width_profile.widths.len()
    );
    assert_eq!(staggered.path.points.len(), staggered.feature_flags.len());
    assert_eq!(
        staggered.feature_flags.first(),
        staggered.feature_flags.last()
    );
    assert_eq!(
        staggered.width_profile.widths.first(),
        staggered.width_profile.widths.last()
    );
}

#[test]
fn xy_closed_loop_with_different_closing_metadata_is_staggered() {
    let mut region = standard_region();
    let mut walls = region.wall_loops().to_vec();
    let inner = &mut walls[1];
    let last = inner.path.points.len() - 1;
    inner.path.points[last].z = 0.6;
    inner.path.points[last].width = 0.9;
    inner.width_profile.widths = vec![0.35, 0.4, 0.45, 0.5, 0.9];
    inner.feature_flags[1].fuzzy_skin = true;
    assert!(inner.path.is_closed());
    region.set_wall_loops(walls.clone());

    let output = run_region(Some(true), region);
    let staggered = &output.rotated_wall_loops()[1].2;

    assert_ne!(staggered.path.points[0], walls[1].path.points[0]);
    assert!(staggered.path.is_closed());
    assert_eq!(
        staggered.path.points.len(),
        staggered.width_profile.widths.len()
    );
    assert_eq!(staggered.path.points.len(), staggered.feature_flags.len());
    assert!(staggered.feature_flags.iter().any(|flags| flags.fuzzy_skin));
    for width in &walls[1].width_profile.widths[..last] {
        assert!(staggered.width_profile.widths.contains(width));
    }
}

#[test]
fn reversed_outer_winding_keeps_corner_correction_direction() {
    let outer = closed_wall(
        ExtrusionRole::OuterWall,
        &[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)],
    );
    let inner = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)],
    );
    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer, inner]);
    region.set_seam_candidates(vec![seam_candidate(
        point(0.0, 0.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let output = run_region(Some(true), region);
    let first = output.rotated_wall_loops()[1].2.path.points[0];
    let expected_x = 1.0 + 1.0 / std::f32::consts::SQRT_2;

    assert!((first.x - expected_x).abs() < 1e-5);
    assert!((first.y - 1.0).abs() < 1e-5);
}

#[test]
fn missing_and_false_are_identical() {
    assert_eq!(run(None), run(Some(false)));
}

#[test]
fn enabled_staggers_only_inner_wall_contained_by_selected_outer() {
    let outer_a = closed_wall(
        ExtrusionRole::OuterWall,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    );
    let inner_a = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)],
    );
    let outer_b = closed_wall(
        ExtrusionRole::OuterWall,
        &[(20.0, 20.0), (30.0, 20.0), (30.0, 30.0), (20.0, 30.0)],
    );
    let inner_b = closed_wall(
        ExtrusionRole::InnerWall,
        &[(21.0, 21.0), (29.0, 21.0), (29.0, 29.0), (21.0, 29.0)],
    );
    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer_a, inner_a, outer_b, inner_b]);
    region.set_seam_candidates(vec![seam_candidate(
        point(0.0, 0.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let baseline = run_region(Some(false), region.clone());
    let staggered = run_region(Some(true), region);
    let baseline = baseline.rotated_wall_loops();
    let staggered = staggered.rotated_wall_loops();

    assert_ne!(
        staggered[1].2, baseline[1].2,
        "inner wall A must be staggered"
    );
    assert_eq!(
        staggered[3].2, baseline[3].2,
        "disjoint inner wall B must remain byte-identical"
    );
}

#[test]
fn nested_outer_owns_its_inner_wall_for_staggering() {
    let outer_a = closed_wall(
        ExtrusionRole::OuterWall,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
    );
    let inner_a = closed_wall(
        ExtrusionRole::InnerWall,
        &[(1.0, 1.0), (19.0, 1.0), (19.0, 19.0), (1.0, 19.0)],
    );
    let outer_b = closed_wall(
        ExtrusionRole::OuterWall,
        &[(10.0, 10.0), (16.0, 10.0), (16.0, 16.0), (10.0, 16.0)],
    );
    let inner_b = closed_wall(
        ExtrusionRole::InnerWall,
        &[(11.0, 11.0), (15.0, 11.0), (15.0, 15.0), (11.0, 15.0)],
    );
    let mut region = PerimeterRegionViewBuilder::new().build();
    region.set_object_id("obj".to_string());
    region.set_region_id(0);
    region.set_wall_loops(vec![outer_a, inner_a, outer_b, inner_b]);
    region.set_seam_candidates(vec![seam_candidate(
        point(0.0, 0.0),
        0.0,
        SeamReason::Sharp,
    )]);

    let baseline = run_region(Some(false), region.clone());
    let staggered = run_region(Some(true), region);
    let baseline = baseline.rotated_wall_loops();
    let staggered = staggered.rotated_wall_loops();

    assert_ne!(staggered[1].2, baseline[1].2, "inner wall A must stagger");
    assert_eq!(
        staggered[3].2, baseline[3].2,
        "nested inner wall B must remain byte-identical"
    );
}

#[test]
fn enabled_staggers_only_inner_wall_and_preserves_parallel_arrays() {
    let baseline = run(Some(false));
    let staggered = run(Some(true));

    assert_eq!(staggered[0], baseline[0], "outer wall must not move");
    assert_eq!(staggered[2], baseline[2], "unrelated loop must not move");
    let inner = &staggered[1];
    assert!(
        inner.path.points[0].x > 1.0 && inner.path.points[0].x < 9.0,
        "staggered seam must be interpolated inside the first forward segment"
    );
    assert!((inner.path.points[0].y - 1.0).abs() < 1e-5);
    assert_eq!(inner.path.points.first(), inner.path.points.last());
    assert_eq!(inner.path.points.len(), inner.width_profile.widths.len());
    assert_eq!(inner.path.points.len(), inner.feature_flags.len());
    assert_eq!(inner.feature_flags.first(), inner.feature_flags.last());
    assert_eq!(
        inner.width_profile.widths.first(),
        inner.width_profile.widths.last()
    );
}
