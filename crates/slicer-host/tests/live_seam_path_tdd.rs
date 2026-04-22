//! Integration TDD tests: live seam commitment on the production host path.
//!
//! Verifies that the live `Layer::PerimetersPostProcess` stage (seam-placer)
//! commits seam-first wall loops via push-reordered-wall-loop.
//!
//! The path being tested:
//!   `dispatch_layer_call("Layer::PerimetersPostProcess")` → seam-placer emits
//!   seam_candidates + rotated wall loops via push-reordered-wall-loop
//!   → `commit_layer_outputs`
//!   → `convert_perimeter_output` (uses rotated_wall_loops when present)
//!   → `PerimeterIR` → `arena.set_perimeter`
//!
//! Key invariants verified:
//!   - Rotated wall loops with seam at points[0] are committed to PerimeterIR
//!   - resolved_seam is committed to PerimeterIR after WallPostProcess dispatch
//!   - resolved_seam.point.z matches the source loop layer Z
//!   - Empty perimeter output (no loops, no candidates) skips commit without error
//!   - Z envelope violations are rejected at push-reordered-wall-loop
//!   - Feature-flags and width_profile.widths cardinality mismatches are rejected

#![allow(missing_docs)]

use slicer_host::dispatch::commit_layer_outputs_for_test;
use slicer_host::wit_host::{ExtrusionRole, HostExecutionContext, Point3WithWidth, WallFeatureFlag, WallLoopView, WallLoopType, Point3};
use slicer_host::wit_host::layer::slicer::world_layer::ir_handles::HostPerimeterOutputBuilder;

/// Helper: make a 2-point horizontal wall loop at a given Z.
fn make_wall_loop(layer_z: f32, x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> WallLoopView {
    WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_host::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth { x: x1, y: y1, z: layer_z, width, flow_factor: 1.0 },
                Point3WithWidth { x: x2, y: y2, z: layer_z, width, flow_factor: 1.0 },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        // feature-flags must be parallel to path.points: 2 points = 2 flags
        feature_flags: vec![
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
        ],
    }
}

/// Test that `commit_layer_outputs` for "Layer::PerimetersPostProcess" commits
/// `PerimeterIR.regions[0].resolved_seam` when seam candidates are present.
#[test]
fn wall_postprocess_commits_resolved_seam_to_perimeter_ir() {
    use wasmtime::component::Resource;

    let module_id = "com.test.seam-placer";
    let layer_index = 0u32;
    let layer_z = 0.2;

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        layer_z,
        0.2,   // effective_layer_height
        None,  // catchup_z_bottom
        None,  // mesh_ir
    );

    // Simulate seam-placer output: one wall loop + seam candidates.
    // (resolved_seam would be set via SDK's set_resolved_seam() if WIT allowed it)
    ctx.perimeter_output.wall_loops.push(make_wall_loop(layer_z, 0.0, 0.0, 10.0, 0.0, 0.4));
    ctx.perimeter_output.wall_loop_origins.push(Some((String::new(), 0)));

    // Seam candidates (pos, score).
    let candidate_pos = Point3 {
        x: 5.0, y: 0.0, z: layer_z,
    };
    ctx.perimeter_output.seam_candidates.push((candidate_pos, 1.0));
    ctx.perimeter_output.seam_candidate_origins.push(Some((String::new(), 0)));

    ctx.current_perimeter_region = Some((String::new(), 0));
    ctx.push_resolved_seam(Resource::new_own(0), candidate_pos, 0)
        .expect("host push_resolved_seam call must succeed")
        .expect("guest push_resolved_seam call must succeed");

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
    )
    .expect("commit must succeed");

    let perimeter_ir = arena
        .perimeter()
        .expect("PerimeterIR must be set after WallPostProcess commit");

    let resolved = &perimeter_ir.regions
        .first()
        .expect("at least one region")
        .resolved_seam;

    assert!(
        resolved.is_some(),
        "PerimeterIR.resolved_seam must be Some after seam-placer commit, got None. \
         Full IR: {perimeter_ir:#?}"
    );

    let resolved = resolved.as_ref().unwrap();
    assert_eq!(
        resolved.point.z, layer_z,
        "resolved_seam.point.z must match layer Z ({layer_z}), got {}",
        resolved.point.z
    );
    assert_eq!(
        resolved.wall_index, 0,
        "resolved_seam.wall_index must be 0, got {}",
        resolved.wall_index
    );
}

