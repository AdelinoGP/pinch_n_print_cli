use crate::common::*;
use slicer_ir::{
    ConfigValue, ConfigView, MeshIR, PrepassRunnerError, SemVer, SurfaceClassificationIR,
};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledStage, ExecutionPlan, PrepassStageRunner,
};
use slicer_wasm_host::WasmRuntimeDispatcher;
use std::collections::HashMap;
use std::sync::Arc;

// Helper to load specific guests
fn load_prepass_guest() -> Arc<slicer_runtime::WasmComponent> {
    wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/prepass-guest.component.wasm"
    )))
}

fn load_layer_planner_default() -> Option<Arc<slicer_runtime::WasmComponent>> {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../modules/core-modules/layer-planner-default/layer-planner-default.wasm"
    ));
    if !path.exists() {
        return None;
    }
    Some(wasm_cache::compiled_component_at(path))
}

fn load_sdk_prepass_guest() -> Option<Arc<slicer_runtime::WasmComponent>> {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/sdk-prepass-guest.component.wasm"
    ));
    if !path.exists() {
        return None;
    }
    Some(wasm_cache::compiled_component_at(path))
}

fn mesh_analysis_emit_config(n: i64) -> ConfigView {
    let mut m = HashMap::new();
    m.insert("emit_mesh_analysis".to_string(), ConfigValue::Int(n));
    ConfigView::from_map(m)
}

fn blackboard_with_objects(object_ids: &[&str]) -> Blackboard {
    let objects: Vec<slicer_ir::ObjectMesh> = object_ids
        .iter()
        .map(|id| slicer_ir::ObjectMesh {
            id: id.to_string(),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![
                    slicer_ir::Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    slicer_ir::Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    slicer_ir::Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: slicer_ir::Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            ..Default::default()
        })
        .collect();
    let mesh = Arc::new(MeshIR {
        objects,
        build_volume: slicer_ir::BoundingBox3 {
            min: slicer_ir::Point3::default(),
            max: slicer_ir::Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
        ..Default::default()
    });
    Blackboard::new(mesh, 0)
}

#[test]
fn layer_planning_dispatch_returns_layer_plan_variant() {
    use slicer_runtime::PrepassStageOutput;

    let mut fixture = dispatch_fixture::for_stage("PrePass::LayerPlanning").build();
    let component = load_prepass_guest();
    fixture.bundle.component = Some(component);

    let result = fixture.run_prepass();
    assert!(result.is_ok());

    match result.unwrap() {
        PrepassStageOutput::LayerPlan(ir) => {
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0
                }
            );
            assert!(ir.object_participation.is_empty());
        }
        other => panic!(
            "expected LayerPlan, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

#[test]
fn layer_plan_committed_to_blackboard_after_execute_prepass() {
    use slicer_runtime::execute_prepass;

    let component = load_prepass_guest();
    let module = CompiledModuleBuilder::new("com.test.lp-commit").build();
    let (module, wasm_handles) = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    }
    .into_module_and_handles();

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![module],
        }],
        ..Default::default()
    };

    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .unwrap();

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let result = execute_prepass(&plan, &mut blackboard, &dispatcher, &wasm_handles);

    assert!(result.is_ok());
    assert!(blackboard.layer_plan().is_some());
    assert_eq!(
        blackboard.layer_plan().unwrap().schema_version,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
}

#[test]
fn layer_plan_harvest_deterministic_across_repeated_calls() {
    use slicer_runtime::PrepassStageOutput;

    let component = load_prepass_guest();
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));

    let run_once = || {
        let module = CompiledModuleBuilder::new("com.test.lp-det").build();
        let bundle = TestModuleBundle {
            module,
            pool: slicer_wasm_host::WasmInstancePool::placeholder(),
            component: Some(Arc::clone(&component)),
        };
        let live = bundle.as_live();
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::LayerPlanning".to_string(),
            &live,
            prepass_input(&blackboard),
        ) {
            Ok(PrepassStageOutput::LayerPlan(ir)) => ir,
            _ => panic!("dispatch failed"),
        }
    };

    let ir_a = run_once();
    let ir_b = run_once();
    assert_eq!(*ir_a, *ir_b);
}

