//! Regression: layer dispatch must preserve support-plan identity metadata.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan,
    SemVer, SliceIR, SlicedRegion, SupportPlanEntry, SupportPlanIR,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmInstancePool,
};

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 100, y: 100 },
                Point2 { x: 0, y: 100 },
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn support_layer_dispatch_joins_plan_identity() {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = slicer_wasm_host::WasmRuntimeDispatcher::new(engine);
    let object_id = "support-dispatch-fixture".to_string();
    let region_id = 7;

    let mut region_map = RegionMapIR::default();
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: object_id.clone(),
            region_id,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    let slice = SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id,
            polygons: vec![square()],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    };
    let support_plan = SupportPlanIR {
        schema_version: slicer_ir::CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
        entries: vec![
            SupportPlanEntry {
                global_layer_index: 0,
                object_id: object_id.clone(),
                region_id,
                family_id: "tree-family".into(),
                demand_ids: vec!["demand-7a".into(), "demand-7b".into()],
                body_ids: vec!["body-7a".into(), "body-7b".into()],
                anchor_layer_index: 0,
                anchor_z: 200,
                roles: Vec::new(),
                skeleton: None,
                capabilities: Vec::new(),
                provenance: Vec::new(),
                decline_reason: None,
            },
            SupportPlanEntry {
                global_layer_index: 0,
                object_id: object_id.clone(),
                region_id,
                family_id: "traditional-family".into(),
                demand_ids: vec!["demand-7c".into(), "demand-7d".into()],
                body_ids: vec!["body-7c".into()],
                anchor_layer_index: 0,
                anchor_z: 200,
                roles: Vec::new(),
                skeleton: None,
                capabilities: Vec::new(),
                provenance: Vec::new(),
                decline_reason: None,
            },
        ],
        raft_plan: None,
    };
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: Some(Arc::new(support_plan)),
        lightning_tree_ir: None,
        region_map: Some(Arc::new(region_map)),
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        infill: None,
    };
    let module_id = "support-dispatch-fixture".to_string();
    let claims = Vec::<String>::new();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_guest(
            "dispatch-layer-support-postprocess-guest",
        )),
        &claims,
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };

    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &module,
        input,
    )
    .expect("support layer dispatch must succeed")
    .expect("support guest must emit output");

    let slicer_ir::LayerStageCommit::SupportPostProcess(ir) = commit else {
        panic!("expected support commit, got {commit:?}");
    };
    assert_eq!(
        ir.entries
            .iter()
            .map(|entry| {
                (
                    entry.family_id.clone(),
                    entry.body_id.clone(),
                    entry.demand_ids.clone(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "tree-family".to_string(),
                "body-7a".to_string(),
                vec!["demand-7a".to_string(), "demand-7b".to_string()],
            ),
            (
                "tree-family".to_string(),
                "body-7b".to_string(),
                vec!["demand-7a".to_string(), "demand-7b".to_string()],
            ),
            (
                "traditional-family".to_string(),
                "body-7c".to_string(),
                vec!["demand-7c".to_string(), "demand-7d".to_string()],
            ),
        ]
    );
}
