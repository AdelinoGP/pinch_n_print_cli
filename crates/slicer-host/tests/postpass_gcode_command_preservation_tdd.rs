#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    Blackboard, CompiledModule, IrAccessMask, LoadedModule, PostpassOutput, PostpassStageRunner,
    WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExtrusionRole, GCodeCommand, GCodeIR, MeshIR, Point3,
    PrintMetadata, RetractMode, SemVer, StageId,
};

const POSTPASS_GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/postpass-guest.component.wasm"
);

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
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: "PostPass::GCodePostProcess".to_string(),
        wit_world: "slicer:world-postpass@1.0.0".to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

fn load_postpass_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    let path = PathBuf::from(POSTPASS_GUEST_COMPONENT);
    assert!(
        path.exists(),
        "postpass guest component missing at {}",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read postpass guest component");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile postpass guest component"),
    )
}

fn make_module_with_config(
    module_id: &str,
    component: Arc<slicer_host::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(module_id);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    CompiledModule {
        module_id: module_id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(config),
        claims: Vec::new(),
        wasm_component: Some(component),
        requires_modules: Vec::new(),
    }
}

fn make_gcode_ir(commands: Vec<GCodeCommand>) -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands,
        metadata: PrintMetadata {
            estimated_print_time_s: 7,
            filament_used_mm: vec![9.5],
            layer_count: 1,
            slicer_version: "test".to_string(),
        },
    }
}

#[test]
fn postpass_gcode_output_preserves_command_order_and_content() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_postpass_guest(&engine);

    let mut fields = HashMap::new();
    fields.insert(
        "postpass_mode".to_string(),
        ConfigValue::String("emit-sample".to_string()),
    );
    let module = make_module_with_config(
        "com.test.postpass-preservation",
        component,
        ConfigView::from_map(fields),
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut gcode_ir = make_gcode_ir(vec![GCodeCommand::Comment {
        text: "placeholder".to_string(),
    }]);

    let result = dispatcher.run_gcode_postprocess(
        &StageId::from("PostPass::GCodePostProcess"),
        &module,
        &blackboard,
        &mut gcode_ir,
    );

    assert!(matches!(result, Ok(PostpassOutput::GCodeSuccess)));
    assert_eq!(
        gcode_ir.commands,
        vec![
            GCodeCommand::Move {
                x: Some(10.0),
                y: Some(20.0),
                z: Some(0.3),
                e: Some(1.25),
                f: Some(1500.0),
                role: ExtrusionRole::OuterWall,
            },
            GCodeCommand::Retract {
                length: 0.8,
                speed: 35.0,
                mode: RetractMode::Gcode,
            },
            GCodeCommand::Unretract {
                length: 0.8,
                speed: 35.0,
                mode: RetractMode::Gcode,
            },
            GCodeCommand::FanSpeed { value: 200 },
            GCodeCommand::Temperature {
                tool: 1,
                celsius: 215.0,
                wait: false,
            },
            GCodeCommand::ToolChange {
                after_entity_index: 0,
                from: 1,
                to: 2
            },
            GCodeCommand::Comment {
                text: "sample comment".to_string(),
            },
            GCodeCommand::Raw {
                text: "M117 sample raw".to_string(),
            },
        ]
    );
}
