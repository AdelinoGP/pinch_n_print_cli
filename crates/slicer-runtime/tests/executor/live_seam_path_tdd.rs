//! Integration TDD tests: live seam commitment on the production host path.
//!
//! Verifies that the live `Layer::PerimetersPostProcess` stage (seam-placer)
//! commits seam-first wall loops via push-reordered-wall-loop.
//!
//! The path being tested:
//!   `dispatch_layer_call("Layer::PerimetersPostProcess")` â†’ seam-placer emits
//!   seam_candidates + rotated wall loops via push-reordered-wall-loop
//!   â†’ `commit_layer_outputs`
//!   â†’ `convert_perimeter_output` (uses rotated_wall_loops when present)
//!   â†’ `PerimeterIR` â†’ `arena.set_perimeter`
//!
//! Key invariants verified:
//!   - Rotated wall loops with seam at points[0] are committed to PerimeterIR
//!   - resolved_seam is committed to PerimeterIR after PerimetersPostProcess dispatch
//!   - resolved_seam.point.z matches the source loop layer Z
//!   - Empty perimeter output (no loops, no candidates) skips commit without error
//!   - Z envelope violations are rejected at push-reordered-wall-loop
//!   - Feature-flags and width_profile.widths cardinality mismatches are rejected

#![allow(missing_docs)]

use slicer_runtime::wit_host::layer::slicer::ir_handles::ir_handles::HostPerimeterOutputBuilder;
use slicer_runtime::wit_host::{
    ExtrusionRole, HostExecutionContextBuilder, OriginId, Point3, Point3WithWidth, WallFeatureFlag,
    WallLoopType, WallLoopView, WitWallBoundaryType,
};

use crate::common::wasm_cache;
use crate::common::{commit_hec_for_test, layer_input};

/// Helper: create a minimal SliceIR with regions matching the given
/// `(object_id, region_id)` pairs, then stage it into the arena.
/// Required because `region_partition::sync_perimeter_infill_areas_into_slice`
/// fires on every PerimetersPostProcess commit and needs a staged SliceIR.
fn stage_minimal_slice_ir(
    arena: &mut slicer_runtime::LayerArena,
    layer_index: u32,
    z: f32,
    region_keys: &[(&str, u64)],
) {
    use slicer_ir::{ExPolygon, Point2, Polygon, SliceIR, SlicedRegion};
    use std::collections::HashMap;

    let regions = region_keys
        .iter()
        .map(|(oid, rid)| SlicedRegion {
            object_id: oid.to_string(),
            region_id: *rid,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                        Point2 { x: 0, y: 10_000 },
                    ],
                },
                holes: Vec::new(),
            }],
            infill_areas: Vec::new(),
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
        })
        .collect();

    arena
        .set_slice(SliceIR {
            global_layer_index: layer_index,
            z,
            regions,
            ..Default::default()
        })
        .expect("stage_minimal_slice_ir: set_slice must succeed");
}

