#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, MeshIR, Point3WithWidth, SemVer,
    SliceIR, StageId, SupportPlanEntry, SupportPlanIR, CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};
use support_surface_ironing::SupportSurfaceIroning;
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

fn support_plan() -> SupportPlanIR {
    SupportPlanIR {
        schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
        entries: vec![SupportPlanEntry {
            global_layer_index: 0,
            object_id: "parity-object".to_string(),
            region_id: 0,
            branch_segments: vec![ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 1.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 9.0,
                        y: 1.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                ],
                ..extrusion_path3d_base(ExtrusionRole::SupportMaterial)
            }],
        }],
        raft_plan: None,
    }
}

#[test]
fn integrated_parity_support_surface_ironing() {
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "ironing_enabled".to_string(),
            slicer_ir::ConfigValue::Bool(true),
        ),
        (
            "ironing_speed".to_string(),
            slicer_ir::ConfigValue::Float(30.0),
        ),
        (
            "ironing_flow_rate".to_string(),
            slicer_ir::ConfigValue::Float(100.0),
        ),
        (
            "ironing_spacing".to_string(),
            slicer_ir::ConfigValue::Float(0.1),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
    ])));
    let mut bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    let slice = SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![slicer_ir::SlicedRegion {
            object_id: "parity-object".to_string(),
            region_id: 0,
            polygons: vec![slicer_ir::ExPolygon {
                contour: slicer_ir::Polygon {
                    points: vec![
                        slicer_ir::Point2 { x: 0, y: 0 },
                        slicer_ir::Point2 { x: 10_000, y: 0 },
                        slicer_ir::Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                        slicer_ir::Point2 { x: 0, y: 10_000 },
                    ],
                },
                holes: Vec::new(),
            }],
            ..Default::default()
        }],
    };
    wasm_arena.set_slice(slice.clone()).expect("set wasm slice");
    native_arena.set_slice(slice).expect("set native slice");
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::SupportPostProcess".to_string();
    bb.commit_support_plan(Arc::new(support_plan()))
        .expect("commit support plan");
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.support-surface-ironing".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../modules/core-modules/support-surface-ironing/support-surface-ironing.wasm",
            ),
            stage: "Layer::SupportPostProcess".into(),
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
            config: Arc::clone(&config),
            native_entry: SupportSurfaceIroning::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let wasm = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                wasm_live,
                crate::common::layer_input(&bb, &wasm_arena),
            )
            .expect("wasm dispatch")
            .expect("wasm commit");
            let native = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                native_live,
                crate::common::layer_input(&bb, &native_arena),
            )
            .expect("native dispatch")
            .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("support ironing native/wasm parity");
}