#[test]
fn layer_planning_module_error_propagates_as_fatal_prepass_error() {
    let component = match load_layer_planner_default() {
        Some(c) => c,
        None => return,
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));

    let mut m = HashMap::new();
    m.insert("layer_height".to_string(), ConfigValue::Float(-1.0));
    let bad_config = ConfigView::from_map(m);

    let module = CompiledModuleBuilder::new("com.core.layer-planner-default")
        .config_view(Arc::new(bad_config))
        .build();
    let bundle = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    };

    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::LayerPlanning".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PrepassRunnerError::FatalModule { .. }
    ));
}

#[test]
fn mesh_analysis_macro_path_forwards_objects_and_drains_output() {
    use slicer_runtime::PrepassStageOutput;

    let component = match load_sdk_prepass_guest() {
        Some(c) => c,
        None => return,
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let module = CompiledModuleBuilder::new("com.test.sdk-prepass-emit")
        .config_view(Arc::new(mesh_analysis_emit_config(3)))
        .build();
    let bundle = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    };

    let blackboard = blackboard_with_objects(&["obj-A", "obj-B"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    );

    let aux = match result {
        Ok(PrepassStageOutput::MeshAnalysisAuxiliary(a)) => a,
        _ => panic!("dispatch failed"),
    };

    assert_eq!(aux.facet_annotations.len(), 6);
    let obj_ids: Vec<&str> = aux
        .facet_annotations
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        obj_ids,
        vec!["obj-A", "obj-A", "obj-A", "obj-B", "obj-B", "obj-B"]
    );
}

#[test]
fn mesh_analysis_macro_path_drain_is_deterministic() {
    use slicer_runtime::PrepassStageOutput;

    let component = match load_sdk_prepass_guest() {
        Some(c) => c,
        None => return,
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));

    let run_once = || {
        let module = CompiledModuleBuilder::new("com.test.sdk-prepass-det-emit")
            .config_view(Arc::new(mesh_analysis_emit_config(2)))
            .build();
        let bundle = TestModuleBundle {
            module,
            pool: slicer_wasm_host::WasmInstancePool::placeholder(),
            component: Some(Arc::clone(&component)),
        };
        let blackboard = blackboard_with_objects(&["obj-1", "obj-2"]);
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::MeshAnalysis".to_string(),
            &bundle.as_live(),
            prepass_input(&blackboard),
        ) {
            Ok(PrepassStageOutput::MeshAnalysisAuxiliary(a)) => a,
            _ => panic!("dispatch failed"),
        }
    };
    let a = run_once();
    let b = run_once();
    assert_eq!(*a, *b);
}

#[test]
fn mesh_analysis_macro_path_empty_drain_returns_none() {
    use slicer_runtime::PrepassStageOutput;

    let component = match load_sdk_prepass_guest() {
        Some(c) => c,
        None => return,
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let module = CompiledModuleBuilder::new("com.test.sdk-prepass-empty")
        .config_view(Arc::new(ConfigView::from_map(HashMap::new())))
        .build();
    let bundle = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    };

    let blackboard = blackboard_with_objects(&["obj-1"]);
    let out = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    )
    .expect("must succeed");
    assert!(matches!(out, PrepassStageOutput::None));
}

#[test]
fn mesh_analysis_output_push_validates_and_rejects_malformed() {
    use slicer_runtime::wit_host::prepass as pm;
    use slicer_runtime::wit_host::HostExecutionContextBuilder;
    use wasmtime::component::Resource;

    let mut ctx =
        HostExecutionContextBuilder::new("com.test.validator".to_string(), 0.0, 0.0).build();
    let handle = ctx
        .push_mesh_analysis_output()
        .expect("push mesh-analysis-output resource");

    let res = <slicer_runtime::wit_host::HostExecutionContext as pm::HostMeshAnalysisOutput>::push_facet_annotation(
        &mut ctx,
        Resource::new_own(handle.rep()),
        String::new(),
        pm::FacetAnnotation { facet_index: 0, slope_angle_deg: 30.0, classification: pm::FacetClass::Normal },
    ).expect("host call must not fail");
    assert!(res.is_err());
}

#[test]
fn prepass_seam_planning_requires_layer_plan_slot() {
    use slicer_runtime::prepass::ensure_stage_prerequisites;

    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let result = ensure_stage_prerequisites(&"PrePass::SeamPlanning".to_string(), &blackboard);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        slicer_runtime::prepass::PrepassExecutionError::MissingRequiredPrepass { .. }
    ));
}

// ── Restored from dispatch_tdd.rs (P-restore) ─────────────────────────────────

