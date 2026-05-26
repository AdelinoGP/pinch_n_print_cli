//! TDD coverage for the host built-ins `PrePass::Slice` and
//! `PrePass::ShellClassification`.
//!
//! Together these replace the per-layer `Layer::Slice` host built-in:
//! `PrePass::Slice` produces `Vec<SliceIR>` once for the whole print (with
//! `slice_closing_radius` wired in), and `PrePass::ShellClassification`
//! refines that Vec with cross-layer `top_shell_index` / `bottom_shell_index`
//! and polygon-precise `top_solid_fill` / `bottom_solid_fill`.
//!
//! Coordinate system: 1 unit = 100 nm; use `Point2::from_mm` for fixtures.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    commit_shell_classification_builtin, commit_slice_builtin, execute_prepass_slice_all_layers,
    Blackboard, BlackboardError, BlackboardPrepassSlot, LayerSliceError, ShellClassificationError,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectMesh,
    Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, Transform3d,
};

// ============================================================================
// Fixture helpers
// ============================================================================

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// 10×10×L mm cuboid mesh centred on origin (XY), bottom at z=0, top at z=L.
fn cuboid_mesh(object_id: &str, l_mm: f32) -> MeshIR {
    // 8 corners
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        v(-5.0, -5.0, 0.0),
        v(5.0, -5.0, 0.0),
        v(5.0, 5.0, 0.0),
        v(-5.0, 5.0, 0.0),
        v(-5.0, -5.0, l_mm),
        v(5.0, -5.0, l_mm),
        v(5.0, 5.0, l_mm),
        v(-5.0, 5.0, l_mm),
    ];
    // 12 triangles (CCW from outside)
    let indices = vec![
        // bottom (-Z normal)
        0, 2, 1, 0, 3, 2, // top (+Z normal)
        4, 5, 6, 4, 6, 7, // +X
        1, 2, 6, 1, 6, 5, // -X
        0, 4, 7, 0, 7, 3, // +Y
        3, 7, 6, 3, 6, 2, // -Y
        0, 1, 5, 0, 5, 4,
    ];
    let mesh = IndexedTriangleSet { vertices, indices };
    MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh,
            transform: identity(),
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: 0.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        },
        ..Default::default()
    }
}

