#![allow(missing_docs)]

use std::sync::Arc;

use machine_gcode_emit::MachineGcodeEmit;
use slicer_ir::{ConfigValue, ConfigView, GCodeCommand, MeshIR, SemVer, StageId};
use slicer_runtime::{Blackboard, PostpassStageInput, PostpassStageRunner};

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants,
};

#[test]
fn integrated_parity_machine_gcode_emit() {
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
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.machine-gcode-emit".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/machine-gcode-emit/machine-gcode-emit.wasm"),
            stage: stage.clone(),
            version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims: Vec::new(),
            config: Arc::clone(&config),
            native_entry: MachineGcodeEmit::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let native = PostpassStageRunner::run_gcode_postprocess(
                dispatcher,
                &stage,
                native_live,
                native_input,
                &mut native_commands,
            )
            .expect("native dispatch");
            let wasm = PostpassStageRunner::run_gcode_postprocess(
                dispatcher,
                &stage,
                wasm_live,
                wasm_input,
                &mut wasm_commands,
            )
            .expect("wasm dispatch");
            (native, wasm)
        },
    );
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