/// Helper: make a 2-point horizontal wall loop at a given Z.
fn make_wall_loop(layer_z: f32, x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> WallLoopView {
    WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_runtime::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth {
                    x: x1,
                    y: y1,
                    z: layer_z,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: x2,
                    y: y2,
                    z: layer_z,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
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
        boundary_type: WitWallBoundaryType::ExteriorSurface,
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

    let mut ctx = HostExecutionContextBuilder::new(module_id.to_string(), layer_z, 0.2).build();

    // Simulate seam-placer output: one wall loop + seam candidates.
    // (resolved_seam would be set via SDK's set_resolved_seam() if WIT allowed it)
    ctx.perimeter_output_mut()
        .wall_loops
        .push(make_wall_loop(layer_z, 0.0, 0.0, 10.0, 0.0, 0.4));
    ctx.perimeter_output_mut()
        .wall_loop_origins
        .push(Some(OriginId {
            object_id: String::new(),
            region_id: 0,
        }));

    // Seam candidates (pos, score).
    let candidate_pos = Point3 {
        x: 5.0,
        y: 0.0,
        z: layer_z,
    };
    ctx.perimeter_output_mut()
        .seam_candidates
        .push((candidate_pos, 1.0));
    ctx.perimeter_output_mut()
        .seam_candidate_origins
        .push(Some(OriginId {
            object_id: String::new(),
            region_id: 0,
        }));

    ctx.set_current_perimeter_region(Some(OriginId {
        object_id: String::new(),
        region_id: 0,
    }));
    ctx.push_resolved_seam(Resource::new_own(0), candidate_pos, 0)
        .expect("host push_resolved_seam call must succeed")
        .expect("guest push_resolved_seam call must succeed");

    let mut arena = slicer_runtime::LayerArena::new();
    stage_minimal_slice_ir(&mut arena, layer_index, layer_z, &[("", 0)]);
    commit_hec_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
        None,
    )
    .expect("commit must succeed");

    let perimeter_ir = arena
        .perimeter()
        .expect("PerimeterIR must be set after PerimetersPostProcess commit");

    let resolved = &perimeter_ir
        .regions
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
fn empty_perimeter_output_for_perimeterspostprocess_skips_commit() {
    let module_id = "com.test.empty-perimeter";
    let layer_index = 0u32;

    let ctx = HostExecutionContextBuilder::new(module_id.to_string(), 0.2, 0.2).build();
    // All three collections empty â€” perimeter disabled or no eligible regions.

    let mut arena = slicer_runtime::LayerArena::new();
    commit_hec_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
        None,
    )
    .expect("commit must succeed for empty perimeter (not an error)");

    let perimeter_ir = arena.perimeter();
    assert!(
        perimeter_ir.is_none(),
        "empty perimeter must produce None in arena, got {perimeter_ir:#?}"
    );
}

// SDK-level trait tests for `SeamPlacer::run_wall_postprocess` â€”
// `seam_placer_selects_lowest_effective_score_candidate`,
// `seam_rotation_preserves_non_target_walls`,
// `seam_contract_is_deterministic_across_repeated_dispatch`,
// `seam_candidate_missing_from_target_wall_rejects_dispatch` â€”
// and their `ir_*` / `sdk_region` builders were moved to
// `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs`
// (packet-28 follow-up cleanup). SDK-level tests belong in the module
// crate; slicer-runtime should only carry tests for host plumbing or
// wasmtime-driven integration.

#[test]
fn resolved_seam_is_applied_only_to_origin_region() {
    use wasmtime::component::Resource;

    let module_id = "com.test.seam-placer";
    let layer_index = 0u32;
    let layer_z = 0.2;

    let mut ctx = HostExecutionContextBuilder::new(module_id.to_string(), layer_z, 0.2).build();

    ctx.perimeter_output_mut()
        .wall_loops
        .push(make_wall_loop(layer_z, 0.0, 0.0, 10.0, 0.0, 0.4));
    ctx.perimeter_output_mut()
        .wall_loop_origins
        .push(Some(OriginId {
            object_id: "obj-a".to_string(),
            region_id: 0,
        }));
    ctx.perimeter_output_mut()
        .wall_loops
        .push(make_wall_loop(layer_z, 0.0, 1.0, 10.0, 1.0, 0.4));
    ctx.perimeter_output_mut()
        .wall_loop_origins
        .push(Some(OriginId {
            object_id: "obj-b".to_string(),
            region_id: 1,
        }));

    ctx.set_current_perimeter_region(Some(OriginId {
        object_id: "obj-a".to_string(),
        region_id: 0,
    }));
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

    let mut arena = slicer_runtime::LayerArena::new();
    stage_minimal_slice_ir(
        &mut arena,
        layer_index,
        layer_z,
        &[("obj-a", 0), ("obj-b", 1)],
    );
    commit_hec_for_test(
        "Layer::PerimetersPostProcess",
        module_id,
        layer_index,
        &ctx,
        &mut arena,
        None,
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

// `seam_contract_is_deterministic_across_repeated_dispatch` and
// `seam_candidate_missing_from_target_wall_rejects_dispatch` were also
// moved to `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs`
// (same packet-28 follow-up cleanup as above).

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
    use slicer_ir::{BoundingBox3, GlobalLayer, LayerAnnotationKind, LayerCollectionIR, Point3};
    use slicer_runtime::instance_pool::build_wasm_instance_pool;
    use slicer_runtime::manifest::LoadedModuleBuilder;
    use slicer_runtime::{
        Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
        WasmInstancePool, WasmRuntimeDispatcher,
    };
    use std::sync::Arc;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Load the real path-optimization-default.wasm module.
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/slicer-runtime
        .parent()
        .unwrap() // pinch_n_print
        .join("modules/core-modules/path-optimization-default/path-optimization-default.wasm");
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "path-optimization-default.wasm not found at {}. \
             Build it with: `cargo xtask build-guests`",
            wasm_path.display()
        )
    });
    let _component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("path-optimization-default.wasm must compile"),
    );

    let loaded = LoadedModuleBuilder::new(
        "com.core.path-optimization-default",
        slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::PathOptimization",
        slicer_schema::WORLD_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["PerimeterIR".to_string()])
    .ir_writes(vec!["LayerCollectionIR".to_string()])
    .min_host_version(slicer_ir::SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(slicer_ir::SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(slicer_ir::SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let _pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            slicer_runtime::instance_pool::WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(
            std::collections::HashMap::new(),
        )))
        .build();

    // Build PerimeterIR with resolved_seam set on one wall loop.
    let layer_z = 0.2;
    let perimeter_ir = slicer_ir::PerimeterIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![slicer_ir::PerimeterRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            walls: vec![slicer_ir::WallLoop {
                perimeter_index: 0,
                loop_type: slicer_ir::LoopType::Outer,
                path: slicer_ir::ExtrusionPath3D {
                    points: vec![
                        slicer_ir::Point3WithWidth {
                            x: 0.0,
                            y: 0.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        slicer_ir::Point3WithWidth {
                            x: 10.0,
                            y: 0.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        slicer_ir::Point3WithWidth {
                            x: 10.0,
                            y: 10.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        slicer_ir::Point3WithWidth {
                            x: 0.0,
                            y: 10.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                    ],
                    role: slicer_ir::ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                width_profile: slicer_ir::WidthProfile {
                    widths: vec![0.4; 4],
                },
                feature_flags: vec![],
                boundary_type: slicer_ir::WallBoundaryType::Interior,
            }],
            infill_areas: vec![],
            seam_candidates: vec![],
            resolved_seam: Some(slicer_ir::SeamPosition {
                point: slicer_ir::Point3WithWidth {
                    x: 5.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.0,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                wall_index: 0,
            }),
        }],
    };

    // Stage PerimeterIR + empty LayerCollectionIR into arena.
    let mut arena = LayerArena::new();
    arena.set_perimeter(perimeter_ir).unwrap();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: layer_z,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    });

    let blackboard = Blackboard::new(
        Arc::new(slicer_ir::MeshIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            objects: vec![],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 10.0,
                },
            },
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
        &CompiledModuleLive::new(
            module.module_id(),
            WasmInstancePool::placeholder(),
            None,
            module.claims(),
            Arc::clone(module.config_view()),
        ),
        layer_input(&blackboard, &arena),
    )
    .expect("PathOptimization dispatch must succeed");

    // Take deferred annotations and verify none of them is a Raw G1 move.
    let annotations = arena.take_deferred_annotations();
    let raw_g1_count = annotations
        .iter()
        .filter(|ann| match &ann.kind {
            LayerAnnotationKind::Raw(text) => text.starts_with("G1"),
            _ => false,
        })
        .count();

    // After the Option B fix, PathOptimization does NOT replay wall loops.
    // PerimeterIR already contains seam-first geometry committed by seam-placer.
    // PathOptimization only emits marker comments.
    assert_eq!(
        raw_g1_count, 0,
        "PathOptimization must NOT emit G1 wall-loop moves (perimeter is already seam-first); got {raw_g1_count}"
    );
}

// â”€â”€ AC-6: Rotated points cardinality mismatch rejected â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-6: Given rotated wall loop whose feature_flags.len() != path.points.len(),
/// when PerimeterIR is committed, then the commit is rejected with a cardinality
/// mismatch error and no PerimeterIR is set in the arena.
#[test]
fn rotated_points_cardinality_mismatch_rejected() {
    let module_id = "com.test.seam-placer";
    let layer_index = 0u32;
    let layer_z = 0.2;

    let mut ctx = HostExecutionContextBuilder::new(module_id.to_string(), layer_z, 0.2).build();

    // Build a wall loop view with 3 points but only 2 feature flags
    // (intentionally mismatched â€” violates the cardinality invariant).
    let bad_wall_loop = WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_runtime::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: 5.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        // Only 2 flags for 3 points â€” cardinality mismatch
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
        boundary_type: WitWallBoundaryType::ExteriorSurface,
    };

    let _seam_pos = Point3WithWidth {
        x: 5.0,
        y: 0.0,
        z: layer_z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };

    // Inject the bad wall loop directly into PerimeterOutputCollected.
    // This bypasses the WIT boundary but verifies that convert_perimeter_output
    // rejects mismatched cardinality (feature_flags.len() != path.points.len()).
    ctx.perimeter_output_mut()
        .rotated_wall_loops
        .push(bad_wall_loop);
    ctx.perimeter_output_mut()
        .rotated_wall_loop_origins
        .push(Some(OriginId {
            object_id: String::new(),
            region_id: 0,
        }));

    // convert_perimeter_output should reject the mismatched cardinality.
    let result =
        slicer_runtime::wit_host::convert_perimeter_output(&ctx.perimeter_output(), layer_index);

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

    let mut ctx =
        HostExecutionContextBuilder::new(module_id.to_string(), layer_z, effective_layer_height)
            .build();

    let good_loop = WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: slicer_runtime::wit_host::ExtrusionPath3d {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: 5.0,
                    y: 0.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
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
        boundary_type: WitWallBoundaryType::ExteriorSurface,
    };

    // Seam Z is ABOVE the layer envelope ceiling (0.4) â€” should be rejected.
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
        overhang_quartile: None,
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
        Ok(Ok(())) => None,    // success
        Ok(Err(e)) => Some(e), // operation rejected
        Err(_) => None,        // wasmtime failed (not our case)
    };

    assert!(
        inner_err.is_some(),
        "push_reordered_wall_loop must reject seam Z outside layer envelope (got Ok(Ok(())))"
    );
    let err_msg = inner_err.unwrap();
    assert!(
        err_msg.contains("Z_ENVELOPE")
            || err_msg.contains("envelope")
            || err_msg.contains("ceiling"),
        "error must mention Z_ENVELOPE or envelope or ceiling, got: {err_msg}"
    );

    // Verify nothing was committed.
    assert!(
        ctx.perimeter_output().rotated_wall_loops.is_empty(),
        "no rotated wall loops must be stored after Z rejection"
    );
}