fn make_layer(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn make_plan(n_layers: u32, layer_height: f32, object_id: &str) -> LayerPlanIR {
    let global_layers: Vec<GlobalLayer> = (0..n_layers)
        .map(|i| make_layer(i, layer_height * (i + 1) as f32, object_id))
        .collect();
    LayerPlanIR {
        global_layers,
        ..Default::default()
    }
}

fn make_region_map(plan: &LayerPlanIR, top_layers: u32, bottom_layers: u32) -> RegionMapIR {
    let mut entries = HashMap::new();
    for gl in &plan.global_layers {
        for active in &gl.active_regions {
            let mut config = active.resolved_config.clone();
            config.top_shell_layers = top_layers;
            config.bottom_shell_layers = bottom_layers;
            entries.insert(
                RegionKey {
                    global_layer_index: gl.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                },
                RegionPlan {
                    config,
                    ..Default::default()
                },
            );
        }
    }
    RegionMapIR {
        entries,
        ..Default::default()
    }
}

fn seeded_blackboard(mesh: MeshIR, plan: LayerPlanIR, region_map: RegionMapIR) -> Blackboard {
    let n_layers = plan.global_layers.len();
    let mut bb = Blackboard::new(Arc::new(mesh), n_layers);
    bb.commit_layer_plan(Arc::new(plan))
        .expect("commit_layer_plan");
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit_region_map");
    bb
}

// ============================================================================
// PrePass::Slice tests
// ============================================================================

#[test]
fn prepass_slice_produces_one_slice_per_global_layer() {
    // Cuboid taller than the plan span so every slice Z falls inside the
    // mesh interior (avoids the on-the-top-face empty-slice corner case).
    let mesh = cuboid_mesh("cube", 1.2);
    let plan = make_plan(5, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice committed");

    let slices = bb.slice_ir().expect("slice_ir committed");
    assert_eq!(slices.len(), 5);
    for (i, s) in slices.iter().enumerate() {
        assert_eq!(s.global_layer_index, i as u32);
        assert!(!s.regions.is_empty(), "layer {i} should have ≥1 region");
        assert!(
            !s.regions[0].polygons.is_empty(),
            "layer {i} region should have ≥1 polygon for a solid cuboid"
        );
    }
}

#[test]
fn prepass_slice_blocks_on_missing_layer_plan() {
    let mesh = cuboid_mesh("cube", 1.0);
    let mut bb = Blackboard::new(Arc::new(mesh), 0);
    let err = commit_slice_builtin(&mut bb).expect_err("missing LayerPlan must fail");
    match err {
        LayerSliceError::MissingLayerPlan => {}
        other => panic!("expected MissingLayerPlan, got {other:?}"),
    }
}

#[test]
fn prepass_slice_dup_commit_surfaces_as_blackboard_error() {
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("first commit");
    let err = commit_slice_builtin(&mut bb).expect_err("dup commit must fail");
    match err {
        LayerSliceError::Blackboard(BlackboardError::DuplicatePrepassCommit { slot }) => {
            assert_eq!(slot, BlackboardPrepassSlot::SliceIR);
        }
        other => panic!("expected DuplicatePrepassCommit, got {other:?}"),
    }
}

#[test]
fn prepass_slice_all_layers_uses_blackboard_inputs() {
    // Pure-function form: ensures `execute_prepass_slice_all_layers` reads
    // from the blackboard without mutating it.
    let mesh = cuboid_mesh("cube", 0.6);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let bb = seeded_blackboard(mesh, plan, region_map);

    let slices: Vec<SliceIR> = execute_prepass_slice_all_layers(&bb).expect("ok");
    assert_eq!(slices.len(), 3);
    assert!(bb.slice_ir().is_none(), "pure path must not commit");
}

// ============================================================================
// PrePass::ShellClassification tests
// ============================================================================

#[test]
fn shell_classification_blocks_on_uncommitted_slice_ir() {
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    let err =
        commit_shell_classification_builtin(&mut bb).expect_err("must require committed SliceIR");
    match err {
        ShellClassificationError::SliceIRNotCommitted => {}
        other => panic!("expected SliceIRNotCommitted, got {other:?}"),
    }
}

#[test]
fn shell_classification_top_and_bottom_layers_for_single_object_cuboid() {
    // A 5-layer cuboid with top_shell=2 and bottom_shell=2 should classify:
    //   layer 0 (z=0.2): bottom_shell_index=Some(0) (no layer below in timeline)
    //   layer 1 (z=0.4): bottom_shell_index=Some(1) (shadow projection from layer 0)
    //   layer 2 (z=0.6): no shell index (interior)
    //   layer 3 (z=0.8): top_shell_index=Some(1)  (shadow from layer 4)
    //   layer 4 (z=1.0): top_shell_index=Some(0)  (no layer above)
    // Cuboid is open-top at z=1.0 → slice at z=1.0 yields empty polygons,
    // making layer index 4 the "absent" top neighbor for shell classification.
    // From layer 3's perspective the upper layer is empty, so layer 3 becomes
    // the exposed top (depth 0). Pass 2 then projects back one layer (k=2-1)
    // to mark layer 2 as depth 1. Bottom side: layer 0 = depth 0, layer 1 =
    // depth 1.
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(5, 0.2, "cube");
    let region_map = make_region_map(&plan, 2, 2);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice");
    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");

    let slices = bb.slice_ir().expect("classified slice_ir present");
    assert_eq!(slices.len(), 5);

    // Bottom shell zone (layers 0..2)
    let layer0 = &slices[0].regions[0];
    assert_eq!(
        layer0.bottom_shell_index,
        Some(0),
        "layer 0 should be exposed bottom"
    );
    assert!(
        !layer0.bottom_solid_fill.is_empty(),
        "exposed bottom must have non-empty bottom_solid_fill"
    );
    let layer1 = &slices[1].regions[0];
    assert_eq!(
        layer1.bottom_shell_index,
        Some(1),
        "layer 1 should be depth-1 below exposed bottom"
    );

    // Top shell zone — layer 4's slice at z=1.0 is empty (on the top face),
    // making layer 3 the effective exposed top (per OrcaSlicer's "empty
    // upper neighbor = exposed surface" semantics).
    let layer3 = &slices[3].regions[0];
    assert_eq!(
        layer3.top_shell_index,
        Some(0),
        "layer 3 should be exposed top when layer 4's slice is empty"
    );
    assert!(
        !layer3.top_solid_fill.is_empty(),
        "exposed top must have non-empty top_solid_fill"
    );
    let layer2 = &slices[2].regions[0];
    assert_eq!(
        layer2.top_shell_index,
        Some(1),
        "layer 2 should be depth-1 below exposed top"
    );
}

#[test]
fn shell_classification_replace_is_atomic_against_prior_slice_ir() {
    // Verify replace_slice_ir behavior: after shell-classification commits, the
    // blackboard's slice_ir slot points at the new Vec, not the original.
    let mesh = cuboid_mesh("cube", 0.6);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 1, 1);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice");
    let pre_addr = Arc::as_ptr(bb.slice_ir().unwrap());

    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");
    let post_addr = Arc::as_ptr(bb.slice_ir().unwrap());

    assert_ne!(
        pre_addr, post_addr,
        "replace_slice_ir must publish a new Arc, not mutate the old one"
    );
}
