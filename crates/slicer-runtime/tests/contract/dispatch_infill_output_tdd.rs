use crate::common::*;
use slicer_ir::{GlobalLayer, LayerCollectionIR, MeshIR, ResolvedConfig, SemVer};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::pipeline::{run_pipeline, PipelineConfig, PipelineStageRunners};
use slicer_runtime::{
    build_wasm_instance_pool, CompiledModuleBuilder, CompiledStage, ExecutionPlan, GCodeEmitter,
    GCodeSerializer, LayerArena, WasmArtifactMetadata,
};
use slicer_wasm_host::WasmRuntimeDispatcher;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
    ) -> Result<slicer_ir::GCodeIR, slicer_runtime::GCodeEmitError> {
        Ok(slicer_ir::GCodeIR {
            metadata: slicer_ir::PrintMetadata {
                slicer_version: "test".into(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(
        &self,
        _gcode_ir: &slicer_ir::GCodeIR,
    ) -> Result<String, slicer_runtime::GCodeEmitError> {
        Ok(String::from("; test gcode"))
    }
}

fn make_prepass_bundle(stage_id: &str) -> TestModuleBundle {
    let component = wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/prepass-guest.component.wasm"
    )));
    let loaded = LoadedModuleBuilder::new(
        "com.test.prepass",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
        slicer_schema::WORLD_PREPASS,
        std::path::PathBuf::from("/dev/null"),
    )
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
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new("com.test.prepass")
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn make_finalization_bundle(stage_id: &str) -> TestModuleBundle {
    let component = wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/finalization-guest.component.wasm"
    )));
    let loaded = LoadedModuleBuilder::new(
        "com.test.fin",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
        slicer_schema::WORLD_FINALIZATION,
        std::path::PathBuf::from("/dev/null"),
    )
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
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new("com.test.fin")
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn make_postpass_bundle(stage_id: &str) -> TestModuleBundle {
    let component = wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/postpass-guest.component.wasm"
    )));
    let loaded = LoadedModuleBuilder::new(
        "com.test.gpost",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
        slicer_schema::WORLD_POSTPASS,
        std::path::PathBuf::from("/dev/null"),
    )
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
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new("com.test.gpost")
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

// ── F. Full pipeline integration with typed dispatch ──────────────────────────

#[test]
fn full_pipeline_with_typed_layer_dispatch() {
    let engine = wasm_cache::shared_engine();

    let fx = dispatch_fixture::for_stage("Layer::Infill").build();
    let (layer_module, mut wasm_handles) = fx.bundle.into_module_and_handles();

    let lp_bundle = make_prepass_bundle("PrePass::LayerPlanning");
    let (lp_module, lp_handles) = lp_bundle.into_module_and_handles();
    wasm_handles.extend(lp_handles);

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![lp_module],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: Arc::new(MeshIR::default()),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
        resolved_configs: std::sync::Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: std::sync::Arc::new(ResolvedConfig::default()),
        bounds: std::sync::Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles,
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline with typed layer dispatch should complete: {:?}",
        result.err()
    );
}

#[test]
fn full_pipeline_multi_tier_with_typed_layer() {
    let engine = wasm_cache::shared_engine();

    let mesh_bundle = make_prepass_bundle("PrePass::MeshAnalysis");
    let (mesh_module, mut wasm_handles) = mesh_bundle.into_module_and_handles();
    let lp_bundle = make_prepass_bundle("PrePass::LayerPlanning");
    let (lp_module, lp_handles) = lp_bundle.into_module_and_handles();
    wasm_handles.extend(lp_handles);

    let fx = dispatch_fixture::for_stage("Layer::Infill").build();
    let (layer_module, layer_handles) = fx.bundle.into_module_and_handles();
    wasm_handles.extend(layer_handles);

    let fin_bundle = make_finalization_bundle("PostPass::LayerFinalization");
    let (fin_module, fin_handles) = fin_bundle.into_module_and_handles();
    wasm_handles.extend(fin_handles);

    let gcode_bundle = make_postpass_bundle("PostPass::GCodePostProcess");
    let (gcode_module, gcode_handles) = gcode_bundle.into_module_and_handles();
    wasm_handles.extend(gcode_handles);

    let plan = ExecutionPlan {
        prepass_stages: vec![
            CompiledStage {
                stage_id: "PrePass::MeshAnalysis".into(),
                modules: vec![mesh_module],
            },
            CompiledStage {
                stage_id: "PrePass::LayerPlanning".into(),
                modules: vec![lp_module],
            },
        ],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: Some(CompiledStage {
            stage_id: "PostPass::LayerFinalization".into(),
            modules: vec![fin_module],
        }),
        postpass_stages: vec![CompiledStage {
            stage_id: "PostPass::GCodePostProcess".into(),
            modules: vec![gcode_module],
        }],
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: Arc::new(MeshIR::default()),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
        resolved_configs: std::sync::Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: std::sync::Arc::new(ResolvedConfig::default()),
        bounds: std::sync::Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles,
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "multi-tier pipeline with typed layer dispatch should complete: {:?}",
        result.err()
    );
}

