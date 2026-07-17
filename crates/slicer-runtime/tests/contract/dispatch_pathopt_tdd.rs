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
        ) -> Result<Option<slicer_ir::LayerStageCommit>, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(Some(slicer_ir::LayerStageCommit::Perimeters(p)));
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
        ) -> Result<Option<slicer_ir::LayerStageCommit>, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(Some(slicer_ir::LayerStageCommit::Perimeters(p)));
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
        tool_index: 0,
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
        ) -> Result<Option<slicer_ir::LayerStageCommit>, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(Some(slicer_ir::LayerStageCommit::Perimeters(p)));
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
        ) -> Result<Option<slicer_ir::LayerStageCommit>, LayerStageError> {
            if stage_id == "Layer::Perimeters" {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    return Ok(Some(slicer_ir::LayerStageCommit::Perimeters(p)));
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

// ---------------------------------------------------------------------------
// Restored regression tests (originally in dispatch_tdd.rs):
//   - path_optimization_receives_real_perimeter_regions
//   - path_optimization_dispatch_emits_per_layer_marker
//   - path_optimization_dispatch_is_deterministic
// ---------------------------------------------------------------------------

const PATH_OPT_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/path-optimization-default/path-optimization-default.wasm"
);

/// Ported verbatim from the original `dispatch_tdd.rs`: load the canonical
/// `path-optimization-default.wasm` component, returning `None` (so the
/// caller skips) when the artifact is missing.
fn load_path_optimization_default(
    _engine: &slicer_runtime::WasmEngine,
) -> Option<Arc<slicer_runtime::WasmComponent>> {
    let path = std::path::Path::new(PATH_OPT_DEFAULT_PATH);
    if !path.exists() {
        return None;
    }
    Some(wasm_cache::compiled_component_at(path))
}

/// Build a `TestModuleBundle` wrapping a caller-supplied component for the
/// given module id/stage. Replicates the construction the original legacy
/// compiled-module test helper performed, using only public
/// `slicer_runtime`/`slicer_wasm_host` APIs (no forbidden legacy helper).
fn bundle_with_component(
    id: &str,
    stage: &str,
    component: Arc<slicer_runtime::WasmComponent>,
) -> TestModuleBundle {
    use slicer_ir::SemVer;
    use slicer_runtime::manifest::LoadedModuleBuilder;
    use slicer_runtime::{build_wasm_instance_pool, CompiledModuleBuilder, WasmArtifactMetadata};

    let loaded = LoadedModuleBuilder::new(
        id,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage,
        slicer_schema::WORLD_LAYER,
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
    let module = CompiledModuleBuilder::new(id)
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

#[test]
fn path_optimization_receives_real_perimeter_regions() {
    // PathOptimization does not commit to an arena slot; it should still
    // consume perimeter-region data (this test proves no panic / error path
    // and is verified by the dispatch succeeding when perimeter IR is staged).
    let mut fx = dispatch_fixture::for_stage("Layer::PathOptimization")
        .with_perimeter(
            ir_builders::perimeter_ir::with_count(4)
                .at_layer(0)
                .walls(2)
                .infill(0)
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

    let r = fx.run_layer(&layer);
    assert!(
        r.is_ok(),
        "path-optimization with real perimeter regions should succeed: {:?}",
        r.err()
    );
}

/// End-to-end guard: the canonical `Layer::PathOptimization` module
/// runs on the real per-layer path against a real Benchy-equivalent
/// set-up — the arena already carries a committed `PerimeterIR` (via
/// the `Layer::Perimeters` stage) and a pre-staged `LayerCollectionIR`
/// with `ordered_entities`. The guest's `push_comment` output
/// survives through to `LayerCollectionIR.annotations`, which the
/// default G-code emitter renders as a `; path-optimization layer X
/// regions=Y entities=Z` line (see benchy_end_to_end_tdd.rs for the
/// observed 239-marker count on the real Benchy run).
#[test]
fn path_optimization_dispatch_emits_per_layer_marker() {
    use slicer_runtime::{Blackboard, LayerArena};

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_path_optimization_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: path-optimization-default.wasm missing");
            return;
        }
    };
    let module = bundle_with_component(
        "com.test.path-opt-dispatch",
        "Layer::PathOptimization",
        component,
    );

    // Pre-seed the arena with a perimeter commit so the guest sees a
    // non-empty region list (region_count=1, entity_count=1 on the
    // guest side). A PerimeterRegion with one wall loop.
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let mut arena = LayerArena::new();
    let wall = slicer_ir::WallLoop {
        perimeter_index: 0,
        loop_type: slicer_ir::LoopType::Outer,
        path: slicer_ir::ExtrusionPath3D {
            points: vec![
                slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                slicer_ir::Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 1.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile {
            widths: vec![0.4; 3],
        },
        feature_flags: vec![
            slicer_ir::WallFeatureFlags {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: HashMap::new(),
            };
            3
        ],
        boundary_type: slicer_ir::WallBoundaryType::ExteriorSurface,
    };
    let perim = slicer_ir::PerimeterIR {
        global_layer_index: 7,
        regions: vec![slicer_ir::PerimeterRegion {
            object_id: "obj".into(),
            region_id: 0,
            walls: vec![wall],
            seam_candidates: Vec::new(),
            infill_areas: Vec::new(),
            resolved_seam: None,
        }],
        ..Default::default()
    };
    arena.set_perimeter(perim).expect("seed perimeter");

    let layer = slicer_ir::GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::PathOptimization",
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    // Dispatch already ran commit_layer_outputs; the comment
    // is now in the arena as a deferred annotation. Verify it.
    let annotations = arena.take_deferred_annotations();
    assert_eq!(
        annotations.len(),
        1,
        "exactly one path-optimization marker expected, got {}",
        annotations.len()
    );
    match &annotations[0].kind {
        slicer_ir::LayerAnnotationKind::Comment(text) => {
            assert!(
                text.contains("path-optimization layer 7"),
                "expected 'path-optimization layer 7' in annotation text, got: {text}"
            );
            assert!(
                text.contains("regions=1"),
                "expected 'regions=1' in annotation text, got: {text}"
            );
            assert!(
                text.contains("entities=1"),
                "expected 'entities=1' (one wall loop) in annotation text, got: {text}"
            );
        }
        other => panic!("expected Comment annotation, got {other:?}"),
    }
}

/// Two back-to-back dispatches with the same arena seed produce
/// byte-identical annotation output.
#[test]
fn path_optimization_dispatch_is_deterministic() {
    use slicer_runtime::{Blackboard, LayerArena};

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_path_optimization_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: path-optimization-default.wasm missing");
            return;
        }
    };
    let layer = slicer_ir::GlobalLayer {
        index: 3,
        z: 0.6,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);

    let run_once = || -> Vec<slicer_ir::LayerAnnotation> {
        let module = bundle_with_component(
            "com.test.path-opt-det",
            "Layer::PathOptimization",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        crate::common::run_layer_and_commit_with_bundle(
            &dispatcher,
            "Layer::PathOptimization",
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        arena.take_deferred_annotations()
    };
    let a = run_once();
    let b = run_once();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.after_entity_index, y.after_entity_index);
        assert_eq!(format!("{:?}", x.kind), format!("{:?}", y.kind));
    }
}
