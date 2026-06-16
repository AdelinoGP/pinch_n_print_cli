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
        "slicer:world-prepass@1.0.0",
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
        "slicer:world-finalization@1.0.0",
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
        "slicer:world-postpass@1.0.0",
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
fn empty_guest_output_does_not_populate_arena() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .no_wasm()
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer)
        .expect("Layer::Infill dispatch+commit should succeed");

    assert!(
        fx.arena.infill().is_none(),
        "infill slot should be empty for no-op stage"
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
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline with output commitment should complete: {:?}",
        result.err()
    );
}