/// Test that an empty perimeter output (no loops, no candidates) does not error
/// and produces no PerimeterIR in the arena.
#[test]
fn empty_perimeter_output_for_wallpostprocess_skips_commit() {
    let module_id = "com.test.empty-perimeter";
    let layer_index = 0u32;

    let ctx = HostExecutionContext::new(
        module_id.to_string(),
        0.2,
        0.2,
        None,
        None,
    );
    // All three collections empty — perimeter disabled or no eligible regions.

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
    )
    .expect("commit must succeed for empty perimeter (not an error)");

    let perimeter_ir = arena.perimeter();
    assert!(
        perimeter_ir.is_none(),
        "empty perimeter must produce None in arena, got {perimeter_ir:#?}"
    );
}

fn empty_seam_config() -> slicer_ir::ConfigView {
    slicer_ir::ConfigView::from_map(std::collections::HashMap::new())
}

fn random_seam_config() -> slicer_ir::ConfigView {
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "seam_mode".to_string(),
        slicer_ir::ConfigValue::String("random".to_string()),
    );
    slicer_ir::ConfigView::from_map(fields)
}

fn ir_point(x: f32, y: f32, z: f32) -> slicer_ir::Point3WithWidth {
    slicer_ir::Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
    }
}

fn ir_flags(count: usize) -> Vec<slicer_ir::WallFeatureFlags> {
    (0..count)
        .map(|_| slicer_ir::WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: std::collections::HashMap::new(),
        })
        .collect()
}