// ── G. Output commitment tests ────────────────────────────────────────────────

#[test]
fn guest_infill_output_committed_to_arena() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_slice(
            ir_builders::slice_ir::with_count(1)
                .at_layer(7)
                .at_z(1.4)
                .build(),
        )
        .build();

    let layer = GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer)
        .expect("Layer::Infill dispatch+commit should succeed");

    let infill = fx
        .arena
        .infill()
        .expect("infill arena slot should be populated");
    assert_eq!(infill.global_layer_index, 7, "layer index should match");
    assert_eq!(infill.regions.len(), 1, "should have 1 region");
    let region = &infill.regions[0];
    assert_eq!(region.sparse_infill.len(), 1, "should have 1 sparse path");
    assert_eq!(
        region.sparse_infill[0].points.len(),
        2,
        "path should have 2 points"
    );
    assert_eq!(
        region.sparse_infill[0].role,
        slicer_ir::ExtrusionRole::SparseInfill,
        "role should be SparseInfill"
    );
}

#[test]
fn output_commitment_deterministic_across_repeated_runs() {
    let fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_slice(ir_builders::slice_ir::with_count(1).build())
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let mut arena = LayerArena::new();
        arena
            .set_slice(ir_builders::slice_ir::with_count(1).build())
            .unwrap();
        run_layer_and_commit_with_bundle(
            &fx.dispatcher,
            "Layer::Infill",
            &layer,
            &fx.bundle,
            &fx.blackboard,
            &mut arena,
        )
        .unwrap();
        let infill = arena.take_infill().expect("infill should be committed");
        results.push(infill);
    }

    assert_eq!(results[0], results[1], "run 0 and 1 should be identical");
    assert_eq!(results[1], results[2], "run 1 and 2 should be identical");
}

#[test]
fn invalid_nan_output_rejected_with_diagnostic() {
    use slicer_runtime::wit_host::{
        convert_infill_output, ExtrusionPath3d, ExtrusionRole, InfillOutputCollected,
        Point3WithWidth,
    };

    let bad_output = InfillOutputCollected {
        sparse_paths: vec![ExtrusionPath3d {
            points: vec![Point3WithWidth {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            }],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        }],
        solid_paths: Vec::new(),
        ironing_paths: Vec::new(),
        ..Default::default()
    };

    let result = convert_infill_output(&bad_output, 0);
    assert!(result.is_err(), "NaN output should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
    assert!(
        msg.contains("point[0]"),
        "error should identify the point index: {msg}"
    );
}

#[test]
fn end_to_end_pipeline_commits_guest_output_to_arena() {
    let engine = wasm_cache::shared_engine();

    let fx = dispatch_fixture::for_stage("Layer::Infill").build();
    let (layer_module, mut wasm_handles) = fx.bundle.into_module_and_handles();

    let lp_bundle = make_prepass_bundle("PrePass::LayerPlanning");
    let (lp_module, lp_handles) = lp_bundle.into_module_and_handles();
    wasm_handles.extend(lp_handles);

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![lp_module],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: Arc::new(MeshIR::default()),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
        resolved_configs: std::sync::Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: std::sync::Arc::new(ResolvedConfig::default()),
        bounds: std::sync::Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles,
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline with output commitment should complete: {:?}",
        result.err()
    );
}

// ── Restored tests (migrated from dispatch_tdd.rs split) ──────────────────────

#[test]
fn infill_output_correct_when_slice_regions_present() {
    // Verify that the existing output commitment for infill is not
    // regressed when real slice region data is provided.
    let mut fields: HashMap<String, slicer_ir::ConfigValue> = HashMap::new();
    fields.insert("infill-spacing".into(), slicer_ir::ConfigValue::Float(3.0));

    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_slice(
            ir_builders::slice_ir::with_count(1)
                .at_layer(5)
                .at_z(1.0)
                .build(),
        )
        .with_config(slicer_ir::ConfigView::from_map(fields))
        .build();

    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer).unwrap();

    let infill = fx.arena.infill().expect("infill should be populated");
    let path = &infill.regions[0].sparse_infill[0];
    // Config spacing=3.0 → second point x = 30.0
    assert_eq!(
        path.points[1].x, 30.0,
        "config wiring still works with slice regions present"
    );
    // First point encodes region data: z from slice, region_count=1, poly_count=1
    // Expected = |base_regions| × |polygons_per_region|; with_count(1) produces
    // 1 SlicedRegion each with exactly 1 ExPolygon.
    // Cross-product: 1 region × 1 ExPolygon = 1 total polygon visible.
    assert_eq!(path.points[0].z, 1.0, "z from slice region");
    assert_eq!(path.points[0].flow_factor, 1.0, "1 region visible");
    assert_eq!(path.points[0].width, 1.0, "1 polygon visible");
    assert_eq!(
        infill.global_layer_index, 5,
        "layer index preserved in output"
    );
}

