#![allow(missing_docs)]

use std::sync::Arc;

use infill_linker::InfillLinker;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR, InfillRegion,
    MeshIR, PerimeterIR, PerimeterRegion, Point2, Point3WithWidth, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            polygons: vec![square()],
            infill_areas: vec![square()],
            sparse_infill_area: vec![square()],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

fn segment(start: f32, end: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: start,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: end,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn infill() -> InfillIR {
    InfillIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            sparse_infill: vec![segment(1.0, 3.0), segment(3.0, 5.0), segment(5.0, 7.0)],
            ..Default::default()
        }],
    }
}

fn perimeter() -> PerimeterIR {
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            infill_areas: vec![square()],
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_infill_linker() {
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "infill_overlap".to_string(),
            slicer_ir::ConfigValue::Float(0.45),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
        (
            "infill_density".to_string(),
            slicer_ir::ConfigValue::Float(0.2),
        ),
    ])));
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    for arena in [&mut wasm_arena, &mut native_arena] {
        arena.set_slice(slice()).expect("set slice");
        arena.set_perimeter(perimeter()).expect("set perimeter");
        arena.set_infill(infill()).expect("set infill");
    }
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::InfillPostProcess".to_string();
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.infill-linker".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/infill-linker/infill-linker.wasm"),
            stage: stage.clone(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 3,
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
            native_entry: InfillLinker::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let wasm = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                wasm_live,
                crate::common::layer_input(&blackboard, &wasm_arena),
            )
            .expect("wasm dispatch")
            .expect("wasm commit");
            let native = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                native_live,
                crate::common::layer_input(&blackboard, &native_arena),
            )
            .expect("native dispatch")
            .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("infill-linker native/wasm parity");
}
