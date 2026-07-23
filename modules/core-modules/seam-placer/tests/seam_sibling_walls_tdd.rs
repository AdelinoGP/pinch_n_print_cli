//! Packet `170-seam-livepath-audit` regression coverage for the wall-preservation
//! invariant across AC-1, AC-2, AC-3, and AC-N1.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamPosition,
    WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{seam_candidate, PerimeterRegionViewBuilder};
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

// All four tests are expected to pass without an implementation fix, but full-vector assertions make sibling-erasure bugs RED.

fn config_with_mode(mode: &str) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        ConfigValue::String(mode.to_string()),
    );
    ConfigView::from_map(fields)
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
    let mut path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
    if let Some(first) = path_points.first().copied() {
        if path_points.last() != Some(&first) {
            path_points.push(first);
        }
    }
    let flags = ir_flags(path_points.len());
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

fn sdk_region(
    object_id: &str,
    region_id: u64,
    walls: Vec<WallLoop>,
    candidates: Vec<slicer_ir::SeamCandidate>,
    resolved_seam: Option<SeamPosition>,
) -> PerimeterRegionView {
    let mut tmp = PerimeterRegionView::default();
    tmp.set_object_id(object_id.to_string());
    tmp.set_region_id(region_id);
    tmp.set_wall_loops(walls);
    tmp.set_infill_areas(vec![]);
    tmp.set_seam_candidates(candidates);
    tmp.set_resolved_seam(resolved_seam);
    tmp
}

fn assert_wall_eq(actual: &WallLoop, expected: &WallLoop) {
    assert_eq!(
        actual.path.points, expected.path.points,
        "path points changed"
    );
    assert_eq!(
        actual.feature_flags, expected.feature_flags,
        "feature flags changed"
    );
    assert_eq!(
        actual.width_profile.widths, expected.width_profile.widths,
        "width profile changed"
    );
    assert_eq!(
        actual.path.points.last(),
        actual.path.points.first(),
        "actual wall lost its explicit closing repeat"
    );
    assert_eq!(
        expected.path.points.last(),
        expected.path.points.first(),
        "expected fixture must have an explicit closing repeat"
    );
}

#[test]
fn siblings_survive_rotation() {
    let config = config_with_mode("nearest");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall_0 = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    let wall_1 = ir_wall(0.2, &[(2.0, 0.0), (3.0, 0.0), (3.0, 1.0), (2.0, 1.0)]);
    let wall_2 = ir_wall(0.2, &[(4.0, 0.0), (5.0, 0.0), (5.0, 1.0), (4.0, 1.0)]);
    let seam_vertex = ir_point(3.0, 0.0, 0.2);
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![wall_0.clone(), wall_1, wall_2.clone()],
        vec![seam_candidate(
            seam_vertex,
            0.1,
            slicer_ir::SeamReason::Aligned,
        )],
        None,
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 3, "all three walls must be emitted");
    assert_wall_eq(&rotated[0].2, &wall_0);
    assert_wall_eq(&rotated[2].2, &wall_2);
    assert_eq!(rotated[1].2.path.points[0], seam_vertex);
    assert_eq!(
        output
            .resolved_seam()
            .expect("seam must be committed")
            .wall_index,
        1
    );
}

#[test]
fn multi_region_wall_counts_preserved() {
    let config = config_with_mode("nearest");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let region_a = vec![
        ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        ir_wall(0.2, &[(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9)]),
        ir_wall(0.2, &[(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)]),
    ];
    let region_b = vec![
        ir_wall(0.2, &[(2.0, 0.0), (3.0, 0.0), (3.0, 1.0), (2.0, 1.0)]),
        ir_wall(0.2, &[(2.1, 0.1), (2.9, 0.1), (2.9, 0.9), (2.1, 0.9)]),
    ];
    let seam_a = ir_point(0.0, 0.0, 0.2);
    let seam_b = ir_point(2.0, 0.0, 0.2);
    let regions = vec![
        sdk_region(
            "obj-a",
            0,
            region_a.clone(),
            vec![seam_candidate(seam_a, 0.1, slicer_ir::SeamReason::Aligned)],
            None,
        ),
        sdk_region(
            "obj-b",
            1,
            region_b.clone(),
            vec![seam_candidate(seam_b, 0.1, slicer_ir::SeamReason::Aligned)],
            None,
        ),
    ];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 5, "both regions' walls must be emitted");
    assert_eq!(
        output.rotated_wall_loop_origins(),
        &[
            Some(("obj-a".to_string(), 0)),
            Some(("obj-a".to_string(), 0)),
            Some(("obj-a".to_string(), 0)),
            Some(("obj-b".to_string(), 1)),
            Some(("obj-b".to_string(), 1)),
        ]
    );
    for (actual, expected) in rotated[1..3].iter().zip(&region_a[1..]) {
        assert_wall_eq(&actual.2, expected);
    }
    for (actual, expected) in rotated[4..].iter().zip(&region_b[1..]) {
        assert_wall_eq(&actual.2, expected);
    }
    assert!(!rotated[0].2.path.points.is_empty());
    assert_eq!(rotated[0].2.path.points[0], seam_a);
    assert_eq!(rotated[0].1, 0);
    assert!(!rotated[3].2.path.points.is_empty());
    assert_eq!(rotated[3].2.path.points[0], seam_b);
    assert_eq!(rotated[3].1, 0);
    assert_eq!(
        output
            .resolved_seam()
            .expect("seam must be committed")
            .wall_index,
        0
    );
}

#[test]
fn aligned_snap_preserves_siblings() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall_0 = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    let wall_1 = ir_wall(0.2, &[(2.0, 0.0), (3.0, 0.0), (3.0, 1.0), (2.0, 1.0)]);
    let wall_2 = ir_wall(0.2, &[(4.0, 0.0), (5.0, 0.0), (5.0, 1.0), (4.0, 1.0)]);
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![wall_0.clone(), wall_1, wall_2.clone()],
        vec![seam_candidate(
            ir_point(3.0, 0.0, 0.2),
            0.1,
            slicer_ir::SeamReason::Aligned,
        )],
        Some(SeamPosition {
            point: ir_point(3.3, 0.0, 0.2),
            wall_index: 1,
        }),
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 3, "all three walls must be emitted");
    assert_wall_eq(&rotated[0].2, &wall_0);
    assert_wall_eq(&rotated[2].2, &wall_2);
    assert_eq!(
        output
            .resolved_seam()
            .expect("seam must be committed")
            .wall_index,
        1
    );
}

#[test]
fn tolerance_miss_emits_all_walls_pristine() {
    let config = config_with_mode("nearest");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let walls = vec![
        ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        ir_wall(0.2, &[(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9)]),
        ir_wall(0.2, &[(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)]),
        ir_wall(0.2, &[(0.3, 0.3), (0.7, 0.3), (0.7, 0.7), (0.3, 0.7)]),
    ];
    let miss = ir_point(0.5, 0.5, 0.2);
    let regions = vec![sdk_region(
        "obj-a",
        0,
        walls.clone(),
        vec![seam_candidate(miss, 0.1, slicer_ir::SeamReason::Aligned)],
        Some(SeamPosition {
            point: miss,
            wall_index: 0,
        }),
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("a tolerance miss must not fail wall postprocess");

    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 4, "all four walls must be emitted");
    for (actual, expected) in rotated.iter().zip(&walls) {
        assert_wall_eq(&actual.2, expected);
    }
    assert!(
        output.resolved_seam().is_none(),
        "a seam missing every wall vertex must not be committed"
    );
}
