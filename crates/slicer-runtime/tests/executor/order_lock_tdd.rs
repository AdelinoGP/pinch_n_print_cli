//! Host order-lock enforcement coverage.

#![allow(missing_docs)]

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion, LayerStageCommit, Point3WithWidth,
    SemVer,
};
use slicer_runtime::{apply_for_test, LayerArena, StageApplyContext};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

fn path(lock: Option<u64>) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: 1.0,
                y: 0.0,
                ..Default::default()
            },
        ],
        order_lock: lock,
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
    }
}

fn infill(paths: Vec<ExtrusionPath3D>) -> InfillIR {
    InfillIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 4,
        regions: vec![InfillRegion {
            sparse_infill: paths,
            ..Default::default()
        }],
    }
}

fn context(stage_id: &'static str) -> StageApplyContext<'static> {
    // exhaustive: order-lock commit fixtures explicitly pin all apply context metadata.
    StageApplyContext {
        stage_id,
        module_id: "com.test.order-lock",
        layer_index: 4,
        seam_plan: None,
        config_view: None,
        committed_slices: None,
    }
}

#[test]
fn order_lock_infill_postprocess_preserves_block() {
    let mut arena = LayerArena::new();
    let mut prior = infill(vec![path(Some(7)), path(Some(7)), path(None)]);
    prior.regions[0].sparse_infill[1].points[1].x = 2.0;
    apply_for_test(
        &mut arena,
        LayerStageCommit::Infill(prior),
        &context("Layer::Infill"),
    )
    .expect("prior infill commit must succeed");
    let committed_prior = arena.infill().expect("prior infill must be staged").clone();

    let mut dropped = committed_prior.clone();
    dropped.regions[0].sparse_infill.remove(1);
    let mut reversed = committed_prior.clone();
    reversed.regions[0].sparse_infill.swap(0, 1);
    let mut altered = committed_prior.clone();
    altered.regions[0].sparse_infill[0].points[0].width += 1.0;

    for bad in [dropped, reversed, altered] {
        assert!(matches!(
            apply_for_test(
                &mut arena,
                LayerStageCommit::InfillPostProcess(bad),
                &context("Layer::InfillPostProcess"),
            ),
            Err(slicer_ir::LayerStageError::OrderLockViolation { .. })
        ));
        assert_eq!(arena.infill(), Some(&committed_prior));
    }

    apply_for_test(
        &mut arena,
        LayerStageCommit::InfillPostProcess(committed_prior.clone()),
        &context("Layer::InfillPostProcess"),
    )
    .expect("unchanged locked block must be accepted");
    let paths = &arena
        .infill()
        .expect("compliant infill must be staged")
        .regions[0]
        .sparse_infill;
    assert_eq!(paths[0].order_lock, Some(1 << 63));
    assert_eq!(paths[1].order_lock, Some((1 << 63) | 1));
}

#[test]
fn order_lock_all_none_neutrality() {
    let input = infill(vec![path(None), path(None), path(None)]);
    let mut arena = LayerArena::new();
    apply_for_test(
        &mut arena,
        LayerStageCommit::Infill(input.clone()),
        &context("Layer::Infill"),
    )
    .expect("unlocked infill commit must succeed");
    assert_eq!(arena.infill(), Some(&input));

    let replacement = infill(vec![path(None), path(None), path(None)]);
    apply_for_test(
        &mut arena,
        LayerStageCommit::InfillPostProcess(replacement.clone()),
        &context("Layer::InfillPostProcess"),
    )
    .expect("unlocked post-process commit must succeed");
    assert_eq!(arena.infill(), Some(&replacement));
}

#[test]
fn order_lock_remap_wired_at_output_boundary() {
    let local = infill(vec![path(Some(1)), path(Some(2)), path(Some(1))]);
    let mut arena = LayerArena::new();
    apply_for_test(
        &mut arena,
        LayerStageCommit::Infill(local),
        &context("Layer::Infill"),
    )
    .expect("infill output commit must remap local tags");
    let paths = &arena.infill().expect("infill must be staged").regions[0].sparse_infill;
    assert_eq!(paths[0].order_lock, Some(1 << 63));
    assert_eq!(paths[1].order_lock, Some((1 << 63) | 1));
    assert_eq!(paths[2].order_lock, Some((1 << 63) | 2));

    let mut post_arena = LayerArena::new();
    apply_for_test(
        &mut post_arena,
        LayerStageCommit::Infill(infill(vec![path(Some(1)), path(Some(2)), path(Some(1))])),
        &context("Layer::Infill"),
    )
    .expect("post-process prior commit must succeed");
    apply_for_test(
        &mut post_arena,
        LayerStageCommit::InfillPostProcess(infill(vec![
            path(Some(1)),
            path(Some(2)),
            path(Some(1)),
        ])),
        &context("Layer::InfillPostProcess"),
    )
    .expect("post-process output commit must remap local tags");
    let paths = &post_arena
        .infill()
        .expect("post-process infill must be staged")
        .regions[0]
        .sparse_infill;
    assert_eq!(paths[0].order_lock, Some((1 << 63) | 3));
    assert_eq!(paths[1].order_lock, Some((1 << 63) | 4));
    assert_eq!(paths[2].order_lock, Some((1 << 63) | 5));
}
