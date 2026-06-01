//! SDK-level dispatch tests for the seam-placer module.
//!
//! Migrated from `crates/slicer-runtime/tests/live_seam_path_tdd.rs` as part
//! of the packet-28 follow-up cleanup: SDK-level trait tests belong inside
//! the module crate, not inside `slicer-runtime`. The host should only carry
//! tests for host plumbing (commit paths, blackboard contracts, dispatch
//! routing). These four tests exercise `SeamPlacer::run_wall_postprocess`
//! directly through the Rust trait — the wasmtime live-dispatch shape is
//! covered separately by host-level integration tests.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamCandidate,
    SeamReason, WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{seam_candidate, PerimeterRegionViewBuilder};
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

// ── Helpers (mirrors live_seam_path_tdd.rs's IR builders) ───────────────

fn empty_seam_config() -> ConfigView {
    ConfigView::from_map(HashMap::new())
}

fn random_seam_config() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        ConfigValue::String("random".to_string()),
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

fn sdk_region(
    object_id: &str,
    region_id: u64,
    walls: Vec<WallLoop>,
    candidates: Vec<SeamCandidate>,
) -> PerimeterRegionView {
    {
        let mut tmp = PerimeterRegionView::default();
        tmp.set_object_id(object_id.to_string());
        tmp.set_region_id(region_id);
        tmp.set_wall_loops(walls);
        tmp.set_infill_areas(vec![]);
        tmp.set_seam_candidates(candidates);
        tmp.set_resolved_seam(None);
        tmp
    }
}

// ── Dispatch tests ─────────────────────────────────────────────────────

#[test]
fn seam_placer_selects_lowest_effective_score_candidate() {
    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![ir_wall(0.2, &[(1.0, 0.0), (2.0, 0.0), (3.0, 0.0)])],
        vec![
            ir_candidate(1.0, 0.0, 0.2, 0.55, SeamReason::Aligned),
            ir_candidate(2.0, 0.0, 0.2, 0.60, SeamReason::Sharp),
            ir_candidate(3.0, 0.0, 0.2, 0.45, SeamReason::Aligned),
        ],
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("selected seam must be committed to output");
    assert_eq!(
        seam.wall_index, 0,
        "single-wall region must resolve to wall 0"
    );
    assert!(
        (seam.point.x - 2.0).abs() < 0.001,
        "lowest effective score candidate should win, got seam at x={} instead of 2.0",
        seam.point.x
    );
}

#[test]
fn seam_rotation_preserves_non_target_walls() {
    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let outer_wall = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    let inner_wall = ir_wall(0.2, &[(0.0, 1.0), (1.0, 1.0), (2.0, 1.0)]);
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![outer_wall.clone(), inner_wall.clone()],
        vec![ir_candidate(1.0, 1.0, 0.2, 0.10, SeamReason::Aligned)],
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("resolved seam must be emitted for the selected wall");
    assert_eq!(
        seam.wall_index, 1,
        "candidate on second wall must resolve to wall 1"
    );

    let rotated_loops = output.rotated_wall_loops();
    assert_eq!(
        rotated_loops.len(),
        2,
        "all region walls must be re-emitted so sibling walls survive commit"
    );
    assert_eq!(
        rotated_loops[0].2, outer_wall,
        "non-target sibling wall must be preserved in original order"
    );
    assert_eq!(
        rotated_loops[1].2.path.points[0],
        ir_point(1.0, 1.0, 0.2),
        "target wall must be rotated so the seam point becomes the first vertex"
    );
}

#[test]
fn seam_contract_is_deterministic_across_repeated_dispatch() {
    let config = random_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");

    let run_once = || {
        let mut output = PerimeterOutputBuilder::new();
        let regions = vec![sdk_region(
            "obj-a",
            0,
            vec![ir_wall(
                0.2,
                &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
            )],
            vec![
                ir_candidate(0.0, 0.0, 0.2, 0.2, SeamReason::Aligned),
                ir_candidate(1.0, 0.0, 0.2, 0.2, SeamReason::Aligned),
                ir_candidate(2.0, 0.0, 0.2, 0.2, SeamReason::Aligned),
                ir_candidate(3.0, 0.0, 0.2, 0.2, SeamReason::Aligned),
            ],
        )];
        module
            .run_wall_postprocess(7, &regions, &mut output, &config)
            .expect("wall postprocess must succeed");
        (
            output.resolved_seam().cloned(),
            output.rotated_wall_loops().to_vec(),
        )
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first.0, second.0,
        "repeated identical dispatches must resolve the same seam"
    );
    assert_eq!(
        first.1, second.1,
        "repeated identical dispatches must emit byte-identical rotated loops"
    );
}

#[test]
fn seam_candidate_missing_from_target_wall_rejects_dispatch() {
    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])],
        vec![ir_candidate(99.0, 99.0, 0.2, 0.10, SeamReason::Aligned)],
    )];
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_wall_postprocess(0, &regions, &mut output, &config);
    assert!(
        result.is_err(),
        "malformed seam candidate that is absent from all walls must reject dispatch"
    );
    assert!(
        output.resolved_seam().is_none(),
        "failed dispatch must not commit a resolved seam"
    );
    assert!(
        output.rotated_wall_loops().is_empty(),
        "failed dispatch must not emit rotated wall loops"
    );
}
