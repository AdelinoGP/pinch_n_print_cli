#![allow(missing_docs)]

use std::sync::Arc;

use path_optimization_default::PathOptimizationDefault;
use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LayerStageCommit, LoopType, MeshIR,
    PerimeterIR, PerimeterRegion, Point3WithWidth, SemVer, StageId, WallBoundaryType, WallLoop,
    WidthProfile,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants,
};


fn perimeter() -> PerimeterIR {
    let points = vec![
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ]
    .into_iter()
    .map(|(x, y)| Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    })
    .collect::<Vec<_>>();
    let region = |object_id: &str, offset: f32| PerimeterRegion {
        object_id: object_id.into(),
        region_id: 0,
        // exhaustive: parity comparison pins every field explicitly
        walls: vec![WallLoop {
            perimeter_index: 0,
            loop_type: LoopType::Outer,
            path: ExtrusionPath3D {
                points: points
                    .iter()
                    .map(|p| Point3WithWidth {
                        x: p.x + offset,
                        y: p.y,
                        ..*p
                    })
                    .collect(),
                ..extrusion_path3d_base(ExtrusionRole::OuterWall)
            },
            width_profile: WidthProfile {
                widths: points.iter().map(|point| point.width).collect(),
            },
            feature_flags: vec![Default::default(); points.len()],
            boundary_type: WallBoundaryType::ExteriorSurface,
        }],
        ..Default::default()
    };
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 1,
        regions: vec![
            region("parity-object-a", 0.0),
            region("parity-object-b", 20.0),
        ],
    }
}

#[test]
fn integrated_parity_path_optimization() {
    let wasm_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let native_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_perimeter(perimeter())
        .expect("set wasm perimeter");
    native_arena
        .set_perimeter(perimeter())
        .expect("set native perimeter");
    let wasm_input = crate::common::layer_input(&wasm_bb, &wasm_arena);
    let native_input = crate::common::layer_input(&native_bb, &native_arena);
    let layer = GlobalLayer {
        index: 1,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::PathOptimization".to_string();
    let (native, wasm) = run_integrated_parity(IntegratedParitySpec { module_id: "com.core.path-optimization-default".into(), wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules/path-optimization-default/path-optimization-default.wasm"), stage: stage.clone(), version: SemVer { major: 1, minor: 0, patch: 0 }, min_ir_schema: SemVer { major: 1, minor: 0, patch: 0 }, max_ir_schema: SemVer { major: 2, minor: 0, patch: 0 }, tier: String::new(), claims: Vec::new(), config: Arc::new(ConfigView::new()), native_entry: PathOptimizationDefault::__slicer_native_entry() }, |dispatcher, native_live, wasm_live| {
        let wasm = LayerStageRunner::run_stage(dispatcher, &stage, &layer, wasm_live, wasm_input).expect("wasm dispatch").expect("wasm commit");
        let native = LayerStageRunner::run_stage(dispatcher, &stage, &layer, native_live, native_input).expect("native dispatch").expect("native commit");
        (native, wasm)
    });
    assert!(matches!(wasm, LayerStageCommit::PathOptimization(_)));
    assert!(matches!(native, LayerStageCommit::PathOptimization(_)));
    parity_invariants::assert_parity_structural(
        &native,
        &wasm,
        parity_invariants::ParityTolerance {
            coord_mm: 1e-3,
            ..Default::default()
        },
        0.4,
    )
    .expect("native and wasm path optimization parity");
}
