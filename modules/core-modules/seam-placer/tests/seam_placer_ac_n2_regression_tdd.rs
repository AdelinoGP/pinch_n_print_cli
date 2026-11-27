//! Named concrete regressions for the two confirmed AC-N2 seam-placer defects.
//!
//! Both defects were reproduced against the pre-fix tree through the public
//! `LayerModule::run_wall_postprocess` path (and, for the intermediate state of
//! the second one, through a direct in-crate call that was deleted after the
//! output was captured), then fixed with the minimal focused change in
//! `seam-placer/src/lib.rs`. Each test here is RNG- and registry-context
//! independent: fixed input, fixed expectation, no `PropertyRun`.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamPosition,
    WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::PerimeterRegionViewBuilder;
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

fn config_with_mode(mode: &str) -> ConfigView {
    let mut map = HashMap::new();
    map.insert(
        "seam_position".to_string(),
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
        ..Default::default()
    }
}

fn ir_wall(layer_z: f32, points: &[(f32, f32)]) -> WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
    let flags = vec![WallFeatureFlags::default(); path_points.len()];
    let path = ExtrusionPath3D {
        points: path_points,
        ..extrusion_path3d_base(ExtrusionRole::OuterWall)
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}

fn region(
    walls: Vec<WallLoop>,
    candidates: Vec<slicer_ir::SeamCandidate>,
    resolved: Option<Point3WithWidth>,
) -> PerimeterRegionView {
    let mut region = PerimeterRegionView::default();
    region.set_object_id("obj-a".to_string());
    region.set_region_id(0);
    region.set_wall_loops(walls);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(candidates);
    region.set_resolved_seam(resolved.map(|point| SeamPosition {
        point,
        wall_index: 0,
    }));
    region
}

/// A closed rectangle: four effective vertices plus the explicit closing repeat.
fn closed_rectangle(layer_z: f32) -> WallLoop {
    ir_wall(
        layer_z,
        &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0), (0.0, 0.0)],
    )
}

#[test]
fn random_empty_candidates_degrade_without_panic() {
    // Root cause: `select_seam_candidate`'s `SeamMode::Random` arm computed
    // `let idx = (layer_index as usize) % candidates.len();` before
    // `candidates.get(idx)`. With an empty candidate list that is a remainder by
    // zero, so `run_wall_postprocess` panicked instead of taking the documented
    // graceful-degradation path.
    //
    // Failing input (pre-fix): the exact call below — `seam_mode = "random"`,
    // `layer_index = 0`, one valid closed wall loop, `seam_candidates == []`,
    // `resolved_seam == None`. Pre-fix output:
    //   thread '...' panicked at modules/core-modules/seam-placer/src/lib.rs:106:23:
    //   attempt to calculate the remainder with a divisor of zero
    //
    // Applicable contract: `docs/05_module_sdk.md` T-082 / P109 graceful
    // degradation ("empty `seam_candidates` and no `resolved_seam` … emits the
    // region's walls pristine, leaves `resolved_seam` unset") and the
    // wall-preservation invariant. The guarantee is mode-agnostic; it is not a
    // property of the `Random` arm.
    let config = config_with_mode("random");
    let module = SeamPlacer::from_config(&config).expect("module init must succeed");
    let wall = closed_rectangle(0.2);
    let input = wall.clone();
    let regions = vec![region(vec![wall], vec![], None)];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("empty candidates must degrade instead of panicking");

    assert!(
        output.resolved_seam().is_none(),
        "no seam may resolve without candidates or an injected plan"
    );
    assert_eq!(
        output.rotated_wall_loops().len(),
        1,
        "the region's wall loop must reach the output"
    );
    let emitted = &output.rotated_wall_loops()[0].2;
    assert_eq!(
        emitted.path.points, input.path.points,
        "the wall must be emitted pristine"
    );
    assert_eq!(
        emitted.feature_flags.len(),
        emitted.path.points.len(),
        "per-point arrays stay parallel"
    );
    assert_eq!(
        emitted.width_profile.widths.len(),
        emitted.path.points.len(),
        "per-point arrays stay parallel"
    );
}

