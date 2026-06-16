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