#[test]
fn empty_perimeter_input_valid_for_infill_postprocess() {
    // When no PerimeterIR is staged, guest sees zero regions and emits no
    // output (per-region loop). The empty-bypass keeps the infill slot empty
    // — this is the documented empty case and must not fail.
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess").build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    // Do not stage any perimeter IR.

    fx.run_layer(&layer).unwrap();

    assert!(
        fx.arena.infill().is_none(),
        "no input regions → no output → empty bypass"
    );
}

#[test]
fn stage_without_perimeter_input_does_not_see_perimeter_state() {
    // Layer::Infill consumes slice regions, not perimeter regions. Even if
    // PerimeterIR is staged in the arena, the infill guest should not
    // observe it — with zero slice regions, the guest emits no geometry.
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        // Stage perimeter data only; no slice data.
        .with_perimeter(
            ir_builders::perimeter_ir::with_count(4)
                .at_layer(0)
                .walls(2)
                .infill(5)
                .build(),
        )
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer).unwrap();

    // No infill output confirms perimeter state was not misrouted into the
    // slice-region view.
    assert!(
        fx.arena.infill().is_none(),
        "Infill stage must not see perimeter data as slice regions"
    );
}

#[test]
fn failed_commit_does_not_leak_into_next_call() {
    // Two sequential calls sharing one arena: first succeeds and populates
    // infill, second (for perimeters) with empty output should not see leaked
    // infill.
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_slice(ir_builders::slice_ir::with_count(1).build())
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // First call: infill (produces output)
    let r1 = fx.run_layer(&layer);
    assert!(r1.is_ok(), "infill should succeed");
    assert!(
        fx.arena.infill().is_some(),
        "infill slot should be populated"
    );

    // Second call: perimeters (no-op — should not contaminate anything).
    // Build a second fixture just to source its bundle for the Perimeters
    // stage, then dispatch against the shared arena/blackboard.
    let fxp = dispatch_fixture::for_stage("Layer::Perimeters").build();
    let r2 = run_layer_and_commit_with_bundle(
        &fx.dispatcher,
        "Layer::Perimeters",
        &layer,
        &fxp.bundle,
        &fx.blackboard,
        &mut fx.arena,
    );
    assert!(r2.is_ok(), "perimeters should succeed");
    // Perimeter slot should be empty (no-op guest), infill slot unchanged.
    assert!(
        fx.arena.perimeter().is_none(),
        "perimeter slot should stay empty"
    );
    assert!(
        fx.arena.infill().is_some(),
        "infill slot should still be populated"
    );
}

// ── IR builder unit tests ─────────────────────────────────────────────────────

#[test]
fn ir_builders_slice_ir_with_count_shape() {
    let ir = crate::common::ir_builders::slice_ir::with_count(3)
        .at_z(0.2)
        .build();
    assert_eq!(ir.global_layer_index, 0);
    assert_eq!(ir.z, 0.2);
    assert_eq!(ir.regions.len(), 3);
    for i in 0..3 {
        assert_eq!(ir.regions[i].object_id, format!("obj-{i}"));
        assert_eq!(ir.regions[i].region_id, i as u64);
        assert_eq!(ir.regions[i].polygons.len(), 1);
        assert_eq!(ir.regions[i].polygons[0].contour.points.len(), 4);
        assert!(ir.regions[i].polygons[0].holes.is_empty());
        assert_eq!(ir.regions[i].effective_layer_height, 0.2);
    }
}

#[test]
fn ir_builders_slice_ir_with_ids_shape() {
    let ir = crate::common::ir_builders::slice_ir::with_ids(&[
        ("custom-obj", 17u64),
        ("other-obj", 99u64),
    ])
    .at_z(0.5)
    .build();
    assert_eq!(ir.regions.len(), 2);
    assert_eq!(ir.regions[0].object_id, "custom-obj");
    assert_eq!(ir.regions[0].region_id, 17);
    assert_eq!(ir.regions[1].object_id, "other-obj");
    assert_eq!(ir.regions[1].region_id, 99);
}
