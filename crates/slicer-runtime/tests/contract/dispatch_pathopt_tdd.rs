// dispatch_pathopt_tdd.rs — Layer::PathOptimization overrides
// (tool changes, z-hops, retracts, unretracts, deferred travel moves, comments, raw fragments)

use crate::common::seed::seed_slice_ir;
use crate::common::*;
use slicer_ir::{GlobalLayer, LayerCollectionIR, LayerStageError, MeshIR, RetractMode};
use slicer_runtime::{Blackboard, CompiledStage, ExecutionPlan, GCodeEmitter, LayerStageRunner};
use slicer_wasm_host::{CompiledModuleLive, LayerStageInput, WasmRuntimeDispatcher};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

// ── L. PathOptimization: ordered_entities threading + GCode override commit ──

#[test]
fn path_optimization_commit_folds_tool_changes_into_deferred_queue() {
    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization")
        .with_perimeter(ir_builders::perimeter_ir::with_count(3).walls(1).build())
        .build();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    fx.run_layer(&layer).unwrap();
    let tcs = fx.arena.take_deferred_tool_changes();
    assert_eq!(tcs.len(), 3, "three tool-changes routed to deferred queue");
    let mapped: Vec<(u32, u32)> = tcs.iter().map(|t| (t.from_tool, t.to_tool)).collect();
    assert_eq!(mapped, vec![(0, 1), (1, 2), (2, 3)]);
}

#[test]
fn path_optimization_end_to_end_populates_layer_collection_tool_changes() {
    use slicer_runtime::execute_per_layer;

    let pathopt_fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let dispatcher = pathopt_fx.dispatcher;
    let (pathopt_module, mut wasm_handles) = pathopt_fx.bundle.into_module_and_handles();

    let seed_fx = dispatch_fixture::for_stage("Layer::Perimeters").build();
    let (seed_module, seed_handles) = seed_fx.bundle.into_module_and_handles();
    wasm_handles.extend(seed_handles);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![seed_module],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![pathopt_module],
            },
        ],
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
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    seed_slice_ir(&mut blackboard, &plan);

    struct SeedingRunner {
        inner: WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl LayerStageRunner for SeedingRunner {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            layer: &GlobalLayer,
            module: &CompiledModuleLive<'_>,
            input: LayerStageInput<'_>,
        ) -> Result<slicer_ir::LayerStageCommitData, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(slicer_ir::LayerStageCommitData {
                        perimeter_output: Some(p),
                        ..Default::default()
                    });
                }
            }
            LayerStageRunner::run_stage(&self.inner, stage_id, layer, module, input)
        }
    }

    let runner = SeedingRunner {
        inner: dispatcher,
        perim: Mutex::new(Some(
            ir_builders::perimeter_ir::with_count(2).walls(1).build(),
        )),
    };

    let layers = execute_per_layer(&plan, &blackboard, &runner, &wasm_handles).expect("exec");
    assert_eq!(layers.len(), 1);
    let l = &layers[0];
    assert_eq!(
        l.ordered_entities.len(),
        2,
        "ordered_entities pre-staged from assembly visible at end",
    );
    assert_eq!(
        l.tool_changes.len(),
        2,
        "guest-emitted tool-change overrides folded into LayerCollectionIR",
    );
    for (i, e) in l.ordered_entities.iter().enumerate() {
        assert_eq!(e.region_key.global_layer_index, 0);
        assert_eq!(e.topo_order, i as u32);
    }
    for (i, tc) in l.tool_changes.iter().enumerate() {
        assert_eq!(
            tc.after_entity_index, i as u32,
            "tool-change {i} should anchor at region index {i}"
        );
    }
}

#[test]
fn path_optimization_empty_input_is_no_op() {
    use slicer_runtime::execute_per_layer;

    let fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let dispatcher = fx.dispatcher;
    let (module, wasm_handles) = fx.bundle.into_module_and_handles();

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::PathOptimization".into(),
            modules: vec![module],
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
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    seed_slice_ir(&mut blackboard, &plan);
    let layers = execute_per_layer(&plan, &blackboard, &dispatcher, &wasm_handles).expect("exec");
    assert!(layers[0].ordered_entities.is_empty());
    assert!(layers[0].tool_changes.is_empty());
}

