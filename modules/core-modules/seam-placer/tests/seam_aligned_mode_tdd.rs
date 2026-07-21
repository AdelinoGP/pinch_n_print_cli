//! TDD tests for aligned seam modes (TASK-274, packet 168).
//!
//! AC-1: `"aligned"` and `"aligned_back"` parse in `on_print_start` and
//! round-trip through `seam_mode()`.
//! AC-N1: explicitly-unknown strings are still rejected with the exact
//! `unknown seam_mode: <value>` message.

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

#[test]
fn aligned_mode_parses() {
    let module =
        SeamPlacer::on_print_start(&config_with_mode("aligned")).expect("\"aligned\" must parse");
    assert_eq!(module.seam_mode(), "aligned");

    let module = SeamPlacer::on_print_start(&config_with_mode("aligned_back"))
        .expect("\"aligned_back\" must parse");
    assert_eq!(module.seam_mode(), "aligned_back");
}

#[test]
fn unknown_mode_still_rejected() {
    let err = match SeamPlacer::on_print_start(&config_with_mode("diagonal")) {
        Ok(_) => panic!("unknown seam_mode must be rejected"),
        Err(err) => err,
    };
    let msg = format!("{err:?}");
    assert!(
        msg.contains("unknown seam_mode: diagonal"),
        "error must contain exact message, got: {msg}"
    );
}

// ── Aligned consumption (AC-6, TASK-274 step 7) ─────────────────────────

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
            custom: HashMap::new()
        };
        count
    ]
}

fn ir_wall(layer_z: f32, points: &[(f32, f32)]) -> WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
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

fn ir_candidate(x: f32, y: f32, z: f32, score: f32, reason: SeamReason) -> SeamCandidate {
    seam_candidate(ir_point(x, y, z), score, reason)
}

fn aligned_region(
    walls: Vec<WallLoop>,
    candidates: Vec<SeamCandidate>,
    resolved: Option<Point3WithWidth>,
) -> PerimeterRegionView {
    let mut tmp = PerimeterRegionView::default();
    tmp.set_object_id("obj-a".to_string());
    tmp.set_region_id(0);
    tmp.set_wall_loops(walls);
    tmp.set_infill_areas(vec![]);
    tmp.set_seam_candidates(candidates);
    tmp.set_resolved_seam(resolved.map(|point| SeamPosition {
        point,
        wall_index: 0,
    }));
    tmp
}

/// AC-6: with a host-injected `resolved_seam` (ADR-0020 channel) that sits
/// deliberately off-vertex (0.3 mm from the nearest wall vertex), aligned
/// modes must snap it to the nearest seam-candidate position by 2D XY
/// distance — not use the raw injected point, and not score candidates.
fn assert_aligned_snaps(mode: &str) {
    let config = config_with_mode(mode);
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    // Injected point 0.3 mm off the (2.0, 0.0) vertex. Candidate at (0,0)
    // has a *better* score, proving the snap ignores scoring.
    let regions = vec![aligned_region(
        vec![wall],
        vec![
            ir_candidate(0.0, 0.0, 0.2, 0.05, SeamReason::Concave),
            ir_candidate(2.0, 0.0, 0.2, 0.90, SeamReason::Aligned),
        ],
        Some(ir_point(2.3, 0.0, 0.2)),
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("aligned mode must resolve a seam from the injected point");
    assert!(
        (seam.point.x - 2.0).abs() < 0.001 && seam.point.y.abs() < 0.001,
        "seam must snap to nearest candidate (2.0, 0.0), got ({}, {})",
        seam.point.x,
        seam.point.y
    );
    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 1, "wall must be preserved");
    let first = rotated[0].2.path.points[0];
    assert!(
        (first.x - 2.0).abs() < 0.001 && first.y.abs() < 0.001,
        "rotated loop must start at snapped vertex, got ({}, {})",
        first.x,
        first.y
    );
}

#[test]
fn aligned_snaps_to_nearest_candidate() {
    assert_aligned_snaps("aligned");
}

#[test]
fn aligned_back_snaps_to_nearest_candidate() {
    assert_aligned_snaps("aligned_back");
}

/// Empty-candidates fallback: the injected point snaps to the nearest
/// wall-loop vertex instead.
#[test]
fn aligned_empty_candidates_snaps_to_nearest_wall_vertex() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    let regions = vec![aligned_region(
        vec![wall],
        vec![],
        Some(ir_point(1.3, 0.0, 0.2)),
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("empty candidates must fall back to nearest wall vertex");
    assert!(
        (seam.point.x - 1.0).abs() < 0.001,
        "seam must snap to nearest wall vertex (1.0, 0.0), got x={}",
        seam.point.x
    );
    let first = output.rotated_wall_loops()[0].2.path.points[0];
    assert!(
        (first.x - 1.0).abs() < 0.001,
        "rotated loop must start at snapped wall vertex, got x={}",
        first.x
    );
}

/// No injected resolved seam in aligned mode: walls emit pristine.
#[test]
fn aligned_without_resolved_seam_emits_pristine_walls() {
    let config = config_with_mode("aligned");
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let wall_points = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
    let wall = ir_wall(0.2, &wall_points);
    let regions = vec![aligned_region(
        vec![wall],
        vec![ir_candidate(1.0, 0.0, 0.2, 0.1, SeamReason::Aligned)],
        None,
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    assert!(
        output.resolved_seam().is_none(),
        "no injected seam → no resolved seam committed"
    );
    let rotated = output.rotated_wall_loops();
    assert_eq!(rotated.len(), 1, "wall must be preserved pristine");
    let emitted_xy: Vec<(f32, f32)> = rotated[0]
        .2
        .path
        .points
        .iter()
        .map(|p| (p.x, p.y))
        .collect();
    assert_eq!(emitted_xy, wall_points.to_vec(), "wall must be un-rotated");
}
