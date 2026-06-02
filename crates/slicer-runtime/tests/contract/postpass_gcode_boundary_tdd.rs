#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExtrusionRole, GCodeCommand, GCodeIR, MeshIR, Point3,
    PrintMetadata, RetractMode, SemVer, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModule, CompiledModuleBuilder, LoadedModule, LoadedModuleBuilder,
    PostpassOutput, PostpassStageRunner, WasmRuntimeDispatcher,
};

use crate::common::{postpass_input, wasm_cache};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    })
}

fn make_loaded_module(id: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        "PostPass::GCodePostProcess",
        "slicer:world-postpass@1.0.0",
        PathBuf::from("/dev/null"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn make_module_with_config(
    module_id: &str,
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(module_id);
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
    CompiledModuleBuilder::new(module_id, pool)
        .config_view(Arc::new(config))
        .wasm_component(Some(component))
        .build()
}

fn make_gcode_ir(commands: Vec<GCodeCommand>) -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands,
        metadata: PrintMetadata {
            estimated_print_time_s: 42,
            filament_used_mm: vec![123.0],
            layer_count: 1,
            slicer_version: "test".to_string(),
        },
    }
}

#[test]
fn postpass_gcode_boundary_carries_all_payload_variants_into_guest() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_guest("postpass-guest");

    let mut fields = HashMap::new();
    fields.insert(
        "postpass_mode".to_string(),
        ConfigValue::String("echo".to_string()),
    );
    let module = make_module_with_config(
        "com.test.postpass-boundary",
        component,
        ConfigView::from_map(fields),
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let expected = vec![
        GCodeCommand::Move {
            x: Some(10.0),
            y: Some(20.0),
            z: Some(0.2),
            e: Some(1.1),
            f: Some(1500.0),
            role: ExtrusionRole::OuterWall,
        },
        GCodeCommand::Move {
            x: Some(11.0),
            y: Some(21.0),
            z: Some(0.25),
            e: Some(1.2),
            f: Some(1550.0),
            role: ExtrusionRole::PrimeTower,
        },
        GCodeCommand::Move {
            x: Some(11.0),
            y: Some(21.0),
            z: Some(0.25),
            e: Some(1.2),
            f: Some(1550.0),
            role: ExtrusionRole::PrimeTower,
        },
        GCodeCommand::Move {
            x: Some(12.0),
            y: Some(22.0),
            z: Some(0.3),
            e: Some(1.3),
            f: Some(1600.0),
            role: ExtrusionRole::Skirt,
        },
        GCodeCommand::Move {
            x: Some(12.0),
            y: Some(22.0),
            z: Some(0.3),
            e: Some(1.3),
            f: Some(1600.0),
            role: ExtrusionRole::Skirt,
        },
        GCodeCommand::Retract {
            length: 0.8,
            speed: 30.0,
            mode: RetractMode::Gcode,
        },
        GCodeCommand::Unretract {
            length: 0.8,
            speed: 30.0,
            mode: RetractMode::Gcode,
        },
        GCodeCommand::FanSpeed { value: 180 },
        GCodeCommand::Temperature {
            tool: 1,
            celsius: 212.5,
            wait: true,
        },
        GCodeCommand::ToolChange {
            after_entity_index: 0,
            from: 1,
            to: 2,
        },
        GCodeCommand::Comment {
            text: "boundary comment".to_string(),
        },
        GCodeCommand::Raw {
            text: "M117 boundary raw".to_string(),
        },
    ];
    let mut gcode_ir = make_gcode_ir(expected.clone());

    let result = dispatcher.run_gcode_postprocess(
        &StageId::from("PostPass::GCodePostProcess"),
        &module.as_live(),
        postpass_input(&blackboard),
        &mut gcode_ir.commands,
    );

    assert!(matches!(result, Ok(PostpassOutput::GCodeSuccess)));
    assert_eq!(gcode_ir.commands, expected);
}