fn layer_planner_config(
    layer_height: f64,
    first_layer_height: f64,
    object_heights: &[(&str, f64)],
) -> ConfigView {
    let mut m = HashMap::new();
    m.insert("layer_height".to_string(), ConfigValue::Float(layer_height));
    m.insert(
        "first_layer_height".to_string(),
        ConfigValue::Float(first_layer_height),
    );
    for (id, h) in object_heights {
        m.insert(format!("object_height:{}", id), ConfigValue::Float(*h));
    }
    ConfigView::from_map(m)
}

/// The rebuilt layer-planner-default.wasm (built from the macro path —
/// see `wit-guest/src/lib.rs` reduced to a `pub use` shim) must emit the
/// SDK planner's real proposal sequence via the macro-authored drain
/// bridge. A 2mm object at 0.2mm layer height must harvest as 10 global
/// layers with strictly ascending Z.
#[test]
fn layer_planner_default_macro_path_emits_real_proposals() {
    use slicer_runtime::PrepassStageOutput;

    let component = match load_layer_planner_default() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: layer-planner-default.wasm not found — rebuild core modules");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let config = layer_planner_config(0.2, 0.2, &[("obj-1", 2.0)]);
    let module = CompiledModuleBuilder::new("com.core.layer-planner-default")
        .config_view(Arc::new(config))
        .build();
    let bundle = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    };
    let blackboard = blackboard_with_objects(&["obj-1"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::LayerPlanning".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    );

    let ir = match result {
        Ok(PrepassStageOutput::LayerPlan(ir)) => ir,
        Ok(other) => panic!(
            "expected PrepassStageOutput::LayerPlan, got {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("dispatch failed: {e}"),
    };

    // 2mm object / 0.2mm layer height = 10 layers.
    assert_eq!(
        ir.global_layers.len(),
        10,
        "macro-path drain must deliver all SDK proposals to the host harvest \
         (expected 10, got {})",
        ir.global_layers.len()
    );

    // Strictly ascending Z, first layer at first_layer_height.
    assert!(
        (ir.global_layers[0].z - 0.2).abs() < 1e-4,
        "first harvested layer z must equal first_layer_height=0.2, got {}",
        ir.global_layers[0].z
    );
    for i in 1..ir.global_layers.len() {
        assert!(
            ir.global_layers[i].z > ir.global_layers[i - 1].z,
            "harvested proposals must preserve SDK push order (ascending Z) — \
             layer {} z={} vs layer {} z={}",
            i - 1,
            ir.global_layers[i - 1].z,
            i,
            ir.global_layers[i].z
        );
    }

    // object_participation must reach downstream scheduling: the planner
    // emitted one region per layer for obj-1.
    let participation = ir
        .object_participation
        .get("obj-1")
        .expect("object_participation must carry obj-1 after drain");
    assert_eq!(
        participation.len(),
        ir.global_layers.len(),
        "obj-1 must participate in every layer it fits in"
    );
}

/// Two independent dispatch calls through the rebuilt
/// layer-planner-default.wasm must produce byte-identical `LayerPlanIR`.
/// The macro-authored drain has no hidden state (no timestamps, no
/// pointer-derived ordering), so determinism holds end-to-end.
#[test]
fn layer_planner_default_macro_path_is_deterministic() {
    use slicer_runtime::PrepassStageOutput;

    let component = match load_layer_planner_default() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: layer-planner-default.wasm not found — rebuild core modules");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));

    let run_once = || {
        let config = layer_planner_config(0.2, 0.2, &[("obj-1", 2.0)]);
        let module = CompiledModuleBuilder::new("com.core.layer-planner-default")
            .config_view(Arc::new(config))
            .build();
        let bundle = TestModuleBundle {
            module,
            pool: slicer_wasm_host::WasmInstancePool::placeholder(),
            component: Some(Arc::clone(&component)),
        };
        let blackboard = blackboard_with_objects(&["obj-1"]);
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::LayerPlanning".to_string(),
            &bundle.as_live(),
            prepass_input(&blackboard),
        ) {
            Ok(PrepassStageOutput::LayerPlan(ir)) => ir,
            Ok(other) => panic!(
                "expected LayerPlan, got {:?}",
                std::mem::discriminant(&other)
            ),
            Err(e) => panic!("dispatch failed: {e}"),
        }
    };

    let a = run_once();
    let b = run_once();
    assert_eq!(
        *a, *b,
        "macro-path layer-planner-default must be deterministic \
         across repeated dispatches"
    );
}

#[test]
fn seam_plan_ir_rejects_duplicate_region_keys() {
    // SeamPlanIR must reject commits that contain duplicate region keys
    // (same global_layer_index + object_id + region_id triple).
    // The validation happens at commit time in the blackboard.
    use slicer_ir::{RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition};
    use slicer_runtime::{Blackboard, BlackboardError, BlackboardPrepassSlot};

    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);

    // Build a minimal valid SeamPosition for the chosen_candidate field.
    let dummy_position = slicer_ir::Point3WithWidth {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    };
    let seam_position = SeamPosition {
        point: dummy_position,
        wall_index: 0,
    };

    // First commit with valid unique entries.
    let seam_plan = SeamPlanIR {
        entries: vec![
            SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "obj-A".to_string(),
                    region_id: 1,
                    variant_chain: Vec::new(),
                },
                chosen_candidate: seam_position.clone(),
                ..Default::default()
            },
            SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "obj-B".to_string(),
                    region_id: 2,
                    variant_chain: Vec::new(),
                },
                chosen_candidate: seam_position.clone(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    // Commit once — should succeed.
    let result = blackboard.commit_seam_plan(std::sync::Arc::new(seam_plan));
    assert!(
        result.is_ok(),
        "first commit with unique keys should succeed"
    );

    // Second commit — same region key (global_layer_index=0, obj-A, region_id=1)
    // is a duplicate and must be rejected.
    let duplicate_seam_plan = SeamPlanIR {
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "obj-A".to_string(),
                region_id: 1, // duplicate of above
                variant_chain: Vec::new(),
            },
            chosen_candidate: seam_position,
            ..Default::default()
        }],
        ..Default::default()
    };
    let result2 = blackboard.commit_seam_plan(std::sync::Arc::new(duplicate_seam_plan));
    assert!(
        result2.is_err(),
        "commit with duplicate region key must be rejected"
    );
    let err = result2.unwrap_err();
    match err {
        BlackboardError::DuplicatePrepassCommit { slot } => {
            assert_eq!(slot, BlackboardPrepassSlot::SeamPlan);
        }
        other => panic!("expected DuplicatePrepassCommit for SeamPlan slot, got {other:?}"),
    }
}