#[test]
fn path_optimization_deterministic_across_repeated_runs() {
    use slicer_runtime::execute_per_layer;

    let pathopt_fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let (pathopt_module, mut wasm_handles) = pathopt_fx.bundle.into_module_and_handles();

    let seed_fx = dispatch_fixture::for_stage("Layer::Perimeters").build();
    let (seed_module, seed_handles) = seed_fx.bundle.into_module_and_handles();
    wasm_handles.extend(seed_handles);

    let make_plan = || ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![seed_module.clone()],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![pathopt_module.clone()],
            },
        ],
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

    struct SeedingRunner {
        inner: WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl LayerStageRunner for SeedingRunner {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            layer: &GlobalLayer,
            module: &CompiledModuleLive<'_>,
            input: LayerStageInput<'_>,
        ) -> Result<slicer_ir::LayerStageCommitData, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(slicer_ir::LayerStageCommitData {
                        perimeter_output: Some(p),
                        ..Default::default()
                    });
                }
            }
            LayerStageRunner::run_stage(&self.inner, stage_id, layer, module, input)
        }
    }

    let engine = wasm_cache::shared_engine();
    let mut results = Vec::new();
    for _ in 0..3 {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        let plan = make_plan();
        seed_slice_ir(&mut blackboard, &plan);
        let runner = SeedingRunner {
            inner: WasmRuntimeDispatcher::new(Arc::clone(&engine)),
            perim: Mutex::new(Some(
                ir_builders::perimeter_ir::with_count(3).walls(1).build(),
            )),
        };
        results.push(execute_per_layer(&plan, &blackboard, &runner, &wasm_handles).unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn path_optimization_rejects_move_override_without_layer_collection_mapping() {
    use slicer_runtime::wit_host::{GcodeCommandCollected, HostExecutionContextBuilder};

    let mut ctx =
        HostExecutionContextBuilder::new("com.test.pathopt-bad".to_string(), 0.0, 0.0).build();
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::FanSpeed(128));
    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let err = commit_hec_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-bad",
        0,
        &ctx,
        &mut fx.arena,
        None,
    )
    .expect_err("fan-speed override must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported GCode command"),
        "diagnostic should describe the rejection: {msg}"
    );
    assert!(
        fx.arena.take_deferred_annotations().is_empty(),
        "rejected command must not enqueue annotations"
    );
}

#[test]
fn path_optimization_commit_routes_comment_and_raw_to_deferred_annotations() {
    use slicer_ir::LayerAnnotationKind;
    use slicer_runtime::wit_host::{GcodeCommandCollected, HostExecutionContextBuilder};

    let mut ctx =
        HostExecutionContextBuilder::new("com.test.pathopt-ann".to_string(), 0.0, 0.0).build();
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::Comment("hello".into()));
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::Raw("M117 hi".into()));

    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    commit_hec_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-ann",
        0,
        &ctx,
        &mut fx.arena,
        None,
    )
    .expect("comment/raw must commit successfully");

    let anns = fx.arena.take_deferred_annotations();
    assert_eq!(anns.len(), 2, "both annotations are committed");
    assert!(matches!(anns[0].kind, LayerAnnotationKind::Comment(ref t) if t == "hello"));
    assert!(matches!(anns[1].kind, LayerAnnotationKind::Raw(ref t) if t == "M117 hi"));
    assert_eq!(anns[0].after_entity_index, 0);
    assert_eq!(anns[1].after_entity_index, 0);
}