fn ir_wall(layer_z: f32, points: &[(f32, f32)]) -> slicer_ir::WallLoop {
    let path_points: Vec<_> = points
        .iter()
        .map(|(x, y)| ir_point(*x, *y, layer_z))
        .collect();
    let point_count = path_points.len();
    slicer_ir::WallLoop {
        perimeter_index: 0,
        loop_type: slicer_ir::LoopType::Outer,
        path: slicer_ir::ExtrusionPath3D {
            points: path_points,
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile {
            widths: vec![0.4; point_count],
        },
        feature_flags: ir_flags(point_count),
        boundary_type: slicer_ir::WallBoundaryType::ExteriorSurface,
    }
}

fn ir_candidate(
    x: f32,
    y: f32,
    z: f32,
    score: f32,
    reason: slicer_ir::SeamReason,
) -> slicer_ir::SeamCandidate {
    slicer_ir::SeamCandidate {
        position: ir_point(x, y, z),
        score,
        reason,
    }
}

fn sdk_region(
    object_id: &str,
    region_id: u64,
    walls: Vec<slicer_ir::WallLoop>,
    candidates: Vec<slicer_ir::SeamCandidate>,
) -> slicer_sdk::views::PerimeterRegionView {
    slicer_sdk::views::PerimeterRegionView::new(
        object_id.to_string(),
        region_id,
        walls,
        vec![],
        candidates,
        None,
    )
}

#[test]
fn seam_placer_selects_lowest_effective_score_candidate() {
    use seam_placer::SeamPlacer;
    use slicer_sdk::builders::PerimeterOutputBuilder;
    use slicer_sdk::traits::LayerModule;

    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![ir_wall(0.2, &[(1.0, 0.0), (2.0, 0.0), (3.0, 0.0)])],
        vec![
            ir_candidate(1.0, 0.0, 0.2, 0.55, slicer_ir::SeamReason::Aligned),
            ir_candidate(2.0, 0.0, 0.2, 0.60, slicer_ir::SeamReason::Sharp),
            ir_candidate(3.0, 0.0, 0.2, 0.45, slicer_ir::SeamReason::Aligned),
        ],
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("selected seam must be committed to output");
    assert_eq!(seam.wall_index, 0, "single-wall region must resolve to wall 0");
    assert!(
        (seam.point.x - 2.0).abs() < 0.001,
        "lowest effective score candidate should win, got seam at x={} instead of 2.0",
        seam.point.x
    );
}

#[test]
fn seam_rotation_preserves_non_target_walls() {
    use seam_placer::SeamPlacer;
    use slicer_sdk::builders::PerimeterOutputBuilder;
    use slicer_sdk::traits::LayerModule;

    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let outer_wall = ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    let inner_wall = ir_wall(0.2, &[(0.0, 1.0), (1.0, 1.0), (2.0, 1.0)]);
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![outer_wall.clone(), inner_wall.clone()],
        vec![ir_candidate(1.0, 1.0, 0.2, 0.10, slicer_ir::SeamReason::Aligned)],
    )];
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_wall_postprocess(0, &regions, &mut output, &config)
        .expect("wall postprocess must succeed");

    let seam = output
        .resolved_seam()
        .expect("resolved seam must be emitted for the selected wall");
    assert_eq!(seam.wall_index, 1, "candidate on second wall must resolve to wall 1");

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
fn resolved_seam_is_applied_only_to_origin_region() {
    use wasmtime::component::Resource;

    let module_id = "com.test.seam-placer";
    let layer_index = 0u32;
    let layer_z = 0.2;

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        layer_z,
        0.2,
        None,
        None,
    );

    ctx.perimeter_output
        .wall_loops
        .push(make_wall_loop(layer_z, 0.0, 0.0, 10.0, 0.0, 0.4));
    ctx.perimeter_output
        .wall_loop_origins
        .push(Some(("obj-a".to_string(), 0)));
    ctx.perimeter_output
        .wall_loops
        .push(make_wall_loop(layer_z, 0.0, 1.0, 10.0, 1.0, 0.4));
    ctx.perimeter_output
        .wall_loop_origins
        .push(Some(("obj-b".to_string(), 1)));

    ctx.current_perimeter_region = Some(("obj-a".to_string(), 0));
    ctx.push_resolved_seam(
        Resource::new_own(0),
        Point3 {
            x: 5.0,
            y: 0.0,
            z: layer_z,
        },
        0,
    )
    .expect("host push_resolved_seam call must succeed")
    .expect("guest push_resolved_seam call must succeed");

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
    )
    .expect("commit must succeed");

    let perimeter_ir = arena.perimeter().expect("PerimeterIR must commit");
    let region_a = perimeter_ir
        .regions
        .iter()
        .find(|region| region.object_id == "obj-a")
        .expect("obj-a region must exist");
    let region_b = perimeter_ir
        .regions
        .iter()
        .find(|region| region.object_id == "obj-b")
        .expect("obj-b region must exist");

    assert!(
        region_a.resolved_seam.is_some(),
        "origin region must keep the emitted resolved_seam"
    );
    assert!(
        region_b.resolved_seam.is_none(),
        "non-origin sibling region must not inherit another region's seam"
    );
}

