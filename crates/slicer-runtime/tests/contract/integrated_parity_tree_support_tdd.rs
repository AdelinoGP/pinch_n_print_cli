#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};
use tree_support::TreeSupport;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
    support_wedge,
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
fn integrated_parity_tree_support() {
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        ("enable_support".to_string(), ConfigValue::Bool(true)),
        ("support_density".to_string(), ConfigValue::Float(20.0)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
    ])));
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(support_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(support_slice())
        .expect("set native slice");
    // Packet 220/222 removed tree-support's missing-plan grid-MST filler: it
    // `continue`s when `support_plan_entries_for` is empty, so a bare Blackboard
    // now yields no `SupportIR` and the harness's `expect("… commit")` panicked.
    // Both sides get the identical plan so the parity comparison stays honest.
    let plan = support_wedge::single_region_support_plan(
        "tree",
        "obj-0",
        0,
        0,
        0.2,
        support_wedge::square_expolygon(10.0),
    );
    let mut wasm_bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    wasm_bb
        .commit_support_plan(Arc::clone(&plan))
        .expect("commit_support_plan must succeed");
    let mut native_bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    native_bb
        .commit_support_plan(plan)
        .expect("commit_support_plan must succeed");
    let (wasm_bb, native_bb) = (wasm_bb, native_bb);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::Support".to_string();
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.tree-support".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/tree-support/tree-support.wasm"),
            stage: "Layer::Support".into(),
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
            config,
            native_entry: TreeSupport::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let wasm = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                wasm_live,
                crate::common::layer_input(&wasm_bb, &wasm_arena),
            )
            .expect("wasm dispatch")
            .expect("wasm commit");
            let native = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                native_live,
                crate::common::layer_input(&native_bb, &native_arena),
            )
            .expect("native dispatch")
            .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("tree support native/wasm parity");
}