#[test]
fn path_optimization_commit_is_deterministic_across_repeats() {
    use slicer_runtime::wit_host::{GcodeCommandCollected, HostExecutionContextBuilder};

    let mk_ctx = || {
        let mut c =
            HostExecutionContextBuilder::new("com.test.pathopt-det2".to_string(), 0.0, 0.0).build();
        c.gcode_output_mut()
            .commands
            .push(GcodeCommandCollected::ToolChange {
                after_entity_index: 0,
                from_tool: 0,
                to_tool: 1,
            });
        c.gcode_output_mut()
            .commands
            .push(GcodeCommandCollected::Comment("a".into()));
        c.gcode_output_mut()
            .commands
            .push(GcodeCommandCollected::Raw("b".into()));
        c
    };

    let mut snapshots = Vec::new();
    for _ in 0..3 {
        let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
        let ctx = mk_ctx();
        commit_hec_for_test(
            "Layer::PathOptimization",
            "com.test.pathopt-det2",
            0,
            &ctx,
            &mut fx.arena,
            None,
        )
        .unwrap();
        snapshots.push((
            fx.arena.take_deferred_tool_changes(),
            fx.arena.take_deferred_annotations(),
        ));
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
}

// ── M. PathOptimization z-hop ─────────────────────────────────────────────────

#[test]
fn path_optimization_commit_routes_z_hops_to_deferred_queue() {
    use slicer_runtime::wit_host::{GcodeCommandCollected, HostExecutionContextBuilder};

    let mut ctx =
        HostExecutionContextBuilder::new("com.test.pathopt-zhop".to_string(), 0.0, 0.0).build();
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::ZHop {
            after_entity_index: 0,
            hop_height: 0.5,
        });
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::ZHop {
            after_entity_index: 0,
            hop_height: 0.75,
        });

    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    commit_hec_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-zhop",
        0,
        &ctx,
        &mut fx.arena,
        None,
    )
    .expect("z-hop must commit");

    let zhops = fx.arena.take_deferred_z_hops();
    assert_eq!(zhops.len(), 2);
    assert_eq!(zhops[0].after_entity_index, 0);
    assert_eq!(zhops[0].hop_height, 0.5);
    assert_eq!(zhops[1].hop_height, 0.75);
}

#[test]
fn path_optimization_z_hop_normalizes_to_global_anchor_with_entities() {
    use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth, RegionKey};
    use slicer_runtime::wit_host::{
        ExtrusionRole as WitRole, GcodeCommandCollected, GcodeMoveCmd, HostExecutionContextBuilder,
    };

    let mut ctx =
        HostExecutionContextBuilder::new("com.test.pathopt-zhop-norm".to_string(), 0.0, 0.0)
            .build();
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::Retract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::ZHop {
            after_entity_index: 999,
            hop_height: 0.2,
        });
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::Move(GcodeMoveCmd {
            x: Some(50.0),
            y: Some(50.0),
            z: None,
            e: None,
            f: None,
            role: WitRole::Custom("travel".to_string()),
        }));
    ctx.gcode_output_mut()
        .commands
        .push(GcodeCommandCollected::Unretract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });

    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let entity = slicer_ir::PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            }],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: String::new(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    };
    fx.arena.set_layer_collection(LayerCollectionIR {
        z: 0.2,
        ordered_entities: vec![entity.clone(), entity],
        ..Default::default()
    });

    commit_hec_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-zhop-norm",
        0,
        &ctx,
        &mut fx.arena,
        None,
    )
    .expect("ZHop with arbitrary entity index must be accepted and normalized to anchor");

    let zhops = fx.arena.take_deferred_z_hops();
    assert_eq!(zhops.len(), 1);
    assert_eq!(
        zhops[0].after_entity_index, 1,
        "ZHop must be anchored at global anchor (entity_count-1=1), got {}",
        zhops[0].after_entity_index
    );

    let retracts = fx.arena.take_deferred_retracts();
    assert_eq!(retracts.len(), 2, "Retract + Unretract = 2");
    assert_eq!(
        retracts[0].after_entity_index, 1,
        "Retract must share anchor with ZHop"
    );
    assert_eq!(
        retracts[1].after_entity_index, 1,
        "Unretract must share anchor with ZHop"
    );

    let travels = fx.arena.take_deferred_travel_moves();
    assert_eq!(travels.len(), 1);
    assert_eq!(
        travels[0].after_entity_index, 1,
        "TravelMove must share anchor with ZHop"
    );
}

#[test]
fn path_optimization_z_hop_rejects_invalid_hop_height() {
    use slicer_runtime::wit_host::{GcodeCommandCollected, HostExecutionContextBuilder};

    for bad in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.pathopt-zhop-bad".to_string(), 0.0, 0.0)
                .build();
        ctx.gcode_output_mut()
            .commands
            .push(GcodeCommandCollected::ZHop {
                after_entity_index: 0,
                hop_height: bad,
            });
        let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
        let err = commit_hec_for_test(
            "Layer::PathOptimization",
            "com.test.pathopt-zhop-bad",
            0,
            &ctx,
            &mut fx.arena,
            None,
        )
        .expect_err("bad hop_height must fail");
        assert!(
            err.to_string().contains("hop-height"),
            "diagnostic should name field for {bad}: {err}"
        );
    }
}

