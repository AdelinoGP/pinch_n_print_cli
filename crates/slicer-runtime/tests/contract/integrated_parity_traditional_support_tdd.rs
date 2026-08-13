#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};
use traditional_support::TraditionalSupport;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

fn support_slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "obj-0".to_string(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                        Point2 { x: 0, y: 10_000 },
                    ],
                },
                holes: Vec::new(),
            }],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_traditional_support() {
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "enable_support".to_string(),
            slicer_ir::ConfigValue::Bool(true),
        ),
        (
            "support_density".to_string(),
            slicer_ir::ConfigValue::Float(20.0),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
    ])));
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(support_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(support_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::Support".to_string();
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.traditional-support".into(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/traditional-support/traditional-support.wasm"),
            stage: stage.clone(),
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
            claims: vec!["support-generator".into()],
            config: Arc::clone(&config),
            native_entry: TraditionalSupport::__slicer_native_entry(),
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
        .expect("traditional support native/wasm parity");
}