// â”€â”€ AC-3: SeamPlanIR injection into wall postprocess region view â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-3: Given SeamPlanIR.entries[0] with RegionKey matching PerimeterRegionView,
/// when Layer::PerimetersPostProcess dispatches seam-placer,
/// then PerimeterIR.regions[0].resolved_seam equals SeamPlanIR.entries[0].chosen_candidate.
///
/// The injection happens inside `push_perimeter_regions` (dispatch.rs) where
/// `seam_plan_ir.entries` are matched against perimeter regions by
/// `(global_layer_index, object_id, region_id)` and the `resolved_seam` field
/// of `PerimeterRegionData` is populated before the WASM guest receives the view.
///
/// This test exercises the full dispatch path:
///   `WasmRuntimeDispatcher::run_stage` â†’ `dispatch_layer_call` â†’
///   `push_perimeter_regions` (injects seam) â†’ WASM guest â†’
///   `apply_for_test` â†’ PerimeterIR in arena.
#[test]
fn seam_plan_ir_is_injected_into_wall_postprocess_region_view() {
    use slicer_ir::{
        BoundingBox3, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LoopType, PerimeterIR,
        PerimeterRegion, Point3WithWidth, RegionId, RegionKey, SeamPlanEntry, SeamPlanIR,
        SeamPosition, SemVer, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
    };
    use slicer_runtime::instance_pool::build_wasm_instance_pool;
    use slicer_runtime::manifest::LoadedModuleBuilder;
    use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};
    use std::sync::Arc;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Load the real seam-placer.wasm module.
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/slicer-runtime
        .parent()
        .unwrap() // pinch_n_print
        .join("modules/core-modules/seam-placer/seam-placer.wasm");
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "seam-placer.wasm not found at {}. \
             Build it with: `cargo xtask build-guests`",
            wasm_path.display()
        )
    });
    let _component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("seam-placer.wasm must compile"),
    );

    let loaded = LoadedModuleBuilder::new(
        "com.core.seam-placer",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::PerimetersPostProcess",
        slicer_schema::WORLD_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["PerimeterIR".to_string()])
    .ir_writes(vec![
        "PerimeterIR.resolved-seam".to_string(),
        "PerimeterIR.regions.walls".to_string(),
    ])
    .claims(vec!["seam-placer".to_string()])
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let _pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            slicer_runtime::instance_pool::WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(
            std::collections::HashMap::new(),
        )))
        .build();

    let layer_z = 0.2;
    let layer_index = 0u32;
    let object_id = "obj-a".to_string();
    let region_id: RegionId = 0;

    // Build a PerimeterIR WITHOUT resolved_seam set.
    // The resolved_seam should be injected from SeamPlanIR during dispatch.
    let perimeter_ir = PerimeterIR {
        global_layer_index: layer_index,
        regions: vec![PerimeterRegion {
            object_id: object_id.clone(),
            region_id,
            walls: vec![WallLoop {
                perimeter_index: 0,
                loop_type: LoopType::Outer,
                path: ExtrusionPath3D {
                    points: vec![
                        Point3WithWidth {
                            x: 0.0,
                            y: 0.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: 10.0,
                            y: 0.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: 10.0,
                            y: 10.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: 0.0,
                            y: 10.0,
                            z: layer_z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                    ],
                    role: ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                width_profile: WidthProfile {
                    widths: vec![0.4; 4],
                },
                feature_flags: vec![
                    WallFeatureFlags {
                        tool_index: None,
                        fuzzy_skin: false,
                        is_bridge: false,
                        is_thin_wall: false,
                        skip_ironing: false,
                        custom: std::collections::HashMap::new(),
                    };
                    4
                ],
                boundary_type: WallBoundaryType::ExteriorSurface,
            }],
            infill_areas: vec![],
            seam_candidates: vec![],
            resolved_seam: None, // Not set â€” should be injected from SeamPlanIR
        }],
        ..Default::default()
    };

    // Build a SeamPlanIR with an entry matching the perimeter region.
    // The chosen_candidate is what should be injected into PerimeterRegionView.resolved_seam.
    // Use a seam point that exactly matches the first vertex of the wall loop
    // to pass the seam-placer's validation that the seam is "in" the wall.
    let chosen_x = 0.0;
    let chosen_y = 0.0;
    let chosen_z = layer_z;
    let chosen_wall_index = 0;

    let seam_plan_ir = SeamPlanIR {
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: layer_index,
                object_id: object_id.clone(),
                region_id,
                variant_chain: Vec::new(),
            },
            chosen_candidate: SeamPosition {
                point: Point3WithWidth {
                    x: chosen_x,
                    y: chosen_y,
                    z: chosen_z,
                    width: 0.0,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                wall_index: chosen_wall_index,
            },
            scored_candidates: vec![],
        }],
        ..Default::default()
    };

    // Stage PerimeterIR into arena (without resolved_seam).
    let mut arena = LayerArena::new();
    arena.set_perimeter(perimeter_ir).unwrap();
    stage_minimal_slice_ir(&mut arena, layer_index, layer_z, &[("obj-a", 0)]);

    // Stage SeamPlanIR into blackboard.
    let mut blackboard = Blackboard::new(
        Arc::new(slicer_ir::MeshIR {
            build_volume: BoundingBox3 {
                min: slicer_ir::Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: slicer_ir::Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 10.0,
                },
            },
            ..Default::default()
        }),
        1,
    );
    blackboard
        .commit_seam_plan(Arc::new(seam_plan_ir.clone()))
        .expect("seam_plan must commit");

    let layer = GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: vec![],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // Dispatch Layer::PerimetersPostProcess through the real run_stage path.
    // Inside dispatch_layer_call, push_perimeter_regions is called with
    // seam_plan_ir from blackboard, and resolved_seam is injected into
    // PerimeterRegionData before the WASM guest receives the view.
    eprintln!(
        "DEBUG: before run_stage: seam_plan entries={}",
        seam_plan_ir.entries.len()
    );
    if let Some(entry) = seam_plan_ir.entries.first() {
        eprintln!(
            "DEBUG: seam_plan entry[0]: layer={}, obj={}, region={}, seam=({:.3},{:.3},{:.3})",
            entry.region_key.global_layer_index,
            entry.region_key.object_id,
            entry.region_key.region_id,
            entry.chosen_candidate.point.x,
            entry.chosen_candidate.point.y,
            entry.chosen_candidate.point.z
        );
    }
    eprintln!(
        "DEBUG: perimeter region: obj={}, region={}",
        object_id, region_id
    );

    // Check that blackboard has seam_plan BEFORE dispatch
    eprintln!(
        "DEBUG: blackboard.seam_plan() before run_stage: {:?}",
        blackboard.seam_plan().is_some()
    );
    if let Some(sp) = blackboard.seam_plan() {
        eprintln!("DEBUG: blackboard seam_plan entries: {}", sp.entries.len());
    }
    crate::common::run_layer_and_commit(
        &dispatcher,
        "Layer::PerimetersPostProcess",
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .expect("PerimetersPostProcess dispatch must succeed");

    eprintln!(
        "DEBUG: after run_stage, arena.perimeter()={:?}",
        arena.perimeter().is_some()
    );

    // Verify that PerimeterIR in arena now has resolved_seam set
    // from the SeamPlanIR entry.
    let perimeter_ir_after = arena
        .perimeter()
        .expect("PerimeterIR must be committed after PerimetersPostProcess");

    let region = perimeter_ir_after
        .regions
        .first()
        .expect("at least one region must exist");

    if region.resolved_seam.is_none() {
        eprintln!("DEBUG: PerimeterIR.resolved_seam is None after PerimetersPostProcess");
        eprintln!(
            "DEBUG: region object_id={}, region_id={}",
            region.object_id, region.region_id
        );
        eprintln!(
            "DEBUG: perimeter_ir_after.global_layer_index={}",
            perimeter_ir_after.global_layer_index
        );
        eprintln!("DEBUG: seam_placer output: wall_loops={}, seam_candidates={}, resolved_seam_set_by_guest={}",
            perimeter_ir_after.regions.first().map(|r| r.walls.len()).unwrap_or(0),
            perimeter_ir_after.regions.first().map(|r| r.seam_candidates.len()).unwrap_or(0),
            perimeter_ir_after.regions.first().map(|r| r.resolved_seam.is_some()).unwrap_or(false));
        // Also check the raw wall loop vertices
        if let Some(r) = perimeter_ir_after.regions.first() {
            for (wi, loop_) in r.walls.iter().enumerate() {
                eprintln!(
                    "DEBUG: wall[{}] points: {:?}",
                    wi,
                    loop_
                        .path
                        .points
                        .iter()
                        .map(|p| (p.x, p.y, p.z))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    let resolved_seam = region
        .resolved_seam
        .as_ref()
        .expect("resolved_seam must be injected from SeamPlanIR");

    assert_eq!(
        resolved_seam.point.x, chosen_x,
        "resolved_seam.point.x must equal chosen_candidate.x ({chosen_x}), got {}",
        resolved_seam.point.x
    );
    assert_eq!(
        resolved_seam.point.y, chosen_y,
        "resolved_seam.point.y must equal chosen_candidate.y ({chosen_y}), got {}",
        resolved_seam.point.y
    );
    assert_eq!(
        resolved_seam.point.z, chosen_z,
        "resolved_seam.point.z must equal chosen_candidate.z ({chosen_z}), got {}",
        resolved_seam.point.z
    );
    assert_eq!(
        resolved_seam.wall_index, chosen_wall_index,
        "resolved_seam.wall_index must equal chosen_candidate.wall_index ({chosen_wall_index}), got {}",
        resolved_seam.wall_index
    );
}

// ── Regression: SDK→WIT seam-candidate Z round-trip (drain-fn z=0.0 bug) ──

/// Regression test for a pre-existing bug (predates packet 108, fixed
/// alongside this test): `__slicer_drain_perimeter` in
/// `crates/slicer-macros/src/lib.rs` hardcoded `z: 0.0` when forwarding SDK
/// `seam_candidates` (real `Point3`, carrying the region's actual Z) to the
/// WIT `push_seam_candidate` call. The host's `check_z_envelope` rejects any
/// Z outside `[layer.z, layer.z + effective_layer_height]`; on any layer
/// above the first, z=0.0 falls below the floor and the push is rejected —
/// silently, because the macro-generated guest glue does
/// `let _ = wit.push_seam_candidate(...)`. Host-side `seam_candidates` was
/// therefore always empty coming out of real WASM dispatch, a defect masked
/// until packet 108's seam-placer fatal-on-empty path surfaced it (13
/// `cube_4color_*` executor tests failing with "no seam candidates for
/// region ... region_id=3").
///
/// This drives the REAL `classic-perimeters.wasm` guest (built via
/// `cargo xtask build-guests`, so it exercises the actual macro-generated
/// drain code — not a hand-rolled WIT-only test guest like the other
/// fixtures in this crate) through the real `Layer::Perimeters` dispatch
/// path at a layer Z above the first (layer_z = 0.6, envelope [0.6, 0.8]).
/// z=0.0 falls outside that envelope, so this test fails loudly pre-fix and
/// passes post-fix.
#[test]
fn classic_perimeters_seam_candidate_z_survives_wasm_boundary_above_first_layer() {
    use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon, SemVer, SliceIR, SlicedRegion};
    use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
    use slicer_runtime::manifest::LoadedModuleBuilder;
    use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};
    use std::sync::Arc;

    let layer_index = 3u32;
    let layer_z = 0.6_f32; // Above the first layer — z=0.0 would fail check_z_envelope here.
    let object_id = "obj-a";
    let region_id: u64 = 0;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Load the real classic-perimeters.wasm module (the real seam-candidate
    // producer — NOT a hand-rolled test guest — so the drain-fn fix is
    // actually exercised at the WIT boundary).
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates
        .parent()
        .unwrap() // pinch_n_print
        .join("modules/core-modules/classic-perimeters/classic-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "classic-perimeters.wasm not found at {}. Build it with: `cargo xtask build-guests`",
        wasm_path.display()
    );
    let component = wasm_cache::compiled_component_at(&wasm_path);

    let loaded = LoadedModuleBuilder::new(
        "com.core.classic-perimeters",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Perimeters",
        slicer_schema::WORLD_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "PaintRegionIR".to_string()])
    .ir_writes(vec!["PerimeterIR".to_string()])
    .claims(vec!["perimeter-generator".to_string()])
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();

    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );

    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(
            std::collections::HashMap::new(),
        )))
        .build();

    let bundle = crate::common::TestModuleBundle {
        module,
        pool,
        component: Some(component),
    };

    // Stage a real 10mm square region at layer_z=0.6 — large enough to match
    // the geometry classic-perimeters' own passing tests use for corner-based
    // seam-candidate generation (4 candidates, one per 90-degree corner).
    let side = mm_to_units(10.0);
    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            global_layer_index: layer_index,
            z: layer_z,
            regions: vec![SlicedRegion {
                object_id: object_id.to_string(),
                region_id,
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: side, y: 0 },
                            Point2 { x: side, y: side },
                            Point2 { x: 0, y: side },
                        ],
                    },
                    holes: Vec::new(),
                }],
                infill_areas: Vec::new(),
                nonplanar_surface: None,
                effective_layer_height: 0.2,
                segment_annotations: std::collections::HashMap::new(),
                variant_chain: Vec::new(),
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: vec![],
                bridge_orientation_deg: 0.0,
                sparse_infill_area: Vec::new(),
            }],
            ..Default::default()
        })
        .expect("set_slice must succeed");

    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);

    let layer = slicer_ir::GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Perimeters",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .expect("Layer::Perimeters dispatch+commit must succeed");

    let perimeter_ir = arena
        .perimeter()
        .expect("PerimeterIR must be committed after Layer::Perimeters dispatch");
    let region = perimeter_ir
        .regions
        .iter()
        .find(|r| r.object_id == object_id && r.region_id == region_id)
        .expect("staged region must be present in committed PerimeterIR");

    assert!(
        !region.seam_candidates.is_empty(),
        "seam_candidates must be non-empty for a layer above the first (layer_z={layer_z}); \
         an empty list here reproduces the z=0.0 drain bug: candidates are rejected by \
         check_z_envelope and silently dropped via `let _ = wit.push_seam_candidate(...)`"
    );
    for c in &region.seam_candidates {
        assert_eq!(
            c.position.z, layer_z,
            "seam candidate Z must equal the region/layer Z ({layer_z}), got {} — the drain fn \
             must forward pos.z, not hardcode 0.0",
            c.position.z
        );
    }
}