#[test]
fn seam_plan_ir_rejects_duplicate_region_keys_within_one_ir() {
    use slicer_ir::{RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition};

    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let seam_position = SeamPosition {
        point: slicer_ir::Point3WithWidth {
            width: 0.4,
            flow_factor: 1.0,
            ..Default::default()
        },
        wall_index: 0,
    };
    let region_key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: 1,
        variant_chain: vec![("material".to_string(), slicer_ir::PaintValue::ToolIndex(1))],
    };
    let plan = SeamPlanIR {
        entries: vec![
            SeamPlanEntry {
                region_key: region_key.clone(),
                chosen_candidate: seam_position.clone(),
                ..Default::default()
            },
            SeamPlanEntry {
                region_key,
                chosen_candidate: seam_position,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let error = blackboard
        .commit_seam_plan(Arc::new(plan))
        .expect_err("one IR with duplicate region keys must be rejected");
    assert!(
        format!("{error:?}")
            .to_ascii_lowercase()
            .contains("duplicate"),
        "expected duplicate-key validation error, got {error:?}"
    );
}

#[test]
fn seam_plan_ir_preserves_variant_chain() {
    let make_entry =
        |region_id: &str, variant_chain| slicer_wasm_host::host::prepass::SeamPlanEntry {
            global_layer_index: 0,
            object_id: "obj-A".to_string(),
            region_id: region_id.to_string(),
            variant_chain,
            chosen_position: slicer_wasm_host::host::prepass::SeamPoint3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            chosen_wall_index: 0,
            scored_candidates: Vec::new(),
        };
    let entries = vec![
        make_entry("1", Vec::new()),
        make_entry(
            "1",
            vec![(
                "material".to_string(),
                slicer_wasm_host::host::prepass::PaintValue::ToolIndex(1),
            )],
        ),
    ];

    let plan = slicer_wasm_host::marshal::in_::harvest_seam_plan_ir_from(entries)
        .expect("valid seam-plan entries should harvest");

    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].region_key.variant_chain, Vec::new());
    assert_eq!(plan.entries[0].region_key.region_id, 1);
    assert_eq!(
        plan.entries[1].region_key.variant_chain,
        vec![("material".to_string(), slicer_ir::PaintValue::ToolIndex(1))]
    );
    assert_eq!(plan.entries[1].region_key.region_id, 1);
}