#[test]
fn seam_contract_is_deterministic_across_repeated_dispatch() {
    use seam_placer::SeamPlacer;
    use slicer_sdk::builders::PerimeterOutputBuilder;
    use slicer_sdk::traits::LayerModule;

    let config = random_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");

    let run_once = || {
        let mut output = PerimeterOutputBuilder::new();
        let regions = vec![sdk_region(
            "obj-a",
            0,
            vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)])],
            vec![
                ir_candidate(0.0, 0.0, 0.2, 0.2, slicer_ir::SeamReason::Aligned),
                ir_candidate(1.0, 0.0, 0.2, 0.2, slicer_ir::SeamReason::Aligned),
                ir_candidate(2.0, 0.0, 0.2, 0.2, slicer_ir::SeamReason::Aligned),
                ir_candidate(3.0, 0.0, 0.2, 0.2, slicer_ir::SeamReason::Aligned),
            ],
        )];
        module
            .run_wall_postprocess(7, &regions, &mut output, &config)
            .expect("wall postprocess must succeed");
        (output.resolved_seam().cloned(), output.rotated_wall_loops().to_vec())
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
    use seam_placer::SeamPlacer;
    use slicer_sdk::builders::PerimeterOutputBuilder;
    use slicer_sdk::traits::LayerModule;

    let config = empty_seam_config();
    let module = SeamPlacer::on_print_start(&config).expect("module init must succeed");
    let regions = vec![sdk_region(
        "obj-a",
        0,
        vec![ir_wall(0.2, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])],
        vec![ir_candidate(99.0, 99.0, 0.2, 0.10, slicer_ir::SeamReason::Aligned)],
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

/// End-to-end test: after seam resolution, PathOptimization remains
/// comment-only and does not replay wall loops as G1 moves.
///
/// The path being tested:
///   1. Stage PerimeterIR with `resolved_seam = Some(...)` into arena
///   2. Dispatch `Layer::PathOptimization` with path-optimization-default module
///   3. Verify deferred annotations remain free of wall-loop Move commands
///
/// This proves the current seam-first contract: seam-placer owns wall-loop
/// geometry changes, while PathOptimization may add only comment/raw/tool-change
/// overrides documented for LayerCollectionIR.
#[test]
fn path_optimization_stays_comment_only_after_seam_resolution() {
    use std::sync::Arc;
    use slicer_host::{Blackboard, CompiledModule, IrAccessMask, LayerArena, LayerStageRunner, WasmEngine, WasmRuntimeDispatcher};
    use slicer_host::instance_pool::build_wasm_instance_pool;
    use slicer_host::manifest::LoadedModule;
    use slicer_ir::{BoundingBox3, GlobalLayer, LayerCollectionIR, LayerAnnotationKind, Point3};

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Load the real path-optimization-default.wasm module.
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap() // crates/slicer-host
        .parent().unwrap() // pinch_n_print
        .join("modules/core-modules/path-optimization-default/path-optimization-default.wasm");
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "path-optimization-default.wasm not found at {}. \
             Build it with: ./modules/core-modules/build-core-modules.sh",
            wasm_path.display()
        )
    });
    let component = Arc::new(engine.compile_component(&bytes)
        .expect("path-optimization-default.wasm must compile"));

    let loaded = LoadedModule {
        id: "com.core.path-optimization-default".to_string(),
        version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        stage: "Layer::PathOptimization".to_string(),
        wit_world: "slicer:world-layer@1.0.0".to_string(),
        ir_reads: vec!["PerimeterIR".to_string()],
        ir_writes: vec!["LayerCollectionIR".to_string()],
        claims: vec![],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: slicer_ir::SemVer { major: 0, minor: 1, patch: 0 },
        min_ir_schema: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        max_ir_schema: slicer_ir::SemVer { major: 2, minor: 0, patch: 0 },
        config_schema: Default::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: true,
        wasm_path: wasm_path.clone(),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(&loaded, 1, slicer_host::instance_pool::WasmArtifactMetadata { uses_shared_memory: false })
            .expect("instance pool must build")
    );
    let module = CompiledModule {
        module_id: loaded.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(slicer_ir::ConfigView::from_map(std::collections::HashMap::new())),
        wasm_component: Some(component),
    };

    // Build PerimeterIR with resolved_seam set on one wall loop.
    let layer_z = 0.2;
    let perimeter_ir = slicer_ir::PerimeterIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        global_layer_index: 0,
        regions: vec![
            slicer_ir::PerimeterRegion {
                object_id: "test-object".to_string(),
                region_id: 0,
                walls: vec![
                    slicer_ir::WallLoop {
                        perimeter_index: 0,
                        loop_type: slicer_ir::LoopType::Outer,
                        path: slicer_ir::ExtrusionPath3D {
                            points: vec![
                                slicer_ir::Point3WithWidth { x: 0.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                                slicer_ir::Point3WithWidth { x: 10.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                                slicer_ir::Point3WithWidth { x: 10.0, y: 10.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                                slicer_ir::Point3WithWidth { x: 0.0, y: 10.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                            ],
                            role: slicer_ir::ExtrusionRole::OuterWall,
                            speed_factor: 1.0,
                        },
                        width_profile: slicer_ir::WidthProfile { widths: vec![0.4; 4] },
                        feature_flags: vec![],
                        boundary_type: slicer_ir::WallBoundaryType::Interior,
                    },
                ],
                infill_areas: vec![],
                seam_candidates: vec![],
                resolved_seam: Some(slicer_ir::SeamPosition {
                    point: slicer_ir::Point3WithWidth { x: 5.0, y: 0.0, z: layer_z, width: 0.0, flow_factor: 1.0 },
                    wall_index: 0,
                }),
            },
        ],
    };

    // Stage PerimeterIR + empty LayerCollectionIR into arena.
    let mut arena = LayerArena::new();
    arena.set_perimeter(perimeter_ir).unwrap();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        global_layer_index: 0,
        z: layer_z,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
    });

    let blackboard = Blackboard::new(
        Arc::new(slicer_ir::MeshIR {
            schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
            objects: vec![],
            build_volume: BoundingBox3 { min: Point3 { x: 0.0, y: 0.0, z: 0.0 }, max: Point3 { x: 200.0, y: 200.0, z: 10.0 } },
        }),
        1,
    );
    let layer = GlobalLayer {
        index: 0,
        z: layer_z,
        active_regions: vec![],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // Dispatch PathOptimization through the real run_stage path.
    // run_stage internally calls dispatch_layer_call and then commit_layer_outputs,
    // which must reject move replay for this stage.
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    ).expect("PathOptimization dispatch must succeed");

    // Take deferred annotations and verify none of them is a Raw G1 move.
    let annotations = arena.take_deferred_annotations();
    let raw_g1_count = annotations.iter().filter(|ann| {
        match &ann.kind {
            LayerAnnotationKind::Raw(text) => text.starts_with("G1"),
            _ => false,
        }
    }).count();

    // After the Option B fix, PathOptimization does NOT replay wall loops.
    // PerimeterIR already contains seam-first geometry committed by seam-placer.
    // PathOptimization only emits marker comments.
    assert_eq!(
        raw_g1_count, 0,
        "PathOptimization must NOT emit G1 wall-loop moves (perimeter is already seam-first); got {raw_g1_count}"
    );
}

