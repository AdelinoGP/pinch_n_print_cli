#![allow(missing_docs)]

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    LayerStageCommit, OrderedEventCollection, Point3,
};
use slicer_sdk::{layer_collection_builder::LayerCollectionBuilder, native::NativeLayerResponse};
use slicer_wasm_host::dispatch::deconstruct_layer_ctx;
use slicer_wasm_host::host::HostExecutionContextBuilder;
use slicer_wasm_host::marshal::native::commit_native_layer_response;

fn collection(geometry: AnchoredGeometryContract, z: f32) -> OrderedEventCollection {
    OrderedEventCollection {
        anchor_global_layer_index: 7,
        // exhaustive: distinctive anchored-event contract fixture
        events: vec![AnchoredEntity {
            local_id: 19,
            anchor_global_layer_index: 7,
            geometry,
            input_capabilities: vec!["mesh".into()],
            output_capabilities: vec!["cooling".into()],
            provenance: AnchoredEntityProvenance {
                requesting_feature: "test-feature".into(),
                source_plan_entry: "plan-7".into(),
            },
            path_points: vec![Point3 { x: 1.0, y: 2.0, z }],
        }],
        runtime_hooks: AnchoredEventRuntimeHooks {
            optimize_paths: false,
            account_cooling: true,
            account_time: false,
        },
    }
}

fn native_commit(input: OrderedEventCollection) -> LayerStageCommit {
    let mut builder = LayerCollectionBuilder::new();
    builder
        .set_anchored_event_collection(input)
        .expect("set anchored collection");
    // exhaustive: test-only response fixture names every stage slot explicitly
    let response = NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: Some(builder),
    };
    commit_native_layer_response(&response, "Layer::AnchoredEvents", 7, None)
        .expect("native anchored-events commit")
        .expect("anchored-events output")
}

#[test]
fn anchored_events_native_and_wasm_legs_agree() {
    let input = collection(AnchoredGeometryContract::Planar { z: 3000 }, 0.3);
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input.clone());
    let wasm = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .expect("wasm anchored-events commit")
    .expect("wasm anchored-events output");
    assert_eq!(wasm, native_commit(input));
}

#[test]
fn anchored_events_planar_geometry_is_rejected() {
    let input = collection(AnchoredGeometryContract::Planar { z: 3000 }, 0.9);
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input);
    let error = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, slicer_ir::LayerStageError::FatalModule { ref message, .. } if message.contains("anchored entity planar z mismatch"))
    );
}

#[test]
fn anchored_events_z_spanning_geometry_is_rejected() {
    let input = collection(
        AnchoredGeometryContract::ZSpanning {
            min_z: 3000,
            max_z: 5000,
        },
        0.9,
    );
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input);
    let error = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, slicer_ir::LayerStageError::FatalModule { ref message, .. } if message.contains("anchored entity z-span violation"))
    );
}