#[test]
fn seam_plan_injection_matches_variant_chain() {
    let position = |x| slicer_ir::SeamPosition {
        point: slicer_ir::Point3WithWidth {
            x,
            ..Default::default()
        },
        ..Default::default()
    };
    let base_key = |variant_chain| slicer_ir::RegionKey {
        global_layer_index: 3,
        object_id: "obj-A".to_string(),
        region_id: 7,
        variant_chain,
    };
    let plan = slicer_ir::SeamPlanIR {
        entries: vec![
            slicer_ir::SeamPlanEntry {
                region_key: base_key(Vec::new()),
                chosen_candidate: position(10.0),
                ..Default::default()
            },
            slicer_ir::SeamPlanEntry {
                region_key: base_key(vec![(
                    "material".to_string(),
                    slicer_ir::PaintValue::ToolIndex(1),
                )]),
                chosen_candidate: position(20.0),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let regions = [
        slicer_ir::PerimeterRegion {
            object_id: "obj-A".to_string(),
            region_id: 7,
            variant_chain: Vec::new(),
            ..Default::default()
        },
        slicer_ir::PerimeterRegion {
            object_id: "obj-A".to_string(),
            region_id: 7,
            variant_chain: vec![("material".to_string(), slicer_ir::PaintValue::ToolIndex(1))],
            ..Default::default()
        },
    ];

    let resolved_x: Vec<_> = regions
        .iter()
        .map(|region| {
            slicer_wasm_host::dispatch::resolve_seam_for_perimeter_region(region, &plan, 3)
                .map(|seam| seam.point.x)
        })
        .collect();

    assert_eq!(resolved_x, vec![Some(10.0), Some(20.0)]);
}

#[test]
fn seam_plan_ir_rejects_invalid_region_identity() {
    let entry = slicer_wasm_host::host::prepass::SeamPlanEntry {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: "not-a-region-id".to_string(),
        variant_chain: Vec::new(),
        chosen_position: slicer_wasm_host::host::prepass::SeamPoint3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        chosen_wall_index: 0,
        scored_candidates: Vec::new(),
    };

    let error = slicer_wasm_host::marshal::in_::harvest_seam_plan_ir_from(vec![entry])
        .expect_err("invalid region identity must reject");
    assert!(
        error.contains("invalid identity"),
        "unexpected error: {error}"
    );
}

/// `PrePass::SeamPlanning` dispatch with the real seam-planner-default module
/// must return `PrepassStageOutput::SeamPlan`. The module is an MVP no-op (emits
/// no entries) but the harvest path must still produce a well-formed `SeamPlanIR`.
///
/// This is the Step 5 exit-condition test for AC-1.
#[test]
fn prepass_seam_planning_commits_seam_plan_ir() {
    use slicer_ir::{LayerPlanIR, SemVer};
    use slicer_runtime::PrepassStageOutput;

    let component = {
        const PATH: &str = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../modules/core-modules/seam-planner-default/seam-planner-default.wasm"
        );
        let path = std::path::Path::new(PATH);
        if !path.exists() {
            eprintln!("SKIP: seam-planner-default.wasm missing — rebuild core modules");
            return;
        }
        wasm_cache::compiled_component_at(path)
    };

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));

    // Build a loaded + compiled module for SeamPlanning.
    let module = CompiledModuleBuilder::new("com.test.seam-planner").build();
    let bundle = TestModuleBundle {
        module,
        pool: slicer_wasm_host::WasmInstancePool::placeholder(),
        component: Some(component),
    };

    // Build a blackboard with a committed LayerPlanIR (SeamPlanning's required slot).
    // The seam-planner-default module may or may not produce entries depending
    // on geometry (it skips empty meshes), so we assert the result is non-empty
    // only when geometry is present — AC-2 is verified via the live seam path test.
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR::default()))
        .expect("commit minimal layer plan for required slot");

    // Also commit SurfaceClassificationIR (MeshAnalysis output that other stages need).
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("commit surface classification for required slot");

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::SeamPlanning".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    );

    match result {
        Ok(PrepassStageOutput::SeamPlan(ir)) => {
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 1,
                    patch: 0
                },
                "SeamPlanIR schema_version must be 1.1.0"
            );
            // seam-planner-default emits seam entries for objects with mesh geometry.
            // Entries may be empty if the blackboard mesh has no objects.
            for entry in &ir.entries {
                assert!(
                    !entry.region_key.object_id.is_empty(),
                    "region_key.object_id must be non-empty"
                );
                assert!(
                    entry.region_key.global_layer_index < 1000,
                    "global_layer_index must be reasonable"
                );
                // Verify the chosen position is valid.
                assert!(entry.chosen_candidate.point.x.is_finite());
                assert!(entry.chosen_candidate.point.y.is_finite());
                assert!(entry.chosen_candidate.point.z.is_finite());
                assert!(entry.chosen_candidate.point.width > 0.0);
                eprintln!(
                    "DEBUG: seam_candidate point=({:.4}, {:.4}, {:.4}) wall_index={} width={:.4}",
                    entry.chosen_candidate.point.x,
                    entry.chosen_candidate.point.y,
                    entry.chosen_candidate.point.z,
                    entry.chosen_candidate.wall_index,
                    entry.chosen_candidate.point.width
                );
            }
            eprintln!(
                "DEBUG: prepass_seam_planning_commits_seam_plan_ir — SeamPlanIR entry count = {}",
                ir.entries.len()
            );
        }
        Ok(other) => panic!(
            "expected PrepassStageOutput::SeamPlan, got {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("SeamPlanning dispatch failed: {e}"),
    }
}

