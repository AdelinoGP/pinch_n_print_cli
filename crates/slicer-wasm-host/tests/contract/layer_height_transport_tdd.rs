//! Regression: native and WASM layer-plan views use the same maximum height.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, GlobalLayer, LayerPlanIR, ObjectLayerRef};
use slicer_wasm_host::binding::{CompiledModuleLive, PrepassStageInput};
use slicer_wasm_host::marshal::{native::build_native_prepass_request, project_layer_plan_view};
use slicer_wasm_host::WasmInstancePool;

#[test]
fn native_and_wasm_layer_views_share_canonical_layer_height() {
    let mut object_participation = HashMap::new();
    for (object_id, height) in [("object-a", 0.1), ("object-b", 0.4), ("object-c", 0.2)] {
        object_participation.insert(
            object_id.to_owned(),
            vec![ObjectLayerRef {
                global_layer_index: 7,
                effective_layer_height: height,
                ..Default::default()
            }],
        );
    }
    let plan = Arc::new(LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 7,
            z: 1.4,
            ..Default::default()
        }],
        object_participation,
        ..Default::default()
    });

    let wasm = project_layer_plan_view(&plan);
    let config = Arc::new(ConfigView::from_map(HashMap::new()));
    let module_id = "layer-height-transport".to_owned();
    let claims = Vec::<String>::new();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        config,
    );
    // This guard is probabilistic against a reverted first-match implementation
    // because HashMap iteration order varies; the deterministic guard is the
    // single-derivation-site verification via `canonical_effective_layer_height`.
    // exhaustive: no Default + lifetime param make FRU impossible; fixture mirrors binding constructor requirements
    let input = PrepassStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        layer_plan: Some(plan),
        slice_ir: None,
        region_map: None,
        support_analysis: None,
        support_geometry: None,
        _phantom: std::marker::PhantomData,
    };
    let native = build_native_prepass_request("PrePass::LayerPlan", &input, &module);

    assert_eq!(wasm.layers[0].effective_layer_height, 0.4);
    assert_eq!(
        native.layer_plan.unwrap().layers[0].effective_layer_height,
        0.4
    );
}