// ── AC-6: Rotated points cardinality mismatch rejected ─────────────────────────────────────────────────

/// AC-6: Given rotated wall loop whose feature_flags.len() != path.points.len(),
/// when PerimeterIR is committed, then the commit is rejected with a cardinality
/// mismatch error and no PerimeterIR is set in the arena.
#[test]
fn rotated_points_cardinality_mismatch_rejected() {
    let module_id = "com.test.seam-placer";
    let layer_index = 0u32;
    let layer_z = 0.2;

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        layer_z,
        0.2,
        None,
        None,
    );

    // Build a wall loop view with 3 points but only 2 feature flags
    // (intentionally mismatched — violates the cardinality invariant).
    let bad_wall_loop = WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_host::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth { x: 0.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                Point3WithWidth { x: 5.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                Point3WithWidth { x: 10.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        // Only 2 flags for 3 points — cardinality mismatch
        feature_flags: vec![
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
        ],
    };

    let _seam_pos = Point3WithWidth {
        x: 5.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0,
    };

    // Inject the bad wall loop directly into PerimeterOutputCollected.
    // This bypasses the WIT boundary but verifies that convert_perimeter_output
    // rejects mismatched cardinality (feature_flags.len() != path.points.len()).
    ctx.perimeter_output.rotated_wall_loops.push(bad_wall_loop);
    ctx.perimeter_output.rotated_wall_loop_origins.push(Some((String::new(), 0)));

    // convert_perimeter_output should reject the mismatched cardinality.
    let result = slicer_host::wit_host::convert_perimeter_output(
        &ctx.perimeter_output,
        layer_index,
    );

    assert!(
        result.is_err(),
        "convert_perimeter_output must reject cardinality mismatch, got Ok"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("CARDINALITY_MISMATCH") || err_msg.contains("feature_flags"),
        "error message must mention CARDINALITY_MISMATCH or feature_flags, got: {err_msg}"
    );
}

/// AC-7: Given a seam position whose Z coordinate exceeds layer.z + effective_layer_height,
/// when push-reordered-wall-loop is called, then the host returns an error and the
/// perimeter output is not committed.
///
/// The Z envelope check lives in `HostPerimeterOutputBuilder::push_reordered_wall_loop`.
/// We verify it indirectly: the commit path must not set PerimeterIR when the
/// resolved_seam Z would have failed the Z check in `push_reordered_wall_loop`.
/// Since push_resolved_seam has no Z check (it stores the seam for later use by
/// seam-placer which DOES check Z), we construct the call that WOULD fail by
/// noting that resolve_seam_path z=0.5 exceeds the layer ceiling of 0.4.
#[test]
fn seam_z_outside_layer_envelope_rejected() {
    let module_id = "com.test.seam-placer";
    let layer_z = 0.2;
    let effective_layer_height = 0.2; // ceiling = 0.4

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        layer_z,
        effective_layer_height,
        None,
        None,
    );

    let good_loop = WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_host::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth { x: 0.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
                Point3WithWidth { x: 5.0, y: 0.0, z: layer_z, width: 0.4, flow_factor: 1.0 },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        feature_flags: vec![
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
            WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            },
        ],
    };

    // Seam Z is ABOVE the layer envelope ceiling (0.4) — should be rejected.
    let bad_seam_pos_z = 0.5; // > layer_z + effective_layer_height (0.4)

    assert!(
        bad_seam_pos_z > layer_z + effective_layer_height,
        "precondition: seam Z should exceed layer ceiling"
    );

    // Directly call the HostPerimeterOutputBuilder::push_reordered_wall_loop method.
    // This is the same code path the WIT bindgen would call from a WASM guest.
    use wasmtime::component::Resource;

    let builder_resource = Resource::new_own(0);
    let seam_pos_with_bad_z = Point3WithWidth {
        x: 5.0,
        y: 0.0,
        z: bad_seam_pos_z,
        width: 0.4,
        flow_factor: 1.0,
    };
    let result = ctx.push_reordered_wall_loop(
        builder_resource,
        seam_pos_with_bad_z,
        0, // wall_index
        good_loop,
    );

    // Result is Result<Result<(), String>, wasmtime::Error>
    // - Ok(Ok(())) = success
    // - Ok(Err("Z_ENVELOPE_VIOLATION")) = operation rejected (what we want)
    // - Err(wasmtime::Error) = wasmtime itself failed
    let inner_err: Option<&String> = match &result {
        Ok(Ok(())) => None,                       // success
        Ok(Err(e)) => Some(e),                   // operation rejected
        Err(_) => None,                           // wasmtime failed (not our case)
    };

    assert!(
        inner_err.is_some(),
        "push_reordered_wall_loop must reject seam Z outside layer envelope (got Ok(Ok(())))"
    );
    let err_msg = inner_err.unwrap();
    assert!(
        err_msg.contains("Z_ENVELOPE") || err_msg.contains("envelope") || err_msg.contains("ceiling"),
        "error must mention Z_ENVELOPE or envelope or ceiling, got: {err_msg}"
    );

    // Verify nothing was committed.
    assert!(
        ctx.perimeter_output.rotated_wall_loops.is_empty(),
        "no rotated wall loops must be stored after Z rejection"
    );
}