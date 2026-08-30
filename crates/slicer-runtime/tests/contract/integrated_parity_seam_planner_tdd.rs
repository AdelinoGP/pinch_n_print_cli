#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use seam_planner_default::SeamPlannerDefault;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, LayerPlanIR, Point2, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId, SurfaceClassificationIR,
};
use slicer_runtime::{Blackboard, PrepassStageRunner};

use crate::common::{
    flat_plate_object, identity_transform,
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    mesh_fixture,
    parity_invariants::{assert_seam_parity_structural, ParityTolerance},
    prepass_input,
};

fn slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "obj-1".into(),
            region_id: 0,
            effective_layer_height: 0.2,
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
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_seam_planner() {
    let config = Arc::new(ConfigView::from_map(
        [("seam_position".into(), ConfigValue::String("nearest".into()))]
            .into_iter()
            .collect(),
    ));
    let mut blackboard = Blackboard::new(
        mesh_fixture(vec![flat_plate_object("obj-1", 0.0, identity_transform())]),
        0,
    );
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            global_layers: vec![slicer_ir::GlobalLayer {
                index: 0,
                z: 0.2,
                ..Default::default()
            }],
            ..Default::default()
        }))
        .expect("commit layer plan");
    blackboard
        .commit_slice_ir(Arc::new(vec![slice()]))
        .expect("commit slice");
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("commit surface classification");
    let stage = StageId::from("PrePass::SeamPlanning");
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.seam-planner-default".into(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/seam-planner-default/seam-planner-default.wasm"),
            stage: "PrePass::SeamPlanning".into(),
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
            native_entry: SeamPlannerDefault::__slicer_native_entry(),
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
    assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect("seam planner native/wasm parity");
}