#[test]
fn path_optimization_end_to_end_populates_z_hops() {
    use slicer_runtime::execute_per_layer;

    let pathopt_fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let (pathopt_module, mut wasm_handles) = pathopt_fx.bundle.into_module_and_handles();

    let seed_fx = dispatch_fixture::for_stage("Layer::Perimeters").build();
    let (seed_module, seed_handles) = seed_fx.bundle.into_module_and_handles();
    wasm_handles.extend(seed_handles);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![seed_module],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![pathopt_module],
            },
        ],
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

    struct SeedingRunner {
        inner: WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl LayerStageRunner for SeedingRunner {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            layer: &GlobalLayer,
            module: &CompiledModuleLive<'_>,
            input: LayerStageInput<'_>,
        ) -> Result<slicer_ir::LayerStageCommitData, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(slicer_ir::LayerStageCommitData {
                        perimeter_output: Some(p),
                        ..Default::default()
                    });
                }
            }
            LayerStageRunner::run_stage(&self.inner, stage_id, layer, module, input)
        }
    }

    let mut runs = Vec::new();
    for _ in 0..2 {
        let runner = SeedingRunner {
            inner: WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine())),
            perim: Mutex::new(Some(
                ir_builders::perimeter_ir::with_count(2).walls(1).build(),
            )),
        };
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        seed_slice_ir(&mut blackboard, &plan);
        runs.push(execute_per_layer(&plan, &blackboard, &runner, &wasm_handles).expect("exec"));
    }
    let layers = &runs[0];
    assert_eq!(layers.len(), 1);
    let l = &layers[0];
    assert_eq!(l.ordered_entities.len(), 2);
    assert_eq!(l.z_hops.len(), 2, "guest emits one z-hop per region");
    for zh in &l.z_hops {
        assert_eq!(zh.after_entity_index, 1);
        assert_eq!(zh.hop_height, 0.5);
    }
    assert_eq!(l.tool_changes.len(), 2);
    assert_eq!(l.annotations.len(), 1);
    assert_eq!(runs[0], runs[1]);
}

#[test]
fn path_optimization_end_to_end_emitter_renders_z_hops() {
    use slicer_runtime::execute_per_layer;
    use slicer_runtime::DefaultGCodeEmitter;

    let pathopt_fx = dispatch_fixture::for_stage("Layer::PathOptimization").build();
    let dispatcher = pathopt_fx.dispatcher;
    let (pathopt_module, mut wasm_handles) = pathopt_fx.bundle.into_module_and_handles();

    let seed_fx = dispatch_fixture::for_stage("Layer::Perimeters").build();
    let (seed_module, seed_handles) = seed_fx.bundle.into_module_and_handles();
    wasm_handles.extend(seed_handles);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![seed_module],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![pathopt_module],
            },
        ],
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

    struct SeedingRunner {
        inner: WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl LayerStageRunner for SeedingRunner {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            layer: &GlobalLayer,
            module: &CompiledModuleLive<'_>,
            input: LayerStageInput<'_>,
        ) -> Result<slicer_ir::LayerStageCommitData, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(slicer_ir::LayerStageCommitData {
                        perimeter_output: Some(p),
                        ..Default::default()
                    });
                }
            }
            LayerStageRunner::run_stage(&self.inner, stage_id, layer, module, input)
        }
    }

    let runner = SeedingRunner {
        inner: dispatcher,
        perim: Mutex::new(Some(
            ir_builders::perimeter_ir::with_count(1).walls(1).build(),
        )),
    };
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    seed_slice_ir(&mut blackboard, &plan);
    let layers = execute_per_layer(&plan, &blackboard, &runner, &wasm_handles).expect("exec");

    let emitter = DefaultGCodeEmitter::new("test".into());
    let gcode = emitter.emit_gcode(&layers).expect("emit");
    let mut hop_lifts = 0;
    for c in &gcode.commands {
        if let slicer_ir::GCodeCommand::Move { z: Some(z), .. } = c {
            if (*z - 0.7).abs() < 1e-4 {
                hop_lifts += 1;
            }
        }
    }
    assert!(
        hop_lifts >= 1,
        "default emitter must lift to layer.z + hop_height for committed z_hops"
    );
}