#[test]
fn prepass_seam_planning_commits_populated_seam_plan_ir_from_slice_ir() {
    let slice_ir = ir_builders::slice_ir::with_ids(&[("obj-A", 1), ("obj-A", 1)]).build();
    let mut region_map = slicer_ir::RegionMapIR::default();
    region_map.entries.insert(
        slicer_ir::RegionKey {
            global_layer_index: 0,
            object_id: "obj-A".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
        slicer_ir::RegionPlan::default(),
    );
    region_map.entries.insert(
        slicer_ir::RegionKey {
            global_layer_index: 0,
            object_id: "obj-A".to_string(),
            region_id: 1,
            variant_chain: vec![("material".to_string(), slicer_ir::PaintValue::ToolIndex(1))],
        },
        slicer_ir::RegionPlan::default(),
    );
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    blackboard
        .commit_slice_ir(Arc::new(vec![slice_ir]))
        .expect("commit representative SliceIR");
    blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("commit representative RegionMapIR");

    let source_keys: Vec<_> = blackboard
        .region_map()
        .expect("committed RegionMapIR")
        .entries
        .keys()
        .filter(|key| key.object_id == "obj-A" && key.region_id == 1)
        .cloned()
        .collect();
    assert_eq!(source_keys.len(), 2);

    let to_wit_variant_chain = |key: &slicer_ir::RegionKey| {
        key.variant_chain
            .iter()
            .map(|(semantic, value)| {
                let value = match value {
                    slicer_ir::PaintValue::Flag(value) => {
                        slicer_wasm_host::host::prepass::PaintValue::Flag(*value)
                    }
                    slicer_ir::PaintValue::Scalar(value) => {
                        slicer_wasm_host::host::prepass::PaintValue::Scalar(*value)
                    }
                    slicer_ir::PaintValue::ToolIndex(value) => {
                        slicer_wasm_host::host::prepass::PaintValue::ToolIndex(*value)
                    }
                    slicer_ir::PaintValue::Custom(_) => panic!("unsupported fixture value"),
                };
                (semantic.clone(), value)
            })
            .collect()
    };
    let seam_entries = source_keys
        .iter()
        .enumerate()
        .map(
            |(index, key)| slicer_wasm_host::host::prepass::SeamPlanEntry {
                global_layer_index: key.global_layer_index,
                object_id: key.object_id.clone(),
                region_id: key.region_id.to_string(),
                variant_chain: to_wit_variant_chain(key),
                chosen_position: slicer_wasm_host::host::prepass::SeamPoint3WithWidth {
                    x: index as f32,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                chosen_wall_index: 0,
                scored_candidates: Vec::new(),
            },
        )
        .collect();

    // The full guest dispatch is covered by the adjacent AC-1 test. Here the
    // direct harvest seam uses variant chains read from the committed map.
    let harvested = slicer_wasm_host::marshal::in_::harvest_seam_plan_ir_from(seam_entries)
        .expect("representative SliceIR/RegionMapIR output should harvest");
    assert_eq!(harvested.entries.len(), 2);
    for source_key in source_keys {
        assert!(harvested
            .entries
            .iter()
            .any(|entry| entry.region_key == source_key));
    }
}