#[test]
fn open_polyline_projection_preserves_open_geometry() {
    // Root cause: `project_onto_wall_segment` enumerated `points[last] ->
    // points[0]` as a real segment for OPEN paths (`end_index` wrapped to 0 when
    // `segment_start + 1 == effective_len`), and `insert_projected_point`
    // unconditionally appended a closing point and width while the flag push was
    // `if is_closed`-gated. An open polyline therefore became force-closed and
    // the per-point arrays desynchronised.
    //
    // Failing input (pre-fix): `seam_mode = "aligned"`, the OPEN 4-point polyline
    // below, `resolved_seam == (0.0, 2.0)`. That target lies on the fabricated
    // closing chord (0,4)->(0,0); pre-fix output was
    //   closed=true points=6 widths=6 flags=6
    // with the projected point inserted at the chord's midpoint, and the direct
    // kernel call on the real segment (0,0)->(4,0) produced
    //   closed=true points=6 widths=6 flags=5   (the cardinality break).
    //
    // Applicable contract: `docs/02_ir_schemas.md` IR 7 ("legitimately open
    // paths … exist. Nothing may fabricate closure for an open line") plus the
    // unconditional per-point-array rule and the `docs/01`/`docs/05` cardinality
    // postcondition ("is preserved after insertion").
    let config = config_with_mode("aligned");
    let module = SeamPlacer::from_config(&config).expect("module init must succeed");
    let wall = ir_wall(0.2, &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    assert!(
        !wall.path.is_closed(),
        "the fixture must be an open polyline"
    );
    let regions = vec![region(vec![wall], vec![], Some(ir_point(1.5, -1.0, 0.2)))];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("the injected seam must project onto real geometry");
    assert!(
        (seam.point.x - 1.5).abs() < 1e-4 && (seam.point.y).abs() < 1e-4,
        "the seam must land on the real bottom segment, got ({}, {})",
        seam.point.x,
        seam.point.y
    );

    let emitted = &output.rotated_wall_loops()[0].2;
    let points = &emitted.path.points;
    assert!(
        !emitted.path.is_closed(),
        "an open polyline must stay open, got {:?}",
        points
    );
    assert_eq!(
        points.len(),
        5,
        "one interior insert into four open records yields five"
    );
    assert_eq!(
        emitted.feature_flags.len(),
        points.len(),
        "feature_flags must stay parallel to path.points"
    );
    assert_eq!(
        emitted.width_profile.widths.len(),
        points.len(),
        "width_profile.widths must stay parallel to path.points"
    );
    assert!(
        points
            .iter()
            .any(|point| (point.x - 1.5).abs() < 1e-4 && point.y.abs() < 1e-4),
        "the projected interior point must be inserted, got {:?}",
        points
    );
    assert!(
        !points
            .iter()
            .any(|point| (point.x - 0.0).abs() < 1e-6 && (point.y - 2.0).abs() < 1e-6),
        "no point may be fabricated on the closing chord, got {:?}",
        points
    );
    assert_ne!(
        points.first(),
        points.last(),
        "an open path must not gain a closing repeat"
    );
}

#[test]
fn closed_loop_with_closing_repeat_metadata_mismatch_keeps_closure_and_alignment() {
    // Root cause: `project_onto_wall_segment` and `insert_projected_point`
    // decided closure by full-struct equality (`points.first() == points.last()`)
    // while `ExtrusionPath3D::is_closed` (used by `rotate_wall_loop`) compares XY
    // only. A closing repeat with the same XY but different per-point metadata
    // (here width 0.5 instead of 0.4) was therefore treated as OPEN by the
    // projector/inserter: the same-XY repeat was kept as a real vertex, an extra
    // closing point/width was appended, and the closure-gated flag push was
    // skipped, leaving `points`/`widths` longer than `feature_flags`.
    //
    // Failing input (pre-fix): `seam_mode = "aligned"`, closed rectangle
    // (0,0)(4,0)(4,4)(0,4)(0,0 with width 0.5), `resolved_seam == (1.5, -1.0)`.
    // Pre-fix output: 7 points (flags 6). Expected: one insert into four effective vertices plus one closing repeat
    // = 6 points, closed by XY, `widths`/`flags` parallel to `points`.
    let config = config_with_mode("aligned");
    let module = SeamPlacer::from_config(&config).expect("module init must succeed");
    let mut wall = closed_rectangle(0.2);
    wall.path.points[4].width = 0.5;
    wall.width_profile.widths[4] = 0.5;
    assert!(wall.path.is_closed());
    assert_ne!(wall.path.points.first(), wall.path.points.last());
    let regions = vec![region(vec![wall], vec![], Some(ir_point(1.5, -1.0, 0.2)))];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let emitted = &output.rotated_wall_loops()[0].2;
    let n = emitted.path.points.len();
    assert!(emitted.path.is_closed(), "closed loop must stay closed");
    assert_eq!(n, 6, "4 vertices + 1 insert + 1 closing repeat, got {n}");
    assert_eq!(emitted.feature_flags.len(), n, "flags parallel to points");
    assert_eq!(
        emitted.width_profile.widths.len(),
        n,
        "widths parallel to points"
    );
}
