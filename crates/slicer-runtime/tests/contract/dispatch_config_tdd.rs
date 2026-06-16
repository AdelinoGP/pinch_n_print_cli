use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use slicer_ir::{ConfigValue, ConfigView, GlobalLayer, MeshIR};
use slicer_runtime::{Blackboard, LayerArena};
use slicer_wasm_host::WasmRuntimeDispatcher;
use witness::RawInfillWitnessPoint1;

use crate::common::dispatch_fixture;
use crate::common::ir_builders;
use crate::common::wasm_cache;

const PATH_OPT_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/path-optimization-default/path-optimization-default.wasm"
);

#[test]
fn real_config_visible_through_production_layer_dispatch() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_config(ConfigView::from_map(
            [("infill-spacing".into(), ConfigValue::Float(5.0))].into(),
        ))
        .with_slice(ir_builders::slice_ir::with_count(1).build())
        .build();

    fx.run_layer(&GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    })
    .unwrap();

    let infill = fx.arena.infill().expect("infill slot should be populated");
    let path = &infill.regions[0].sparse_infill[0];
    let p1 = RawInfillWitnessPoint1::decode(&path.points);
    assert_eq!(p1.spacing_x10, 50.0, "spacing_x10=50.0");
}

#[test]
fn different_configs_produce_different_output() {
    let slice = ir_builders::slice_ir::with_count(1).build();

    let mut fx_a = dispatch_fixture::for_stage("Layer::Infill")
        .with_config(ConfigView::from_map(
            [("infill-spacing".into(), ConfigValue::Float(3.0))].into(),
        ))
        .with_slice(slice.clone())
        .build();

    fx_a.run_layer(&GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    })
    .unwrap();

    let mut fx_b = dispatch_fixture::for_stage("Layer::Infill")
        .with_config(ConfigView::from_map(
            [("infill-spacing".into(), ConfigValue::Float(7.0))].into(),
        ))
        .with_slice(slice)
        .build();

    fx_b.run_layer(&GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    })
    .unwrap();

    let p_a = RawInfillWitnessPoint1::decode(
        &fx_a.arena.infill().unwrap().regions[0].sparse_infill[0].points,
    );
    let p_b = RawInfillWitnessPoint1::decode(
        &fx_b.arena.infill().unwrap().regions[0].sparse_infill[0].points,
    );
    assert_ne!(p_a.spacing_x10, p_b.spacing_x10);
}

#[test]
fn repeated_identical_config_produces_deterministic_output() {
    let slice = ir_builders::slice_ir::with_count(1).build();
    let config = ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(4.0))].into());
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let mut fx = dispatch_fixture::for_stage("Layer::Infill")
            .with_config(config.clone())
            .with_slice(slice.clone())
            .build();
        fx.run_layer(&layer).unwrap();
        results.push(fx.arena.take_infill().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn config_isolation_across_sequential_calls() {
    let slice = ir_builders::slice_ir::with_count(1).build();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_config(ConfigView::from_map(
            [("infill-spacing".into(), ConfigValue::Float(2.0))].into(),
        ))
        .with_slice(slice.clone())
        .build();
    fx.run_layer(&layer).unwrap();

    let mut fx2 = dispatch_fixture::for_stage("Layer::Infill")
        .with_config(ConfigView::from_map(
            [("infill-spacing".into(), ConfigValue::Float(8.0))].into(),
        ))
        .with_slice(slice)
        .build();
    fx2.run_layer(&layer).unwrap();

    let p1 = RawInfillWitnessPoint1::decode(
        &fx.arena.infill().unwrap().regions[0].sparse_infill[0].points,
    );
    let p2 = RawInfillWitnessPoint1::decode(
        &fx2.arena.infill().unwrap().regions[0].sparse_infill[0].points,
    );
    assert_eq!(p1.spacing_x10, 20.0);
    assert_eq!(p2.spacing_x10, 80.0);
}

#[test]
fn path_optimization_emit_layer_markers_false_suppresses_output() {
    let path = Path::new(PATH_OPT_DEFAULT_PATH);
    if !path.exists() {
        eprintln!("SKIP: path-optimization-default.wasm missing");
        return;
    }
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_component_at(path);

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "path_optimization_emit_layer_markers".into(),
        ConfigValue::Bool(false),
    );
    let config = ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str()));

    let bundle = make_bundle_with_config(
        "com.test.path-opt-silent",
        "Layer::PathOptimization",
        Some(component),
        config,
    );
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let mut arena = LayerArena::new();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::PathOptimization",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let annotations = arena.take_deferred_annotations();
    assert!(annotations.is_empty(), "must be empty");
}

fn make_bundle_with_config(
    id: &str,
    stage: &str,
    component: Option<Arc<slicer_runtime::WasmComponent>>,
    config: ConfigView,
) -> crate::common::TestModuleBundle {
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
        "slicer:world-layer@1.0.0",
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
        .config_view(Arc::new(config))
        .build();

    crate::common::TestModuleBundle {
        module,
        pool,
        component,
    }
}
