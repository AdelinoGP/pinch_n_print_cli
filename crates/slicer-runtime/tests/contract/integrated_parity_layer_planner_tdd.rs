#![allow(missing_docs)]

use std::sync::Arc;

use layer_planner_default::DefaultLayerPlanner;
use slicer_core::PrepassStageOutput;
use slicer_ir::{ConfigValue, ConfigView, SemVer, StageId};
use slicer_runtime::{Blackboard, PrepassStageRunner};

use crate::common::{
    flat_plate_object, identity_transform,
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    mesh_fixture,
    parity_invariants::{assert_layer_plan_parity_structural, ParityTolerance},
    prepass_input,
};

#[test]
fn integrated_parity_layer_planner() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("layer_height".into(), ConfigValue::Float(0.2)),
            ("first_layer_height".into(), ConfigValue::Float(0.2)),
            ("object_height:obj-1".into(), ConfigValue::Float(2.0)),
        ]
        .into_iter()
        .collect(),
    ));

    let blackboard = Blackboard::new(
        mesh_fixture(vec![flat_plate_object("obj-1", 0.0, identity_transform())]),
        0,
    );
    let stage = StageId::from("PrePass::LayerPlanning");
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.layer-planner-default".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../modules/core-modules/layer-planner-default/layer-planner-default.wasm",
            ),
            stage: stage.to_string(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 5,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims: Vec::new(),
            config,
            native_entry: DefaultLayerPlanner::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let native = PrepassStageRunner::run_stage(
                dispatcher,
                &stage,
                native_live,
                prepass_input(&blackboard),
            )
            .expect("native dispatch");
            let wasm = PrepassStageRunner::run_stage(
                dispatcher,
                &stage,
                wasm_live,
                prepass_input(&blackboard),
            )
            .expect("wasm dispatch");
            (native, wasm)
        },
    );
    assert!(matches!(native, PrepassStageOutput::LayerPlan(_)));
    assert!(matches!(wasm, PrepassStageOutput::LayerPlan(_)));
    assert_layer_plan_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect("layer planner native/wasm parity");
}
