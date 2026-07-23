//! End-to-end contract coverage for the infill paint view.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, GlobalLayer, LightningTreeEntry, LightningTreeIR, Point2, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmInstancePool,
};

#[test]
fn lightning_infill_guest_calls_lightning_tree_segments() {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = slicer_wasm_host::WasmRuntimeDispatcher::new(engine);

    let object_id = "lightning-fixture";
    let region_id = 7;
    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig {
        sparse_fill_holder: "lightning-infill".to_string(),
        ..ResolvedConfig::default()
    });
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: object_id.to_string(),
            region_id,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
            ..RegionPlan::default()
        },
    );

    let slice_ir = SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: object_id.to_string(),
            region_id,
            effective_layer_height: 0.2,
            ..SlicedRegion::default()
        }],
    };
    let lightning_tree_ir = LightningTreeIR {
        entries: vec![LightningTreeEntry {
            object_id: object_id.to_string(),
            global_layer_index: 0,
            region_id,
            tree_edge_segments: vec![
                [Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }],
                [Point2 { x: 20, y: 20 }, Point2 { x: 30, y: 30 }],
            ],
        }],
        ..LightningTreeIR::default()
    };

    let module_id = "lightning-infill".to_string();
    let claims = vec!["claim:sparse-fill".to_string()];
    let config = Arc::new(ConfigView::from_map(HashMap::from([(
        "infill-spacing".to_string(),
        ConfigValue::Float(2.0),
    )])));
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_guest(
            "layer-infill-guest",
        )),
        &claims,
        config,
    );

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..GlobalLayer::default()
    };
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: Some(Arc::new(lightning_tree_ir)),
        region_map: Some(Arc::new(region_map)),
        slice: Some(&slice_ir),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        infill: None,
    };

    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        input,
    )
    .expect("lightning infill dispatch must succeed")
    .expect("guest must emit infill output");

    let sparse_paths = match commit {
        slicer_ir::LayerStageCommit::Infill(ir) => ir
            .regions
            .into_iter()
            .flat_map(|region| region.sparse_infill)
            .collect::<Vec<_>>(),
        other => panic!("expected Layer::Infill commit, got {other:?}"),
    };
    let witness = sparse_paths
        .iter()
        .find(|path| path.points.len() == 1 && (path.points[0].width - 137.0).abs() < f32::EPSILON)
        .expect("guest must emit a lightning-tree witness path");
    assert_eq!(witness.points[0].x, 2.0);
}
