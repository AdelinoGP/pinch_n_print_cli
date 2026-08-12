#![allow(missing_docs)]

use std::sync::Arc;

use machine_gcode_emit::MachineGcodeEmit;
use slicer_ir::{ConfigValue, ConfigView, GCodeCommand, MeshIR, SemVer, StageId};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LoadedModuleBuilder, PostpassStageInput,
    PostpassStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{parity_invariants, wasm_cache};

fn wasm_live<'a>(
    module: &'a slicer_runtime::CompiledModule,
) -> (CompiledModuleLive<'a>, Arc<slicer_runtime::WasmComponent>) {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "PostPass::GCodePostProcess",
        String::new(),
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
        .expect("build instance pool"),
    );
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/machine-gcode-emit/machine-gcode-emit.wasm");
    let component = wasm_cache::compiled_component_at(&path);
    (
        CompiledModuleLive::new(
            module.module_id(),
            pool,
            Some(Arc::clone(&component)),
            module.claims(),
            Arc::clone(module.config_view()),
        ),
        component,
    )
}

#[test]
fn integrated_parity_machine_gcode_emit() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let id = "com.core.machine-gcode-emit".to_string();
    let config = Arc::new(ConfigView::from_map(
        [
            (
                "machine_start_gcode".into(),
                ConfigValue::String("START".into()),
            ),
            (
                "machine_end_gcode".into(),
                ConfigValue::String("END".into()),
            ),
        ]
        .into_iter()
        .collect(),
    ));
    let wasm_module = CompiledModuleBuilder::new(id.clone())
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new(id).config_view(config).build();
    let (wasm_live, _component) = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(MachineGcodeEmit::__slicer_native_entry());
    let wasm_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let native_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let wasm_input = PostpassStageInput {
        mesh: wasm_bb.mesh().clone(),
        _phantom: std::marker::PhantomData,
    };
    let native_input = PostpassStageInput {
        mesh: native_bb.mesh().clone(),
        _phantom: std::marker::PhantomData,
    };
    let stage: StageId = "PostPass::GCodePostProcess".to_string();
    let mut wasm_commands: Vec<GCodeCommand> = vec![GCodeCommand::FanSpeed { value: 255 }];
    let mut native_commands: Vec<GCodeCommand> = vec![GCodeCommand::FanSpeed { value: 255 }];
    let wasm = PostpassStageRunner::run_gcode_postprocess(
        &dispatcher,
        &stage,
        &wasm_live,
        wasm_input,
        &mut wasm_commands,
    )
    .expect("wasm dispatch");
    let native = PostpassStageRunner::run_gcode_postprocess(
        &dispatcher,
        &stage,
        &native_live,
        native_input,
        &mut native_commands,
    )
    .expect("native dispatch");
    assert!(matches!(wasm, slicer_ir::PostpassOutput::GCodeSuccess));
    assert!(matches!(native, slicer_ir::PostpassOutput::GCodeSuccess));
    assert!(
        !wasm_commands.is_empty() && !native_commands.is_empty(),
        "the gcode transport must be exercised (nonempty output)"
    );
    parity_invariants::assert_gcode_sequence_parity_structural(
        &native_commands,
        &wasm_commands,
        parity_invariants::ParityTolerance {
            coord_mm: 1e-3,
            ..Default::default()
        },
    )
    .expect("native and wasm gcode parity");
}
